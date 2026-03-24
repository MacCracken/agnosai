//! SandboxManager — selects backend based on tool policy.
//!
//! Backends (in order of isolation strength):
//! 1. In-process (native Rust tools — no sandbox needed)
//! 2. WASM (wasmtime — memory isolation, capability-controlled)
//! 3. Process (subprocess + resource limits + kill-on-drop)
//! 4. OCI (container sandbox — strongest isolation)

use std::time::Duration;

use crate::core::error::AgnosaiError;
use tracing::{debug, info};

use super::oci::{OciSandbox, OciSandboxConfig};
use super::policy::{IsolationLevel, SandboxPolicy};
use super::process::ProcessSandbox;

/// Unified result from any sandbox backend.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SandboxResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub backend: IsolationLevel,
}

/// Configuration for available backends.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SandboxManagerConfig {
    /// Default timeout for process/OCI backends.
    pub default_timeout: Duration,
    /// OCI runtime binary.
    pub oci_runtime: String,
    /// Default OCI image for container sandbox.
    pub oci_default_image: String,
    /// OCI memory limit string.
    pub oci_memory_limit: String,
}

impl Default for SandboxManagerConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            oci_runtime: "docker".into(),
            oci_default_image: "alpine:latest".into(),
            oci_memory_limit: "256m".into(),
        }
    }
}

/// Selects and dispatches to the appropriate sandbox backend based on policy.
pub struct SandboxManager {
    config: SandboxManagerConfig,
}

impl SandboxManager {
    pub fn new(config: SandboxManagerConfig) -> Self {
        info!(
            oci_runtime = %config.oci_runtime,
            "sandbox manager initialized"
        );
        Self { config }
    }

    /// Execute a program with explicit arguments in the sandbox backend.
    ///
    /// This is the safe API — no shell interpretation of the arguments.
    /// For `IsolationLevel::Wasm`, use `WasmSandbox` directly.
    pub async fn execute_argv(
        &self,
        policy: &SandboxPolicy,
        argv: &[&str],
        input: &str,
    ) -> crate::core::Result<SandboxResult> {
        let level = policy.effective_isolation();
        let timeout = if policy.max_duration_secs > 0 {
            Duration::from_secs(policy.max_duration_secs)
        } else {
            self.config.default_timeout
        };

        debug!(?level, ?argv, "sandbox manager dispatching (argv)");

        match level {
            IsolationLevel::None => Ok(SandboxResult {
                stdout: input.to_owned(),
                stderr: String::new(),
                exit_code: 0,
                timed_out: false,
                backend: IsolationLevel::None,
            }),
            IsolationLevel::Wasm => Err(AgnosaiError::Sandbox(
                "WASM tools must be executed via WasmSandbox directly with a compiled module"
                    .into(),
            )),
            IsolationLevel::Process => {
                let sandbox = ProcessSandbox::shell(timeout);
                let result = sandbox.execute_argv(argv, input).await?;
                Ok(SandboxResult {
                    stdout: result.stdout,
                    stderr: result.stderr,
                    exit_code: result.exit_code,
                    timed_out: result.timed_out,
                    backend: IsolationLevel::Process,
                })
            }
            IsolationLevel::Oci => {
                let oci_config = OciSandboxConfig {
                    runtime: self.config.oci_runtime.clone(),
                    image: self.config.oci_default_image.clone(),
                    timeout,
                    memory_limit: self.config.oci_memory_limit.clone(),
                    allow_network: policy.needs_network,
                    env: Vec::new(),
                    volumes: Vec::new(),
                };
                let sandbox = OciSandbox::new(oci_config)?;
                let result = sandbox.execute(argv, input).await?;
                Ok(SandboxResult {
                    stdout: result.stdout,
                    stderr: result.stderr,
                    exit_code: result.exit_code,
                    timed_out: result.timed_out,
                    backend: IsolationLevel::Oci,
                })
            }
        }
    }

    /// Check which isolation level a policy resolves to.
    pub fn resolve_backend(&self, policy: &SandboxPolicy) -> IsolationLevel {
        policy.effective_isolation()
    }
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new(SandboxManagerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_native() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::native();
        assert_eq!(mgr.resolve_backend(&policy), IsolationLevel::None);
    }

    #[test]
    fn resolve_wasm() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::wasm();
        assert_eq!(mgr.resolve_backend(&policy), IsolationLevel::Wasm);
    }

    #[test]
    fn resolve_process() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::process();
        assert_eq!(mgr.resolve_backend(&policy), IsolationLevel::Process);
    }

    #[test]
    fn resolve_container() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::container();
        assert_eq!(mgr.resolve_backend(&policy), IsolationLevel::Oci);
    }

    #[tokio::test]
    async fn execute_native_passthrough() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::native();
        let result = mgr
            .execute_argv(&policy, &["true"], "test input")
            .await
            .expect("native should succeed");
        assert_eq!(result.backend, IsolationLevel::None);
        assert_eq!(result.stdout, "test input");
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn execute_wasm_returns_error() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::wasm();
        let result = mgr.execute_argv(&policy, &["noop"], "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn execute_process_echo() {
        let mgr = SandboxManager::default();
        let policy = SandboxPolicy::process();
        let result = mgr
            .execute_argv(&policy, &["echo", "hello"], "")
            .await
            .expect("process should succeed");
        assert_eq!(result.backend, IsolationLevel::Process);
        assert_eq!(result.stdout.trim(), "hello");
        assert_eq!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn default_manager_creation() {
        let mgr = SandboxManager::default();
        assert_eq!(mgr.config.oci_runtime, "docker");
    }
}
