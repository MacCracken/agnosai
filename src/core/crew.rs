use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::agent::AgentDefinition;
use crate::core::task::{ProcessMode, Task, TaskId, TaskResult};

/// Unique identifier for a crew.
pub type CrewId = Uuid;

/// Specification defining a crew of agents and the tasks they execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CrewSpec {
    /// Unique crew identifier.
    pub id: CrewId,
    /// Human-readable crew name.
    pub name: String,
    /// Agent definitions participating in this crew.
    pub agents: Vec<AgentDefinition>,
    /// Tasks to be executed by the crew.
    pub tasks: Vec<Task>,
    /// Execution mode (sequential, parallel, DAG, hierarchical).
    #[serde(default)]
    pub process: ProcessMode,
    /// Arbitrary metadata attached to the crew.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Trust level for sandbox isolation policy ("minimal", "basic", "strict").
    ///
    /// When the `kavach` feature is enabled, this controls the externalization
    /// gate thresholds applied to tool outputs.  Defaults to `"basic"`.
    #[serde(default = "default_trust_level")]
    pub trust_level: String,
}

fn default_trust_level() -> String {
    "basic".into()
}

impl CrewSpec {
    /// Create a new crew spec with the given name and default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            agents: Vec::new(),
            tasks: Vec::new(),
            process: ProcessMode::default(),
            metadata: std::collections::HashMap::new(),
            trust_level: default_trust_level(),
        }
    }

    /// Set the crew's agent roster.
    pub fn with_agents(mut self, agents: Vec<AgentDefinition>) -> Self {
        self.agents = agents;
        self
    }

    /// Set the crew's task list.
    pub fn with_tasks(mut self, tasks: Vec<Task>) -> Self {
        self.tasks = tasks;
        self
    }

    /// Set the crew's execution mode.
    pub fn with_process(mut self, process: ProcessMode) -> Self {
        self.process = process;
        self
    }

    /// Set the crew's trust level for sandbox isolation ("minimal", "basic", "strict").
    pub fn with_trust_level(mut self, level: impl Into<String>) -> Self {
        self.trust_level = level.into();
        self
    }
}

/// Runtime state of a crew execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CrewState {
    /// ID of the crew this state belongs to.
    pub crew_id: CrewId,
    /// Current overall status of the crew run.
    pub status: CrewStatus,
    /// Task results collected so far.
    pub results: Vec<TaskResult>,
    /// Execution profile (always collected, lightweight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<CrewProfile>,
}

/// Lightweight execution profile for a crew run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CrewProfile {
    /// Total wall-clock time in milliseconds.
    pub wall_ms: u64,
    /// Per-task durations in milliseconds, keyed by task ID.
    pub task_ms: HashMap<TaskId, u64>,
    /// Number of tasks that ran.
    pub task_count: usize,
    /// Total estimated inference cost in USD.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cost_usd: f64,
    /// Kavach sandbox strength score (0–100) for the isolation level used.
    /// Only present when the `kavach` feature is enabled and a sandbox policy is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sandbox_strength: Option<u8>,
}

fn is_zero(v: &f64) -> bool {
    // Treat NaN and infinities as "zero" so they are skipped during serialization.
    !v.is_finite() || *v == 0.0
}

