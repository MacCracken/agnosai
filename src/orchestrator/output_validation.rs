//! Structured output validation for LLM responses.
//!
//! When a task specifies an `output_schema` (a JSON Schema value), this module
//! validates the LLM's response against it.  On failure, it builds a retry
//! prompt containing the validation error so the LLM can self-correct.
//!
//! Validation is lightweight: it checks that the response is valid JSON and
//! that required top-level keys are present.  Full JSON Schema draft validation
//! would require a heavy dependency — this covers the most common case (the
//! LLM returns a JSON object with specific fields) without adding deps.

use serde_json::Value;
use std::fmt::Write;
use tracing::{debug, warn};

/// Maximum number of retry attempts when output validation fails.
pub const MAX_VALIDATION_RETRIES: usize = 2;

/// Result of validating an LLM response against an output schema.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidationResult {
    /// The output conforms to the schema.
    Valid,
    /// The output does not conform.  The string describes the mismatch.
    Invalid(String),
}

/// Validate an LLM response against a task's output schema.
///
/// The schema is expected to be a JSON object with a `"properties"` field
/// listing expected keys, and optionally a `"required"` array.
///
/// Returns [`ValidationResult::Valid`] if:
/// - The output is valid JSON.
/// - All `"required"` keys from the schema are present.
/// - The `"type"` field (if present) matches the top-level JSON type.
///
/// Returns [`ValidationResult::Invalid`] with a human-readable error otherwise.
#[must_use]
pub fn validate_output(output: &str, schema: &Value) -> ValidationResult {
    // Step 1: Parse as JSON.
    let parsed: Value = match serde_json::from_str(output) {
        Ok(v) => v,
        Err(e) => {
            warn!("output validation: response is not valid JSON: {e}");
            return ValidationResult::Invalid(format!("Response is not valid JSON: {e}"));
        }
    };

    // Step 2: Check top-level type if specified.
    if let Some(expected_type) = schema.get("type").and_then(|v| v.as_str()) {
        let actual_type = json_type_name(&parsed);
        if actual_type != expected_type {
            warn!(
                expected_type,
                actual_type, "output validation: type mismatch"
            );
            return ValidationResult::Invalid(format!(
                "Expected top-level type \"{expected_type}\", got \"{actual_type}\""
            ));
        }
    }

    // Step 3: Check required keys.
    if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
        if let Some(obj) = parsed.as_object() {
            let missing: Vec<&str> = required
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|key| !obj.contains_key(*key))
                .collect();
            if !missing.is_empty() {
                warn!(fields = %missing.join(", "), "output validation: missing required fields");
                return ValidationResult::Invalid(format!(
                    "Missing required fields: {}",
                    missing.join(", ")
                ));
            }
        } else if !required.is_empty() {
            return ValidationResult::Invalid(
                "Schema requires fields but response is not a JSON object".into(),
            );
        }
    }

    ValidationResult::Valid
}

/// Build a retry prompt that includes the failed output and validation error,
/// instructing the LLM to produce a corrected response.
#[must_use]
pub fn build_retry_prompt(
    original_prompt: &str,
    failed_output: &str,
    validation_error: &str,
    schema: &Value,
) -> String {
    let mut prompt = String::with_capacity(
        original_prompt.len() + failed_output.len() + validation_error.len() + 512,
    );

    let _ = write!(
        prompt,
        "{original_prompt}\n\n\
         Your previous response failed validation.\n\
         Error: {validation_error}\n\
         Your previous response was:\n```\n{failed_output}\n```\n\n\
         Please respond with ONLY valid JSON matching this schema:\n```json\n{schema}\n```"
    );

    prompt
}

/// Attempt to validate and optionally extract JSON from a response that may
/// contain markdown fences or prose around the JSON.
///
/// Tries the raw output first, then looks for ```json ... ``` blocks.
#[must_use]
pub fn extract_and_validate(output: &str, schema: &Value) -> (String, ValidationResult) {
    // Try raw output first.
    let result = validate_output(output, schema);
    if result == ValidationResult::Valid {
        debug!("output validated successfully (raw)");
        return (output.to_string(), result);
    }

    // Try extracting from markdown fence.
    if let Some(json_str) = extract_json_block(output) {
        let fenced_result = validate_output(json_str, schema);
        if fenced_result == ValidationResult::Valid {
            debug!("output validated successfully (extracted from fence)");
            return (json_str.to_string(), fenced_result);
        }
        // Return the fenced attempt's error since it's more likely what was intended.
        return (json_str.to_string(), fenced_result);
    }

    (output.to_string(), result)
}

