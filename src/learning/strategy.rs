//! UCB1 multi-armed bandit for strategy selection.

/// Statistics tracked per arm.
pub struct ArmStats {
    /// Human-readable arm name.
    pub name: String,
    /// Cumulative reward received from this arm.
    pub total_reward: f64,
    /// Number of times this arm has been pulled.
    pub count: u32,
}

impl ArmStats {
    fn mean_reward(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.total_reward / self.count as f64
    }
}

/// UCB1 multi-armed bandit implementation.
pub struct Ucb1 {
    arms: Vec<ArmStats>,
}

impl Ucb1 {
    /// Create a new UCB1 bandit with the given arm names.
    pub fn new(arm_names: Vec<String>) -> Self {
        Self {
            arms: arm_names
                .into_iter()
                .map(|name| ArmStats {
                    name,
                    total_reward: 0.0,
                    count: 0,
                })
                .collect(),
        }
    }

    /// Select the next arm using UCB1: mean + sqrt(2 * ln(N) / n).
    /// Arms that have never been pulled are selected first (lowest index).
    ///
    /// Returns `None` if there are no arms.
    #[must_use]
    pub fn select(&self, total_rounds: u32) -> Option<usize> {
        if self.arms.is_empty() {
            return None;
        }

        // Select any unexplored arm first.
        for (i, arm) in self.arms.iter().enumerate() {
            if arm.count == 0 {
                return Some(i);
            }
        }

        if total_rounds == 0 {
            return Some(0);
        }

        let ln_n = (total_rounds as f64).ln();
        self.arms
            .iter()
            .enumerate()
            .map(|(i, arm)| {
                let ucb = arm.mean_reward() + (2.0 * ln_n / arm.count as f64).sqrt();
                (i, ucb)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    }

    /// Record a reward for the given arm index.
    pub fn update(&mut self, arm: usize, reward: f64) {
        if let Some(stats) = self.arms.get_mut(arm) {
            stats.total_reward += reward;
            stats.count += 1;
        }
    }

    /// Return the arm index with the highest mean reward.
    /// Returns `None` if there are no arms.
    #[must_use]
    pub fn best_arm(&self) -> Option<usize> {
        self.arms
            .iter()
            .enumerate()
            .max_by(|a, b| {
                a.1.mean_reward()
                    .partial_cmp(&b.1.mean_reward())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// Number of arms.
    #[must_use]
    pub fn arm_count(&self) -> usize {
        self.arms.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_unexplored_first() {
        let bandit = Ucb1::new(vec!["a".into(), "b".into(), "c".into()]);
        // With no rounds played, should select first unexplored arm (index 0).
        assert_eq!(bandit.select(0), Some(0));
    }

    #[test]
    fn select_cycles_through_unexplored() {
        let mut bandit = Ucb1::new(vec!["a".into(), "b".into(), "c".into()]);
        // Pull arm 0.
        bandit.update(0, 1.0);
        // Next select should pick arm 1 (first unexplored).
        assert_eq!(bandit.select(1), Some(1));
        bandit.update(1, 0.5);
        // Now arm 2 is unexplored.
        assert_eq!(bandit.select(2), Some(2));
    }

    #[test]
    fn update_affects_selection() {
        let mut bandit = Ucb1::new(vec!["a".into(), "b".into()]);
        // Explore both arms.
        bandit.update(0, 0.1);
        bandit.update(1, 0.9);

        // After many rounds the arm with higher mean should be preferred
        // (though UCB1 also considers exploration bonus).
        // Give arm 1 many high rewards to ensure it dominates.
        for _ in 0..20 {
            bandit.update(1, 0.9);
        }
        for _ in 0..20 {
            bandit.update(0, 0.1);
        }
        // With enough exploitation data, best_arm should be 1.
        assert_eq!(bandit.best_arm(), Some(1));
    }

    #[test]
    fn best_arm_returns_highest_mean() {
        let mut bandit = Ucb1::new(vec!["a".into(), "b".into(), "c".into()]);
        bandit.update(0, 0.3);
        bandit.update(1, 0.9);
        bandit.update(2, 0.5);
        assert_eq!(bandit.best_arm(), Some(1));
    }

    #[test]
    fn arm_count() {
        let bandit = Ucb1::new(vec!["a".into(), "b".into()]);
        assert_eq!(bandit.arm_count(), 2);
    }

    #[test]
    fn empty_bandit() {
        let bandit = Ucb1::new(vec![]);
        assert_eq!(bandit.arm_count(), 0);
        assert_eq!(bandit.select(0), None);
        assert_eq!(bandit.best_arm(), None);
    }
}
