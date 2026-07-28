//! Container/VM environment detection and resource limit discovery.
//!
//! Detects whether the process is running bare-metal, in a container, a VM,
//! or on Kubernetes. Also reads cgroup-based resource limits when available.

use std::path::Path;

/// The runtime environment detected for this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeEnvironment {
    /// Running directly on hardware (no container or VM detected).
    Bare,
    /// Running inside a container (Docker, Podman, etc.).
    Container,
    /// Running inside a virtual machine.
    Vm,
    /// Running inside a Kubernetes pod.
    Kubernetes,
    /// Unable to determine the environment.
    Unknown,
}

impl std::fmt::Display for RuntimeEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bare => write!(f, "bare-metal"),
            Self::Container => write!(f, "container"),
            Self::Vm => write!(f, "vm"),
            Self::Kubernetes => write!(f, "kubernetes"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Resource limits as reported by the cgroup filesystem.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ResourceLimits {
    /// CPU quota as a fraction of one core (e.g. `2.0` = 2 cores).
    /// `None` if unlimited or unreadable.
    pub cpu_quota: Option<f64>,
    /// Memory limit in bytes. `None` if unlimited or unreadable.
    pub memory_limit_bytes: Option<u64>,
}

/// Detect the runtime environment of the current process.
///
/// Detection order:
/// 1. `KUBERNETES_SERVICE_HOST` env var -> Kubernetes
/// 2. `/proc/1/cgroup` containing `docker`, `containerd`, `kubepods` -> Container (or K8s)
/// 3. `/.dockerenv` file exists -> Container
/// 4. `/sys/hypervisor/type` exists -> VM
/// 5. Otherwise -> Bare (or Unknown if `/proc` is not accessible)
#[must_use]
pub fn detect() -> RuntimeEnvironment {
    // Kubernetes check (most specific).
    if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
        tracing::info!("detected Kubernetes environment via KUBERNETES_SERVICE_HOST");
        return RuntimeEnvironment::Kubernetes;
    }

    // Container check via cgroup.
    if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
        let lower = cgroup.to_lowercase();
        if lower.contains("kubepods") {
            tracing::info!("detected Kubernetes environment via /proc/1/cgroup");
            return RuntimeEnvironment::Kubernetes;
        }
        if lower.contains("docker")
            || lower.contains("containerd")
            || lower.contains("lxc")
            || lower.contains("/system.slice/containerd")
        {
            tracing::info!("detected container environment via /proc/1/cgroup");
            return RuntimeEnvironment::Container;
        }
    }

    // Container check via .dockerenv sentinel.
    if Path::new("/.dockerenv").exists() {
        tracing::info!("detected container environment via /.dockerenv");
        return RuntimeEnvironment::Container;
    }

    // VM check.
    if Path::new("/sys/hypervisor/type").exists() {
        tracing::info!("detected VM environment via /sys/hypervisor/type");
        return RuntimeEnvironment::Vm;
    }

    // If /proc exists, we're on a Linux system without container/VM indicators.
    if Path::new("/proc/1/cgroup").exists() {
        tracing::debug!("no container or VM indicators found, assuming bare-metal");
        return RuntimeEnvironment::Bare;
    }

    tracing::debug!("unable to determine runtime environment");
    RuntimeEnvironment::Unknown
}

/// Read resource limits from the cgroup filesystem.
///
/// Attempts to read cgroup v2 limits first, then falls back to cgroup v1.
/// Returns defaults (`None` for each field) if limits cannot be determined.
#[must_use]
pub fn resource_limits() -> ResourceLimits {
    let cpu_quota = read_cgroup_v2_cpu().or_else(read_cgroup_v1_cpu);
    let memory_limit_bytes = read_cgroup_v2_memory().or_else(read_cgroup_v1_memory);

    tracing::debug!(
        ?cpu_quota,
        ?memory_limit_bytes,
        "resource limits read from cgroup"
    );

    ResourceLimits {
        cpu_quota,
        memory_limit_bytes,
    }
}

