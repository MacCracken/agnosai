use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::resource::{AcceleratorType, HardwareRequirement};

/// Unique identifier for an agent instance.
pub type AgentId = Uuid;

/// Definition of an agent's identity, capabilities, and configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AgentDefinition {
    /// Unique key identifying this agent type.
    pub agent_key: String,
    /// Display name of the agent.
    pub name: String,
    /// Role the agent fulfils within a crew.
    pub role: String,
    /// High-level objective for the agent.
    pub goal: String,
    /// Optional narrative backstory for prompt context.
    #[serde(default)]
    pub backstory: Option<String>,
    /// Domain of expertise (e.g. `"quality"`, `"security"`).
    #[serde(default)]
    pub domain: Option<String>,
    /// Tool names this agent is allowed to use.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Complexity tier (`"low"`, `"medium"`, `"high"`).
    #[serde(default = "default_complexity")]
    pub complexity: String,
    /// Override LLM model identifier, if any.
    #[serde(default)]
    pub llm_model: Option<String>,
    /// Legacy: prefer [`hardware`](Self::hardware) field instead.
    #[serde(default)]
    pub gpu_required: bool,
    /// Legacy: prefer [`hardware`](Self::hardware) field instead.
    #[serde(default)]
    pub gpu_preferred: bool,
    /// Legacy: prefer [`hardware`](Self::hardware) field instead.
    #[serde(default)]
    pub gpu_memory_min_mb: Option<u64>,
    /// Hardware requirements for this agent's workloads.
    ///
    /// When set, takes precedence over the legacy `gpu_required` / `gpu_preferred` /
    /// `gpu_memory_min_mb` fields. See [`hardware_requirement()`](Self::hardware_requirement).
    #[serde(default)]
    pub hardware: Option<HardwareRequirement>,

    /// Personality profile from bhava (optional, requires `personality` feature).
    ///
    /// When set, the agent's system prompt includes behavioral disposition
    /// derived from the personality traits.
    #[cfg(feature = "personality")]
    #[serde(default)]
    pub personality: Option<bhava::traits::PersonalityProfile>,
}

fn default_complexity() -> String {
    "medium".to_string()
}

impl AgentDefinition {
    /// Create a minimal agent definition with the required fields.
    pub fn new(
        agent_key: impl Into<String>,
        role: impl Into<String>,
        goal: impl Into<String>,
    ) -> Self {
        let key = agent_key.into();
        Self {
            name: key.clone(),
            agent_key: key,
            role: role.into(),
            goal: goal.into(),
            backstory: None,
            domain: None,
            tools: Vec::new(),
            complexity: default_complexity(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
            #[cfg(feature = "personality")]
            personality: None,
        }
    }

    /// Set the agent's tool list.
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the agent's domain.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set the agent's name (defaults to agent_key).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set hardware requirements.
    pub fn with_hardware(mut self, hw: HardwareRequirement) -> Self {
        self.hardware = Some(hw);
        self
    }

    /// Set a bhava personality profile for this agent.
    #[cfg(feature = "personality")]
    pub fn with_personality(mut self, profile: bhava::traits::PersonalityProfile) -> Self {
        self.personality = Some(profile);
        self
    }

    /// Deserialize an agent definition from a JSON string.
    pub fn from_json(json: &str) -> crate::core::Result<Self> {
        serde_json::from_str(json).map_err(crate::core::AgnosaiError::Serialization)
    }

    /// Get hardware requirements, preferring the explicit `hardware` field
    /// but falling back to legacy GPU fields for backward compatibility.
    pub fn hardware_requirement(&self) -> HardwareRequirement {
        if let Some(ref hw) = self.hardware {
            return hw.clone();
        }
        // Legacy fallback
        let mut req = HardwareRequirement::default();
        if self.gpu_required {
            req.accelerators = vec![AcceleratorType::Cuda, AcceleratorType::Rocm];
            if let Some(min_mb) = self.gpu_memory_min_mb {
                req.min_memory_mb = min_mb;
            }
        }
        req
    }
}

/// Runtime state of an agent instance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AgentState {
    /// Agent is idle and available for assignment.
    #[default]
    Idle,
    /// Agent has been assigned a task but has not started.
    Assigned,
    /// Agent is actively working on a task.
    Working,
    /// Agent is blocked waiting on a dependency.
    Blocked,
    /// Agent finished its task successfully.
    Completed,
    /// Agent's task execution failed.
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
            hardware: None,
            #[cfg(feature = "personality")]
            personality: None,
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
    fn hardware_requirement_with_explicit_hardware_field() {
        let agent = AgentDefinition {
            agent_key: "hw-agent".into(),
            name: "HW Agent".into(),
            role: "worker".into(),
            goal: "do work".into(),
            backstory: None,
            domain: None,
            tools: vec![],
            complexity: "medium".into(),
            llm_model: None,
            gpu_required: true, // should be ignored when hardware is set
            gpu_preferred: false,
            gpu_memory_min_mb: Some(4096), // should be ignored
            hardware: Some(HardwareRequirement {
                accelerators: vec![AcceleratorType::Tpu],
                min_memory_mb: 32768,
                min_device_count: 2,
                min_cpu_cores: 0,
                required_family: None,
            }),
            #[cfg(feature = "personality")]
            personality: None,
        };
        let req = agent.hardware_requirement();
        assert_eq!(req.accelerators, vec![AcceleratorType::Tpu]);
        assert_eq!(req.min_memory_mb, 32768);
        assert_eq!(req.min_device_count, 2);
    }

