//! Kubernetes CustomResourceDefinition types for AgnosAI.
//!
//! Provides serde-compatible CRD structs that match the K8s CRD format
//! for `CrewSpec` and `AgentDefinition` resources. These types can be
//! used to generate CRD YAML manifests or to deserialize K8s webhook
//! payloads.
//!
//! No Kubernetes client dependency — just the type definitions.

use serde::{Deserialize, Serialize};

/// Kubernetes-style metadata for a custom resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ObjectMeta {
    /// Resource name (must be DNS-compatible).
    #[serde(default)]
    pub name: String,
    /// Kubernetes namespace.
    #[serde(default)]
    pub namespace: String,
    /// Labels for filtering.
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
    /// Annotations for metadata.
    #[serde(default)]
    pub annotations: std::collections::HashMap<String, String>,
}

/// A Kubernetes custom resource wrapping an AgnosAI crew specification.
///
/// ```yaml
/// apiVersion: agnosai.io/v1
/// kind: Crew
/// metadata:
///   name: my-crew
/// spec:
///   name: "Data Pipeline Crew"
///   agents: [...]
///   tasks: [...]
///   processMode: sequential
///   trustLevel: basic
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CrewCrd {
    /// API version (always `agnosai.io/v1`).
    pub api_version: String,
    /// Resource kind (always `Crew`).
    pub kind: String,
    /// Kubernetes metadata.
    pub metadata: ObjectMeta,
    /// The crew specification.
    pub spec: CrewCrdSpec,
}

/// Spec portion of a Crew CRD.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CrewCrdSpec {
    /// Human-readable crew name.
    pub name: String,
    /// Inline agent definitions.
    #[serde(default)]
    pub agents: Vec<AgentCrdSpec>,
    /// Task definitions.
    #[serde(default)]
    pub tasks: Vec<TaskCrdSpec>,
    /// Execution mode.
    #[serde(default = "default_process_mode")]
    pub process_mode: String,
    /// Trust level for sandbox isolation.
    #[serde(default = "default_trust")]
    pub trust_level: String,
}

/// Agent definition within a CRD.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AgentCrdSpec {
    /// Unique agent key.
    pub agent_key: String,
    /// Display name.
    pub name: String,
    /// Agent role.
    pub role: String,
    /// Agent goal.
    pub goal: String,
    /// Available tools.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Complexity level.
    #[serde(default = "default_complexity")]
    pub complexity: String,
    /// Optional domain expertise.
    #[serde(default)]
    pub domain: Option<String>,
}

/// Task definition within a CRD.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct TaskCrdSpec {
    /// Task description.
    pub description: String,
    /// Expected output format.
    #[serde(default)]
    pub expected_output: Option<String>,
    /// Task priority.
    #[serde(default = "default_priority")]
    pub priority: String,
    /// Risk level for approval gating.
    #[serde(default = "default_risk")]
    pub risk: String,
    /// Dependency indices.
    #[serde(default)]
    pub dependencies: Vec<usize>,
}

fn default_process_mode() -> String {
    "sequential".into()
}
fn default_trust() -> String {
    "basic".into()
}
fn default_complexity() -> String {
    "medium".into()
}
fn default_priority() -> String {
    "normal".into()
}
fn default_risk() -> String {
    "low".into()
}

/// CRD API group and version.
pub const API_GROUP: &str = "agnosai.io";
pub const API_VERSION: &str = "agnosai.io/v1";

impl CrewCrd {
    /// Create a new Crew CRD with the given name and spec.
    #[must_use]
    pub fn new(name: impl Into<String>, spec: CrewCrdSpec) -> Self {
        Self {
            api_version: API_VERSION.into(),
            kind: "Crew".into(),
            metadata: ObjectMeta {
                name: name.into(),
                ..Default::default()
            },
            spec,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crew_crd_serde_roundtrip() {
        let crd = CrewCrd::new(
            "test-crew",
            CrewCrdSpec {
                name: "Test Crew".into(),
                agents: vec![AgentCrdSpec {
                    agent_key: "agent-a".into(),
                    name: "Agent A".into(),
                    role: "tester".into(),
                    goal: "test things".into(),
                    tools: vec!["echo".into()],
                    complexity: "medium".into(),
                    domain: None,
                }],
                tasks: vec![TaskCrdSpec {
                    description: "do something".into(),
                    expected_output: None,
                    priority: "normal".into(),
                    risk: "low".into(),
                    dependencies: vec![],
                }],
                process_mode: "sequential".into(),
                trust_level: "basic".into(),
            },
        );

        let json = serde_json::to_string_pretty(&crd).unwrap();
        assert!(json.contains("agnosai.io/v1"));
        assert!(json.contains("Crew"));

        let restored: CrewCrd = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.spec.name, "Test Crew");
        assert_eq!(restored.spec.agents.len(), 1);
        assert_eq!(restored.spec.tasks.len(), 1);
    }

    #[test]
    fn crew_crd_yaml_compatible() {
        let crd = CrewCrd::new(
            "my-crew",
            CrewCrdSpec {
                name: "My Crew".into(),
                agents: vec![],
                tasks: vec![],
                process_mode: "parallel".into(),
                trust_level: "strict".into(),
            },
        );
        let json = serde_json::to_value(&crd).unwrap();
        assert_eq!(json["apiVersion"], "agnosai.io/v1");
        assert_eq!(json["kind"], "Crew");
        assert_eq!(json["metadata"]["name"], "my-crew");
    }

    #[test]
    fn defaults_applied() {
        let json = r#"{
            "apiVersion": "agnosai.io/v1",
            "kind": "Crew",
            "metadata": {"name": "minimal"},
            "spec": {"name": "Minimal"}
        }"#;
        let crd: CrewCrd = serde_json::from_str(json).unwrap();
        assert_eq!(crd.spec.process_mode, "sequential");
        assert_eq!(crd.spec.trust_level, "basic");
    }

    #[test]
    fn api_constants() {
        assert_eq!(API_GROUP, "agnosai.io");
        assert_eq!(API_VERSION, "agnosai.io/v1");
    }
}
