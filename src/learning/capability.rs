//! Dynamic capability confidence scoring with trend detection.

use std::collections::HashMap;

/// Direction the capability confidence is moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Improving,
    Stable,
    Declining,
}

/// Score data for a single capability.
pub struct CapabilityScore {
    pub confidence: f64,
    pub successes: u32,
    pub failures: u32,
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

    fn update_trend(&mut self) {
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
pub struct CapabilityScorer {
    scores: HashMap<String, CapabilityScore>,
}

impl CapabilityScorer {
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
        }
    }

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
    pub fn confidence(&self, capability: &str) -> f64 {
        self.scores.get(capability).map_or(0.5, |s| s.confidence)
    }

    /// Get the trend for a capability. Returns Stable for unknown capabilities.
    pub fn trend(&self, capability: &str) -> Trend {
        self.scores
            .get(capability)
            .map_or(Trend::Stable, |s| s.trend)
    }

    /// Return all capability names and their scores.
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
