use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::core::AgentId;

/// Unique identifier for a task.
pub type TaskId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    #[serde(default)]
    pub expected_output: Option<String>,
    #[serde(default)]
    pub assigned_agent: Option<AgentId>,
    #[serde(default)]
    pub priority: TaskPriority,
    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub dependencies: Vec<TaskId>,
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

impl Task {
    /// Create a new task with a description.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            expected_output: None,
            assigned_agent: None,
            priority: TaskPriority::default(),
            status: TaskStatus::default(),
            dependencies: Vec::new(),
            context: HashMap::new(),
        }
    }

    /// Set the expected output.
    pub fn with_expected_output(mut self, output: impl Into<String>) -> Self {
        self.expected_output = Some(output.into());
        self
    }

    /// Set the task priority.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add a dependency on another task.
    pub fn with_dependency(mut self, dep: TaskId) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Add a context key-value pair.
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TaskPriority {
    Background = 0,
    Low = 1,
    #[default]
    Normal = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TaskStatus {
    #[default]
    Pending,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TaskResult {
    pub task_id: TaskId,
    pub output: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDAG {
    pub tasks: HashMap<String, Task>,
    pub edges: Vec<(String, String)>,
    #[serde(default)]
    pub process: ProcessMode,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProcessMode {
    #[default]
    Sequential,
    Hierarchical {
        manager: AgentId,
    },
    Dag,
    Parallel {
        max_concurrency: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn task_new_generates_unique_ids() {
        let t1 = Task::new("task one");
        let t2 = Task::new("task two");
        assert_ne!(t1.id, t2.id);
    }

    #[test]
    fn task_new_sets_correct_defaults() {
        let t = Task::new("do something");
        assert_eq!(t.description, "do something");
        assert!(t.expected_output.is_none());
        assert!(t.assigned_agent.is_none());
        assert_eq!(t.priority, TaskPriority::Normal);
        assert_eq!(t.status, TaskStatus::Pending);
        assert!(t.dependencies.is_empty());
        assert!(t.context.is_empty());
    }

    #[test]
    fn task_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
        assert!(TaskPriority::Low > TaskPriority::Background);
    }

    #[test]
    fn task_priority_default_is_normal() {
        assert_eq!(TaskPriority::default(), TaskPriority::Normal);
    }

    #[test]
    fn task_status_default_is_pending() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }

    #[test]
    fn task_status_serde_round_trip_all_variants() {
        let variants = [
            TaskStatus::Pending,
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Cancelled,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let restored: TaskStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, restored);
        }
    }

    #[test]
    fn process_mode_serde_sequential() {
        let mode = ProcessMode::Sequential;
        let json = serde_json::to_string(&mode).unwrap();
        let restored: ProcessMode = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, ProcessMode::Sequential));
    }

    #[test]
    fn process_mode_serde_hierarchical() {
        let manager_id = Uuid::new_v4();
        let mode = ProcessMode::Hierarchical {
            manager: manager_id,
        };
        let json = serde_json::to_string(&mode).unwrap();
        let restored: ProcessMode = serde_json::from_str(&json).unwrap();
        match restored {
            ProcessMode::Hierarchical { manager } => assert_eq!(manager, manager_id),
            other => panic!("expected Hierarchical, got {other:?}"),
        }
    }

    #[test]
    fn process_mode_serde_dag() {
        let mode = ProcessMode::Dag;
        let json = serde_json::to_string(&mode).unwrap();
        let restored: ProcessMode = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, ProcessMode::Dag));
    }

    #[test]
    fn process_mode_serde_parallel() {
        let mode = ProcessMode::Parallel { max_concurrency: 8 };
        let json = serde_json::to_string(&mode).unwrap();
        let restored: ProcessMode = serde_json::from_str(&json).unwrap();
        match restored {
            ProcessMode::Parallel { max_concurrency } => assert_eq!(max_concurrency, 8),
            _ => panic!("expected Parallel"),
        }
    }

    #[test]
    fn process_mode_default_is_sequential() {
        assert!(matches!(ProcessMode::default(), ProcessMode::Sequential));
    }

    #[test]
    fn task_dag_serde_round_trip() {
        let t = Task::new("subtask");
        let tid = "t1".to_string();
        let mut tasks = HashMap::new();
        tasks.insert(tid.clone(), t);
        let dag = TaskDAG {
            tasks,
            edges: vec![("t1".into(), "t2".into())],
            process: ProcessMode::Dag,
        };
        let json = serde_json::to_string(&dag).unwrap();
        let restored: TaskDAG = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tasks.len(), 1);
        assert!(restored.tasks.contains_key("t1"));
        assert_eq!(restored.edges.len(), 1);
        assert_eq!(restored.edges[0], ("t1".into(), "t2".into()));
        assert!(matches!(restored.process, ProcessMode::Dag));
    }

    #[test]
    fn task_result_serde_round_trip() {
        let task_id = Uuid::new_v4();
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), serde_json::Value::String("value".into()));
        let result = TaskResult {
            task_id,
            output: "all good".into(),
            status: TaskStatus::Completed,
            metadata: meta,
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.task_id, task_id);
        assert_eq!(restored.output, "all good");
        assert_eq!(restored.status, TaskStatus::Completed);
        assert_eq!(
            restored.metadata.get("key").unwrap(),
            &serde_json::Value::String("value".into())
        );
    }

    #[test]
    fn task_serde_round_trip() {
        let t = Task::new("round trip test");
        let json = serde_json::to_string(&t).unwrap();
        let restored: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, t.id);
        assert_eq!(restored.description, "round trip test");
        assert_eq!(restored.priority, TaskPriority::Normal);
        assert_eq!(restored.status, TaskStatus::Pending);
    }
}
