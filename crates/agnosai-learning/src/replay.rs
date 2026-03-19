//! Prioritized experience replay buffer.

use rand::Rng;

/// A single experience tuple stored in the replay buffer.
#[derive(Debug, Clone)]
pub struct Experience {
    pub state: String,
    pub action: String,
    pub reward: f64,
    pub next_state: String,
    pub priority: f64,
}

/// Fixed-capacity replay buffer that evicts lowest-priority experiences when full.
pub struct ReplayBuffer {
    experiences: Vec<Experience>,
    max_size: usize,
}

impl ReplayBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            experiences: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Push an experience. If at capacity, evicts the experience with the lowest priority.
    pub fn push(&mut self, exp: Experience) {
        if self.experiences.len() >= self.max_size {
            // Find the index of the lowest-priority experience.
            if let Some((min_idx, min_exp)) = self
                .experiences
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.priority.partial_cmp(&b.1.priority).unwrap())
            {
                // Only evict if the new experience has higher priority.
                if exp.priority > min_exp.priority {
                    self.experiences.swap_remove(min_idx);
                    self.experiences.push(exp);
                }
            }
        } else {
            self.experiences.push(exp);
        }
    }

    /// Sample a batch of experiences, weighted by priority.
    /// Returns up to `batch_size` references (may return fewer if buffer is smaller).
    pub fn sample(&self, batch_size: usize) -> Vec<&Experience> {
        if self.experiences.is_empty() {
            return Vec::new();
        }

        let n = batch_size.min(self.experiences.len());
        let total_priority: f64 = self.experiences.iter().map(|e| e.priority.abs()).sum();

        if total_priority == 0.0 {
            // All zero priority — return first n.
            return self.experiences.iter().take(n).collect();
        }

        let mut rng = rand::thread_rng();
        let mut result = Vec::with_capacity(n);
        let mut selected = vec![false; self.experiences.len()];

        for _ in 0..n {
            let mut r = rng.r#gen::<f64>() * total_priority;
            let mut chosen = 0;
            for (i, exp) in self.experiences.iter().enumerate() {
                if selected[i] {
                    continue;
                }
                r -= exp.priority.abs();
                if r <= 0.0 {
                    chosen = i;
                    break;
                }
                chosen = i;
            }
            if !selected[chosen] {
                selected[chosen] = true;
                result.push(&self.experiences[chosen]);
            }
        }

        // If we didn't get enough due to collisions, fill from unselected.
        if result.len() < n {
            for (i, exp) in self.experiences.iter().enumerate() {
                if result.len() >= n {
                    break;
                }
                if !selected[i] {
                    result.push(exp);
                }
            }
        }

        result
    }

    pub fn len(&self) -> usize {
        self.experiences.len()
    }

    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }

    /// Update the priority of an experience at the given index.
    pub fn update_priority(&mut self, index: usize, new_priority: f64) {
        if let Some(exp) = self.experiences.get_mut(index) {
            exp.priority = new_priority;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_exp(state: &str, priority: f64) -> Experience {
        Experience {
            state: state.to_string(),
            action: "act".to_string(),
            reward: 1.0,
            next_state: "next".to_string(),
            priority,
        }
    }

    #[test]
    fn push_and_sample() {
        let mut buf = ReplayBuffer::new(10);
        buf.push(make_exp("s1", 1.0));
        buf.push(make_exp("s2", 2.0));
        buf.push(make_exp("s3", 3.0));

        assert_eq!(buf.len(), 3);
        assert!(!buf.is_empty());

        let batch = buf.sample(2);
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn max_size_eviction() {
        let mut buf = ReplayBuffer::new(3);
        buf.push(make_exp("s1", 1.0));
        buf.push(make_exp("s2", 2.0));
        buf.push(make_exp("s3", 3.0));
        assert_eq!(buf.len(), 3);

        // Push higher priority — should evict lowest (s1, priority 1.0).
        buf.push(make_exp("s4", 5.0));
        assert_eq!(buf.len(), 3);

        // All remaining should have priority >= 2.0.
        for exp in &buf.experiences {
            assert!(exp.priority >= 2.0);
        }
    }

    #[test]
    fn eviction_skips_lower_priority() {
        let mut buf = ReplayBuffer::new(2);
        buf.push(make_exp("s1", 5.0));
        buf.push(make_exp("s2", 3.0));

        // Try to push a very low priority experience — should not evict anything.
        buf.push(make_exp("s3", 1.0));
        assert_eq!(buf.len(), 2);
        // s3 should not be in the buffer.
        assert!(buf.experiences.iter().all(|e| e.state != "s3"));
    }

    #[test]
    fn priority_ordering_in_sample() {
        let mut buf = ReplayBuffer::new(100);
        // Push one very high priority and many low-priority experiences.
        buf.push(make_exp("high", 100.0));
        for i in 0..20 {
            buf.push(make_exp(&format!("low-{i}"), 0.01));
        }

        // Sample many times — the high-priority experience should appear frequently.
        let mut high_count = 0;
        for _ in 0..50 {
            let batch = buf.sample(5);
            if batch.iter().any(|e| e.state == "high") {
                high_count += 1;
            }
        }
        // Should appear in the vast majority of samples.
        assert!(
            high_count > 30,
            "high-priority item appeared {high_count}/50 times"
        );
    }

    #[test]
    fn update_priority() {
        let mut buf = ReplayBuffer::new(10);
        buf.push(make_exp("s1", 1.0));
        buf.update_priority(0, 99.0);
        assert!((buf.experiences[0].priority - 99.0).abs() < 1e-9);
    }

    #[test]
    fn empty_buffer() {
        let buf = ReplayBuffer::new(10);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(buf.sample(5).is_empty());
    }

    #[test]
    fn sample_more_than_buffer() {
        let mut buf = ReplayBuffer::new(10);
        buf.push(make_exp("s1", 1.0));
        buf.push(make_exp("s2", 2.0));

        let batch = buf.sample(10);
        assert_eq!(batch.len(), 2);
    }
}