/// Read CPU quota from cgroup v2: /sys/fs/cgroup/cpu.max
fn read_cgroup_v2_cpu() -> Option<f64> {
    let content = std::fs::read_to_string("/sys/fs/cgroup/cpu.max").ok()?;
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() >= 2 {
        let quota = parts[0].parse::<f64>().ok()?;
        let period = parts[1].parse::<f64>().ok()?;
        if quota < 0.0 || period <= 0.0 {
            return None; // "max" means unlimited
        }
        Some(quota / period)
    } else {
        None
    }
}

/// Read CPU quota from cgroup v1.
fn read_cgroup_v1_cpu() -> Option<f64> {
    let quota = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us")
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()?;
    if quota < 0.0 {
        return None; // -1 means unlimited
    }
    let period = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us")
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()?;
    if period <= 0.0 {
        return None;
    }
    Some(quota / period)
}

/// Read memory limit from cgroup v2: /sys/fs/cgroup/memory.max
fn read_cgroup_v2_memory() -> Option<u64> {
    let content = std::fs::read_to_string("/sys/fs/cgroup/memory.max").ok()?;
    let trimmed = content.trim();
    if trimmed == "max" {
        return None; // unlimited
    }
    trimmed.parse().ok()
}

/// Read memory limit from cgroup v1.
fn read_cgroup_v1_memory() -> Option<u64> {
    let content = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok()?;
    let value: u64 = content.trim().parse().ok()?;
    // Very large values (close to u64::MAX) indicate unlimited.
    if value > (1u64 << 62) {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_environment_display() {
        assert_eq!(RuntimeEnvironment::Bare.to_string(), "bare-metal");
        assert_eq!(RuntimeEnvironment::Container.to_string(), "container");
        assert_eq!(RuntimeEnvironment::Vm.to_string(), "vm");
        assert_eq!(RuntimeEnvironment::Kubernetes.to_string(), "kubernetes");
        assert_eq!(RuntimeEnvironment::Unknown.to_string(), "unknown");
    }

    #[test]
    fn runtime_environment_eq() {
        assert_eq!(RuntimeEnvironment::Bare, RuntimeEnvironment::Bare);
        assert_ne!(RuntimeEnvironment::Bare, RuntimeEnvironment::Container);
    }

    #[test]
    fn runtime_environment_clone() {
        let env = RuntimeEnvironment::Kubernetes;
        let cloned = env;
        assert_eq!(env, cloned);
    }

    #[test]
    fn resource_limits_default() {
        let limits = ResourceLimits::default();
        assert!(limits.cpu_quota.is_none());
        assert!(limits.memory_limit_bytes.is_none());
    }

    #[test]
    fn resource_limits_with_values() {
        let limits = ResourceLimits {
            cpu_quota: Some(2.0),
            memory_limit_bytes: Some(4 * 1024 * 1024 * 1024),
        };
        assert!((limits.cpu_quota.unwrap() - 2.0).abs() < f64::EPSILON);
        assert_eq!(limits.memory_limit_bytes.unwrap(), 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn detect_returns_valid_variant() {
        // We can't predict the exact environment, but the function should
        // return a valid variant without panicking.
        let env = detect();
        let display = env.to_string();
        assert!(!display.is_empty());
    }

    #[test]
    fn resource_limits_runs_without_panic() {
        // On a dev machine, limits may be None (unlimited), but should not panic.
        let limits = resource_limits();
        // Just verify the struct is valid.
        let _ = format!("{limits:?}");
    }

    #[test]
    fn runtime_environment_all_variants() {
        // Ensure all variants exist and are distinct.
        let variants = [
            RuntimeEnvironment::Bare,
            RuntimeEnvironment::Container,
            RuntimeEnvironment::Vm,
            RuntimeEnvironment::Kubernetes,
            RuntimeEnvironment::Unknown,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }
}
