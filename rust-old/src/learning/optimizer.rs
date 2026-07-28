//! Q-learning and policy gradient optimization.

use std::collections::HashMap;
use tracing::debug;

/// String-interning table for efficient Q-table lookups.
///
/// Maps string keys to compact `u32` indices, avoiding repeated heap
/// allocations when the same state/action strings are used many times.
struct StringInterner {
    map: HashMap<String, u32>,
    next_id: u32,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            next_id: 0,
        }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("StringInterner: exhausted u32 ID space (>4 billion unique strings)");
        self.map.insert(s.to_string(), id);
        id
    }
}

/// Tabular Q-learning agent with interned state/action keys.
///
/// Uses numeric indices internally for efficient lookups while
/// accepting `&str` keys at the API boundary.
#[non_exhaustive]
pub struct QLearner {
    q_table: HashMap<(u32, u32), f64>,
    interner: StringInterner,
    /// Learning rate (alpha).
    learning_rate: f64,
    /// Discount factor (gamma).
    discount_factor: f64,
}

impl QLearner {
    /// Create a new Q-learner with the given hyperparameters.
    pub fn new(learning_rate: f64, discount_factor: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            interner: StringInterner::new(),
            learning_rate,
            discount_factor,
        }
    }

    /// Get the Q-value for a (state, action) pair. Defaults to 0.0 if unseen.
    #[must_use]
    pub fn get_value(&self, state: &str, action: &str) -> f64 {
        let s = self.interner.map.get(state);
        let a = self.interner.map.get(action);
        match (s, a) {
            (Some(&s_id), Some(&a_id)) => self.q_table.get(&(s_id, a_id)).copied().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    /// Q-learning update: Q(s,a) = Q(s,a) + lr * (reward + gamma * max_Q(s',a') - Q(s,a))
    pub fn update(
        &mut self,
        state: &str,
        action: &str,
        reward: f64,
        next_state: &str,
        next_actions: &[&str],
    ) {
        let current_q = self.get_value(state, action);
        let max_next_q = next_actions
            .iter()
            .map(|a| self.get_value(next_state, a))
            .fold(0.0_f64, f64::max);
        let td_target = reward + self.discount_factor * max_next_q;
        let new_q = current_q + self.learning_rate * (td_target - current_q);

        debug!(state, action, reward, new_q, "q-value updated");

        let s_id = self.interner.intern(state);
        let a_id = self.interner.intern(action);
        self.q_table.insert((s_id, a_id), new_q);
    }

    /// Return the action with the highest Q-value in the given state.
    /// Returns `None` if `actions` is empty.
    #[must_use]
    pub fn best_action(&self, state: &str, actions: &[&str]) -> Option<String> {
        actions
            .iter()
            .max_by(|a, b| {
                self.get_value(state, a)
                    .partial_cmp(&self.get_value(state, b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_values_are_zero() {
        let q = QLearner::new(0.1, 0.9);
        assert!((q.get_value("s1", "a1") - 0.0).abs() < 1e-9);
        assert!((q.get_value("any", "thing") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn update_changes_value() {
        let mut q = QLearner::new(0.5, 0.9);
        q.update("s1", "a1", 1.0, "s2", &["a1", "a2"]);
        let val = q.get_value("s1", "a1");
        // Q(s1,a1) = 0 + 0.5 * (1.0 + 0.9*0 - 0) = 0.5
        assert!((val - 0.5).abs() < 1e-9);
    }

    #[test]
    fn update_with_existing_next_state() {
        let mut q = QLearner::new(0.5, 0.9);
        // Set up Q(s2, a1) = 2.0 via update.
        q.update("s2", "a1", 2.0, "terminal", &[]);
        // Q(s2,a1) = 0 + 0.5*(2.0 + 0.9*0 - 0) = 1.0
        // Now update s1 referencing s2.
        q.update("s1", "a1", 1.0, "s2", &["a1", "a2"]);
        let val = q.get_value("s1", "a1");
        // Q(s1,a1) = 0 + 0.5 * (1.0 + 0.9*1.0 - 0) = 0.5 * 1.9 = 0.95
        assert!((val - 0.95).abs() < 1e-9);
    }

    #[test]
    fn best_action_selects_highest_q() {
        let mut q = QLearner::new(0.1, 0.9);
        // Direct table manipulation via updates to create known Q-values.
        // After update: Q = 0 + 0.1*(reward + 0) = 0.1*reward
        q.update("s1", "a1", 3.0, "term", &[]);
        q.update("s1", "a2", 9.0, "term", &[]);
        q.update("s1", "a3", 5.0, "term", &[]);

        let best = q.best_action("s1", &["a1", "a2", "a3"]).unwrap();
        assert_eq!(best, "a2");
    }

    #[test]
    fn best_action_empty_returns_none() {
        let q = QLearner::new(0.1, 0.9);
        assert!(q.best_action("s1", &[]).is_none());
    }

    #[test]
    fn best_action_with_unseen_state() {
        let q = QLearner::new(0.1, 0.9);
        // All actions have Q=0, any is valid.
        let best = q.best_action("unknown", &["a1", "a2"]);
        assert!(best.is_some());
    }

    #[test]
    fn repeated_updates_converge() {
        let mut q = QLearner::new(0.1, 0.9);
        for _ in 0..100 {
            q.update("s1", "a1", 1.0, "s2", &["a1"]);
        }
        let val = q.get_value("s1", "a1");
        assert!(
            (val - 1.0).abs() < 0.01,
            "Q-value should converge to ~1.0, got {val}"
        );
    }

    #[test]
    fn interning_reuses_ids() {
        let mut q = QLearner::new(0.1, 0.9);
        q.update("state", "action", 1.0, "next", &["a"]);
        q.update("state", "action", 2.0, "next", &["a"]);
        // Only (state, action) pairs are interned via update; lookups don't intern.
        // Two updates with the same state/action → still only 2 interned strings.
        assert_eq!(q.interner.map.len(), 2); // state, action
    }
}
