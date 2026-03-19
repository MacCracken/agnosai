use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::AgentDefinition;
use crate::task::{ProcessMode, Task, TaskResult};

pub type CrewId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewSpec {
    pub id: CrewId,
    pub name: String,
    pub agents: Vec<AgentDefinition>,
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub process: ProcessMode,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl CrewSpec {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            agents: Vec::new(),
            tasks: Vec::new(),
            process: ProcessMode::default(),
            metadata: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewState {
    pub crew_id: CrewId,
    pub status: CrewStatus,
    pub results: Vec<TaskResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrewStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Default for CrewStatus {
    fn default() -> Self {
        Self::Pending
    }
}
