//! Multi-tenancy support with per-tenant budget enforcement.
//!
//! Provides a `TenantRegistry` backed by `DashMap` for concurrent tenant
//! registration and budget lookup, plus budget checking for token, cost,
//! and concurrency limits.

use dashmap::DashMap;

/// Unique identifier for a tenant.
pub type TenantId = String;

/// Budget constraints for a single tenant.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct TenantBudget {
    /// Maximum total tokens the tenant may consume.
    pub max_tokens: u64,
    /// Maximum total inference cost in USD.
    pub max_cost_usd: f64,
    /// Maximum number of concurrent crews the tenant may run.
    pub max_concurrent_crews: usize,
}

impl TenantBudget {
    /// Create a new tenant budget with the given limits.
    pub fn new(max_tokens: u64, max_cost_usd: f64, max_concurrent_crews: usize) -> Self {
        Self {
            max_tokens,
            max_cost_usd,
            max_concurrent_crews,
        }
    }
}

/// Result of checking a tenant's current usage against their budget.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BudgetCheckResult {
    /// Usage is within limits.
    Ok,
    /// Token limit exceeded.
    TokensExceeded,
    /// Cost limit exceeded.
    CostExceeded,
    /// Concurrent crew limit exceeded.
    ConcurrencyExceeded,
    /// Tenant not found in registry.
    TenantNotFound,
}

/// Thread-safe registry of tenants and their budgets.
///
/// Backed by `DashMap` for concurrent access without external locking.
#[derive(Debug)]
pub struct TenantRegistry {
    budgets: DashMap<TenantId, TenantBudget>,
}

impl Default for TenantRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantRegistry {
    /// Create an empty tenant registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            budgets: DashMap::new(),
        }
    }

    /// Register a tenant with the given budget.
    ///
    /// If the tenant already exists, their budget is replaced.
    pub fn register_tenant(&self, tenant_id: impl Into<TenantId>, budget: TenantBudget) {
        let id = tenant_id.into();
        tracing::info!(tenant_id = %id, "registering tenant");
        self.budgets.insert(id, budget);
    }

    /// Look up a tenant's budget.
    ///
    /// Returns `None` if the tenant is not registered.
    #[must_use]
    pub fn get_budget(&self, tenant_id: &str) -> Option<TenantBudget> {
        self.budgets.get(tenant_id).map(|entry| entry.clone())
    }

    /// Check whether a tenant's current usage is within their budget.
    ///
    /// # Arguments
    ///
    /// * `tenant_id` — the tenant to check
    /// * `used_tokens` — tokens consumed so far
    /// * `used_cost_usd` — cost consumed so far
    /// * `active_crews` — number of crews currently running
    #[must_use]
    pub fn check_tenant_budget(
        &self,
        tenant_id: &str,
        used_tokens: u64,
        used_cost_usd: f64,
        active_crews: usize,
    ) -> BudgetCheckResult {
        let Some(budget) = self.budgets.get(tenant_id) else {
            tracing::warn!(tenant_id, "budget check for unknown tenant");
            return BudgetCheckResult::TenantNotFound;
        };

        if used_tokens > budget.max_tokens {
            tracing::warn!(
                tenant_id,
                used_tokens,
                max_tokens = budget.max_tokens,
                "tenant token limit exceeded"
            );
            return BudgetCheckResult::TokensExceeded;
        }

        if used_cost_usd > budget.max_cost_usd {
            tracing::warn!(
                tenant_id,
                used_cost_usd,
                max_cost_usd = budget.max_cost_usd,
                "tenant cost limit exceeded"
            );
            return BudgetCheckResult::CostExceeded;
        }

        if active_crews > budget.max_concurrent_crews {
            tracing::warn!(
                tenant_id,
                active_crews,
                max_concurrent_crews = budget.max_concurrent_crews,
                "tenant concurrency limit exceeded"
            );
            return BudgetCheckResult::ConcurrencyExceeded;
        }

        BudgetCheckResult::Ok
    }

    /// Remove a tenant from the registry.
    ///
    /// Returns the removed budget, or `None` if not found.
    pub fn remove_tenant(&self, tenant_id: &str) -> Option<TenantBudget> {
        self.budgets.remove(tenant_id).map(|(_, v)| v)
    }

    /// Number of registered tenants.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.budgets.len()
    }

    /// Returns `true` if no tenants are registered.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.budgets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_budget() -> TenantBudget {
        TenantBudget::new(100_000, 10.0, 5)
    }

    #[test]
    fn register_and_get_budget() {
        let registry = TenantRegistry::new();
        registry.register_tenant("tenant-1", sample_budget());
        let budget = registry.get_budget("tenant-1").unwrap();
        assert_eq!(budget.max_tokens, 100_000);
        assert!((budget.max_cost_usd - 10.0).abs() < f64::EPSILON);
        assert_eq!(budget.max_concurrent_crews, 5);
    }

    #[test]
    fn get_budget_unknown_tenant() {
        let registry = TenantRegistry::new();
        assert!(registry.get_budget("unknown").is_none());
    }

    #[test]
    fn register_replaces_existing() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", TenantBudget::new(100, 1.0, 1));
        registry.register_tenant("t1", TenantBudget::new(200, 2.0, 2));
        let budget = registry.get_budget("t1").unwrap();
        assert_eq!(budget.max_tokens, 200);
    }

    #[test]
    fn check_budget_ok() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", sample_budget());
        let result = registry.check_tenant_budget("t1", 50_000, 5.0, 3);
        assert_eq!(result, BudgetCheckResult::Ok);
    }

    #[test]
    fn check_budget_tokens_exceeded() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", sample_budget());
        let result = registry.check_tenant_budget("t1", 100_001, 5.0, 3);
        assert_eq!(result, BudgetCheckResult::TokensExceeded);
    }

    #[test]
    fn check_budget_cost_exceeded() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", sample_budget());
        let result = registry.check_tenant_budget("t1", 50_000, 10.01, 3);
        assert_eq!(result, BudgetCheckResult::CostExceeded);
    }

    #[test]
    fn check_budget_concurrency_exceeded() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", sample_budget());
        let result = registry.check_tenant_budget("t1", 50_000, 5.0, 6);
        assert_eq!(result, BudgetCheckResult::ConcurrencyExceeded);
    }

    #[test]
    fn check_budget_tenant_not_found() {
        let registry = TenantRegistry::new();
        let result = registry.check_tenant_budget("missing", 0, 0.0, 0);
        assert_eq!(result, BudgetCheckResult::TenantNotFound);
    }

    #[test]
    fn check_budget_at_exact_limit() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", sample_budget());
        // At exact limit should be OK (not exceeded).
        let result = registry.check_tenant_budget("t1", 100_000, 10.0, 5);
        assert_eq!(result, BudgetCheckResult::Ok);
    }

    #[test]
    fn remove_tenant() {
        let registry = TenantRegistry::new();
        registry.register_tenant("t1", sample_budget());
        assert_eq!(registry.len(), 1);
        let removed = registry.remove_tenant("t1");
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
        assert!(registry.get_budget("t1").is_none());
    }

    #[test]
    fn remove_nonexistent_tenant() {
        let registry = TenantRegistry::new();
        assert!(registry.remove_tenant("nope").is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let registry = TenantRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        registry.register_tenant("t1", sample_budget());
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn default_creates_empty_registry() {
        let registry = TenantRegistry::default();
        assert!(registry.is_empty());
    }
}