/// Extract the first ```json ... ``` or ``` ... ``` block from text.
///
/// Uses `\n```" as the closing delimiter (newline before backticks) to avoid
/// false matches on literal triple backticks inside JSON string values.
fn extract_json_block(text: &str) -> Option<&str> {
    let start_markers = ["```json\n", "```json\r\n", "```\n", "```\r\n"];
    for marker in start_markers {
        if let Some(start) = text.find(marker) {
            let content_start = start + marker.len();
            let remainder = &text[content_start..];
            // Look for closing fence on its own line.
            let end = remainder
                .find("\n```")
                .or_else(|| remainder.find("\r\n```"))?;
            let block = remainder[..end].trim();
            if !block.is_empty() {
                return Some(block);
            }
        }
    }
    None
}

/// Map a serde_json Value to its JSON type name.
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Log a validation retry attempt.
pub fn log_retry(task_id: &str, attempt: usize, error: &str) {
    warn!(
        task_id,
        attempt,
        max = MAX_VALIDATION_RETRIES,
        error,
        "output validation failed, retrying"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn valid_json_object_with_required_fields() {
        let schema = json!({
            "type": "object",
            "required": ["name", "score"],
        });
        let output = r#"{"name": "test", "score": 42}"#;
        assert_eq!(validate_output(output, &schema), ValidationResult::Valid);
    }

    #[test]
    fn missing_required_field() {
        let schema = json!({
            "type": "object",
            "required": ["name", "score"],
        });
        let output = r#"{"name": "test"}"#;
        assert!(matches!(
            validate_output(output, &schema),
            ValidationResult::Invalid(ref s) if s.contains("score")
        ));
    }

    #[test]
    fn not_valid_json() {
        let schema = json!({"type": "object"});
        assert!(matches!(
            validate_output("this is not json", &schema),
            ValidationResult::Invalid(ref s) if s.contains("not valid JSON")
        ));
    }

    #[test]
    fn wrong_type() {
        let schema = json!({"type": "object"});
        let output = r#"[1, 2, 3]"#;
        assert!(matches!(
            validate_output(output, &schema),
            ValidationResult::Invalid(ref s) if s.contains("array")
        ));
    }

    #[test]
    fn array_type_valid() {
        let schema = json!({"type": "array"});
        let output = r#"[1, 2, 3]"#;
        assert_eq!(validate_output(output, &schema), ValidationResult::Valid);
    }

    #[test]
    fn no_schema_constraints_passes_any_json() {
        let schema = json!({});
        assert_eq!(
            validate_output(r#"{"anything": true}"#, &schema),
            ValidationResult::Valid
        );
        assert_eq!(validate_output("42", &schema), ValidationResult::Valid);
        assert_eq!(
            validate_output(r#""hello""#, &schema),
            ValidationResult::Valid
        );
    }

    #[test]
    fn extract_json_from_markdown_fence() {
        let text = "Here is the result:\n```json\n{\"key\": \"value\"}\n```\nDone.";
        let schema = json!({"type": "object", "required": ["key"]});
        let (extracted, result) = extract_and_validate(text, &schema);
        assert_eq!(result, ValidationResult::Valid);
        assert_eq!(extracted, r#"{"key": "value"}"#);
    }

    #[test]
    fn extract_json_from_plain_fence() {
        let text = "Result:\n```\n{\"a\": 1}\n```";
        let schema = json!({"type": "object"});
        let (_, result) = extract_and_validate(text, &schema);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn no_fence_falls_back_to_raw() {
        let text = r#"{"valid": true}"#;
        let schema = json!({"type": "object"});
        let (_, result) = extract_and_validate(text, &schema);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn build_retry_prompt_includes_error() {
        let prompt = build_retry_prompt(
            "Summarize this",
            "not json",
            "not valid JSON",
            &json!({"type": "object"}),
        );
        assert!(prompt.contains("Summarize this"));
        assert!(prompt.contains("not valid JSON"));
        assert!(prompt.contains("not json"));
        assert!(prompt.contains("schema"));
    }

    #[test]
    fn fence_with_backticks_in_json_value() {
        // Triple backticks inside a JSON string should not break extraction.
        let text = "Result:\n```json\n{\"code\": \"use ```markdown``` here\"}\n```\nDone.";
        let schema = json!({"type": "object", "required": ["code"]});
        let (extracted, result) = extract_and_validate(text, &schema);
        assert_eq!(result, ValidationResult::Valid);
        assert!(extracted.contains("markdown"));
    }

    #[test]
    fn required_on_non_object_fails() {
        let schema = json!({"type": "string", "required": ["field"]});
        assert!(matches!(
            validate_output(r#""just a string""#, &schema),
            ValidationResult::Invalid(ref s) if s.contains("not a JSON object")
        ));
    }
}
