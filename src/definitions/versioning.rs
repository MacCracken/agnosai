//! Definition versioning and rollback.

use std::collections::HashMap;

use crate::core::agent::AgentDefinition;
use chrono::{DateTime, Utc};

/// A single versioned snapshot of an agent definition.
#[non_exhaustive]
pub struct DefinitionVersion {
    /// Auto-incrementing version number.
    pub version: u32,
    /// The agent definition at this version.
    pub definition: AgentDefinition,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
    /// Optional human-readable commit message.
    pub message: Option<String>,
}

/// In-memory version store for agent definitions, keyed by agent_key.
#[non_exhaustive]
pub struct VersionStore {
    versions: HashMap<String, Vec<DefinitionVersion>>,
}

impl VersionStore {
    /// Create an empty version store.
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
        }
    }

    /// Save a new version of the definition. Returns the assigned version number.
    pub fn save(&mut self, definition: AgentDefinition, message: Option<String>) -> u32 {
        let key = definition.agent_key.clone();
        let history = self.versions.entry(key).or_default();
        let version = history.last().map_or(1, |v| v.version + 1);
        history.push(DefinitionVersion {
            version,
            definition,
            created_at: Utc::now(),
            message,
        });
        version
    }

    /// Get a specific version of an agent definition.
    pub fn get(&self, agent_key: &str, version: u32) -> Option<&DefinitionVersion> {
        self.versions
            .get(agent_key)?
            .iter()
            .find(|v| v.version == version)
    }

    /// Get the latest version of an agent definition.
    pub fn latest(&self, agent_key: &str) -> Option<&DefinitionVersion> {
        self.versions.get(agent_key)?.last()
    }

    /// List all versions of an agent definition.
    pub fn list_versions(&self, agent_key: &str) -> Vec<&DefinitionVersion> {
        self.versions
            .get(agent_key)
            .map_or_else(Vec::new, |v| v.iter().collect())
    }

    /// Rollback to a previous version by copying it as a new latest version.
    /// Returns the cloned definition if the target version exists.
    pub fn rollback(&mut self, agent_key: &str, version: u32) -> Option<AgentDefinition> {
        let old_def = self
            .versions
            .get(agent_key)?
            .iter()
            .find(|v| v.version == version)?
            .definition
            .clone();

        let new_version = self.save(
            old_def.clone(),
            Some(format!("Rollback to version {version}")),
        );
        let _ = new_version;
        Some(old_def)
    }
}

impl Default for VersionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_def(key: &str, role: &str) -> AgentDefinition {
        AgentDefinition {
            agent_key: key.to_string(),
            name: format!("{key} Agent"),
            role: role.to_string(),
            goal: "test".to_string(),
            backstory: None,
            domain: None,
            tools: vec![],
            complexity: "medium".to_string(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
            personality: None,
        }
    }

    #[test]
    fn save_and_retrieve() {
        let mut store = VersionStore::new();
        let v = store.save(make_def("agent-a", "tester"), Some("initial".into()));
        assert_eq!(v, 1);

        let retrieved = store.get("agent-a", 1).unwrap();
        assert_eq!(retrieved.version, 1);
        assert_eq!(retrieved.definition.role, "tester");
        assert_eq!(retrieved.message.as_deref(), Some("initial"));
    }

    #[test]
    fn version_numbers_increment() {
        let mut store = VersionStore::new();
        let v1 = store.save(make_def("agent-a", "tester"), None);
        let v2 = store.save(make_def("agent-a", "senior tester"), None);
        let v3 = store.save(make_def("agent-a", "lead tester"), None);
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
        assert_eq!(v3, 3);
    }

    #[test]
    fn latest_returns_newest() {
        let mut store = VersionStore::new();
        store.save(make_def("agent-a", "v1-role"), None);
        store.save(make_def("agent-a", "v2-role"), None);

        let latest = store.latest("agent-a").unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.definition.role, "v2-role");
    }

    #[test]
    fn latest_returns_none_for_unknown() {
        let store = VersionStore::new();
        assert!(store.latest("nonexistent").is_none());
    }

    #[test]
    fn list_versions() {
        let mut store = VersionStore::new();
        store.save(make_def("agent-a", "v1"), None);
        store.save(make_def("agent-a", "v2"), None);

        let versions = store.list_versions("agent-a");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[1].version, 2);
    }

    #[test]
    fn list_versions_empty_for_unknown() {
        let store = VersionStore::new();
        assert!(store.list_versions("nonexistent").is_empty());
    }

    #[test]
    fn rollback_creates_new_version_from_old() {
        let mut store = VersionStore::new();
        store.save(make_def("agent-a", "original"), None);
        store.save(make_def("agent-a", "modified"), None);

        // Rollback to version 1.
        let rolled = store.rollback("agent-a", 1).unwrap();
        assert_eq!(rolled.role, "original");

        // Should now have 3 versions.
        let versions = store.list_versions("agent-a");
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[2].version, 3);
        assert_eq!(versions[2].definition.role, "original");
        assert_eq!(
            versions[2].message.as_deref(),
            Some("Rollback to version 1")
        );
    }

    #[test]
    fn rollback_nonexistent_version_returns_none() {
        let mut store = VersionStore::new();
        store.save(make_def("agent-a", "v1"), None);
        assert!(store.rollback("agent-a", 99).is_none());
    }

    #[test]
    fn rollback_nonexistent_agent_returns_none() {
        let mut store = VersionStore::new();
        assert!(store.rollback("nonexistent", 1).is_none());
    }

    #[test]
    fn separate_agents_have_independent_versions() {
        let mut store = VersionStore::new();
        let v1 = store.save(make_def("agent-a", "role-a"), None);
        let v2 = store.save(make_def("agent-b", "role-b"), None);
        assert_eq!(v1, 1);
        assert_eq!(v2, 1);

        assert_eq!(store.list_versions("agent-a").len(), 1);
        assert_eq!(store.list_versions("agent-b").len(), 1);
    }
}
