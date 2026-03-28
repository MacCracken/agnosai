//! Exponential backoff retry for LLM inference calls.
//!
//! Wraps inference requests with configurable retry logic for transient
//! failures (rate limits, server errors, timeouts).

use std::time::Duration;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Configuration for inference retry behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries).
    pub max_retries: u32,
    /// Base delay before the first retry.
    pub base_delay: Duration,
    /// Maximum delay between retries (caps exponential growth).
    pub max_delay: Duration,
    /// Whether to add random jitter to delays.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// No retries — fail immediately on first error.
    #[must_use]
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Aggressive retry for high-priority tasks.
    #[must_use]
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(60),
            jitter: true,
        }
    }
}

/// Compute the delay for a given retry attempt using exponential backoff.
///
/// Formula: `min(base_delay * 2^attempt, max_delay) + jitter`
#[must_use]
pub fn compute_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let exp_delay = config
        .base_delay
        .saturating_mul(1u32.checked_shl(attempt).unwrap_or(u32::MAX));
    let capped = exp_delay.min(config.max_delay);

    if config.jitter {
        let jitter_range = capped.as_millis() as u64 / 4; // +0–25% jitter
        if jitter_range > 0 {
            let jitter = rand::rng().random_range(0..=jitter_range);
            capped + Duration::from_millis(jitter)
        } else {
            capped
        }
    } else {
        capped
    }
}

/// Determine whether an error message indicates a transient/retryable failure.
#[must_use]
pub fn is_retryable(error_msg: &str) -> bool {
    let lower = error_msg.to_ascii_lowercase();
    lower.contains("rate limit")
        || lower.contains("429")
        || lower.contains("503")
        || lower.contains("502")
        || lower.contains("timeout")
        || lower.contains("timed out")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
        || lower.contains("temporarily unavailable")
        || lower.contains("service unavailable")
        || lower.contains("server error")
        || lower.contains("500")
        || lower.contains("overloaded")
        || lower.contains("capacity")
}

/// Execute an async inference call with retry.
///
/// Returns the first successful result or the last error after all retries
/// are exhausted.
pub async fn with_retry<F, Fut, T, E>(
    config: &RetryConfig,
    task_id: &str,
    mut call: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_err = None;

    for attempt in 0..=config.max_retries {
        match call().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!(task_id, attempt, "inference succeeded after retry");
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= config.max_retries {
                    warn!(
                        task_id,
                        attempt,
                        error = %e,
                        "inference failed, retries exhausted"
                    );
                    return Err(e);
                }

                let retryable = is_retryable(&e.to_string());
                if !retryable {
                    warn!(
                        task_id,
                        error = %e,
                        "inference failed with non-retryable error"
                    );
                    return Err(e);
                }

                let delay = compute_delay(config, attempt);
                warn!(
                    task_id,
                    attempt,
                    delay_ms = delay.as_millis() as u64,
                    error = %e,
                    "inference failed, retrying"
                );
                last_err = Some(e);
                tokio::time::sleep(delay).await;
            }
        }
    }

    Err(last_err.expect("unreachable: loop must have run at least once"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn default_config() {
        let c = RetryConfig::default();
        assert_eq!(c.max_retries, 3);
        assert_eq!(c.base_delay, Duration::from_millis(500));
        assert!(c.jitter);
    }

    #[test]
    fn none_config_no_retries() {
        let c = RetryConfig::none();
        assert_eq!(c.max_retries, 0);
    }

    #[test]
    fn compute_delay_exponential() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: false,
            ..Default::default()
        };
        assert_eq!(compute_delay(&config, 0), Duration::from_millis(100));
        assert_eq!(compute_delay(&config, 1), Duration::from_millis(200));
        assert_eq!(compute_delay(&config, 2), Duration::from_millis(400));
        assert_eq!(compute_delay(&config, 3), Duration::from_millis(800));
    }

    #[test]
    fn compute_delay_capped() {
        let config = RetryConfig {
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            jitter: false,
            ..Default::default()
        };
        assert_eq!(compute_delay(&config, 10), Duration::from_secs(5));
    }

    #[test]
    fn compute_delay_with_jitter() {
        let config = RetryConfig {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: true,
            ..Default::default()
        };
        let delay = compute_delay(&config, 0);
        // Base is 100ms, jitter adds up to 25ms.
        assert!(delay >= Duration::from_millis(100));
        assert!(delay <= Duration::from_millis(125));
    }

    #[test]
    fn is_retryable_rate_limit() {
        assert!(is_retryable("rate limit exceeded"));
        assert!(is_retryable("HTTP 429 Too Many Requests"));
        assert!(is_retryable("503 Service Unavailable"));
        assert!(is_retryable("connection timed out"));
        assert!(is_retryable("server overloaded"));
    }

    #[test]
    fn is_retryable_non_retryable() {
        assert!(!is_retryable("invalid API key"));
        assert!(!is_retryable("model not found"));
        assert!(!is_retryable("invalid request"));
    }

    #[tokio::test]
    async fn with_retry_succeeds_first_try() {
        let config = RetryConfig::none();
        let result: Result<&str, String> = with_retry(&config, "t1", || async { Ok("ok") }).await;
        assert_eq!(result.unwrap(), "ok");
    }

    #[tokio::test]
    async fn with_retry_succeeds_after_transient_failure() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: false,
        };
        let counter = AtomicU32::new(0);
        let result: Result<&str, String> = with_retry(&config, "t2", || {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            async move {
                if n < 2 {
                    Err("503 Service Unavailable".to_string())
                } else {
                    Ok("recovered")
                }
            }
        })
        .await;
        assert_eq!(result.unwrap(), "recovered");
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn with_retry_exhausts_retries() {
        let config = RetryConfig {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: false,
        };
        let result: Result<&str, String> = with_retry(&config, "t3", || async {
            Err::<&str, String>("rate limit exceeded".into())
        })
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn with_retry_skips_non_retryable() {
        let config = RetryConfig::default();
        let counter = AtomicU32::new(0);
        let result: Result<&str, String> = with_retry(&config, "t4", || {
            counter.fetch_add(1, Ordering::SeqCst);
            async { Err::<&str, String>("invalid API key".into()) }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // No retries
    }
}
