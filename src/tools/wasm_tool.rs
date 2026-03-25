//! WASM tool loading and execution via the sandbox.
//!
//! Bridges `WasmSandbox` to the
//! `NativeTool` trait so WASM modules can be registered in the
//! `ToolRegistry` alongside native tools.

#[cfg(feature = "sandbox")]
mod inner {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use serde::{Deserialize, Serialize};

    use crate::sandbox::wasm::{WasmModule, WasmSandbox};
    use crate::tools::native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};

    /// Metadata about a WASM tool, loaded from the module or provided at registration.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WasmToolManifest {
        pub name: String,
        pub description: String,
        pub parameters: Vec<ParameterSchema>,
    }

    /// A tool backed by a WASM module.
    ///
    /// The module must accept JSON on stdin and write JSON on stdout.
    /// The input format is: `{"parameters": {...}}`
    /// The output format is: `{"result": ..., "success": true/false, "error": null}`
    pub struct WasmTool {
        manifest: WasmToolManifest,
        sandbox: Arc<WasmSandbox>,
        module: Arc<WasmModule>,
    }

    impl WasmTool {
        /// Create a WASM tool from a pre-compiled module.
        pub fn new(
            manifest: WasmToolManifest,
            sandbox: Arc<WasmSandbox>,
            module: Arc<WasmModule>,
        ) -> Self {
            Self {
                manifest,
                sandbox,
                module,
            }
        }

        /// Load a WASM tool from raw bytes.
        pub fn from_bytes(
            manifest: WasmToolManifest,
            sandbox: Arc<WasmSandbox>,
            wasm_bytes: &[u8],
        ) -> crate::core::Result<Self> {
            let module = Arc::new(sandbox.load_module(wasm_bytes)?);
            Ok(Self::new(manifest, sandbox, module))
        }
    }

    impl std::fmt::Debug for WasmTool {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("WasmTool")
                .field("name", &self.manifest.name)
                .finish()
        }
    }

    impl NativeTool for WasmTool {
        fn name(&self) -> &str {
            &self.manifest.name
        }

        fn description(&self) -> &str {
            &self.manifest.description
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema {
                name: self.manifest.name.clone(),
                description: self.manifest.description.clone(),
                parameters: self.manifest.parameters.clone(),
            }
        }

        fn execute(
            &self,
            input: ToolInput,
        ) -> Pin<Box<dyn Future<Output = ToolOutput> + Send + '_>> {
            Box::pin(async move {
                let input_json = match serde_json::to_string(&serde_json::json!({
                    "parameters": input.parameters,
                })) {
                    Ok(j) => j,
                    Err(e) => return ToolOutput::err(format!("failed to serialize input: {e}")),
                };

                match self.sandbox.execute(&self.module, &input_json) {
                    Ok(result) => {
                        if result.exit_code != 0 {
                            return ToolOutput::err(format!(
                                "WASM module exited with code {}",
                                result.exit_code
                            ));
                        }
                        match serde_json::from_str::<serde_json::Value>(&result.stdout) {
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
                            // If stdout isn't JSON, return the raw text as the result.
                            Err(_) => ToolOutput::ok(serde_json::Value::String(result.stdout)),
                        }
                    }
                    Err(e) => ToolOutput::err(format!("WASM execution failed: {e}")),
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
    use crate::tools::native::ParameterSchema;

    fn sample_manifest() -> WasmToolManifest {
        WasmToolManifest {
            name: "test_tool".to_string(),
            description: "A test WASM tool".to_string(),
            parameters: vec![ParameterSchema {
                name: "input".to_string(),
                description: "The input value".to_string(),
                param_type: "string".to_string(),
                required: true,
            }],
        }
    }

    #[test]
    fn manifest_serde_round_trip() {
        let manifest = sample_manifest();
        let json = serde_json::to_string(&manifest).expect("serialize");
        let parsed: WasmToolManifest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.name, "test_tool");
        assert_eq!(parsed.description, "A test WASM tool");
        assert_eq!(parsed.parameters.len(), 1);
        assert_eq!(parsed.parameters[0].name, "input");
        assert_eq!(parsed.parameters[0].param_type, "string");
        assert!(parsed.parameters[0].required);
    }

    #[test]
    fn manifest_from_json_string() {
        let json = r#"{
            "name": "fetch",
            "description": "Fetch a URL",
            "parameters": [
                {
                    "name": "url",
                    "description": "Target URL",
                    "param_type": "string",
                    "required": true
                },
                {
                    "name": "timeout",
                    "description": "Timeout in seconds",
                    "param_type": "number",
                    "required": false
                }
            ]
        }"#;
        let manifest: WasmToolManifest = serde_json::from_str(json).expect("parse");
        assert_eq!(manifest.name, "fetch");
        assert_eq!(manifest.parameters.len(), 2);
        assert!(!manifest.parameters[1].required);
    }

    #[test]
    fn manifest_empty_parameters() {
        let json = r#"{
            "name": "noop",
            "description": "Does nothing",
            "parameters": []
        }"#;
        let manifest: WasmToolManifest = serde_json::from_str(json).expect("parse");
        assert_eq!(manifest.name, "noop");
        assert!(manifest.parameters.is_empty());
    }

    #[test]
    fn manifest_clone_and_debug() {
        let manifest = sample_manifest();
        let cloned = manifest.clone();
        assert_eq!(cloned.name, manifest.name);
        let debug_str = format!("{manifest:?}");
        assert!(debug_str.contains("test_tool"));
    }

    #[test]
    fn tool_output_success_parsing() {
        // Simulates parsing the JSON output a WASM module would produce
        let stdout = r#"{"result": "hello", "success": true}"#;
        let value: serde_json::Value = serde_json::from_str(stdout).unwrap();
        let success = value
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(success);
        assert_eq!(value.get("result").unwrap(), "hello");
    }

    #[test]
    fn tool_output_failure_parsing() {
        let stdout = r#"{"success": false, "error": "bad input"}"#;
        let value: serde_json::Value = serde_json::from_str(stdout).unwrap();
        let success = value
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(!success);
        assert_eq!(
            value.get("error").and_then(|v| v.as_str()).unwrap(),
            "bad input"
        );
    }

    #[test]
    fn tool_output_missing_success_defaults_true() {
        let stdout = r#"{"result": 42}"#;
        let value: serde_json::Value = serde_json::from_str(stdout).unwrap();
        let success = value
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        assert!(success);
    }

    #[test]
    fn tool_output_non_json_returns_raw_string() {
        let stdout = "plain text output";
        let result = serde_json::from_str::<serde_json::Value>(stdout);
        assert!(result.is_err());
        // The code path wraps raw text in Value::String
        let fallback = serde_json::Value::String(stdout.to_string());
        assert_eq!(fallback.as_str().unwrap(), "plain text output");
    }
}
