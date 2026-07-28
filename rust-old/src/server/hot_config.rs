//! Hot-reload configuration via `tokio::sync::watch`.
//!
//! Provides a `ConfigHolder<T>` that allows runtime configuration updates
//! without restart. Readers get a snapshot via `borrow()`; writers push
//! updates via `update()`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

/// Runtime-reloadable configuration holder.
///
/// Uses a `watch` channel internally: single writer, many readers.
/// Readers always see the latest value with zero contention.
pub struct ConfigHolder<T: Clone + Send + Sync + 'static> {
    tx: watch::Sender<Arc<T>>,
    rx: watch::Receiver<Arc<T>>,
}

impl<T: Clone + Send + Sync + 'static> ConfigHolder<T> {
    /// Create a new config holder with an initial value.
    pub fn new(initial: T) -> Self {
        let (tx, rx) = watch::channel(Arc::new(initial));
        Self { tx, rx }
    }

    /// Get a snapshot of the current configuration.
    #[must_use]
    pub fn get(&self) -> Arc<T> {
        Arc::clone(&self.rx.borrow())
    }

    /// Get a receiver that can be cloned to other tasks.
    #[must_use]
    pub fn receiver(&self) -> watch::Receiver<Arc<T>> {
        self.rx.clone()
    }

    /// Update the configuration. All readers see the new value immediately.
    pub fn update(&self, new_config: T) {
        let _ = self.tx.send(Arc::new(new_config));
    }
}

/// AgnosAI runtime configuration (hot-reloadable subset).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RuntimeConfig {
    /// Maximum concurrent crew executions.
    pub max_concurrent_crews: usize,
    /// Default inference timeout in seconds.
    pub inference_timeout_secs: u64,
    /// Whether authentication is enabled.
    pub auth_enabled: bool,
    /// Log level filter string (e.g. "agnosai=info").
    pub log_level: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_crews: 10,
            inference_timeout_secs: 300,
            auth_enabled: false,
            log_level: "agnosai=info".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_holder_initial_value() {
        let holder = ConfigHolder::new(42u32);
        assert_eq!(*holder.get(), 42);
    }

    #[test]
    fn config_holder_update() {
        let holder = ConfigHolder::new("old".to_string());
        assert_eq!(*holder.get(), "old");
        holder.update("new".to_string());
        assert_eq!(*holder.get(), "new");
    }

    #[test]
    fn config_holder_receiver_sees_updates() {
        let holder = ConfigHolder::new(1u64);
        let rx = holder.receiver();
        holder.update(2);
        assert_eq!(**rx.borrow(), 2);
    }

    #[test]
    fn runtime_config_defaults() {
        let cfg = RuntimeConfig::default();
        assert_eq!(cfg.max_concurrent_crews, 10);
        assert_eq!(cfg.inference_timeout_secs, 300);
        assert!(!cfg.auth_enabled);
    }

    #[test]
    fn runtime_config_serde_roundtrip() {
        let cfg = RuntimeConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: RuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_concurrent_crews, cfg.max_concurrent_crews);
    }
}
