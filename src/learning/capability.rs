//! Dynamic capability confidence scoring with trend detection.

use std::collections::HashMap;

/// Direction the capability confidence is moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Trend {
    /// Confidence is increasing over recent observations.
    Improving,
    /// Confidence is roughly constant.
    Stable,
    /// Confidence is decreasing over recent observations.
    Declining,
}

/// Score data for a single capability.
#[non_exhaustive]
pub struct CapabilityScore {
    /// Current confidence level (0.0 to 1.0).
    pub confidence: f64,
    /// Total number of successful outcomes.
    pub successes: u32,
    /// Total number of failed outcomes.
    pub failures: u32,
    /// Direction the confidence is trending.
    pub trend: Trend,
    /// Recent results (true = success), used for trend detection.
    recent: Vec<bool>,
}

impl CapabilityScore {
    fn new() -> Self {
        Self {
            confidence: 0.5,
            successes: 0,
            failures: 0,
            trend: Trend::Stable,
            recent: Vec::new(),
        }
    }

    fn update_confidence(&mut self) {
        let total = self.successes + self.failures;
        if total > 0 {
            self.confidence = self.successes as f64 / total as f64;
        }
    }

    /// Maximum number of recent observations kept for trend detection.
    const MAX_RECENT: usize = 64;

    fn update_trend(&mut self) {
        // Trim to bounded window to prevent unbounded growth.
        if self.recent.len() > Self::MAX_RECENT {
            let drain_count = self.recent.len() - Self::MAX_RECENT;
            self.recent.drain(..drain_count);
        }

        let window = 5;
        if self.recent.len() < window {
            self.trend = Trend::Stable;
            return;
        }

        let last_n: &[bool] = &self.recent[self.recent.len() - window..];
        let recent_rate = last_n.iter().filter(|&&b| b).count() as f64 / window as f64;

        let diff = recent_rate - self.confidence;
        if diff > 0.1 {
            self.trend = Trend::Improving;
        } else if diff < -0.1 {
            self.trend = Trend::Declining;
        } else {
            self.trend = Trend::Stable;
        }
    }
}

/// Tracks capability confidence scores across multiple capabilities.
#[non_exhaustive]
pub struct CapabilityScorer {
    scores: HashMap<String, CapabilityScore>,
}

impl CapabilityScorer {
    /// Create a new scorer with no recorded capabilities.
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
        }
    }

    /// Record a successful outcome for the given capability.
    pub fn record_success(&mut self, capability: &str) {
        let score = self
            .scores
            .entry(capability.to_string())
            .or_insert_with(CapabilityScore::new);
        score.successes += 1;
        score.recent.push(true);
        score.update_confidence();
        score.update_trend();
    }

    /// Record a failed outcome for the given capability.
    pub fn record_failure(&mut self, capability: &str) {
        let score = self
            .scores
            .entry(capability.to_string())
            .or_insert_with(CapabilityScore::new);
        score.failures += 1;
        score.recent.push(false);
        score.update_confidence();
        score.update_trend();
    }

    /// Get confidence for a capability. Returns 0.5 for unknown capabilities.
    #[must_use]
    pub fn confidence(&self, capability: &str) -> f64 {
        self.scores.get(capability).map_or(0.5, |s| s.confidence)
    }

    /// Get the trend for a capability. Returns Stable for unknown capabilities.
    #[must_use]
    pub fn trend(&self, capability: &str) -> Trend {
        self.scores
            .get(capability)
            .map_or(Trend::Stable, |s| s.trend)
    }

    /// Return all capability names and their scores.
    #[must_use]
    pub fn all_scores(&self) -> Vec<(&str, &CapabilityScore)> {
        self.scores.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }
}

impl Default for CapabilityScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_confidence_is_default() {
        let scorer = CapabilityScorer::new();
        assert!((scorer.confidence("unknown") - 0.5).abs() < 1e-9);
    }

    #[test]
    fn confidence_improves_after_success() {
        let mut scorer = CapabilityScorer::new();
        scorer.record_success("build");
        assert!((scorer.confidence("build") - 1.0).abs() < 1e-9);

        scorer.record_failure("build");
        assert!((scorer.confidence("build") - 0.5).abs() < 1e-9);

        scorer.record_success("build");
        let expected = 2.0 / 3.0;
        assert!((scorer.confidence("build") - expected).abs() < 1e-9);
    }

    #[test]
    fn confidence_declines_after_failure() {
        let mut scorer = CapabilityScorer::new();
        scorer.record_failure("deploy");
        assert!((scorer.confidence("deploy") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn trend_improving() {
        let mut scorer = CapabilityScorer::new();
        // Create a poor overall record, then recent successes.
        for _ in 0..10 {
            scorer.record_failure("test");
        }
        // Overall rate: 0/10 = 0.0
        // Now add 5 successes as recent.
        for _ in 0..5 {
            scorer.record_success("test");
        }
        // Overall: 5/15 = 0.333, last 5: 5/5 = 1.0 -> improving.
        assert_eq!(scorer.trend("test"), Trend::Improving);
    }

    #[test]
    fn trend_declining() {
        let mut scorer = CapabilityScorer::new();
        // Create a good overall record, then recent failures.
        for _ in 0..10 {
            scorer.record_success("test");
        }
        // Overall rate: 10/10 = 1.0
        // Now add 5 failures as recent.
        for _ in 0..5 {
            scorer.record_failure("test");
        }
        // Overall: 10/15 = 0.667, last 5: 0/5 = 0.0 -> declining.
        assert_eq!(scorer.trend("test"), Trend::Declining);
    }

    #[test]
    fn trend_stable_with_few_records() {
        let mut scorer = CapabilityScorer::new();
        scorer.record_success("test");
        scorer.record_failure("test");
        // Fewer than 5 recent records -> Stable.
        assert_eq!(scorer.trend("test"), Trend::Stable);
    }

    #[test]
    fn unknown_capability_trend_is_stable() {
        let scorer = CapabilityScorer::new();
        assert_eq!(scorer.trend("nonexistent"), Trend::Stable);
    }

    #[test]
    fn all_scores_returns_all() {
        let mut scorer = CapabilityScorer::new();
        scorer.record_success("a");
        scorer.record_failure("b");
        let scores = scorer.all_scores();
        assert_eq!(scores.len(), 2);
    }
}
