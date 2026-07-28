//! Per-endpoint rate limiting backed by majra's token bucket limiter.
//!
//! Provides an axum middleware layer that enforces per-IP request limits
//! using majra's `RateLimiter`. Returns HTTP 429 when a client exceeds
//! their bucket.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use majra::ratelimit::RateLimiter;
use tracing::warn;

/// Shared rate limiter state.
pub struct RateLimitState {
    limiter: RateLimiter,
    /// How often to evict stale keys.
    eviction_interval: Duration,
}

impl RateLimitState {
    /// Create a new rate limiter.
    ///
    /// `rate` — requests per second per key.
    /// `burst` — maximum burst size per key.
    pub fn new(rate: f64, burst: usize) -> Self {
        Self {
            limiter: RateLimiter::new(rate, burst),
            eviction_interval: Duration::from_secs(300),
        }
    }

    /// Check if a request from the given key is allowed.
    #[must_use]
    pub fn check(&self, key: &str) -> bool {
        self.limiter.check(key)
    }

    /// Evict stale keys older than the configured interval.
    #[must_use]
    pub fn evict_stale(&self) -> usize {
        self.limiter.evict_stale(self.eviction_interval)
    }

    /// Get current statistics.
    pub fn stats(&self) -> majra::ratelimit::RateLimitStats {
        self.limiter.stats()
    }
}

/// Extract the client key from a request (IP address or fallback).
fn extract_client_key(req: &Request<Body>) -> String {
    // Try X-Forwarded-For first, then X-Real-IP, then peer address.
    if let Some(xff) = req.headers().get("x-forwarded-for")
        && let Ok(s) = xff.to_str()
        && let Some(first_ip) = s.split(',').next()
    {
        return first_ip.trim().to_string();
    }
    if let Some(xri) = req.headers().get("x-real-ip")
        && let Ok(s) = xri.to_str()
    {
        return s.trim().to_string();
    }
    // Fallback: use a generic key (single-bucket for all clients).
    "unknown".to_string()
}

/// Axum middleware that enforces rate limiting.
///
/// Returns HTTP 429 Too Many Requests when the client's bucket is empty.
pub async fn rate_limit_middleware(req: Request<Body>, next: Next) -> Response {
    // Extract rate limiter from extensions (set by the layer).
    let limiter = req.extensions().get::<Arc<RateLimitState>>();
    if let Some(limiter) = limiter {
        let key = extract_client_key(&req);
        if !limiter.check(&key) {
            warn!(client = %key, "rate limit exceeded");
            return StatusCode::TOO_MANY_REQUESTS.into_response();
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_state_allows_within_burst() {
        let state = RateLimitState::new(10.0, 5);
        for _ in 0..5 {
            assert!(state.check("client-1"));
        }
        // 6th request should be rejected (burst exhausted).
        assert!(!state.check("client-1"));
    }

    #[test]
    fn rate_limit_separate_keys() {
        let state = RateLimitState::new(10.0, 2);
        assert!(state.check("client-a"));
        assert!(state.check("client-a"));
        assert!(!state.check("client-a")); // Exhausted.
        assert!(state.check("client-b")); // Different key, still has tokens.
    }

    #[test]
    fn rate_limit_stats() {
        let state = RateLimitState::new(10.0, 1);
        let _ = state.check("key");
        let _ = state.check("key"); // Should be rejected.
        let stats = state.stats();
        assert_eq!(stats.total_allowed, 1);
        assert_eq!(stats.total_rejected, 1);
    }

    #[test]
    fn evict_stale_returns_count() {
        let state = RateLimitState::new(10.0, 5);
        let _ = state.check("key-a");
        // Nothing is stale yet.
        assert_eq!(state.evict_stale(), 0);
    }

    #[test]
    fn extract_key_from_xff_header() {
        let req = Request::builder()
            .header("x-forwarded-for", "1.2.3.4, 5.6.7.8")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_client_key(&req), "1.2.3.4");
    }

    #[test]
    fn extract_key_from_xri_header() {
        let req = Request::builder()
            .header("x-real-ip", "10.0.0.1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_client_key(&req), "10.0.0.1");
    }

    #[test]
    fn extract_key_fallback() {
        let req = Request::builder().body(Body::empty()).unwrap();
        assert_eq!(extract_client_key(&req), "unknown");
    }
}
