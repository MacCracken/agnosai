//! Sandbox profiles with strength scoring.
//!
//! Each tool declares a [`SandboxPolicy`] that determines the minimum isolation
//! backend required to run it. The [`SandboxManager`](super::manager::SandboxManager)
//! uses the policy to pick a backend.

use serde::{Deserialize, Serialize};

/// Isolation level, ordered from weakest to strongest.
///
/// The numeric strength value is used by the sandbox manager to select the
/// cheapest backend that meets a tool's minimum requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// In-process execution (native Rust tools). No sandbox overhead.
    None = 0,
    /// WASM sandbox — memory-isolated, no filesystem/network.
    Wasm = 1,
    /// Subprocess sandbox — seccomp + Landlock + cgroups.
    Process = 2,
    /// OCI container — strongest isolation.
    Oci = 3,
}

impl IsolationLevel {
    /// Numeric strength (0–3). Higher is more isolated.
    pub fn strength(self) -> u8 {
        self as u8
    }
}

impl Default for IsolationLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Policy governing how a tool should be sandboxed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Minimum isolation level required.
    pub min_isolation: IsolationLevel,
    /// Whether the tool needs filesystem access.
    pub needs_filesystem: bool,
    /// Whether the tool needs network access.
    pub needs_network: bool,
    /// Maximum execution time in seconds.
    pub max_duration_secs: u64,
    /// Maximum memory in bytes (0 = default).
    pub max_memory_bytes: usize,
}

impl SandboxPolicy {
    /// Policy for trusted native tools — no sandbox needed.
    pub fn native() -> Self {
        Self {
            min_isolation: IsolationLevel::None,
            needs_filesystem: false,
            needs_network: false,
            max_duration_secs: 300,
            max_memory_bytes: 0,
        }
    }

    /// Policy for WASM tools — pure compute, no I/O.
    pub fn wasm() -> Self {
        Self {
            min_isolation: IsolationLevel::Wasm,
            needs_filesystem: false,
            needs_network: false,
            max_duration_secs: 30,
            max_memory_bytes: 64 * 1024 * 1024,
        }
    }

    /// Policy for subprocess tools (e.g. Python scripts).
    pub fn process() -> Self {
        Self {
            min_isolation: IsolationLevel::Process,
            needs_filesystem: true,
            needs_network: false,
            max_duration_secs: 60,
            max_memory_bytes: 256 * 1024 * 1024,
        }
    }

    /// Policy for untrusted tools requiring full container isolation.
    pub fn container() -> Self {
        Self {
            min_isolation: IsolationLevel::Oci,
            needs_filesystem: true,
            needs_network: true,
            max_duration_secs: 120,
            max_memory_bytes: 512 * 1024 * 1024,
        }
    }

    /// Select the effective isolation level based on the policy requirements.
    ///
    /// If the tool needs filesystem or network access, the level is upgraded
    /// to at least `Process` or `Oci` respectively.
    pub fn effective_isolation(&self) -> IsolationLevel {
        let mut level = self.min_isolation;
        if self.needs_network && level < IsolationLevel::Oci {
            level = IsolationLevel::Oci;
        } else if self.needs_filesystem && level < IsolationLevel::Process {
            level = IsolationLevel::Process;
        }
        level
    }
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self::wasm()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolation_level_ordering() {
        assert!(IsolationLevel::None < IsolationLevel::Wasm);
        assert!(IsolationLevel::Wasm < IsolationLevel::Process);
        assert!(IsolationLevel::Process < IsolationLevel::Oci);
    }

    #[test]
    fn isolation_strength_values() {
        assert_eq!(IsolationLevel::None.strength(), 0);
        assert_eq!(IsolationLevel::Wasm.strength(), 1);
        assert_eq!(IsolationLevel::Process.strength(), 2);
        assert_eq!(IsolationLevel::Oci.strength(), 3);
    }

    #[test]
    fn native_policy_no_sandbox() {
        let p = SandboxPolicy::native();
        assert_eq!(p.min_isolation, IsolationLevel::None);
        assert_eq!(p.effective_isolation(), IsolationLevel::None);
    }

    #[test]
    fn wasm_policy_defaults() {
        let p = SandboxPolicy::wasm();
        assert_eq!(p.min_isolation, IsolationLevel::Wasm);
        assert!(!p.needs_filesystem);
        assert!(!p.needs_network);
        assert_eq!(p.effective_isolation(), IsolationLevel::Wasm);
    }

    #[test]
    fn process_policy_needs_filesystem() {
        let p = SandboxPolicy::process();
        assert!(p.needs_filesystem);
        assert_eq!(p.effective_isolation(), IsolationLevel::Process);
    }

    #[test]
    fn container_policy_needs_network() {
        let p = SandboxPolicy::container();
        assert!(p.needs_network);
        assert_eq!(p.effective_isolation(), IsolationLevel::Oci);
    }

    #[test]
    fn effective_isolation_upgrades_for_network() {
        let p = SandboxPolicy {
            min_isolation: IsolationLevel::Wasm,
            needs_filesystem: false,
            needs_network: true,
            max_duration_secs: 30,
            max_memory_bytes: 0,
        };
        assert_eq!(p.effective_isolation(), IsolationLevel::Oci);
    }

    #[test]
    fn effective_isolation_upgrades_for_filesystem() {
        let p = SandboxPolicy {
            min_isolation: IsolationLevel::Wasm,
            needs_filesystem: true,
            needs_network: false,
            max_duration_secs: 30,
            max_memory_bytes: 0,
        };
        assert_eq!(p.effective_isolation(), IsolationLevel::Process);
    }

    #[test]
    fn effective_isolation_no_downgrade() {
        let p = SandboxPolicy {
            min_isolation: IsolationLevel::Oci,
            needs_filesystem: false,
            needs_network: false,
            max_duration_secs: 30,
            max_memory_bytes: 0,
        };
        assert_eq!(p.effective_isolation(), IsolationLevel::Oci);
    }

    #[test]
    fn default_policy_is_wasm() {
        let p = SandboxPolicy::default();
        assert_eq!(p.min_isolation, IsolationLevel::Wasm);
    }

    #[test]
    fn serde_roundtrip() {
        let p = SandboxPolicy::container();
        let json = serde_json::to_string(&p).unwrap();
        let p2: SandboxPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(p2.min_isolation, IsolationLevel::Oci);
        assert!(p2.needs_network);
    }
}
