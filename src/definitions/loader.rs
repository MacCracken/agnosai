//! JSON/YAML agent definition and preset loading.

use std::path::Path;

use crate::core::agent::AgentDefinition;
use crate::core::{AgnosaiError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Agent definition loading
// ---------------------------------------------------------------------------

/// Load an agent definition from a JSON string.
pub fn load_from_json(json: &str) -> Result<AgentDefinition> {
    serde_json::from_str(json).map_err(AgnosaiError::Serialization)
}

/// Load an agent definition from a YAML string.
pub fn load_from_yaml(yaml: &str) -> Result<AgentDefinition> {
    serde_yaml::from_str(yaml).map_err(|e| AgnosaiError::InvalidDefinition(e.to_string()))
}

/// Load an agent definition from a file, auto-detecting format by extension.
///
/// Supported extensions: `.json`, `.yaml`, `.yml`.
pub fn load_from_file(path: &Path) -> Result<AgentDefinition> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = std::fs::read_to_string(path)?;

    match ext.as_str() {
        "json" => load_from_json(&content),
        "yaml" | "yml" => load_from_yaml(&content),
        other => Err(AgnosaiError::InvalidDefinition(format!(
            "unsupported file extension: {other}"
        ))),
    }
}

/// Load all agent definitions from `.json`, `.yaml`, and `.yml` files in a directory.
///
/// Non-matching files are silently skipped. Parse errors are propagated.
pub fn load_all_from_dir(dir: &Path) -> Result<Vec<AgentDefinition>> {
    let mut definitions = Vec::new();

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(ext.as_str(), "json" | "yaml" | "yml") {
            definitions.push(load_from_file(&path)?);
        }
    }

    Ok(definitions)
}

// ---------------------------------------------------------------------------
// Preset loading
// ---------------------------------------------------------------------------

/// A preset crew specification containing a named team of agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetSpec {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub size: String,
    pub version: String,
    pub agents: Vec<AgentDefinition>,
}

/// Load a preset from a JSON string.
pub fn load_preset_from_json(json: &str) -> Result<PresetSpec> {
    serde_json::from_str(json).map_err(AgnosaiError::Serialization)
}

/// Load a preset from a file.
pub fn load_preset_from_file(path: &Path) -> Result<PresetSpec> {
    let content = std::fs::read_to_string(path)?;
    load_preset_from_json(&content)
}

/// Load all presets from a directory (`.json` files only).
pub fn load_all_presets(dir: &Path) -> Result<Vec<PresetSpec>> {
    let mut presets = Vec::new();

    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "json" {
            presets.push(load_preset_from_file(&path)?);
        }
    }

    Ok(presets)
}

