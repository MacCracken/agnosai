//! Legacy Python tool bridge — runs tools in sandboxed subprocess.
//!
//! Bridges [`PythonSandbox`](crate::sandbox::python::PythonSandbox) to the
//! [`NativeTool`] trait so Python-based tools can be registered in the
//! [`ToolRegistry`](crate::tools::registry::ToolRegistry).

#[cfg(feature = "sandbox")]
mod inner {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use crate::sandbox::python::PythonSandbox;
    use crate::tools::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};

    /// A tool backed by a Python script running in a sandboxed subprocess.
    ///
    /// The Python source must define a `create_tool()` function returning an
    /// object with an `execute(params)` method that returns a JSON-serializable
    /// result.
    pub struct PythonTool {
        name: String,
        description: String,
        parameters: Vec<ParameterSchema>,
        source: String,
        sandbox: Arc<PythonSandbox>,
    }

    impl PythonTool {
        pub fn new(
            name: impl Into<String>,
            description: impl Into<String>,
            parameters: Vec<ParameterSchema>,
            source: impl Into<String>,
            sandbox: Arc<PythonSandbox>,
        ) -> Self {
            Self {
                name: name.into(),
                description: description.into(),
                parameters,
                source: source.into(),
                sandbox,
            }
        }
    }

    impl std::fmt::Debug for PythonTool {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PythonTool")
                .field("name", &self.name)
                .finish()
        }
    }

    impl NativeTool for PythonTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema {
                name: self.name.clone(),
                description: self.description.clone(),
                parameters: self.parameters.clone(),
            }
        }

        fn execute(
            &self,
            input: ToolInput,
        ) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
            Box::pin(async move {
                let params = serde_json::Value::Object(
                    input
                        .parameters
                        .into_iter()
                        .collect::<serde_json::Map<String, serde_json::Value>>(),
                );

                match self
                    .sandbox
                    .execute_tool(&self.source, &self.name, &params)
                    .await
                {
                    Ok(result) => {
                        if result.timed_out {
                            return ToolOutput::err("python tool execution timed out");
                        }
                        if result.exit_code != 0 {
                            let err_msg = if result.stderr.is_empty() {
                                format!("python tool exited with code {}", result.exit_code)
                            } else {
                                result.stderr.trim().to_string()
                            };
                            return ToolOutput::err(err_msg);
                        }
                        match serde_json::from_str::<serde_json::Value>(result.stdout.trim()) {
                            Ok(value) => {
                                let success = value
                                    .get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true);
                                if success {
                                    ToolOutput::ok(value.get("result").cloned().unwrap_or(value))
                                } else {
                                    let err = value
                                        .get("error")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown error");
                                    ToolOutput::err(err)
                                }
                            }
                            Err(_) => ToolOutput::ok(serde_json::Value::String(
                                result.stdout.trim().to_string(),
                            )),
                        }
                    }
                    Err(e) => ToolOutput::err(format!("python sandbox error: {e}")),
                }
            })
        }
    }
}

#[cfg(feature = "sandbox")]
pub use inner::*;

#[cfg(test)]
#[cfg(feature = "sandbox")]
mod tests {
    use super::*;
    use crate::tools::native::{NativeTool, ParameterSchema};
    use std::sync::Arc;

    use crate::sandbox::python::PythonSandbox;

    fn sample_tool(sandbox: Arc<PythonSandbox>) -> PythonTool {
        PythonTool::new(
            "greet",
            "Greet a user",
            vec![ParameterSchema {
                name: "name".to_string(),
                description: "The user's name".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
            r#"
def create_tool():
    class Greeter:
        def execute(self, params):
            return {"greeting": f"Hello, {params['name']}!"}
    return Greeter()
"#,
            sandbox,
        )
    }

    #[test]
    fn tool_name_and_description() {
        let sandbox = Arc::new(PythonSandbox::new());
        let tool = sample_tool(sandbox);
        assert_eq!(tool.name(), "greet");
        assert_eq!(tool.description(), "Greet a user");
    }

    #[test]
    fn tool_schema_matches() {
        let sandbox = Arc::new(PythonSandbox::new());
        let tool = sample_tool(sandbox);
        let schema = tool.schema();
        assert_eq!(schema.name, "greet");
        assert_eq!(schema.description, "Greet a user");
        assert_eq!(schema.parameters.len(), 1);
        assert_eq!(schema.parameters[0].name, "name");
        assert!(schema.parameters[0].required);
    }

    #[test]
    fn tool_debug_format() {
        let sandbox = Arc::new(PythonSandbox::new());
        let tool = sample_tool(sandbox);
        let debug_str = format!("{tool:?}");
        assert!(debug_str.contains("PythonTool"));
        assert!(debug_str.contains("greet"));
    }

    #[test]
    fn output_success_parsing() {
        let stdout = r#"{"result": "ok", "success": true}"#;
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let success = value
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(success);
        assert_eq!(value.get("result").unwrap(), "ok");
    }

    #[test]
    fn output_failure_parsing() {
        let stdout = r#"{"success": false, "error": "missing param"}"#;
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let success = value
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(!success);
        assert_eq!(
            value.get("error").and_then(|v| v.as_str()).unwrap(),
            "missing param"
        );
    }

    #[test]
    fn output_missing_success_defaults_true() {
        let stdout = r#"{"result": {"key": "value"}}"#;
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let success = value
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(success);
    }

    #[test]
    fn output_non_json_returns_raw() {
        let stdout = "not json at all\n";
        let result = serde_json::from_str::<serde_json::Value>(stdout.trim());
        assert!(result.is_err());
        let fallback = serde_json::Value::String(stdout.trim().to_string());
        assert_eq!(fallback.as_str().unwrap(), "not json at all");
    }

    #[test]
    fn output_missing_error_defaults_unknown() {
        let stdout = r#"{"success": false}"#;
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let err = value
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        assert_eq!(err, "unknown error");
    }

    #[test]
    fn output_result_extracted_when_present() {
        let stdout = r#"{"result": [1, 2, 3], "success": true}"#;
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let result = value.get("result").cloned().unwrap_or(value.clone());
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 3);
    }

    #[test]
    fn output_whole_value_when_no_result_key() {
        let stdout = r#"{"data": "something"}"#;
        let value: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        let result = value.get("result").cloned().unwrap_or(value.clone());
        // Should fall back to the whole value
        assert!(result.get("data").is_some());
    }
}
