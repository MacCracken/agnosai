//! Per-agent token accounting and cost budgets.

use std::collections::HashMap;
use std::fmt;

/// Per-agent token budget tracker with optional global limit.
pub struct TokenBudget {
    budgets: HashMap<String, AgentBudget>,
    global_limit: Option<u64>,
    global_used: u64,
}

/// Budget state for a single agent.
pub struct AgentBudget {
    pub limit: u64,
    pub used: u64,
}

/// Error returned when a usage recording would exceed a budget.
#[derive(Debug)]
pub struct BudgetExceeded {
    pub agent_key: String,
    pub limit: u64,
    pub used: u64,
    pub requested: u64,
}

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "token budget exceeded for '{}': limit={}, used={}, requested={}",
            self.agent_key, self.limit, self.used, self.requested
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// Snapshot of all budget state.
pub struct BudgetSummary {
    pub global_used: u64,
    pub global_limit: Option<u64>,
    /// `(agent_key, used, limit)` tuples.
    pub agents: Vec<(String, u64, u64)>,
}

impl TokenBudget {
    /// Create a budget tracker with no global limit.
    pub fn new() -> Self {
        Self {
            budgets: HashMap::new(),
            global_limit: None,
            global_used: 0,
        }
    }

    /// Create a budget tracker with a global token limit.
    pub fn with_global_limit(limit: u64) -> Self {
        Self {
            budgets: HashMap::new(),
            global_limit: Some(limit),
            global_used: 0,
        }
    }

    /// Set or update the token limit for a specific agent.
    pub fn set_agent_limit(&mut self, agent_key: &str, limit: u64) {
        let budget = self
            .budgets
            .entry(agent_key.to_string())
            .or_insert(AgentBudget { limit: 0, used: 0 });
        budget.limit = limit;
    }

    /// Record token usage for an agent. Returns `Err` if the agent or global limit would be exceeded.
    pub fn record_usage(&mut self, agent_key: &str, tokens: u64) -> Result<(), BudgetExceeded> {
        // Check global limit first.
        if let Some(global_limit) = self.global_limit
            && self.global_used + tokens > global_limit
        {
            return Err(BudgetExceeded {
                agent_key: "__global__".to_string(),
                limit: global_limit,
                used: self.global_used,
                requested: tokens,
            });
        }

        // Check agent limit if one is set.
        if let Some(budget) = self.budgets.get(agent_key)
            && budget.used + tokens > budget.limit
        {
            return Err(BudgetExceeded {
                agent_key: agent_key.to_string(),
                limit: budget.limit,
                used: budget.used,
                requested: tokens,
            });
        }

        // Apply usage.
        self.global_used += tokens;
        if let Some(budget) = self.budgets.get_mut(agent_key) {
            budget.used += tokens;
        }

        Ok(())
    }

    /// Remaining tokens for a specific agent, or `None` if no limit is set.
    pub fn remaining(&self, agent_key: &str) -> Option<u64> {
        self.budgets
            .get(agent_key)
            .map(|b| b.limit.saturating_sub(b.used))
    }

    /// Remaining tokens globally, or `None` if no global limit.
    pub fn global_remaining(&self) -> Option<u64> {
        self.global_limit
            .map(|limit| limit.saturating_sub(self.global_used))
    }

    /// Reset all usage counters (keeps limits).
    pub fn reset(&mut self) {
        self.global_used = 0;
        for budget in self.budgets.values_mut() {
            budget.used = 0;
        }
    }

    /// Reset usage for a single agent (keeps limit).
    pub fn reset_agent(&mut self, agent_key: &str) {
        if let Some(budget) = self.budgets.get_mut(agent_key) {
            budget.used = 0;
        }
    }

    /// Get a summary of all budget state.
    pub fn usage_summary(&self) -> BudgetSummary {
        let agents = self
            .budgets
            .iter()
            .map(|(k, b)| (k.clone(), b.used, b.limit))
            .collect();

        BudgetSummary {
            global_used: self.global_used,
            global_limit: self.global_limit,
            agents,
        }
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_check_limit() {
        let mut budget = TokenBudget::new();
        budget.set_agent_limit("agent-1", 1000);
        assert_eq!(budget.remaining("agent-1"), Some(1000));
    }

    #[test]
    fn record_usage_within_limit() {
        let mut budget = TokenBudget::new();
        budget.set_agent_limit("agent-1", 1000);
        budget.record_usage("agent-1", 400).unwrap();
        assert_eq!(budget.remaining("agent-1"), Some(600));
    }

    #[test]
    fn record_usage_exceeds_agent_limit() {
        let mut budget = TokenBudget::new();
        budget.set_agent_limit("agent-1", 100);
        budget.record_usage("agent-1", 60).unwrap();

        let err = budget.record_usage("agent-1", 50).unwrap_err();
        assert_eq!(err.agent_key, "agent-1");
        assert_eq!(err.limit, 100);
        assert_eq!(err.used, 60);
        assert_eq!(err.requested, 50);
    }

    #[test]
    fn global_limit() {
        let mut budget = TokenBudget::with_global_limit(500);
        budget.record_usage("a", 200).unwrap();
        budget.record_usage("b", 200).unwrap();
        assert_eq!(budget.global_remaining(), Some(100));

        let err = budget.record_usage("c", 150).unwrap_err();
        assert_eq!(err.agent_key, "__global__");
        assert_eq!(err.limit, 500);
    }

    #[test]
    fn no_global_limit_returns_none() {
        let budget = TokenBudget::new();
        assert_eq!(budget.global_remaining(), None);
    }

    #[test]
    fn unknown_agent_remaining_is_none() {
        let budget = TokenBudget::new();
        assert_eq!(budget.remaining("unknown"), None);
    }

    #[test]
    fn reset_clears_usage_keeps_limits() {
        let mut budget = TokenBudget::with_global_limit(1000);
        budget.set_agent_limit("agent-1", 500);
        budget.record_usage("agent-1", 300).unwrap();

        budget.reset();

        assert_eq!(budget.remaining("agent-1"), Some(500));
        assert_eq!(budget.global_remaining(), Some(1000));
    }

    #[test]
    fn reset_agent() {
        let mut budget = TokenBudget::new();
        budget.set_agent_limit("agent-1", 500);
        budget.record_usage("agent-1", 300).unwrap();

        budget.reset_agent("agent-1");
        assert_eq!(budget.remaining("agent-1"), Some(500));
    }

    #[test]
    fn usage_summary() {
        let mut budget = TokenBudget::with_global_limit(10000);
        budget.set_agent_limit("a", 5000);
        budget.set_agent_limit("b", 3000);
        budget.record_usage("a", 1000).unwrap();
        budget.record_usage("b", 500).unwrap();

        let summary = budget.usage_summary();
        assert_eq!(summary.global_used, 1500);
        assert_eq!(summary.global_limit, Some(10000));
        assert_eq!(summary.agents.len(), 2);
    }

    #[test]
    fn usage_without_agent_limit_still_tracks_global() {
        let mut budget = TokenBudget::with_global_limit(1000);
        // No agent limit set, but global limit applies.
        budget.record_usage("no-limit-agent", 500).unwrap();
        assert_eq!(budget.global_remaining(), Some(500));
    }
}
