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

        fn execute(&self, input: ToolInput) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
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
                                    ToolOutput::ok(
                                        value.get("result").cloned().unwrap_or(value),
                                    )
                                } else {
                                    let err = value
                                        .get("error")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown error");
                                    ToolOutput::err(err)
                                }
                            }
                            Err(_) => {
                                ToolOutput::ok(serde_json::Value::String(result.stdout.trim().to_string()))
                            }
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
