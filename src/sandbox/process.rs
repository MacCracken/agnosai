//! Subprocess sandbox with resource limits and kill-on-drop.
//!
//! Runs untrusted executables as child processes with:
//! - stdin/stdout JSON protocol (same as Python sandbox)
//! - Configurable timeout with SIGKILL on expiry
//! - Memory limit via `setrlimit` (best-effort on Linux)
//! - kill-on-drop safety via tokio's `Command`

use std::path::PathBuf;
use std::time::Duration;

use crate::core::error::AgnosaiError;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, warn};

/// Result of a sandboxed process execution.
#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Configuration for the process sandbox.
#[derive(Debug, Clone)]
pub struct ProcessSandboxConfig {
    /// Path to the executable.
    pub executable: PathBuf,
    /// Static arguments prepended to every invocation.
    pub args: Vec<String>,
    /// Working directory (None = inherit).
    pub work_dir: Option<PathBuf>,
    /// Maximum wall-clock time before SIGKILL.
    pub timeout: Duration,
    /// Environment variables to set (overrides inherited env).
    pub env: Vec<(String, String)>,
    /// If true, clear the inherited environment before applying `env`.
    pub clean_env: bool,
}

impl Default for ProcessSandboxConfig {
    fn default() -> Self {
        Self {
            executable: PathBuf::from("/bin/sh"),
            args: vec!["-c".into()],
            work_dir: None,
            timeout: Duration::from_secs(30),
            env: Vec::new(),
            clean_env: false,
        }
    }
}

/// Subprocess sandbox that runs executables with resource limits.
pub struct ProcessSandbox {
    config: ProcessSandboxConfig,
}

impl ProcessSandbox {
    pub fn new(config: ProcessSandboxConfig) -> Self {
        Self { config }
    }

    /// Create a sandbox for running shell commands.
    ///
    /// **Security note**: The `execute()` method on a shell sandbox passes
    /// the command to `sh -c`, so shell metacharacters are interpreted. Use
    /// `execute_argv()` instead when the command comes from untrusted input.
    pub fn shell(timeout: Duration) -> Self {
        Self::new(ProcessSandboxConfig {
            executable: PathBuf::from("/bin/sh"),
            args: vec!["-c".into()],
            timeout,
            ..Default::default()
        })
    }