/// Lifecycle status of a crew execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CrewStatus {
    /// Crew has been created but execution has not started.
    #[default]
    Pending,
    /// Crew is actively executing tasks.
    Running,
    /// All tasks completed successfully.
    Completed,
    /// One or more tasks failed.
    Failed,
    /// Crew execution was cancelled.
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crew_spec_new_generates_unique_id() {
        let c1 = CrewSpec::new("crew-a");
        let c2 = CrewSpec::new("crew-b");
        assert_ne!(c1.id, c2.id);
    }

    #[test]
    fn crew_spec_new_defaults() {
        let c = CrewSpec::new("my crew");
        assert_eq!(c.name, "my crew");
        assert!(c.agents.is_empty());
        assert!(c.tasks.is_empty());
        assert!(matches!(c.process, ProcessMode::Sequential));
        assert!(c.metadata.is_empty());
    }

    #[test]
    fn crew_status_default_is_pending() {
        assert_eq!(CrewStatus::default(), CrewStatus::Pending);
    }

    #[test]
    fn crew_status_serde_round_trip_all_variants() {
        let variants = [
            CrewStatus::Pending,
            CrewStatus::Running,
            CrewStatus::Completed,
            CrewStatus::Failed,
            CrewStatus::Cancelled,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let restored: CrewStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, restored);
        }
    }

    #[test]
    fn crew_status_serde_snake_case() {
        assert_eq!(
            serde_json::to_string(&CrewStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&CrewStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&CrewStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&CrewStatus::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&CrewStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn crew_spec_serde_round_trip() {
        let mut c = CrewSpec::new("test crew");
        c.metadata.insert(
            "env".to_string(),
            serde_json::Value::String("staging".into()),
        );
        let json = serde_json::to_string(&c).unwrap();
        let restored: CrewSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, c.id);
        assert_eq!(restored.name, "test crew");
        assert!(restored.agents.is_empty());
        assert!(restored.tasks.is_empty());
        assert!(matches!(restored.process, ProcessMode::Sequential));
        assert_eq!(
            restored.metadata.get("env").unwrap(),
            &serde_json::Value::String("staging".into())
        );
    }

    #[test]
    fn crew_state_serde_round_trip() {
        let state = CrewState {
            crew_id: Uuid::new_v4(),
            status: CrewStatus::Running,
            results: vec![],
            profile: None,
        };
        let json = serde_json::to_string(&state).unwrap();
        let restored: CrewState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.crew_id, state.crew_id);
        assert_eq!(restored.status, CrewStatus::Running);
        assert!(restored.results.is_empty());
    }

    #[test]
    fn crew_profile_serde_with_cost() {
        let profile = CrewProfile {
            wall_ms: 1500,
            task_ms: HashMap::new(),
            task_count: 3,
            cost_usd: 0.0025,
            sandbox_strength: None,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("cost_usd"));
        let restored: CrewProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.wall_ms, 1500);
        assert_eq!(restored.task_count, 3);
        assert!((restored.cost_usd - 0.0025).abs() < f64::EPSILON);
    }

    #[test]
    fn crew_profile_skips_zero_cost() {
        let profile = CrewProfile {
            wall_ms: 100,
            task_ms: HashMap::new(),
            task_count: 1,
            cost_usd: 0.0,
            sandbox_strength: None,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(!json.contains("cost_usd"), "zero cost should be omitted");
    }

    #[test]
    fn crew_profile_deserializes_missing_cost_as_zero() {
        let json = r#"{"wall_ms":100,"task_ms":{},"task_count":1}"#;
        let profile: CrewProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.cost_usd, 0.0);
    }

    #[test]
    fn crew_spec_default_trust_level() {
        let c = CrewSpec::new("trust-test");
        assert_eq!(c.trust_level, "basic");
    }

    #[test]
    fn crew_spec_with_trust_level() {
        let c = CrewSpec::new("strict-crew").with_trust_level("strict");
        assert_eq!(c.trust_level, "strict");
    }

    #[test]
    fn crew_profile_sandbox_strength_serialized_when_present() {
        let profile = CrewProfile {
            wall_ms: 100,
            task_ms: HashMap::new(),
            task_count: 1,
            cost_usd: 0.0,
            sandbox_strength: Some(60),
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("sandbox_strength"));
        let restored: CrewProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.sandbox_strength, Some(60));
    }

    #[test]
    fn crew_profile_sandbox_strength_omitted_when_none() {
        let profile = CrewProfile {
            wall_ms: 100,
            task_ms: HashMap::new(),
            task_count: 1,
            cost_usd: 0.0,
            sandbox_strength: None,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(!json.contains("sandbox_strength"));
    }

    #[test]
    fn crew_spec_trust_level_serde() {
        let c = CrewSpec::new("serde-trust").with_trust_level("minimal");
        let json = serde_json::to_string(&c).unwrap();
        let restored: CrewSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.trust_level, "minimal");
    }

    #[test]
    fn is_zero_handles_special_values() {
        assert!(is_zero(&0.0));
        assert!(is_zero(&f64::NAN));
        assert!(is_zero(&f64::INFINITY));
        assert!(is_zero(&f64::NEG_INFINITY));
        assert!(!is_zero(&0.001));
        assert!(!is_zero(&-0.001));
    }
}
