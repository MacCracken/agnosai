//! Kavach integration — bridge AgnosAI sandbox policies to kavach sandboxes.
//!
//! Maps AgnosAI's [`IsolationLevel`] to kavach [`Backend`]s, converts
//! [`SandboxPolicy`] to kavach [`SandboxConfig`], executes tools through
//! kavach sandboxes with externalization gate scanning, and produces
//! [`kavach::StrengthScore`] for crew metadata.

use kavach::scanning::types::{ExternalizationPolicy, ScanVerdict, Severity};
use kavach::{Backend, ExecResult, Sandbox, SandboxConfig, StrengthScore};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::policy::{IsolationLevel, SandboxPolicy};

/// Result of executing a tool through kavach, including security metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KavachToolResult {
    /// Tool output (stdout).
    pub output: String,
    /// Tool stderr.
    pub stderr: String,
    /// Exit code from the sandbox.
    pub exit_code: i32,
    /// Whether the execution timed out.
    pub timed_out: bool,
    /// Execution duration in milliseconds.
    pub duration_ms: u64,
    /// Kavach strength score for the isolation level used.
    pub strength: StrengthScore,
    /// Scan verdict from the externalization gate (if enabled).
    pub scan_verdict: Option<ScanVerdict>,
}

/// Map an AgnosAI [`IsolationLevel`] to a kavach [`Backend`].
#[must_use]
pub fn map_backend(level: IsolationLevel) -> Backend {
    match level {
        IsolationLevel::None => Backend::Noop,
        IsolationLevel::Wasm => Backend::Wasm,
        IsolationLevel::Process => Backend::Process,
        IsolationLevel::Oci => Backend::Oci,
    }
}

/// Build a kavach [`SandboxConfig`] from an AgnosAI [`SandboxPolicy`].
#[must_use]
pub fn build_config(policy: &SandboxPolicy, agent_id: Option<&str>) -> SandboxConfig {
    let backend = map_backend(policy.effective_isolation());
    let mut builder = SandboxConfig::builder()
        .backend(backend)
        .network(policy.needs_network)
        .timeout_ms(policy.max_duration_secs * 1000);

    if let Some(id) = agent_id {
        builder = builder.agent_id(id);
    }

    // Enable externalization gate with default thresholds.
    builder = builder.externalization(ExternalizationPolicy::default());

    // Apply seccomp for process-level isolation and above.
    if policy.effective_isolation() >= IsolationLevel::Process {
        builder = builder.policy_seccomp("basic");
    }

    builder.build()
}

/// Compute the kavach strength score for a given AgnosAI sandbox policy.
#[must_use]
pub fn strength_for_policy(policy: &SandboxPolicy) -> StrengthScore {
    let backend = map_backend(policy.effective_isolation());
    let kavach_policy = to_kavach_policy(policy);
    kavach::score_backend(backend, &kavach_policy)
}

/// Convert an AgnosAI `SandboxPolicy` to a kavach `SandboxPolicy`.
fn to_kavach_policy(policy: &SandboxPolicy) -> kavach::SandboxPolicy {
    let mut kp = kavach::SandboxPolicy::basic();
    kp.network.enabled = policy.needs_network;
    if policy.effective_isolation() >= IsolationLevel::Process {
        kp.seccomp_enabled = true;
        kp.seccomp_profile = Some("basic".into());
    }
    kp
}

/// Execute a command through a kavach sandbox.
///
/// Creates a sandbox, starts it, executes the command, applies the
/// externalization gate, destroys the sandbox, and returns the result
/// with security metadata.
pub async fn execute(
    command: &str,
    policy: &SandboxPolicy,
    agent_id: Option<&str>,
) -> crate::core::Result<KavachToolResult> {
    let config = build_config(policy, agent_id);
    let backend = config.backend;
    let strength = strength_for_policy(policy);

    debug!(
        backend = %backend,
        strength = strength.value(),
        timeout_ms = config.timeout_ms,
        "creating kavach sandbox"
    );

    let mut sandbox = Sandbox::create(config)
        .await
        .map_err(|e| crate::core::AgnosaiError::Sandbox(format!("kavach create failed: {e}")))?;

    sandbox
        .transition(kavach::SandboxState::Running)
        .map_err(|e| crate::core::AgnosaiError::Sandbox(format!("kavach start failed: {e}")))?;

    let result = sandbox.exec(command).await;

    // Always destroy the sandbox, even on exec failure.
    if let Err(e) = sandbox.transition(kavach::SandboxState::Stopped) {
        warn!(error = %e, "kavach stop failed (continuing with destroy)");
    }
    if let Err(e) = sandbox.transition(kavach::SandboxState::Destroyed) {
        warn!(error = %e, "kavach destroy failed");
    }

    let exec_result = result
        .map_err(|e| crate::core::AgnosaiError::Sandbox(format!("kavach exec failed: {e}")))?;

    info!(
        exit_code = exec_result.exit_code,
        duration_ms = exec_result.duration_ms,
        timed_out = exec_result.timed_out,
        strength = strength.value(),
        "kavach tool execution completed"
    );

    Ok(KavachToolResult {
        output: exec_result.stdout,
        stderr: exec_result.stderr,
        exit_code: exec_result.exit_code,
        timed_out: exec_result.timed_out,
        duration_ms: exec_result.duration_ms,
        strength,
        scan_verdict: None, // Verdict is applied internally by kavach's gate
    })
}