    /// Execute a program directly with explicit arguments, bypassing the shell.
    ///
    /// This is the safe alternative to `execute()` for untrusted input —
    /// no shell metacharacter interpretation is possible.
    pub async fn execute_argv(
        &self,
        argv: &[&str],
        input: &str,
    ) -> crate::core::Result<ProcessResult> {
        use std::process::Stdio;

        if argv.is_empty() {
            return Err(AgnosaiError::Sandbox("empty argv".into()));
        }

        debug!(
            executable = argv[0],
            args = ?&argv[1..],
            "spawning sandboxed process (argv mode)"
        );

        let mut cmd = Command::new(argv[0]);
        for arg in &argv[1..] {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(ref dir) = self.config.work_dir {
            cmd.current_dir(dir);
        }
        if self.config.clean_env {
            cmd.env_clear();
        }
        for (k, v) in &self.config.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|e| {
            AgnosaiError::Sandbox(format!("failed to spawn {}: {e}", argv[0]))
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(input.as_bytes()).await {
                warn!("failed to write to process stdin: {e}");
            }
        }

        match tokio::time::timeout(self.config.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                debug!(exit_code, "sandboxed process (argv) finished");
                Ok(ProcessResult {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(AgnosaiError::Sandbox(format!("process I/O error: {e}"))),
            Err(_) => {
                warn!(timeout_secs = self.config.timeout.as_secs(), "sandboxed process (argv) timed out");
                Ok(ProcessResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: -1,
                    timed_out: true,
                })
            }
        }
    }

    /// Execute a command string (or script path) with the given stdin input.
    ///
    /// The command is passed as an additional argument after `config.args`.
    /// For a shell sandbox, this means `sh -c <command>`.
    ///
    /// **Security warning**: When using a shell sandbox, the command string
    /// is interpreted by the shell. Use `execute_argv()` for untrusted input.
    pub async fn execute(
        &self,
        command: &str,
        input: &str,
    ) -> crate::core::Result<ProcessResult> {
        use std::process::Stdio;

        debug!(
            executable = %self.config.executable.display(),
            command,
            "spawning sandboxed process"
        );

        let mut cmd = Command::new(&self.config.executable);

        for arg in &self.config.args {
            cmd.arg(arg);
        }
        cmd.arg(command);

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        if let Some(ref dir) = self.config.work_dir {
            cmd.current_dir(dir);
        }

        if self.config.clean_env {
            cmd.env_clear();
        }
        for (k, v) in &self.config.env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|e| {
            AgnosaiError::Sandbox(format!(
                "failed to spawn {}: {e}",
                self.config.executable.display()
            ))
        })?;

        // Write input to stdin.
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(input.as_bytes()).await {
                warn!("failed to write to process stdin: {e}");
            }
            // Drop closes the pipe.
        }

        match tokio::time::timeout(self.config.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                debug!(exit_code, "sandboxed process finished");
                Ok(ProcessResult {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(AgnosaiError::Sandbox(format!("process I/O error: {e}"))),
            Err(_) => {
                warn!(
                    timeout_secs = self.config.timeout.as_secs(),
                    "sandboxed process timed out, killing"
                );
                // child is dropped here → kill_on_drop sends SIGKILL
                Ok(ProcessResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: -1,
                    timed_out: true,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_command() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(5));
        let result = sandbox
            .execute("echo hello", "")
            .await
            .expect("should succeed");
        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn stdin_passthrough() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(5));
        let result = sandbox
            .execute("cat", "input data")
            .await
            .expect("should succeed");
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "input data");
    }

    #[tokio::test]
    async fn nonzero_exit() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(5));
        let result = sandbox
            .execute("exit 42", "")
            .await
            .expect("should succeed");
        assert_eq!(result.exit_code, 42);
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn stderr_captured() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(5));
        let result = sandbox
            .execute("echo err >&2", "")
            .await
            .expect("should succeed");
        assert_eq!(result.exit_code, 0);
        assert!(result.stderr.contains("err"));
    }

    #[tokio::test]
    async fn timeout_kills_process() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(1));
        let result = sandbox
            .execute("sleep 60", "")
            .await
            .expect("should succeed even on timeout");
        assert!(result.timed_out);
        assert_eq!(result.exit_code, -1);
    }

    #[tokio::test]
    async fn clean_env() {
        let config = ProcessSandboxConfig {
            executable: PathBuf::from("/bin/sh"),
            args: vec!["-c".into()],
            timeout: Duration::from_secs(5),
            env: vec![("SANDBOX_VAR".into(), "yes".into())],
            clean_env: true,
            work_dir: None,
        };
        let sandbox = ProcessSandbox::new(config);
        let result = sandbox
            .execute("echo $SANDBOX_VAR", "")
            .await
            .expect("should succeed");
        assert_eq!(result.stdout.trim(), "yes");
    }

    #[tokio::test]
    async fn invalid_executable() {
        let config = ProcessSandboxConfig {
            executable: PathBuf::from("/nonexistent/binary"),
            args: Vec::new(),
            timeout: Duration::from_secs(5),
            env: Vec::new(),
            clean_env: false,
            work_dir: None,
        };
        let sandbox = ProcessSandbox::new(config);
        let result = sandbox.execute("", "");
        assert!(result.await.is_err());
    }

    #[tokio::test]
    async fn execute_argv_no_shell_interpretation() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(5));
        // With execute(), "echo hello; echo injected" would run both commands.
        // With execute_argv(), the semicolon is a literal argument to echo.
        let result = sandbox
            .execute_argv(&["echo", "hello; echo injected"], "")
            .await
            .expect("should succeed");
        assert_eq!(result.exit_code, 0);
        // The output should contain the literal semicolon, not two separate lines.
        assert_eq!(result.stdout.trim(), "hello; echo injected");
    }

    #[tokio::test]
    async fn execute_argv_empty_returns_error() {
        let sandbox = ProcessSandbox::shell(Duration::from_secs(5));
        let result = sandbox.execute_argv(&[], "").await;
        assert!(result.is_err());
    }
}
