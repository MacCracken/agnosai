//! Provider health tracking with ring buffer.
//!
//! Maintains the last N inference results per provider. A provider is
//! considered unhealthy when it accumulates 3+ consecutive failures,
//! triggering failover to the next provider. A single success resets
//! the consecutive-failure count.

use std::collections::VecDeque;

/// Ring-buffer health tracker for a single provider.
pub struct ProviderHealth {
    buffer: VecDeque<bool>,
    capacity: usize,
}

impl ProviderHealth {
    /// Create a new tracker with capacity 5.
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(5),
            capacity: 5,
        }
    }

    /// Create a tracker with a custom capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Record an inference outcome (true = success, false = failure).
    pub fn record(&mut self, success: bool) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(success);
    }

    /// A provider is healthy if it does NOT have 3+ consecutive failures
    /// at the tail of the buffer.
    pub fn is_healthy(&self) -> bool {
        let consecutive_failures = self
            .buffer
            .iter()
            .rev()
            .take_while(|&&ok| !ok)
            .count();
        consecutive_failures < 3
    }

    /// Success rate across all recorded results (0.0–1.0).
    /// Returns 1.0 if no results recorded yet (optimistic default).
    pub fn success_rate(&self) -> f64 {
        if self.buffer.is_empty() {
            return 1.0;
        }
        let successes = self.buffer.iter().filter(|&&ok| ok).count();
        successes as f64 / self.buffer.len() as f64
    }
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_healthy() {
        let h = ProviderHealth::new();
        assert!(h.is_healthy());
        assert_eq!(h.success_rate(), 1.0);
    }

    #[test]
    fn all_successes() {
        let mut h = ProviderHealth::new();
        for _ in 0..5 {
            h.record(true);
        }
        assert!(h.is_healthy());
        assert_eq!(h.success_rate(), 1.0);
    }

    #[test]
    fn two_failures_still_healthy() {
        let mut h = ProviderHealth::new();
        h.record(true);
        h.record(false);
        h.record(false);
        assert!(h.is_healthy());
    }

    #[test]
    fn three_consecutive_failures_unhealthy() {
        let mut h = ProviderHealth::new();
        h.record(true);
        h.record(false);
        h.record(false);
        h.record(false);
        assert!(!h.is_healthy());
    }

    #[test]
    fn success_resets_consecutive_failures() {
        let mut h = ProviderHealth::new();
        h.record(false);
        h.record(false);
        h.record(false);
        assert!(!h.is_healthy());

        h.record(true);
        assert!(h.is_healthy());
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut h = ProviderHealth::new();
        // Fill with successes
        for _ in 0..5 {
            h.record(true);
        }
        // Add 3 failures — pushes out 3 successes
        h.record(false);
        h.record(false);
        h.record(false);
        assert!(!h.is_healthy());
        // Buffer is now [true, true, false, false, false]
        assert_eq!(h.success_rate(), 2.0 / 5.0);
    }

    #[test]
    fn success_rate_mixed() {
        let mut h = ProviderHealth::new();
        h.record(true);
        h.record(false);
        h.record(true);
        h.record(true);
        // 3 / 4 = 0.75
        assert!((h.success_rate() - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn custom_capacity() {
        let mut h = ProviderHealth::with_capacity(3);
        h.record(true);
        h.record(true);
        h.record(true);
        h.record(false); // evicts first true
        assert_eq!(h.success_rate(), 2.0 / 3.0);
    }
}
