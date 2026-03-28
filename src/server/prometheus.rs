//! Prometheus-compatible metrics collection and exposition.
//!
//! Provides `AgnosMetrics` with atomic counters and gauges for crew execution,
//! task completion, inference tokens, and cost tracking. The `AgnosMetrics::gather`
//! method formats all metrics in Prometheus text exposition format.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

/// Atomic floating-point accumulator stored as fixed-point (microdollars).
///
/// We store USD * 1_000_000 as u64 to allow atomic operations on cost values.
const MICRODOLLAR: f64 = 1_000_000.0;

/// Prometheus-compatible metrics for AgnosAI.
///
/// All counters use relaxed atomic ordering — they are monotonically increasing
/// and do not require cross-thread synchronization barriers.
#[derive(Debug)]
#[non_exhaustive]
pub struct AgnosMetrics {
    /// Total number of crews created (monotonic counter).
    crews_total: AtomicU64,
    /// Number of currently active (running) crews (gauge).
    crews_active: AtomicU64,
    /// Total tasks completed successfully (monotonic counter).
    tasks_completed: AtomicU64,
    /// Total tasks that failed (monotonic counter).
    tasks_failed: AtomicU64,
    /// Total inference tokens processed (monotonic counter).
    inference_tokens_total: AtomicU64,
    /// Total inference cost in microdollars (monotonic counter).
    inference_cost_micro_usd: AtomicU64,
}

impl Default for AgnosMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl AgnosMetrics {
    /// Create a new metrics instance with all counters at zero.
    #[must_use]
    pub fn new() -> Self {
        Self {
            crews_total: AtomicU64::new(0),
            crews_active: AtomicU64::new(0),
            tasks_completed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            inference_tokens_total: AtomicU64::new(0),
            inference_cost_micro_usd: AtomicU64::new(0),
        }
    }

    /// Record that a new crew has started execution.
    pub fn record_crew_started(&self) {
        self.crews_total.fetch_add(1, Ordering::Relaxed);
        self.crews_active.fetch_add(1, Ordering::Relaxed);
        tracing::trace!("metrics: crew started");
    }

