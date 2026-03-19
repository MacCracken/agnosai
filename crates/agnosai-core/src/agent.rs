use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type AgentId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub agent_key: String,
    pub name: String,
    pub role: String,
    pub goal: String,
    #[serde(default)]
    pub backstory: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_complexity")]
    pub complexity: String,
    #[serde(default)]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub gpu_required: bool,
    #[serde(default)]
    pub gpu_preferred: bool,
    #[serde(default)]
    pub gpu_memory_min_mb: Option<u64>,
}

fn default_complexity() -> String {
    "medium".to_string()
}

impl AgentDefinition {
    pub fn from_json(json: &str) -> crate::Result<Self> {
        serde_json::from_str(json).map_err(crate::AgnosaiError::Serialization)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    Idle,
    Assigned,
    Working,
    Blocked,
    Completed,
    Failed,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::Idle
    }
}