/// Apply the kavach externalization gate to a raw output string.
///
/// This is a standalone function for scanning tool outputs that didn't
/// go through a kavach sandbox (e.g. native Rust tools).
#[must_use]
pub fn scan_output(output: &str) -> ScanVerdict {
    let gate = kavach::ExternalizationGate::new();
    let result = ExecResult {
        exit_code: 0,
        stdout: output.to_string(),
        stderr: String::new(),
        duration_ms: 0,
        timed_out: false,
    };
    let policy = ExternalizationPolicy::default();
    match gate.apply(result, &policy) {
        Ok(_) => ScanVerdict::Pass,
        Err(e) => {
            warn!(error = %e, "externalization gate blocked output");
            ScanVerdict::Block
        }
    }
}

/// Map crew trust levels to kavach externalization policy presets.
#[must_use]
pub fn policy_for_trust(trust: &str) -> ExternalizationPolicy {
    match trust {
        "minimal" => ExternalizationPolicy {
            enabled: true,
            max_artifact_size_bytes: 1024 * 1024, // 1 MB
            block_threshold: Severity::Medium,
            quarantine_threshold: Severity::Low,
            redact_secrets: true,
        },
        "strict" => ExternalizationPolicy {
            enabled: true,
            max_artifact_size_bytes: 10 * 1024 * 1024, // 10 MB
            block_threshold: Severity::High,
            quarantine_threshold: Severity::Medium,
            redact_secrets: true,
        },
        _ => ExternalizationPolicy::default(), // "basic" or unrecognized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_backend_none_to_noop() {
        assert_eq!(map_backend(IsolationLevel::None), Backend::Noop);
    }

    #[test]
    fn map_backend_wasm() {
        assert_eq!(map_backend(IsolationLevel::Wasm), Backend::Wasm);
    }

    #[test]
    fn map_backend_process() {
        assert_eq!(map_backend(IsolationLevel::Process), Backend::Process);
    }

    #[test]
    fn map_backend_oci() {
        assert_eq!(map_backend(IsolationLevel::Oci), Backend::Oci);
    }

    #[test]
    fn strength_for_native_policy() {
        let p = SandboxPolicy::native();
        let score = strength_for_policy(&p);
        // Noop backend with basic policy modifiers.
        assert!(
            score.value() <= 20,
            "native should be low, got {}",
            score.value()
        );
    }

    #[test]
    fn strength_for_process_policy() {
        let p = SandboxPolicy::process();
        let score = strength_for_policy(&p);
        // Process backend with seccomp.
        assert!(
            score.value() >= 50,
            "process should be ≥50, got {}",
            score.value()
        );
    }

    #[test]
    fn strength_for_wasm_policy() {
        let p = SandboxPolicy::wasm();
        let score = strength_for_policy(&p);
        // WASM backend score.
        assert!(
            score.value() >= 60,
            "wasm should be ≥60, got {}",
            score.value()
        );
    }

    #[test]
    fn strength_ordering_matches_isolation() {
        let native = strength_for_policy(&SandboxPolicy::native());
        let wasm = strength_for_policy(&SandboxPolicy::wasm());
        let process = strength_for_policy(&SandboxPolicy::process());
        assert!(native.value() < wasm.value());
        assert!(native.value() < process.value());
    }

    #[test]
    fn build_config_sets_backend() {
        let p = SandboxPolicy::process();
        let config = build_config(&p, Some("agent-1"));
        assert_eq!(config.backend, Backend::Process);
        assert_eq!(config.agent_id.as_deref(), Some("agent-1"));
    }

    #[test]
    fn build_config_enables_externalization() {
        let p = SandboxPolicy::wasm();
        let config = build_config(&p, None);
        assert!(config.externalization.is_some());
    }

    #[test]
    fn policy_for_trust_minimal() {
        let p = policy_for_trust("minimal");
        assert_eq!(p.block_threshold, Severity::Medium);
    }

    #[test]
    fn policy_for_trust_strict() {
        let p = policy_for_trust("strict");
        assert_eq!(p.block_threshold, Severity::High);
    }

    #[test]
    fn policy_for_trust_default() {
        let p = policy_for_trust("basic");
        assert_eq!(p.block_threshold, Severity::Critical);
    }

    #[test]
    fn scan_clean_output_passes() {
        let verdict = scan_output("hello world, task completed successfully");
        assert_eq!(verdict, ScanVerdict::Pass);
    }

    #[test]
    fn scan_secret_output_blocks() {
        // AWS access key pattern should be detected.
        let verdict = scan_output("result: AKIAIOSFODNN7EXAMPLE");
        assert_eq!(verdict, ScanVerdict::Block);
    }

    #[test]
    fn strength_label_ranges() {
        assert_eq!(StrengthScore(0).label(), "minimal");
        assert_eq!(StrengthScore(50).label(), "standard");
        assert_eq!(StrengthScore(90).label(), "fortress");
    }
}
