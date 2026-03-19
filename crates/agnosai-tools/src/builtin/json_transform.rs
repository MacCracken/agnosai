//! JSON transform tool — extracts a field from a JSON value.

use crate::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
use std::future::Future;
use std::pin::Pin;

/// Extracts a top-level field from a JSON object.
pub struct JsonTransformTool;

impl NativeTool for JsonTransformTool {
    fn name(&self) -> &str {
        "json_transform"
    }

    fn description(&self) -> &str {
        "Extracts a named field from a JSON object."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: vec![
                ParameterSchema {
                    name: "data".to_owned(),
                    description: "The JSON object to extract from".to_owned(),
                    param_type: "object".to_owned(),
                    required: true,
                },
                ParameterSchema {
                    name: "field".to_owned(),
                    description: "The name of the field to extract".to_owned(),
                    param_type: "string".to_owned(),
                    required: true,
                },
            ],
        }
    }

    fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
        Box::pin(async move {
            let Some(data) = input.parameters.get("data") else {
                return ToolOutput::err("missing required parameter: data");
            };
            let Some(field) = input.parameters.get("field").and_then(|v| v.as_str()) else {
                return ToolOutput::err("missing required parameter: field (must be a string)");
            };

            match data.get(field) {
                Some(value) => ToolOutput::ok(value.clone()),
                None => ToolOutput::err(format!("field '{field}' not found in data")),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn extract_field() {
        let tool = JsonTransformTool;
        let mut parameters = HashMap::new();
        parameters.insert("data".to_owned(), json!({"name": "Alice", "age": 30}));
        parameters.insert("field".to_owned(), json!("name"));

        let output = tool.execute(ToolInput { parameters }).await;
        assert!(output.success);
        assert_eq!(output.result, json!("Alice"));
    }

    #[tokio::test]
    async fn missing_field() {
        let tool = JsonTransformTool;
        let mut parameters = HashMap::new();
        parameters.insert("data".to_owned(), json!({"name": "Alice"}));
        parameters.insert("field".to_owned(), json!("email"));

        let output = tool.execute(ToolInput { parameters }).await;
        assert!(!output.success);
        assert!(output.error.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn missing_data_param() {
        let tool = JsonTransformTool;
        let mut parameters = HashMap::new();
        parameters.insert("field".to_owned(), json!("name"));

        let output = tool.execute(ToolInput { parameters }).await;
        assert!(!output.success);
    }
}
