//! Token and cost budget enforcement for crew execution.
//!
//! Checks `ResourceBudget` limits before each inference call and aborts
//! with `BudgetExceeded` if a limit has been reached.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::core::ResourceBudget;

/// Error returned when a budget limit is exceeded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BudgetExceeded {
    /// Total token usage exceeded the budget.
    Tokens { used: u64, limit: u64 },
    /// Total cost exceeded the budget.
    Cost { used_units: u64, limit_units: u64 },
}

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tokens { used, limit } => {
                write!(f, "token budget exceeded: {used} used, {limit} limit")
            }
            Self::Cost {
                used_units,
                limit_units,
            } => {
                write!(
                    f,
                    "cost budget exceeded: ${:.4} used, ${:.4} limit",
                    *used_units as f64 / 10_000.0,
                    *limit_units as f64 / 10_000.0
                )
            }
        }
    }
}

/// Tracks token and cost usage against a `ResourceBudget`.
///
/// Thread-safe via atomics — can be shared across concurrent task executions.
pub struct BudgetTracker {
    tokens_used: AtomicU64,
    /// Cost in 1/10000 USD (to avoid floating-point atomics).
    cost_units: AtomicU64,
    max_tokens: Option<u64>,
    /// Stored as 1/10000 USD.
    max_cost_units: Option<u64>,
}

impl BudgetTracker {
    /// Create a new budget tracker from a resource budget.
    #[must_use]
    pub fn new(budget: &ResourceBudget) -> Self {
        Self {
            tokens_used: AtomicU64::new(0),
            cost_units: AtomicU64::new(0),
            max_tokens: budget.max_tokens,
            max_cost_units: budget.max_cost_usd.map(|c| (c * 10_000.0) as u64),
        }
    }

    /// Check whether the budget allows another inference call.
    ///
    /// Returns `Ok(())` if within budget, or `Err(BudgetExceeded)` if a
    /// limit has been reached.
    pub fn check(&self) -> Result<(), BudgetExceeded> {
        if let Some(limit) = self.max_tokens {
            let used = self.tokens_used.load(Ordering::Relaxed);
            if used >= limit {
                warn!(used, limit, "token budget exceeded");
                return Err(BudgetExceeded::Tokens { used, limit });
            }
        }
        if let Some(limit_units) = self.max_cost_units {
            let used_units = self.cost_units.load(Ordering::Relaxed);
            if used_units >= limit_units {
                warn!(
                    used_usd = used_units as f64 / 10_000.0,
                    limit_usd = limit_units as f64 / 10_000.0,
                    "cost budget exceeded"
                );
                return Err(BudgetExceeded::Cost {
                    used_units,
                    limit_units,
                });
            }
        }
        Ok(())
    }

    /// Record token usage from a completed inference call.
    pub fn record_tokens(&self, tokens: u64) {
        self.tokens_used.fetch_add(tokens, Ordering::Relaxed);
    }

    /// Record cost from a completed inference call (in USD).
    pub fn record_cost(&self, cost_usd: f64) {
        let units = (cost_usd * 10_000.0) as u64;
        self.cost_units.fetch_add(units, Ordering::Relaxed);
    }

    /// Current total tokens used.
    #[must_use]
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used.load(Ordering::Relaxed)
    }

    /// Current total cost in USD.
    #[must_use]
    pub fn cost_usd(&self) -> f64 {
        self.cost_units.load(Ordering::Relaxed) as f64 / 10_000.0
    }

    /// Whether any budget limit is configured.
    #[must_use]
    pub fn has_limits(&self) -> bool {
        self.max_tokens.is_some() || self.max_cost_units.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget_with_tokens(max: u64) -> ResourceBudget {
        ResourceBudget::new(Some(max), None, None, None)
    }

    fn budget_with_cost(max_usd: f64) -> ResourceBudget {
        ResourceBudget::new(None, Some(max_usd), None, None)
    }

    #[test]
    fn no_limits_always_passes() {
        let tracker = BudgetTracker::new(&ResourceBudget::default());
        assert!(!tracker.has_limits());
        assert!(tracker.check().is_ok());
    }

    #[test]
    fn token_budget_enforced() {
        let tracker = BudgetTracker::new(&budget_with_tokens(1000));
        assert!(tracker.has_limits());
        assert!(tracker.check().is_ok());

        tracker.record_tokens(500);
        assert!(tracker.check().is_ok());

        tracker.record_tokens(500);
        assert!(matches!(
            tracker.check(),
            Err(BudgetExceeded::Tokens {
                used: 1000,
                limit: 1000
            })
        ));
    }

    #[test]
    fn cost_budget_enforced() {
        let tracker = BudgetTracker::new(&budget_with_cost(0.10));
        assert!(tracker.check().is_ok());

        tracker.record_cost(0.05);
        assert!(tracker.check().is_ok());

        tracker.record_cost(0.06);
        assert!(matches!(tracker.check(), Err(BudgetExceeded::Cost { .. })));
    }

    #[test]
    fn tokens_used_accumulates() {
        let tracker = BudgetTracker::new(&budget_with_tokens(10_000));
        tracker.record_tokens(100);
        tracker.record_tokens(200);
        assert_eq!(tracker.tokens_used(), 300);
    }

    #[test]
    fn cost_usd_accumulates() {
        let tracker = BudgetTracker::new(&budget_with_cost(1.0));
        tracker.record_cost(0.25);
        tracker.record_cost(0.30);
        assert!((tracker.cost_usd() - 0.55).abs() < 0.001);
    }

    #[test]
    fn budget_exceeded_display() {
        let e = BudgetExceeded::Tokens {
            used: 1500,
            limit: 1000,
        };
        assert!(e.to_string().contains("1500"));
        assert!(e.to_string().contains("1000"));
    }
}