    #[test]
    fn hardware_requirement_falls_back_to_legacy_gpu_fields() {
        let agent = AgentDefinition {
            agent_key: "legacy-gpu".into(),
            name: "Legacy GPU".into(),
            role: "worker".into(),
            goal: "do work".into(),
            backstory: None,
            domain: None,
            tools: vec![],
            complexity: "medium".into(),
            llm_model: None,
            gpu_required: true,
            gpu_preferred: true,
            gpu_memory_min_mb: Some(8192),
            hardware: None,
            #[cfg(feature = "personality")]
            personality: None,
        };
        let req = agent.hardware_requirement();
        assert_eq!(
            req.accelerators,
            vec![AcceleratorType::Cuda, AcceleratorType::Rocm]
        );
        assert_eq!(req.min_memory_mb, 8192);
    }

    #[test]
    fn hardware_requirement_no_gpu_returns_empty() {
        let agent = AgentDefinition {
            agent_key: "cpu-only".into(),
            name: "CPU Only".into(),
            role: "worker".into(),
            goal: "do work".into(),
            backstory: None,
            domain: None,
            tools: vec![],
            complexity: "medium".into(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
            #[cfg(feature = "personality")]
            personality: None,
        };
        let req = agent.hardware_requirement();
        assert!(req.accelerators.is_empty());
        assert_eq!(req.min_memory_mb, 0);
        assert_eq!(req.min_device_count, 0);
        assert_eq!(req.min_cpu_cores, 0);
    }

    #[test]
    fn serde_agent_with_hardware_field_round_trips() {
        let agent = AgentDefinition {
            agent_key: "hw-rt".into(),
            name: "HW RT".into(),
            role: "worker".into(),
            goal: "do work".into(),
            backstory: None,
            domain: None,
            tools: vec![],
            complexity: "medium".into(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: Some(HardwareRequirement {
                accelerators: vec![AcceleratorType::Cuda, AcceleratorType::Tpu],
                min_memory_mb: 16384,
                min_device_count: 1,
                min_cpu_cores: 4,
                required_family: None,
            }),
            #[cfg(feature = "personality")]
            personality: None,
        };
        let json = serde_json::to_string(&agent).unwrap();
        let restored: AgentDefinition = serde_json::from_str(&json).unwrap();
        let hw = restored.hardware.unwrap();
        assert_eq!(hw.accelerators.len(), 2);
        assert_eq!(hw.min_memory_mb, 16384);
        assert_eq!(hw.min_device_count, 1);
        assert_eq!(hw.min_cpu_cores, 4);
    }

    #[test]
    fn serde_agent_without_hardware_field_backward_compat() {
        let json = r#"{
            "agent_key": "old-style",
            "name": "Old Style",
            "role": "worker",
            "goal": "do work",
            "gpu_required": true,
            "gpu_memory_min_mb": 4096
        }"#;
        let agent = AgentDefinition::from_json(json).unwrap();
        assert!(agent.hardware.is_none());
        assert!(agent.gpu_required);
        assert_eq!(agent.gpu_memory_min_mb, Some(4096));
        // Legacy fallback should still work
        let req = agent.hardware_requirement();
        assert_eq!(
            req.accelerators,
            vec![AcceleratorType::Cuda, AcceleratorType::Rocm]
        );
        assert_eq!(req.min_memory_mb, 4096);
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
