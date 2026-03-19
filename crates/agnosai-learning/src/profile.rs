//! PerformanceProfile — success rates, duration tracking per agent per action type.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};

/// A single recorded action for an agent.
pub struct ActionRecord {
    pub action_type: String,
    pub duration: Duration,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

/// Tracks performance records per agent, enabling success-rate and duration queries.
pub struct PerformanceProfile {
    records: HashMap<String, Vec<ActionRecord>>,
}

impl PerformanceProfile {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    /// Record an action outcome for the given agent.
    pub fn record(
        &mut self,
        agent_key: &str,
        action_type: &str,
        duration: Duration,
        success: bool,
    ) {
        self.records
            .entry(agent_key.to_string())
            .or_default()
            .push(ActionRecord {
                action_type: action_type.to_string(),
                duration,
                success,
                timestamp: Utc::now(),
            });
    }

    /// Overall success rate for the agent across all action types.
    pub fn success_rate(&self, agent_key: &str) -> Option<f64> {
        let records = self.records.get(agent_key)?;
        if records.is_empty() {
            return None;
        }
        let successes = records.iter().filter(|r| r.success).count();
        Some(successes as f64 / records.len() as f64)
    }

    /// Success rate for a specific action type.
    pub fn success_rate_for_action(
        &self,
        agent_key: &str,
        action_type: &str,
    ) -> Option<f64> {
        let records = self.records.get(agent_key)?;
        let filtered: Vec<_> = records
            .iter()
            .filter(|r| r.action_type == action_type)
            .collect();
        if filtered.is_empty() {
            return None;
        }
        let successes = filtered.iter().filter(|r| r.success).count();
        Some(successes as f64 / filtered.len() as f64)
    }

    /// Average duration across all actions for the agent.
    pub fn avg_duration(&self, agent_key: &str) -> Option<Duration> {
        let records = self.records.get(agent_key)?;
        if records.is_empty() {
            return None;
        }
        let total: Duration = records.iter().map(|r| r.duration).sum();
        Some(total / records.len() as u32)
    }

    /// Average duration for a specific action type.
    pub fn avg_duration_for_action(
        &self,
        agent_key: &str,
        action_type: &str,
    ) -> Option<Duration> {
        let records = self.records.get(agent_key)?;
        let filtered: Vec<_> = records
            .iter()
            .filter(|r| r.action_type == action_type)
            .collect();
        if filtered.is_empty() {
            return None;
        }
        let total: Duration = filtered.iter().map(|r| r.duration).sum();
        Some(total / filtered.len() as u32)
    }

    /// Total number of recorded actions for the agent.
    pub fn total_actions(&self, agent_key: &str) -> usize {
        self.records
            .get(agent_key)
            .map_or(0, |records| records.len())
    }
}

impl Default for PerformanceProfile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_success_rate() {
        let mut profile = PerformanceProfile::new();
        profile.record("agent-a", "build", Duration::from_millis(100), true);
        profile.record("agent-a", "build", Duration::from_millis(200), false);
        profile.record("agent-a", "build", Duration::from_millis(150), true);

        let rate = profile.success_rate("agent-a").unwrap();
        assert!((rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn success_rate_for_action() {
        let mut profile = PerformanceProfile::new();
        profile.record("agent-a", "build", Duration::from_millis(100), true);
        profile.record("agent-a", "test", Duration::from_millis(200), false);
        profile.record("agent-a", "build", Duration::from_millis(150), false);

        assert!((profile.success_rate_for_action("agent-a", "build").unwrap() - 0.5).abs() < 1e-9);
        assert!(
            (profile
                .success_rate_for_action("agent-a", "test")
                .unwrap()
                - 0.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn avg_duration() {
        let mut profile = PerformanceProfile::new();
        profile.record("agent-a", "build", Duration::from_millis(100), true);
        profile.record("agent-a", "build", Duration::from_millis(200), true);

        let avg = profile.avg_duration("agent-a").unwrap();
        assert_eq!(avg, Duration::from_millis(150));
    }

    #[test]
    fn avg_duration_for_action() {
        let mut profile = PerformanceProfile::new();
        profile.record("agent-a", "build", Duration::from_millis(100), true);
        profile.record("agent-a", "test", Duration::from_millis(300), true);
        profile.record("agent-a", "build", Duration::from_millis(200), true);

        let avg = profile
            .avg_duration_for_action("agent-a", "build")
            .unwrap();
        assert_eq!(avg, Duration::from_millis(150));
    }

    #[test]
    fn total_actions() {
        let mut profile = PerformanceProfile::new();
        assert_eq!(profile.total_actions("agent-a"), 0);

        profile.record("agent-a", "build", Duration::from_millis(100), true);
        profile.record("agent-a", "test", Duration::from_millis(200), false);
        assert_eq!(profile.total_actions("agent-a"), 2);
    }

    #[test]
    fn empty_agent_returns_none() {
        let profile = PerformanceProfile::new();
        assert!(profile.success_rate("nonexistent").is_none());
        assert!(profile
            .success_rate_for_action("nonexistent", "build")
            .is_none());
        assert!(profile.avg_duration("nonexistent").is_none());
        assert!(profile
            .avg_duration_for_action("nonexistent", "build")
            .is_none());
    }

    #[test]
    fn no_actions_of_type_returns_none() {
        let mut profile = PerformanceProfile::new();
        profile.record("agent-a", "build", Duration::from_millis(100), true);

        assert!(profile
            .success_rate_for_action("agent-a", "deploy")
            .is_none());
        assert!(profile
            .avg_duration_for_action("agent-a", "deploy")
            .is_none());
    }
}