/// Get the built-in presets (embedded at compile time).
pub fn builtin_presets() -> Vec<PresetSpec> {
    let jsons = [
        // Quality
        include_str!("../presets/quality-lean.json"),
        include_str!("../presets/quality-standard.json"),
        include_str!("../presets/quality-large.json"),
        // Software Engineering
        include_str!("../presets/software-engineering-lean.json"),
        include_str!("../presets/software-engineering-standard.json"),
        include_str!("../presets/software-engineering-large.json"),
        // DevOps
        include_str!("../presets/devops-lean.json"),
        include_str!("../presets/devops-standard.json"),
        include_str!("../presets/devops-large.json"),
        // Data Engineering
        include_str!("../presets/data-engineering-lean.json"),
        include_str!("../presets/data-engineering-standard.json"),
        include_str!("../presets/data-engineering-large.json"),
        // Design
        include_str!("../presets/design-lean.json"),
        include_str!("../presets/design-standard.json"),
        include_str!("../presets/design-large.json"),
        // Security
        include_str!("../presets/security-lean.json"),
        include_str!("../presets/security-standard.json"),
        include_str!("../presets/security-large.json"),
    ];
    jsons
        .iter()
        .filter_map(|j| load_preset_from_json(j).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // -----------------------------------------------------------------------
    // Agent definition tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_from_json() {
        let json = r#"{
            "agent_key": "lint-agent",
            "name": "Lint Agent",
            "role": "linter",
            "goal": "Lint all the things",
            "tools": ["ruff", "eslint"],
            "complexity": "low"
        }"#;
        let def = load_from_json(json).unwrap();
        assert_eq!(def.agent_key, "lint-agent");
        assert_eq!(def.name, "Lint Agent");
        assert_eq!(def.tools, vec!["ruff", "eslint"]);
        assert_eq!(def.complexity, "low");
        assert!(def.domain.is_none());
    }

    #[test]
    fn test_load_from_json_minimal() {
        let json = r#"{
            "agent_key": "min",
            "name": "Minimal",
            "role": "r",
            "goal": "g"
        }"#;
        let def = load_from_json(json).unwrap();
        assert_eq!(def.agent_key, "min");
        assert!(def.tools.is_empty());
        assert_eq!(def.complexity, "medium"); // default
        assert!(!def.gpu_required);
    }

    #[test]
    fn test_load_from_json_invalid() {
        let result = load_from_json("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_yaml() {
        let yaml = r#"
agent_key: scan-agent
name: Security Scanner
role: scanner
goal: Find vulnerabilities
domain: quality
tools:
  - trivy
  - semgrep
complexity: high
gpu_required: false
"#;
        let def = load_from_yaml(yaml).unwrap();
        assert_eq!(def.agent_key, "scan-agent");
        assert_eq!(def.domain, Some("quality".to_string()));
        assert_eq!(def.tools, vec!["trivy", "semgrep"]);
        assert_eq!(def.complexity, "high");
    }

    #[test]
    fn test_load_from_yaml_minimal() {
        let yaml = r#"
agent_key: y
name: Y
role: r
goal: g
"#;
        let def = load_from_yaml(yaml).unwrap();
        assert_eq!(def.agent_key, "y");
        assert!(def.tools.is_empty());
        assert_eq!(def.complexity, "medium");
    }

    #[test]
    fn test_load_from_yaml_invalid() {
        let result = load_from_yaml(":::bad yaml[[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_file_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.json");
        fs::write(
            &path,
            r#"{"agent_key":"f","name":"F","role":"r","goal":"g"}"#,
        )
        .unwrap();
        let def = load_from_file(&path).unwrap();
        assert_eq!(def.agent_key, "f");
    }

    #[test]
    fn test_load_from_file_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.yml");
        fs::write(&path, "agent_key: fy\nname: FY\nrole: r\ngoal: g\n").unwrap();
        let def = load_from_file(&path).unwrap();
        assert_eq!(def.agent_key, "fy");
    }

    #[test]
    fn test_load_from_file_unsupported_ext() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("agent.toml");
        fs::write(&path, "").unwrap();
        let result = load_from_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_all_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.json"),
            r#"{"agent_key":"a","name":"A","role":"r","goal":"g"}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("b.yaml"),
            "agent_key: b\nname: B\nrole: r\ngoal: g\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("c.yml"),
            "agent_key: c\nname: C\nrole: r\ngoal: g\n",
        )
        .unwrap();
        // This file should be skipped
        fs::write(dir.path().join("readme.txt"), "ignored").unwrap();

        let defs = load_all_from_dir(dir.path()).unwrap();
        assert_eq!(defs.len(), 3);
        let keys: Vec<&str> = defs.iter().map(|d| d.agent_key.as_str()).collect();
        assert!(keys.contains(&"a"));
        assert!(keys.contains(&"b"));
        assert!(keys.contains(&"c"));
    }

    #[test]
    fn test_load_all_from_dir_empty() {
        let dir = tempfile::tempdir().unwrap();
        let defs = load_all_from_dir(dir.path()).unwrap();
        assert!(defs.is_empty());
    }

    // -----------------------------------------------------------------------
    // Preset tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_builtin_presets_non_empty() {
        let presets = builtin_presets();
        assert!(!presets.is_empty());
        assert_eq!(presets.len(), 18);
    }

    #[test]
    fn test_builtin_presets_have_valid_agents() {
        let presets = builtin_presets();
        for preset in &presets {
            assert!(!preset.name.is_empty());
            assert!(!preset.domain.is_empty());
            assert!(!preset.size.is_empty());
            assert!(!preset.agents.is_empty());
            for agent in &preset.agents {
                assert!(!agent.agent_key.is_empty());
                assert!(!agent.name.is_empty());
                assert!(!agent.role.is_empty());
                assert!(!agent.goal.is_empty());
            }
        }
    }

    #[test]
    fn test_load_preset_from_json_round_trip() {
        let json = r#"{
            "name": "test-preset",
            "description": "A test preset",
            "domain": "quality",
            "size": "lean",
            "version": "1.0.0",
            "agents": [
                {
                    "agent_key": "agent-a",
                    "name": "Agent A",
                    "role": "tester",
                    "goal": "test things"
                }
            ]
        }"#;
        let preset = load_preset_from_json(json).unwrap();
        assert_eq!(preset.name, "test-preset");
        assert_eq!(preset.domain, "quality");
        assert_eq!(preset.size, "lean");
        assert_eq!(preset.agents.len(), 1);
        assert_eq!(preset.agents[0].agent_key, "agent-a");

        // Round-trip back to JSON and parse again.
        let serialized = serde_json::to_string(&preset).unwrap();
        let restored = load_preset_from_json(&serialized).unwrap();
        assert_eq!(restored.name, preset.name);
        assert_eq!(restored.agents.len(), 1);
    }

    #[test]
    fn test_load_preset_from_json_invalid() {
        let result = load_preset_from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_all_presets_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let preset_json = r#"{
            "name": "p1",
            "description": "desc",
            "domain": "quality",
            "size": "lean",
            "version": "1.0.0",
            "agents": [{"agent_key":"a","name":"A","role":"r","goal":"g"}]
        }"#;
        fs::write(dir.path().join("p1.json"), preset_json).unwrap();
        // Non-JSON files should be skipped.
        fs::write(dir.path().join("readme.txt"), "ignored").unwrap();

        let presets = load_all_presets(dir.path()).unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "p1");
    }

    #[test]
    fn test_load_preset_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let preset_json = r#"{
            "name": "file-preset",
            "description": "from file",
            "domain": "devops",
            "size": "lean",
            "version": "2.0.0",
            "agents": [{"agent_key":"b","name":"B","role":"r","goal":"g"}]
        }"#;
        let path = dir.path().join("preset.json");
        fs::write(&path, preset_json).unwrap();

        let preset = load_preset_from_file(&path).unwrap();
        assert_eq!(preset.name, "file-preset");
        assert_eq!(preset.domain, "devops");
        assert_eq!(preset.version, "2.0.0");
    }
}
