//! Native Rust tool trait and in-process execution.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Description of a single tool parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ParameterSchema {
    /// Parameter name.
    pub name: String,
    /// Human-readable description of the parameter.
    pub description: String,
    /// JSON-schema type string (e.g. `"string"`, `"number"`, `"array"`).
    pub param_type: String,
    /// Whether this parameter must be provided.
    pub required: bool,
}

/// Schema describing a tool's name, purpose, and accepted parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolSchema {
    /// Unique tool name.
    pub name: String,
    /// Human-readable tool description.
    pub description: String,
    /// Parameter definitions accepted by this tool.
    pub parameters: Vec<ParameterSchema>,
}

/// Input passed to a tool's `execute` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolInput {
    /// Parameter values keyed by name.
    pub parameters: HashMap<String, Value>,
}

impl ToolInput {
    /// Get a required string parameter.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.parameters.get(key).and_then(|v| v.as_str())
    }

    /// Get an optional number parameter.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.parameters.get(key).and_then(|v| v.as_f64())
    }

    /// Get an optional u64 parameter.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.parameters.get(key).and_then(|v| v.as_u64())
    }
}

/// Output returned from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ToolOutput {
    /// Whether execution succeeded.
    pub success: bool,
    /// Result value (meaningful when `success` is true).
    pub result: Value,
    /// Error message (present when `success` is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolOutput {
    /// Create a successful output.
    pub fn ok(result: Value) -> Self {
        Self {
            success: true,
            result,
            error: None,
        }
    }

    /// Create a failed output.
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            result: Value::Null,
            error: Some(msg.into()),
        }
    }
}

/// Core trait for all native Rust tools.
///
/// Implementations must be `Send + Sync` so they can live inside the
/// thread-safe [`crate::registry::ToolRegistry`].
///
/// The `execute` method returns a pinned, boxed future so the trait is
/// object-safe and can be used as `dyn NativeTool`.
pub trait NativeTool: Send + Sync {
    /// Unique tool name (e.g. `"synapse_infer"`).
    fn name(&self) -> &str;

    /// Human-readable description of what the tool does.
    fn description(&self) -> &str;

    /// Structured schema describing the tool's parameters.
    fn schema(&self) -> ToolSchema;

    /// Execute the tool with the given input.
    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_output_ok_has_correct_fields() {
        let output = ToolOutput::ok(serde_json::json!(42));
        assert!(output.success);
        assert_eq!(output.result, serde_json::json!(42));
        assert!(output.error.is_none());
    }

    #[test]
    fn tool_output_err_has_correct_fields() {
        let output = ToolOutput::err("something broke");
        assert!(!output.success);
        assert_eq!(output.result, serde_json::Value::Null);
        assert_eq!(output.error.as_deref(), Some("something broke"));
    }

    #[test]
    fn tool_input_get_str_returns_none_for_missing() {
        let input = ToolInput {
            parameters: HashMap::new(),
        };
        assert!(input.get_str("missing").is_none());
    }

    #[test]
    fn tool_input_get_str_returns_none_for_wrong_type() {
        let input = ToolInput {
            parameters: HashMap::from([("count".into(), serde_json::json!(42))]),
        };
        assert!(input.get_str("count").is_none());
    }

    #[test]
    fn tool_input_get_str_returns_value() {
        let input = ToolInput {
            parameters: HashMap::from([("name".into(), serde_json::json!("alice"))]),
        };
        assert_eq!(input.get_str("name"), Some("alice"));
    }

    #[test]
    fn tool_input_get_f64_returns_value() {
        let input = ToolInput {
            parameters: HashMap::from([("ratio".into(), serde_json::json!(3.14))]),
        };
        assert_eq!(input.get_f64("ratio"), Some(3.14));
    }

    #[test]
    fn tool_input_get_u64_returns_value() {
        let input = ToolInput {
            parameters: HashMap::from([("count".into(), serde_json::json!(42))]),
        };
        assert_eq!(input.get_u64("count"), Some(42));
    }

    #[test]
    fn tool_input_get_u64_returns_none_for_negative() {
        let input = ToolInput {
            parameters: HashMap::from([("count".into(), serde_json::json!(-1))]),
        };
        assert!(input.get_u64("count").is_none());
    }

    #[test]
    fn tool_output_ok_serde_round_trip() {
        let output = ToolOutput::ok(serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&output).unwrap();
        let restored: ToolOutput = serde_json::from_str(&json).unwrap();
        assert!(restored.success);
        assert_eq!(restored.result["key"], "value");
    }

    #[test]
    fn tool_output_err_skips_none_error_in_ok() {
        let output = ToolOutput::ok(serde_json::json!(1));
        let json = serde_json::to_string(&output).unwrap();
        assert!(!json.contains("error"));
    }

    #[test]
    fn tool_schema_serde_round_trip() {
        let schema = ToolSchema {
            name: "test".into(),
            description: "desc".into(),
            parameters: vec![ParameterSchema {
                name: "p1".into(),
                description: "param 1".into(),
                param_type: "string".into(),
                required: true,
            }],
        };
        let json = serde_json::to_string(&schema).unwrap();
        let restored: ToolSchema = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.parameters.len(), 1);
        assert!(restored.parameters[0].required);
    }
}
