//! AgnosAI Tool SDK — build WASM tools for the AgnosAI platform.
//!
//! This crate provides the types and helpers needed to write community tools
//! that run inside AgnosAI's WASM sandbox. Tools compiled to
//! `wasm32-wasip1` are executed with memory isolation, CPU limits, and no
//! filesystem or network access.
//!
//! # Protocol
//!
//! A WASM tool is a WASI binary that:
//! 1. Reads JSON from **stdin**: `{"parameters": {"key": "value", ...}}`
//! 2. Writes JSON to **stdout**: `{"result": ..., "success": true}` or
//!    `{"error": "...", "success": false}`
//!
//! # Quick start
//!
//! ```rust,no_run
//! use agnosai_tool_sdk::{run_tool, ToolInput, ToolResult};
//!
//! fn execute(input: ToolInput) -> ToolResult {
//!     let name = input.get_str("name").unwrap_or("world");
//!     ToolResult::ok(serde_json::json!({"greeting": format!("Hello, {name}!")}))
//! }
//!
//! fn main() {
//!     run_tool(execute);
//! }
//! ```
//!
//! # Building
//!
//! ```bash
//! rustup target add wasm32-wasip1
//! cargo build --target wasm32-wasip1 --release
//! ```
//!
//! The resulting `.wasm` file can be registered in AgnosAI via the tool
//! registry API or loaded from disk.
//!
//! # Manifest
//!
//! Tools are distributed as a `.wasm` file alongside a `manifest.json`:
//!
//! ```json
//! {
//!   "name": "hello",
//!   "description": "A greeting tool",
//!   "version": "1.0.0",
//!   "parameters": [
//!     {"name": "name", "description": "Who to greet", "param_type": "string", "required": false}
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Read, Write};

/// Input received by the tool. Mirrors AgnosAI's `ToolInput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    /// Parameter values keyed by name.
    pub parameters: HashMap<String, Value>,
}

impl ToolInput {
    /// Get a string parameter.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.parameters.get(key).and_then(|v| v.as_str())
    }

    /// Get a number parameter.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.parameters.get(key).and_then(|v| v.as_f64())
    }

    /// Get a u64 parameter.
    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.parameters.get(key).and_then(|v| v.as_u64())
    }

    /// Get a boolean parameter.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.parameters.get(key).and_then(|v| v.as_bool())
    }

    /// Get a raw JSON value parameter.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.parameters.get(key)
    }
}

/// Result returned by the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool succeeded.
    pub success: bool,
    /// Result value (on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error message (on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    /// Create a successful result.
    pub fn ok(value: Value) -> Self {
        Self {
            success: true,
            result: Some(value),
            error: None,
        }
    }

    /// Create a failed result.
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(msg.into()),
        }
    }
}

/// Parameter schema for the tool manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    /// Parameter name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON schema type (`"string"`, `"number"`, `"boolean"`, `"object"`, `"array"`).
    pub param_type: String,
    /// Whether the parameter is required.
    pub required: bool,
}

/// Tool manifest — metadata about the tool for registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Unique tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Semantic version.
    pub version: String,
    /// Parameter definitions.
    pub parameters: Vec<ParameterSchema>,
}

/// Read input from stdin, call the handler, and write the result to stdout.
///
/// This is the main entry point for WASM tools. Call it from `fn main()`.
///
/// ```rust,no_run
/// use agnosai_tool_sdk::{run_tool, ToolInput, ToolResult};
///
/// fn my_tool(input: ToolInput) -> ToolResult {
///     ToolResult::ok(serde_json::json!({"message": "done"}))
/// }
///
/// fn main() {
///     run_tool(my_tool);
/// }
/// ```
pub fn run_tool(handler: fn(ToolInput) -> ToolResult) {
    let mut buf = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buf) {
        let result = ToolResult::err(format!("failed to read stdin: {e}"));
        let _ = write_result(&result);
        return;
    }

    let input = if buf.trim().is_empty() {
        ToolInput {
            parameters: HashMap::new(),
        }
    } else {
        match serde_json::from_str::<ToolInput>(&buf) {
            Ok(input) => input,
            Err(e) => {
                let result = ToolResult::err(format!("invalid input JSON: {e}"));
                let _ = write_result(&result);
                return;
            }
        }
    };

    let result = handler(input);
    let _ = write_result(&result);
}

fn write_result(result: &ToolResult) -> io::Result<()> {
    let json = serde_json::to_string(result).unwrap_or_else(|_| {
        r#"{"success":false,"error":"failed to serialize result"}"#.to_string()
    });
    io::stdout().write_all(json.as_bytes())?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_ok_serializes() {
        let r = ToolResult::ok(serde_json::json!(42));
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"result\":42"));
        assert!(!json.contains("error"));
    }

    #[test]
    fn tool_result_err_serializes() {
        let r = ToolResult::err("boom");
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"boom\""));
        assert!(!json.contains("result"));
    }

    #[test]
    fn tool_input_get_helpers() {
        let input = ToolInput {
            parameters: HashMap::from([
                ("name".into(), serde_json::json!("alice")),
                ("count".into(), serde_json::json!(42)),
                ("ratio".into(), serde_json::json!(3.14)),
                ("flag".into(), serde_json::json!(true)),
            ]),
        };
        assert_eq!(input.get_str("name"), Some("alice"));
        assert_eq!(input.get_u64("count"), Some(42));
        assert_eq!(input.get_f64("ratio"), Some(3.14));
        assert_eq!(input.get_bool("flag"), Some(true));
        assert_eq!(input.get_str("missing"), None);
    }

    #[test]
    fn empty_input_parses() {
        let input: ToolInput = serde_json::from_str(r#"{"parameters":{}}"#).unwrap();
        assert!(input.parameters.is_empty());
    }

    #[test]
    fn manifest_round_trip() {
        let manifest = ToolManifest {
            name: "test".into(),
            description: "A test tool".into(),
            version: "1.0.0".into(),
            parameters: vec![ParameterSchema {
                name: "input".into(),
                description: "The input".into(),
                param_type: "string".into(),
                required: true,
            }],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let restored: ToolManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.parameters.len(), 1);
    }
}
