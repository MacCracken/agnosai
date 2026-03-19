use std::collections::VecDeque;

use agnosai_core::task::{Task, TaskPriority};

/// Priority-based task scheduler with per-level queues.
///
/// Tasks are enqueued into one of five priority tiers (Critical → Background).
/// Dequeue returns the highest-priority task available — O(1) per tier.
pub struct Scheduler {
    queues: [VecDeque<Task>; 5],
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queues: Default::default(),
        }
    }

    pub fn enqueue(&mut self, task: Task) {
        let tier = task.priority as usize;
        self.queues[tier].push_back(task);
    }

    pub fn dequeue(&mut self) -> Option<Task> {
        // Drain from highest priority first
        for tier in (0..5).rev() {
            if let Some(task) = self.queues[tier].pop_front() {
                return Some(task);
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.queues.iter().map(|q| q.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.queues.iter().all(|q| q.is_empty())
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}