    /// Record that a crew has completed (successfully or otherwise).
    pub fn record_crew_completed(&self) {
        // Saturating subtract: active gauge should not underflow.
        self.crews_active
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(1))
            })
            .ok();
        tracing::trace!("metrics: crew completed");
    }

    /// Record a successful task completion.
    pub fn record_task_completed(&self) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed task.
    pub fn record_task_failed(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an inference call with token count and cost.
    pub fn record_inference(&self, tokens: u64, cost_usd: f64) {
        self.inference_tokens_total
            .fetch_add(tokens, Ordering::Relaxed);
        let micro = (cost_usd * MICRODOLLAR) as u64;
        self.inference_cost_micro_usd
            .fetch_add(micro, Ordering::Relaxed);
        tracing::trace!(tokens, cost_usd, "metrics: inference recorded");
    }

    /// Current crews_total counter value.
    #[must_use]
    #[inline]
    pub fn crews_total(&self) -> u64 {
        self.crews_total.load(Ordering::Relaxed)
    }

    /// Current crews_active gauge value.
    #[must_use]
    #[inline]
    pub fn crews_active(&self) -> u64 {
        self.crews_active.load(Ordering::Relaxed)
    }

    /// Current tasks_completed counter value.
    #[must_use]
    #[inline]
    pub fn tasks_completed(&self) -> u64 {
        self.tasks_completed.load(Ordering::Relaxed)
    }

    /// Current tasks_failed counter value.
    #[must_use]
    #[inline]
    pub fn tasks_failed(&self) -> u64 {
        self.tasks_failed.load(Ordering::Relaxed)
    }

    /// Current inference_tokens_total counter value.
    #[must_use]
    #[inline]
    pub fn inference_tokens_total(&self) -> u64 {
        self.inference_tokens_total.load(Ordering::Relaxed)
    }

    /// Current inference cost in USD (from microdollar accumulator).
    #[must_use]
    #[inline]
    pub fn inference_cost_usd(&self) -> f64 {
        self.inference_cost_micro_usd.load(Ordering::Relaxed) as f64 / MICRODOLLAR
    }

    /// Gather all metrics in Prometheus text exposition format.
    ///
    /// Each metric includes a `# HELP` and `# TYPE` comment line followed
    /// by the metric value.
    #[must_use]
    pub fn gather(&self) -> String {
        let mut out = String::with_capacity(1024);

        let _ = writeln!(
            out,
            "# HELP agnosai_crews_total Total number of crews created."
        );
        let _ = writeln!(out, "# TYPE agnosai_crews_total counter");
        let _ = writeln!(out, "agnosai_crews_total {}", self.crews_total());

        let _ = writeln!(
            out,
            "# HELP agnosai_crews_active Number of currently active crews."
        );
        let _ = writeln!(out, "# TYPE agnosai_crews_active gauge");
        let _ = writeln!(out, "agnosai_crews_active {}", self.crews_active());

        let _ = writeln!(
            out,
            "# HELP agnosai_tasks_completed_total Total tasks completed successfully."
        );
        let _ = writeln!(out, "# TYPE agnosai_tasks_completed_total counter");
        let _ = writeln!(
            out,
            "agnosai_tasks_completed_total {}",
            self.tasks_completed()
        );

        let _ = writeln!(
            out,
            "# HELP agnosai_tasks_failed_total Total tasks that failed."
        );
        let _ = writeln!(out, "# TYPE agnosai_tasks_failed_total counter");
        let _ = writeln!(out, "agnosai_tasks_failed_total {}", self.tasks_failed());

        let _ = writeln!(
            out,
            "# HELP agnosai_inference_tokens_total Total inference tokens processed."
        );
        let _ = writeln!(out, "# TYPE agnosai_inference_tokens_total counter");
        let _ = writeln!(
            out,
            "agnosai_inference_tokens_total {}",
            self.inference_tokens_total()
        );

        let _ = writeln!(
            out,
            "# HELP agnosai_inference_cost_usd_total Total inference cost in USD."
        );
        let _ = writeln!(out, "# TYPE agnosai_inference_cost_usd_total counter");
        let _ = writeln!(
            out,
            "agnosai_inference_cost_usd_total {:.6}",
            self.inference_cost_usd()
        );

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_metrics_all_zero() {
        let m = AgnosMetrics::new();
        assert_eq!(m.crews_total(), 0);
        assert_eq!(m.crews_active(), 0);
        assert_eq!(m.tasks_completed(), 0);
        assert_eq!(m.tasks_failed(), 0);
        assert_eq!(m.inference_tokens_total(), 0);
        assert!((m.inference_cost_usd() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn record_crew_lifecycle() {
        let m = AgnosMetrics::new();
        m.record_crew_started();
        assert_eq!(m.crews_total(), 1);
        assert_eq!(m.crews_active(), 1);

        m.record_crew_started();
        assert_eq!(m.crews_total(), 2);
        assert_eq!(m.crews_active(), 2);

        m.record_crew_completed();
        assert_eq!(m.crews_total(), 2);
        assert_eq!(m.crews_active(), 1);

        m.record_crew_completed();
        assert_eq!(m.crews_active(), 0);
    }

    #[test]
    fn crew_completed_does_not_underflow() {
        let m = AgnosMetrics::new();
        // Complete without starting — should stay at 0.
        m.record_crew_completed();
        assert_eq!(m.crews_active(), 0);
    }

    #[test]
    fn record_tasks() {
        let m = AgnosMetrics::new();
        m.record_task_completed();
        m.record_task_completed();
        m.record_task_failed();
        assert_eq!(m.tasks_completed(), 2);
        assert_eq!(m.tasks_failed(), 1);
    }

    #[test]
    fn record_inference() {
        let m = AgnosMetrics::new();
        m.record_inference(1000, 0.05);
        m.record_inference(2000, 0.10);
        assert_eq!(m.inference_tokens_total(), 3000);
        assert!((m.inference_cost_usd() - 0.15).abs() < 1e-6);
    }

    #[test]
    fn gather_format() {
        let m = AgnosMetrics::new();
        m.record_crew_started();
        m.record_task_completed();
        m.record_inference(500, 0.025);

        let output = m.gather();

        assert!(output.contains("# TYPE agnosai_crews_total counter"));
        assert!(output.contains("agnosai_crews_total 1"));
        assert!(output.contains("# TYPE agnosai_crews_active gauge"));
        assert!(output.contains("agnosai_crews_active 1"));
        assert!(output.contains("agnosai_tasks_completed_total 1"));
        assert!(output.contains("agnosai_tasks_failed_total 0"));
        assert!(output.contains("agnosai_inference_tokens_total 500"));
        assert!(output.contains("agnosai_inference_cost_usd_total 0.025000"));
    }

    #[test]
    fn gather_empty_metrics() {
        let m = AgnosMetrics::new();
        let output = m.gather();
        // Should contain all metric families even when zero.
        assert!(output.contains("agnosai_crews_total 0"));
        assert!(output.contains("agnosai_crews_active 0"));
        assert!(output.contains("agnosai_tasks_completed_total 0"));
        assert!(output.contains("agnosai_tasks_failed_total 0"));
        assert!(output.contains("agnosai_inference_tokens_total 0"));
        assert!(output.contains("agnosai_inference_cost_usd_total 0.000000"));
    }

    #[test]
    fn default_is_same_as_new() {
        let d = AgnosMetrics::default();
        assert_eq!(d.crews_total(), 0);
        assert_eq!(d.crews_active(), 0);
    }
}
