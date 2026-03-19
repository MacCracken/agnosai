//! Tool registration, lookup, and capability matching.

use crate::native::{NativeTool, ToolSchema};
use dashmap::DashMap;
use std::sync::Arc;

/// Thread-safe registry for native tools.
///
/// Tools are keyed by their [`NativeTool::name`]. The registry is backed by
/// [`DashMap`] so concurrent reads and writes are lock-free.
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn NativeTool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
        }
    }

    /// Register a tool.  If a tool with the same name already exists it will
    /// be replaced.
    pub fn register(&self, tool: Arc<dyn NativeTool>) {
        let name = tool.name().to_owned();
        tracing::debug!(tool = %name, "registering tool");
        self.tools.insert(name, tool);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn NativeTool>> {
        self.tools.get(name).map(|entry| Arc::clone(entry.value()))
    }

    /// Return the schemas of all registered tools (unordered).
    pub fn list(&self) -> Vec<ToolSchema> {
        self.tools.iter().map(|entry| entry.value().schema()).collect()
    }

    /// Check whether a tool with the given name is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Remove a tool by name.  Returns `true` if it was present.
    pub fn remove(&self, name: &str) -> bool {
        self.tools.remove(name).is_some()
    }

    /// Number of currently registered tools.
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::echo::EchoTool;
    use crate::builtin::json_transform::JsonTransformTool;

    #[test]
    fn register_and_get() {
        let reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        assert!(reg.has("echo"));
        assert_eq!(reg.count(), 1);
        let tool = reg.get("echo").expect("tool should exist");
        assert_eq!(tool.name(), "echo");
    }

    #[test]
    fn list_schemas() {
        let reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        reg.register(Arc::new(JsonTransformTool));
        let schemas = reg.list();
        assert_eq!(schemas.len(), 2);
        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"json_transform"));
    }

    #[test]
    fn remove_tool() {
        let reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        assert!(reg.remove("echo"));
        assert!(!reg.has("echo"));
        assert_eq!(reg.count(), 0);
        // removing again returns false
        assert!(!reg.remove("echo"));
    }

    #[test]
    fn get_missing_returns_none() {
        let reg = ToolRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn register_multiple_tools_verify_count() {
        let reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        reg.register(Arc::new(JsonTransformTool));
        assert_eq!(reg.count(), 2);
        assert!(reg.has("echo"));
        assert!(reg.has("json_transform"));
    }

    #[test]
    fn register_same_name_overwrites() {
        let reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        assert_eq!(reg.count(), 1);
        let schema_before = reg.get("echo").unwrap().schema();
        assert_eq!(schema_before.parameters.len(), 1);

        // Register another EchoTool under the same name — count stays 1.
        reg.register(Arc::new(EchoTool));
        assert_eq!(reg.count(), 1);
        assert!(reg.has("echo"));
    }

    #[tokio::test]
    async fn concurrent_access() {
        let reg = Arc::new(ToolRegistry::new());
        let mut handles = Vec::new();

        // Spawn 20 tasks that register tools concurrently.
        for i in 0..20 {
            let reg_clone = reg.clone();
            handles.push(tokio::spawn(async move {
                // Alternate between registering echo and json_transform to
                // exercise concurrent insert + read paths.
                if i % 2 == 0 {
                    reg_clone.register(Arc::new(EchoTool));
                } else {
                    reg_clone.register(Arc::new(JsonTransformTool));
                }
                // Concurrent reads while writes are happening.
                let _ = reg_clone.list();
                let _ = reg_clone.get("echo");
                let _ = reg_clone.has("json_transform");
                reg_clone.count()
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        // After all tasks complete, both tools should be registered.
        assert!(reg.has("echo"));
        assert!(reg.has("json_transform"));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn list_returns_schemas_for_all_registered() {
        let reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        reg.register(Arc::new(JsonTransformTool));

        let schemas = reg.list();
        assert_eq!(schemas.len(), 2);

        for schema in &schemas {
            assert!(!schema.name.is_empty());
            assert!(!schema.description.is_empty());
        }

        let names: Vec<&str> = schemas.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"echo"));
        assert!(names.contains(&"json_transform"));

        // Verify parameter details are present.
        let echo_schema = schemas.iter().find(|s| s.name == "echo").unwrap();
        assert_eq!(echo_schema.parameters.len(), 1);
        assert_eq!(echo_schema.parameters[0].name, "message");

        let jt_schema = schemas.iter().find(|s| s.name == "json_transform").unwrap();
        assert_eq!(jt_schema.parameters.len(), 2);
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let reg = ToolRegistry::new();
        assert!(!reg.remove("does_not_exist"));
        assert!(!reg.remove(""));
        assert!(!reg.remove("some_random_tool"));
    }
}
