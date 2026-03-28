//! Durable crew state persistence and recovery.
//!
//! Provides a `StateStore` trait for pluggable state backends, with
//! `FileStateStore` for file-system persistence. Crew state is serialized
//! to JSON for human-readable snapshots.

use std::path::PathBuf;

use crate::core::AgnosaiError;
use crate::core::crew::CrewState;
use crate::core::error::Result;

/// Trait for durable crew state storage backends.
///
/// Implementations persist serialized crew state so that crews can be
/// resumed after crashes or restarts.
pub trait StateStore: Send + Sync {
    /// Save crew state as raw bytes.
    fn save(
        &self,
        crew_id: &str,
        state: &[u8],
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Load crew state, returning `None` if no state exists for this crew.
    fn load(
        &self,
        crew_id: &str,
    ) -> impl std::future::Future<Output = Result<Option<Vec<u8>>>> + Send;
}

/// File-system backed state store.
///
/// Writes each crew's state to `{base_dir}/{crew_id}.json`. The base directory
/// is created on first write if it does not exist.
#[derive(Debug, Clone)]
pub struct FileStateStore {
    base_dir: PathBuf,
}

impl FileStateStore {
    /// Create a new file state store rooted at `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Return the file path for a given crew ID.
    #[must_use]
    #[inline]
    fn path_for(&self, crew_id: &str) -> PathBuf {
        self.base_dir.join(format!("{crew_id}.json"))
    }
}

impl StateStore for FileStateStore {
    async fn save(&self, crew_id: &str, state: &[u8]) -> Result<()> {
        tracing::debug!(crew_id, dir = %self.base_dir.display(), "saving crew state to file");
        tokio::fs::create_dir_all(&self.base_dir).await?;
        let path = self.path_for(crew_id);
        tokio::fs::write(&path, state).await?;
        tracing::info!(crew_id, path = %path.display(), "crew state saved");
        Ok(())
    }

    async fn load(&self, crew_id: &str) -> Result<Option<Vec<u8>>> {
        let path = self.path_for(crew_id);
        tracing::debug!(crew_id, path = %path.display(), "loading crew state from file");
        match tokio::fs::read(&path).await {
            Ok(data) => {
                tracing::info!(crew_id, bytes = data.len(), "crew state loaded");
                Ok(Some(data))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(crew_id, "no saved state found");
                Ok(None)
            }
            Err(e) => Err(AgnosaiError::Io(e)),
        }
    }
}

/// Serialize a [`CrewState`] to JSON bytes.
///
/// # Errors
///
/// Returns `AgnosaiError::Serialization` if serialization fails.
#[must_use = "serialized bytes should be persisted or transmitted"]
pub fn serialize_crew_state(state: &CrewState) -> std::result::Result<Vec<u8>, AgnosaiError> {
    let bytes = serde_json::to_vec_pretty(state)?;
    Ok(bytes)
}

/// Deserialize a [`CrewState`] from JSON bytes.
///
/// # Errors
///
/// Returns `AgnosaiError::Serialization` if deserialization fails.
pub fn deserialize_crew_state(data: &[u8]) -> std::result::Result<CrewState, AgnosaiError> {
    let state = serde_json::from_slice(data)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::crew::{CrewState, CrewStatus};
    use uuid::Uuid;

    fn sample_state() -> CrewState {
        CrewState {
            crew_id: Uuid::new_v4(),
            status: CrewStatus::Running,
            results: vec![],
            profile: None,
        }
    }

    #[test]
    fn serialize_deserialize_round_trip() {
        let state = sample_state();
        let bytes = serialize_crew_state(&state).unwrap();
        let restored = deserialize_crew_state(&bytes).unwrap();
        assert_eq!(restored.crew_id, state.crew_id);
        assert_eq!(restored.status, CrewStatus::Running);
        assert!(restored.results.is_empty());
    }

    #[test]
    fn serialize_produces_valid_json() {
        let state = sample_state();
        let bytes = serialize_crew_state(&state).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json.get("crew_id").is_some());
        assert!(json.get("status").is_some());
    }

    #[test]
    fn deserialize_invalid_data() {
        let bad = b"not valid json";
        let result = deserialize_crew_state(bad);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn file_state_store_save_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path());

        let state = sample_state();
        let crew_id = state.crew_id.to_string();
        let bytes = serialize_crew_state(&state).unwrap();

        store.save(&crew_id, &bytes).await.unwrap();

        let loaded = store.load(&crew_id).await.unwrap();
        assert!(loaded.is_some());
        let restored = deserialize_crew_state(&loaded.unwrap()).unwrap();
        assert_eq!(restored.crew_id, state.crew_id);
    }

    #[tokio::test]
    async fn file_state_store_load_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path());
        let loaded = store.load("nonexistent-crew").await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn file_state_store_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("sub").join("dir");
        let store = FileStateStore::new(&nested);

        let bytes = b"test data";
        store.save("crew-1", bytes).await.unwrap();

        assert!(nested.exists());
        let loaded = store.load("crew-1").await.unwrap().unwrap();
        assert_eq!(loaded, bytes);
    }

    #[tokio::test]
    async fn file_state_store_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let store = FileStateStore::new(tmp.path());

        store.save("crew-x", b"version-1").await.unwrap();
        store.save("crew-x", b"version-2").await.unwrap();

        let loaded = store.load("crew-x").await.unwrap().unwrap();
        assert_eq!(loaded, b"version-2");
    }

    #[test]
    fn file_state_store_path_for() {
        let store = FileStateStore::new("/tmp/agnosai-state");
        let path = store.path_for("my-crew-123");
        assert_eq!(path, PathBuf::from("/tmp/agnosai-state/my-crew-123.json"));
    }
}
