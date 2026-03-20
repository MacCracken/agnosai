//! Echo tool — returns the input verbatim. Useful for testing.

use crate::tools::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use std::future::Future;
use std::pin::Pin;

/// A trivial tool that echoes its `message` parameter back.
pub struct EchoTool;

impl NativeTool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Returns the input message unchanged. Useful for testing."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![ParameterSchema {
                name: "message".to_owned(),
                description: "The message to echo back".to_owned(),
                param_type: "string".to_owned(),
                required: true,
            }],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            match input.parameters.get("message") {
                Some(value) => ToolOutput::ok(value.clone()),
                None => ToolOutput::err("missing required parameter: message"),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn echo_returns_message() {
        let tool = EchoTool;
        let mut parameters = HashMap::new();
        parameters.insert(
            "message".to_owned(),
            serde_json::Value::String("hello".to_owned()),
        );
        let output = tool.execute(ToolInput { parameters }).await;
        assert!(output.success);
        assert_eq!(output.result, serde_json::Value::String("hello".to_owned()));
        assert!(output.error.is_none());
    }

    #[tokio::test]
    async fn echo_missing_param() {
        let tool = EchoTool;
        let output = tool
            .execute(ToolInput {
                parameters: HashMap::new(),
            })
            .await;
        assert!(!output.success);
        assert!(output.error.is_some());
    }

    #[test]
    fn echo_schema() {
        let tool = EchoTool;
        let schema = tool.schema();
        assert_eq!(schema.name, "echo");
        assert_eq!(schema.parameters.len(), 1);
        assert!(schema.parameters[0].required);
    }
}
