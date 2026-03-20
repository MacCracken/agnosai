//! WASM tool loading and execution via the sandbox.
//!
//! Bridges [`WasmSandbox`](crate::sandbox::wasm::WasmSandbox) to the
//! [`NativeTool`] trait so WASM modules can be registered in the
//! [`ToolRegistry`](crate::tools::registry::ToolRegistry) alongside native tools.

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
