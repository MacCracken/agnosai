//! Native Rust tool trait and in-process execution.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Description of a single tool parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    pub name: String,
    pub description: String,
    /// JSON-schema type string (e.g. `"string"`, `"number"`, `"array"`).
    pub param_type: String,
    pub required: bool,
}

/// Schema describing a tool's name, purpose, and accepted parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterSchema>,
}

/// Input passed to a tool's `execute` method.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct ToolOutput {
    pub success: bool,
    pub result: Value,
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
