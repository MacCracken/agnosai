use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::AgentId;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_new_sets_timestamp() {
        let before = chrono::Utc::now();
        let msg = Message::new(
            Uuid::new_v4(),
            MessageTarget::Broadcast,
            serde_json::json!({"hello": "world"}),
        );
        let after = chrono::Utc::now();
        assert!(msg.timestamp >= before);
        assert!(msg.timestamp <= after);
    }

    #[test]
    fn message_new_generates_unique_id() {
        let from = Uuid::new_v4();
        let m1 = Message::new(from, MessageTarget::Broadcast, serde_json::Value::Null);
        let m2 = Message::new(from, MessageTarget::Broadcast, serde_json::Value::Null);
        assert_ne!(m1.id, m2.id);
    }

    #[test]
    fn message_target_serde_agent() {
        let agent_id = Uuid::new_v4();
        let target = MessageTarget::Agent(agent_id);
        let json = serde_json::to_string(&target).unwrap();
        let restored: MessageTarget = serde_json::from_str(&json).unwrap();
        match restored {
            MessageTarget::Agent(id) => assert_eq!(id, agent_id),
            _ => panic!("expected Agent variant"),
        }
    }

    #[test]
    fn message_target_serde_topic() {
        let target = MessageTarget::Topic("my-topic".into());
        let json = serde_json::to_string(&target).unwrap();
        let restored: MessageTarget = serde_json::from_str(&json).unwrap();
        match restored {
            MessageTarget::Topic(t) => assert_eq!(t, "my-topic"),
            _ => panic!("expected Topic variant"),
        }
    }

    #[test]
    fn message_target_serde_broadcast() {
        let target = MessageTarget::Broadcast;
        let json = serde_json::to_string(&target).unwrap();
        let restored: MessageTarget = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, MessageTarget::Broadcast));
    }

    #[test]
    fn message_serde_round_trip() {
        let msg = Message::new(
            Uuid::new_v4(),
            MessageTarget::Topic("events".into()),
            serde_json::json!({"type": "test"}),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, msg.id);
        assert_eq!(restored.from, msg.from);
        assert_eq!(restored.timestamp, msg.timestamp);
        assert_eq!(restored.payload, msg.payload);
    }
}
