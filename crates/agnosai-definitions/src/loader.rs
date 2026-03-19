//! JSON/YAML agent definition loading.

use std::path::Path;

use agnosai_core::agent::AgentDefinition;
use agnosai_core::{AgnosaiError, Result};

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
}
