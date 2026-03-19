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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    #[default]
    Idle,
    Assigned,
    Working,
    Blocked,
    Completed,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip() {
        let agent = AgentDefinition {
            agent_key: "test-agent".into(),
            name: "Test Agent".into(),
            role: "tester".into(),
            goal: "test things".into(),
            backstory: Some("a backstory".into()),
            domain: Some("quality".into()),
            tools: vec!["tool_a".into(), "tool_b".into()],
            complexity: "high".into(),
            llm_model: Some("gpt-4".into()),
            gpu_required: true,
            gpu_preferred: false,
            gpu_memory_min_mb: Some(4096),
        };
        let json = serde_json::to_string(&agent).unwrap();
        let restored: AgentDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.agent_key, "test-agent");
        assert_eq!(restored.name, "Test Agent");
        assert_eq!(restored.role, "tester");
        assert_eq!(restored.goal, "test things");
        assert_eq!(restored.backstory.as_deref(), Some("a backstory"));
        assert_eq!(restored.domain.as_deref(), Some("quality"));
        assert_eq!(restored.tools, vec!["tool_a", "tool_b"]);
        assert_eq!(restored.complexity, "high");
        assert_eq!(restored.llm_model.as_deref(), Some("gpt-4"));
        assert!(restored.gpu_required);
        assert!(!restored.gpu_preferred);
        assert_eq!(restored.gpu_memory_min_mb, Some(4096));
    }

    #[test]
    fn from_json_valid_full() {
        let json = r#"{
            "agent_key": "qa-lead",
            "name": "QA Lead",
            "role": "lead tester",
            "goal": "ensure quality",
            "backstory": "experienced tester",
            "domain": "quality",
            "tools": ["selenium"],
            "complexity": "high",
            "llm_model": "claude-3",
            "gpu_required": false,
            "gpu_preferred": true,
            "gpu_memory_min_mb": 2048
        }"#;
        let agent = AgentDefinition::from_json(json).unwrap();
        assert_eq!(agent.agent_key, "qa-lead");
        assert_eq!(agent.name, "QA Lead");
        assert_eq!(agent.backstory.as_deref(), Some("experienced tester"));
        assert_eq!(agent.domain.as_deref(), Some("quality"));
        assert_eq!(agent.tools, vec!["selenium"]);
        assert_eq!(agent.complexity, "high");
        assert_eq!(agent.llm_model.as_deref(), Some("claude-3"));
        assert!(!agent.gpu_required);
        assert!(agent.gpu_preferred);
        assert_eq!(agent.gpu_memory_min_mb, Some(2048));
    }

    #[test]
    fn from_json_minimal_uses_defaults() {
        let json = r#"{
            "agent_key": "min",
            "name": "Minimal",
            "role": "worker",
            "goal": "do work"
        }"#;
        let agent = AgentDefinition::from_json(json).unwrap();
        assert_eq!(agent.agent_key, "min");
        assert_eq!(agent.complexity, "medium");
        assert!(agent.backstory.is_none());
        assert!(agent.domain.is_none());
        assert!(agent.tools.is_empty());
        assert!(agent.llm_model.is_none());
        assert!(!agent.gpu_required);
        assert!(!agent.gpu_preferred);
        assert!(agent.gpu_memory_min_mb.is_none());
    }

    #[test]
    fn from_json_invalid_returns_error() {
        let result = AgentDefinition::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn from_json_missing_required_field_returns_error() {
        let json = r#"{"agent_key": "k", "name": "N"}"#;
        assert!(AgentDefinition::from_json(json).is_err());
    }

    #[test]
    fn default_complexity_is_medium() {
        assert_eq!(default_complexity(), "medium");
    }

    #[test]
    fn agent_state_default_is_idle() {
        assert_eq!(AgentState::default(), AgentState::Idle);
    }

    #[test]
    fn agent_state_serde_round_trip_all_variants() {
        let variants = [
            AgentState::Idle,
            AgentState::Assigned,
            AgentState::Working,
            AgentState::Blocked,
            AgentState::Completed,
            AgentState::Failed,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).unwrap();
            let restored: AgentState = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, restored);
        }
    }

    #[test]
    fn agent_state_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&AgentState::Idle).unwrap(),
            "\"idle\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Assigned).unwrap(),
            "\"assigned\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Working).unwrap(),
            "\"working\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Blocked).unwrap(),
            "\"blocked\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&AgentState::Failed).unwrap(),
            "\"failed\""
        );
    }
}
