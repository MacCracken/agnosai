//! Q-learning and policy gradient optimization.

use std::collections::HashMap;

/// Tabular Q-learning agent.
pub struct QLearner {
    q_table: HashMap<(String, String), f64>,
    learning_rate: f64,
    discount_factor: f64,
}

impl QLearner {
    pub fn new(learning_rate: f64, discount_factor: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate,
            discount_factor,
        }
    }

    /// Get the Q-value for a (state, action) pair. Defaults to 0.0 if unseen.
    pub fn get_value(&self, state: &str, action: &str) -> f64 {
        self.q_table
            .get(&(state.to_string(), action.to_string()))
            .copied()
            .unwrap_or(0.0)
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
        self.q_table
            .insert((state.to_string(), action.to_string()), new_q);
    }

    /// Return the action with the highest Q-value in the given state.
    /// Returns `None` if `actions` is empty.
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
        // Set up Q(s2, a1) = 2.0
        q.q_table
            .insert(("s2".to_string(), "a1".to_string()), 2.0);

        q.update("s1", "a1", 1.0, "s2", &["a1", "a2"]);
        // Q(s1,a1) = 0 + 0.5 * (1.0 + 0.9*2.0 - 0) = 0.5 * 2.8 = 1.4
        let val = q.get_value("s1", "a1");
        assert!((val - 1.4).abs() < 1e-9);
    }

    #[test]
    fn best_action_selects_highest_q() {
        let mut q = QLearner::new(0.1, 0.9);
        q.q_table
            .insert(("s1".to_string(), "a1".to_string()), 0.3);
        q.q_table
            .insert(("s1".to_string(), "a2".to_string()), 0.9);
        q.q_table
            .insert(("s1".to_string(), "a3".to_string()), 0.5);

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
        // Repeatedly update s1->s2 with reward 1.0.
        // Q(s1,a1) converges to reward = 1.0 since Q(s2,a1) stays 0.
        for _ in 0..100 {
            q.update("s1", "a1", 1.0, "s2", &["a1"]);
        }
        let val = q.get_value("s1", "a1");
        assert!(
            (val - 1.0).abs() < 0.01,
            "Q-value should converge to ~1.0, got {val}"
        );
    }
}
