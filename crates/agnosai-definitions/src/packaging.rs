//! .agpkg ZIP bundle export/import.

use std::io::{Cursor, Read, Write};
use std::path::Path;

use agnosai_core::agent::AgentDefinition;
use agnosai_core::{AgnosaiError, Result};
use serde::{Deserialize, Serialize};

/// Manifest stored at the root of an `.agpkg` ZIP bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageManifest {
    name: String,
    version: String,
    metadata: serde_json::Value,
}

/// An `.agpkg` package containing agent definitions and metadata.
#[derive(Debug)]
pub struct AgnosPackage {
    pub name: String,
    pub version: String,
    pub definitions: Vec<AgentDefinition>,
    pub metadata: serde_json::Value,
}

impl AgnosPackage {
    /// Create a new empty package.
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            definitions: Vec::new(),
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Add an agent definition to the package.
    pub fn add_definition(&mut self, def: AgentDefinition) {
        self.definitions.push(def);
    }

    /// Export to a ZIP bundle in memory (returns bytes).
    ///
    /// ZIP layout:
    /// - `manifest.json` — package name, version, and metadata
    /// - `definitions/<agent_key>.json` — one file per agent definition
    pub fn export(&self) -> Result<Vec<u8>> {
        let buf = Vec::new();
        let cursor = Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        // Write manifest.
        let manifest = PackageManifest {
            name: self.name.clone(),
            version: self.version.clone(),
            metadata: self.metadata.clone(),
        };
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        zip.start_file("manifest.json", options)
            .map_err(|e| AgnosaiError::Other(format!("zip error: {e}")))?;
        zip.write_all(manifest_json.as_bytes())?;

        // Write each definition.
        for def in &self.definitions {
            let filename = format!("definitions/{}.json", def.agent_key);
            let def_json = serde_json::to_string_pretty(def)?;
            zip.start_file(&filename, options)
                .map_err(|e| AgnosaiError::Other(format!("zip error: {e}")))?;
            zip.write_all(def_json.as_bytes())?;
        }

        let cursor = zip
            .finish()
            .map_err(|e| AgnosaiError::Other(format!("zip finish error: {e}")))?;
        Ok(cursor.into_inner())
    }

    /// Import from ZIP bytes.
    pub fn import(data: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AgnosaiError::InvalidDefinition(format!("invalid zip: {e}")))?;

        // Read manifest.
        let manifest: PackageManifest = {
            let mut file = archive.by_name("manifest.json").map_err(|e| {
                AgnosaiError::InvalidDefinition(format!("missing manifest.json: {e}"))
            })?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            serde_json::from_str(&content)?
        };

        // Read definitions.
        let mut definitions = Vec::new();
        let def_names: Vec<String> = (0..archive.len())
            .filter_map(|i| {
                let file = archive.by_index(i).ok()?;
                let name = file.name().to_string();
                if name.starts_with("definitions/") && name.ends_with(".json") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        for name in def_names {
            let mut file = archive
                .by_name(&name)
                .map_err(|e| AgnosaiError::Other(format!("zip read error: {e}")))?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let def: AgentDefinition = serde_json::from_str(&content)?;
            definitions.push(def);
        }

        Ok(Self {
            name: manifest.name,
            version: manifest.version,
            definitions,
            metadata: manifest.metadata,
        })
    }

    /// Export to a file on disk.
    pub fn export_to_file(&self, path: &Path) -> Result<()> {
        let bytes = self.export()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Import from a file on disk.
    pub fn import_from_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::import(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_definition(key: &str) -> AgentDefinition {
        AgentDefinition {
            agent_key: key.to_string(),
            name: format!("Agent {key}"),
            role: "tester".to_string(),
            goal: "test things".to_string(),
            backstory: None,
            domain: Some("quality".to_string()),
            tools: vec!["tool_a".to_string()],
            complexity: "medium".to_string(),
            llm_model: None,
            gpu_required: false,
            gpu_preferred: false,
            gpu_memory_min_mb: None,
            hardware: None,
        }
    }

    #[test]
    fn test_export_import_round_trip() {
        let mut pkg = AgnosPackage::new("test-pkg".into(), "1.0.0".into());
        pkg.add_definition(make_definition("agent-one"));

        let bytes = pkg.export().unwrap();
        let restored = AgnosPackage::import(&bytes).unwrap();

        assert_eq!(restored.name, "test-pkg");
        assert_eq!(restored.version, "1.0.0");
        assert_eq!(restored.definitions.len(), 1);
        assert_eq!(restored.definitions[0].agent_key, "agent-one");
        assert_eq!(restored.definitions[0].name, "Agent agent-one");
        assert_eq!(restored.definitions[0].domain.as_deref(), Some("quality"));
    }

    #[test]
    fn test_package_with_multiple_definitions() {
        let mut pkg = AgnosPackage::new("multi-pkg".into(), "2.0.0".into());
        pkg.add_definition(make_definition("alpha"));
        pkg.add_definition(make_definition("beta"));
        pkg.add_definition(make_definition("gamma"));

        let bytes = pkg.export().unwrap();
        let restored = AgnosPackage::import(&bytes).unwrap();

        assert_eq!(restored.definitions.len(), 3);
        let keys: Vec<&str> = restored
            .definitions
            .iter()
            .map(|d| d.agent_key.as_str())
            .collect();
        assert!(keys.contains(&"alpha"));
        assert!(keys.contains(&"beta"));
        assert!(keys.contains(&"gamma"));
    }

    #[test]
    fn test_import_invalid_data_returns_error() {
        let result = AgnosPackage::import(b"this is not a zip file");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_empty_zip_missing_manifest_returns_error() {
        // Create a valid ZIP with no manifest.json.
        let buf = Vec::new();
        let cursor = Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("other.txt", options).unwrap();
        zip.write_all(b"hello").unwrap();
        let cursor = zip.finish().unwrap();
        let bytes = cursor.into_inner();

        let result = AgnosPackage::import(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_export_import_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.agpkg");

        let mut pkg = AgnosPackage::new("file-pkg".into(), "3.0.0".into());
        pkg.add_definition(make_definition("file-agent"));
        pkg.metadata = serde_json::json!({"author": "test"});

        pkg.export_to_file(&path).unwrap();
        assert!(path.exists());

        let restored = AgnosPackage::import_from_file(&path).unwrap();
        assert_eq!(restored.name, "file-pkg");
        assert_eq!(restored.version, "3.0.0");
        assert_eq!(restored.definitions.len(), 1);
        assert_eq!(restored.definitions[0].agent_key, "file-agent");
        assert_eq!(restored.metadata["author"], "test");
    }

    #[test]
    fn test_new_package_has_empty_metadata() {
        let pkg = AgnosPackage::new("empty".into(), "0.1.0".into());
        assert!(pkg.definitions.is_empty());
        assert!(pkg.metadata.is_object());
        assert_eq!(pkg.metadata.as_object().unwrap().len(), 0);
    }

    #[test]
    fn test_metadata_preserved_through_round_trip() {
        let mut pkg = AgnosPackage::new("meta-test".into(), "1.0.0".into());
        pkg.metadata = serde_json::json!({
            "created_by": "agnosai",
            "tags": ["quality", "lean"],
            "priority": 5
        });
        pkg.add_definition(make_definition("m"));

        let bytes = pkg.export().unwrap();
        let restored = AgnosPackage::import(&bytes).unwrap();

        assert_eq!(restored.metadata["created_by"], "agnosai");
        assert_eq!(restored.metadata["tags"][0], "quality");
        assert_eq!(restored.metadata["priority"], 5);
    }
}
