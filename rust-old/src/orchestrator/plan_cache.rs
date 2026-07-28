//! Plan caching for repeated crew executions.
//!
//! Caches agent assignment decisions and task execution plans keyed by a
//! normalized hash of the crew specification. When a crew with the same
//! agents, tasks, and process mode is submitted again, the cached plan
//! is reused instead of recomputing agent scoring and task ordering.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::core::task::TaskId;

/// Maximum number of cached plans before LRU eviction.
const MAX_CACHED_PLANS: usize = 256;

/// Default TTL for cached plans.
const DEFAULT_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// A cached execution plan for a crew.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CachedPlan {
    /// Agent assignment: task_id → agent_key.
    pub assignments: HashMap<TaskId, String>,
    /// Ordered task execution sequence.
    pub execution_order: Vec<TaskId>,
    /// Model selections: task_id → model name.
    pub model_selections: HashMap<TaskId, String>,
}

/// Entry in the plan cache with LRU metadata.
struct CacheEntry {
    plan: CachedPlan,
    created_at: Instant,
    last_accessed: Instant,
}

/// Normalized key for plan cache lookup.
///
/// Built from the sorted agent keys, sorted task descriptions, and process mode.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlanKey {
    hash: u64,
}

impl PlanKey {
    /// Compute a plan key from crew specification components.
    #[must_use]
    pub fn from_crew(
        agent_keys: &[String],
        task_descriptions: &[String],
        process_mode: &str,
    ) -> Self {
        let mut hasher = std::hash::DefaultHasher::new();
        // Sort for deterministic hashing regardless of input order.
        let mut sorted_agents: Vec<&str> = agent_keys.iter().map(|s| s.as_str()).collect();
        sorted_agents.sort();
        for key in &sorted_agents {
            key.hash(&mut hasher);
        }
        let mut sorted_tasks: Vec<&str> = task_descriptions.iter().map(|s| s.as_str()).collect();
        sorted_tasks.sort();
        for desc in &sorted_tasks {
            desc.hash(&mut hasher);
        }
        process_mode.hash(&mut hasher);
        Self {
            hash: hasher.finish(),
        }
    }
}

/// LRU plan cache with TTL expiration.
pub struct PlanCache {
    entries: HashMap<PlanKey, CacheEntry>,
    ttl: Duration,
}

impl PlanCache {
    /// Create a new plan cache with the default TTL.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            ttl: DEFAULT_TTL,
        }
    }

    /// Create a plan cache with a custom TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    /// Look up a cached plan. Returns `None` if not found or expired.
    #[must_use]
    pub fn get(&mut self, key: &PlanKey) -> Option<&CachedPlan> {
        let now = Instant::now();
        // Check expiry first.
        if let Some(entry) = self.entries.get(key)
            && now.duration_since(entry.created_at) > self.ttl
        {
            self.entries.remove(key);
            debug!("plan cache entry expired");
            return None;
        }
        // Update last_accessed for LRU.
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_accessed = now;
            debug!("plan cache hit");
            Some(&entry.plan)
        } else {
            None
        }
    }

    /// Insert a plan into the cache, evicting the least recently used entry
    /// if at capacity.
    pub fn insert(&mut self, key: PlanKey, plan: CachedPlan) {
        if self.entries.len() >= MAX_CACHED_PLANS {
            self.evict_lru();
        }
        let now = Instant::now();
        self.entries.insert(
            key,
            CacheEntry {
                plan,
                created_at: now,
                last_accessed: now,
            },
        );
    }

    /// Number of cached plans.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Evict the least recently used entry.
    fn evict_lru(&mut self) {
        if let Some(oldest_key) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
            debug!("plan cache: evicted LRU entry");
        }
    }
}

impl Default for PlanCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_key() -> PlanKey {
        PlanKey::from_crew(
            &["agent-a".into(), "agent-b".into()],
            &["do X".into(), "do Y".into()],
            "sequential",
        )
    }

    fn sample_plan() -> CachedPlan {
        CachedPlan {
            assignments: HashMap::new(),
            execution_order: Vec::new(),
            model_selections: HashMap::new(),
        }
    }

    #[test]
    fn insert_and_get() {
        let mut cache = PlanCache::new();
        let key = sample_key();
        cache.insert(key.clone(), sample_plan());
        assert_eq!(cache.len(), 1);
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn miss_returns_none() {
        let mut cache = PlanCache::new();
        let key = sample_key();
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn different_agents_different_key() {
        let k1 = PlanKey::from_crew(&["a".into()], &["task".into()], "seq");
        let k2 = PlanKey::from_crew(&["b".into()], &["task".into()], "seq");
        assert_ne!(k1, k2);
    }

    #[test]
    fn order_independent_hashing() {
        let k1 = PlanKey::from_crew(
            &["agent-b".into(), "agent-a".into()],
            &["Y".into(), "X".into()],
            "seq",
        );
        let k2 = PlanKey::from_crew(
            &["agent-a".into(), "agent-b".into()],
            &["X".into(), "Y".into()],
            "seq",
        );
        assert_eq!(k1, k2);
    }

    #[test]
    fn ttl_expiry() {
        let mut cache = PlanCache::with_ttl(Duration::from_millis(1));
        let key = sample_key();
        cache.insert(key.clone(), sample_plan());
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn lru_eviction_at_capacity() {
        let mut cache = PlanCache::new();
        // Fill to capacity.
        for i in 0..MAX_CACHED_PLANS {
            let key = PlanKey::from_crew(&[format!("agent-{i}")], &["task".into()], "seq");
            cache.insert(key, sample_plan());
        }
        assert_eq!(cache.len(), MAX_CACHED_PLANS);
        // One more should trigger eviction.
        let key = PlanKey::from_crew(&["new-agent".into()], &["task".into()], "seq");
        cache.insert(key, sample_plan());
        assert_eq!(cache.len(), MAX_CACHED_PLANS);
    }

    #[test]
    fn empty_cache() {
        let cache = PlanCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }
}
