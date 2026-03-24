//! OCI container sandbox for strongest isolation.
//!
//! Runs tools inside ephemeral containers using an OCI-compatible runtime
//! (`docker` or `podman`). Communication happens via stdin/stdout JSON,
//! just like the process and Python sandboxes.
//!
//! The container is created with:
//! - `--rm` — auto-remove on exit
//! - `--network=none` — no network by default
//! - `--memory` — memory limit
//! - `--read-only` — read-only root filesystem
//! - `--tmpfs /tmp` — writable /tmp only

use std::time::Duration;

use crate::core::error::AgnosaiError;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, warn};

/// Result of an OCI container execution.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OciResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Configuration for the OCI sandbox.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OciSandboxConfig {
    /// Runtime binary (e.g. "docker", "podman").
    pub runtime: String,
    /// Container image to use.
    pub image: String,
    /// Maximum wall-clock time.
    pub timeout: Duration,
    /// Memory limit (e.g. "256m").
    pub memory_limit: String,
    /// Allow network access.
    pub allow_network: bool,
    /// Extra `--env` pairs.
    pub env: Vec<(String, String)>,
    /// Volume mounts in `host:container` format.
    pub volumes: Vec<String>,
}

impl OciSandboxConfig {
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            runtime: "docker".into(),
            image: image.into(),
            timeout: Duration::from_secs(60),
            memory_limit: "256m".into(),
            allow_network: false,
            env: Vec::new(),
            volumes: Vec::new(),
        }
    }
}

/// Validate an OCI image reference.
///
/// Rejects images containing shell metacharacters, flags (`--`), or other
/// characters that could inject arguments into the container runtime.
/// Accepts standard Docker image references: `[registry/]name[:tag][@digest]`.
fn validate_image_ref(image: &str) -> Result<(), String> {
    if image.is_empty() {
        return Err("image name is empty".into());
    }
    if image.starts_with('-') {
        return Err("image name must not start with '-' (flag injection)".into());
    }
    // Allow alphanumeric, /, -, _, ., :, @ (standard Docker image ref characters).
    let invalid = image
        .chars()
        .find(|c| !c.is_alphanumeric() && !"-_./:#@".contains(*c));
    if let Some(ch) = invalid {
        return Err(format!("image name contains invalid character: '{ch}'"));
    }
    Ok(())
}

/// OCI container sandbox.
pub struct OciSandbox {
    config: OciSandboxConfig,
}

impl OciSandbox {
    /// Create a new OCI sandbox, validating the image reference.
    pub fn new(config: OciSandboxConfig) -> crate::core::Result<Self> {
        validate_image_ref(&config.image).map_err(|e| {
            crate::core::error::AgnosaiError::Sandbox(format!("invalid OCI image: {e}"))
        })?;
        Ok(Self { config })
    }

    /// Execute a command inside an ephemeral container.
    ///
    /// The `input` string is piped to the container's stdin. stdout and stderr
    /// are captured. The container is destroyed after execution.
    pub async fn execute(
        &self,
        entrypoint_args: &[&str],
        input: &str,
    ) -> crate::core::Result<OciResult> {
        use std::process::Stdio;

        debug!(
            image = %self.config.image,
            "launching OCI container"
        );

        let mut cmd = Command::new(&self.config.runtime);
        cmd.arg("run")
            .arg("--rm")
            .arg("-i")
            .arg("--read-only")
            .arg("--tmpfs")
            .arg("/tmp")
            .arg("--memory")
            .arg(&self.config.memory_limit);

        if !self.config.allow_network {
            cmd.arg("--network=none");
        }

        for (k, v) in &self.config.env {
            cmd.arg("--env").arg(format!("{k}={v}"));
        }

        for vol in &self.config.volumes {
            cmd.arg("-v").arg(vol);
        }

        cmd.arg(&self.config.image);

        for arg in entrypoint_args {
            cmd.arg(arg);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| {
            AgnosaiError::Sandbox(format!(
                "failed to spawn {} container: {e}",
                self.config.runtime
            ))
        })?;

        if let Some(mut stdin) = child.stdin.take()
            && let Err(e) = stdin.write_all(input.as_bytes()).await
        {
            warn!("failed to write to container stdin: {e}");
        }

        match tokio::time::timeout(self.config.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => {
                let exit_code = output.status.code().unwrap_or(-1);
                debug!(exit_code, "OCI container finished");
                Ok(OciResult {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    exit_code,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => Err(AgnosaiError::Sandbox(format!("container I/O error: {e}"))),
            Err(_) => {
                warn!(
                    timeout_secs = self.config.timeout.as_secs(),
                    "OCI container timed out, killing"
                );
                Ok(OciResult {
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

    #[test]
    fn config_defaults() {
        let cfg = OciSandboxConfig::new("alpine:latest");
        assert_eq!(cfg.runtime, "docker");
        assert_eq!(cfg.image, "alpine:latest");
        assert!(!cfg.allow_network);
        assert_eq!(cfg.memory_limit, "256m");
    }

    #[test]
    fn config_with_network() {
        let mut cfg = OciSandboxConfig::new("python:3.12-slim");
        cfg.allow_network = true;
        assert!(cfg.allow_network);
    }

    #[test]
    fn config_with_env() {
        let mut cfg = OciSandboxConfig::new("node:20-slim");
        cfg.env.push(("NODE_ENV".into(), "production".into()));
        assert_eq!(cfg.env.len(), 1);
    }

    #[tokio::test]
    async fn missing_runtime_returns_error() {
        let mut cfg = OciSandboxConfig::new("alpine:latest");
        cfg.runtime = "/nonexistent/runtime".into();
        let sandbox = OciSandbox::new(cfg).unwrap();
        let result = sandbox.execute(&["echo", "hi"], "").await;
        assert!(result.is_err());
    }

    #[test]
    fn rejects_image_with_flag_injection() {
        let mut cfg = OciSandboxConfig::new("alpine");
        cfg.image = "--privileged".into();
        assert!(OciSandbox::new(cfg).is_err());
    }

    #[test]
    fn rejects_image_with_shell_metacharacters() {
        let mut cfg = OciSandboxConfig::new("alpine");
        cfg.image = "alpine; rm -rf /".into();
        assert!(OciSandbox::new(cfg).is_err());
    }

    #[test]
    fn accepts_valid_image_refs() {
        for image in &[
            "alpine",
            "alpine:latest",
            "ubuntu:22.04",
            "registry.example.com/myapp:v1.2.3",
            "ghcr.io/org/repo:sha-abc123",
            "myimage@sha256:abcdef1234567890",
        ] {
            let cfg = OciSandboxConfig::new(*image);
            assert!(OciSandbox::new(cfg).is_ok(), "should accept: {image}");
        }
    }

    #[test]
    fn rejects_empty_image() {
        let mut cfg = OciSandboxConfig::new("alpine");
        cfg.image = String::new();
        assert!(OciSandbox::new(cfg).is_err());
    }
}
