use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::AgentId;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessMode {
    Sequential,
    Hierarchical { manager: AgentId },
    Dag,
    Parallel { max_concurrency: usize },
}

impl Default for ProcessMode {
    fn default() -> Self {
        Self::Sequential
    }
}
