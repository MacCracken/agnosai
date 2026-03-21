//! WASM tool package loader.
//!
//! Loads community WASM tools from a directory containing:
//! - `manifest.json` — tool metadata (name, description, parameters)
//! - `<name>.wasm` — the compiled WASM module
//!
//! # Directory layout
//!
//! ```text
//! tools/
//! ├── hello/
//! │   ├── manifest.json
//! │   └── hello.wasm
//! └── calculator/
//!     ├── manifest.json
//!     └── calculator.wasm
//! ```

#[cfg(feature = "sandbox")]
mod inner {
    use std::path::Path;
    use std::sync::Arc;

    use serde::{Deserialize, Serialize};
    use tracing::{info, warn};

    use crate::core::error::AgnosaiError;
    use crate::sandbox::wasm::WasmSandbox;
    use crate::tools::native::{NativeTool, ParameterSchema};
    use crate::tools::registry::ToolRegistry;
    use crate::tools::wasm_tool::{WasmTool, WasmToolManifest};

    /// On-disk manifest format (superset of WasmToolManifest with version).
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PackageManifest {
        pub name: String,
        pub description: String,
        #[serde(default = "default_version")]
        pub version: String,
        #[serde(default)]
        pub parameters: Vec<ParameterSchema>,
    }

    fn default_version() -> String {
        "0.0.0".to_string()
    }

    /// Load a single WASM tool from a package directory.
    ///
    /// Expects `dir/manifest.json` and `dir/<name>.wasm`.
    pub fn load_tool_package(
        dir: &Path,
        sandbox: &Arc<WasmSandbox>,
    ) -> crate::core::Result<WasmTool> {
        let manifest_path = dir.join("manifest.json");
        let manifest_str = std::fs::read_to_string(&manifest_path).map_err(|e| {
            AgnosaiError::InvalidDefinition(format!(
                "failed to read {}: {e}",
                manifest_path.display()
            ))
        })?;

        let pkg: PackageManifest = serde_json::from_str(&manifest_str)?;

        let wasm_path = dir.join(format!("{}.wasm", pkg.name));
        let wasm_bytes = std::fs::read(&wasm_path).map_err(|e| {
            AgnosaiError::InvalidDefinition(format!("failed to read {}: {e}", wasm_path.display()))
        })?;

        let manifest = WasmToolManifest {
            name: pkg.name.clone(),
            description: pkg.description,
            parameters: pkg.parameters,
        };

        info!(
            tool = %pkg.name,
            version = %pkg.version,
            wasm_size = wasm_bytes.len(),
            "loading WASM tool package"
        );

        WasmTool::from_bytes(manifest, sandbox.clone(), &wasm_bytes)
    }

    /// Scan a directory for WASM tool packages and register them all.
    ///
    /// Each subdirectory that contains a `manifest.json` is treated as a
    /// tool package. Tools that fail to load are logged and skipped.
    pub fn load_all_tool_packages(
        tools_dir: &Path,
        sandbox: &Arc<WasmSandbox>,
        registry: &ToolRegistry,
    ) -> crate::core::Result<usize> {
        let entries = std::fs::read_dir(tools_dir)?;
        let mut loaded = 0;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if !path.join("manifest.json").exists() {
                continue;
            }

            match load_tool_package(&path, sandbox) {
                Ok(tool) => {
                    info!(tool = tool.name(), "registered WASM tool");
                    registry.register(Arc::new(tool));
                    loaded += 1;
                }
                Err(e) => {
                    warn!(
                        dir = %path.display(),
                        error = %e,
                        "failed to load WASM tool package, skipping"
                    );
                }
            }
        }

        info!(count = loaded, dir = %tools_dir.display(), "WASM tool packages loaded");
        Ok(loaded)
    }
}

#[cfg(feature = "sandbox")]
pub use inner::*;

#[cfg(test)]
#[cfg(feature = "sandbox")]
mod tests {
    use super::*;
    use crate::sandbox::wasm::WasmSandbox;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn load_tool_package_missing_manifest() {
        let dir = tempdir().unwrap();
        let sandbox = Arc::new(WasmSandbox::new().unwrap());
        let result = load_tool_package(dir.path(), &sandbox);
        assert!(result.is_err());
    }

    #[test]
    fn load_tool_package_missing_wasm() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("manifest.json"),
            r#"{"name":"test","description":"test","parameters":[]}"#,
        )
        .unwrap();
        let sandbox = Arc::new(WasmSandbox::new().unwrap());
        let result = load_tool_package(dir.path(), &sandbox);
        assert!(result.is_err());
    }

    #[test]
    fn load_all_empty_dir() {
        let dir = tempdir().unwrap();
        let sandbox = Arc::new(WasmSandbox::new().unwrap());
        let registry = crate::tools::ToolRegistry::new();
        let count = load_all_tool_packages(dir.path(), &sandbox, &registry).unwrap();
        assert_eq!(count, 0);
    }
}
