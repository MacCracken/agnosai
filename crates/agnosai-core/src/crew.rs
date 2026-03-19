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
        };
        let json = serde_json::to_string(&state).unwrap();
        let restored: CrewState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.crew_id, state.crew_id);
        assert_eq!(restored.status, CrewStatus::Running);
        assert!(restored.results.is_empty());
    }
}
