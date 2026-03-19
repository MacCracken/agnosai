use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AgentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub from: AgentId,
    pub to: MessageTarget,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageTarget {
    Agent(AgentId),
    Topic(String),
    Broadcast,
}

impl Message {
    pub fn new(from: AgentId, to: MessageTarget, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            from,
            to,
            payload,
            timestamp: chrono::Utc::now(),
        }
    }
}
