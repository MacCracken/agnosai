use thiserror::Error;

pub type Result<T> = std::result::Result<T, AgnosaiError>;

#[derive(Debug, Error)]
pub enum AgnosaiError {
    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("crew not found: {0}")]
    CrewNotFound(String),

    #[error("invalid definition: {0}")]
    InvalidDefinition(String),

    #[error("task DAG contains a cycle")]
    CyclicDAG,

    #[error("scheduling error: {0}")]
    Scheduling(String),

    #[error("LLM provider error: {0}")]
    LlmProvider(String),

    #[error("tool execution error: {0}")]
    ToolExecution(String),

    #[error("sandbox error: {0}")]
    Sandbox(String),

    #[error("fleet error: {0}")]
    Fleet(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("{0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_not_found_display() {
        let e = AgnosaiError::AgentNotFound("agent-42".into());
        assert_eq!(e.to_string(), "agent not found: agent-42");
    }

    #[test]
    fn task_not_found_display() {
        let e = AgnosaiError::TaskNotFound("task-99".into());
        assert_eq!(e.to_string(), "task not found: task-99");
    }

    #[test]
    fn crew_not_found_display() {
        let e = AgnosaiError::CrewNotFound("crew-x".into());
        assert_eq!(e.to_string(), "crew not found: crew-x");
    }

    #[test]
    fn invalid_definition_display() {
        let e = AgnosaiError::InvalidDefinition("missing role".into());
        assert_eq!(e.to_string(), "invalid definition: missing role");
    }

    #[test]
    fn cyclic_dag_display() {
        let e = AgnosaiError::CyclicDAG;
        assert_eq!(e.to_string(), "task DAG contains a cycle");
    }

    #[test]
    fn scheduling_display() {
        let e = AgnosaiError::Scheduling("no slots".into());
        assert_eq!(e.to_string(), "scheduling error: no slots");
    }

    #[test]
    fn llm_provider_display() {
        let e = AgnosaiError::LlmProvider("rate limited".into());
        assert_eq!(e.to_string(), "LLM provider error: rate limited");
    }

    #[test]
    fn tool_execution_display() {
        let e = AgnosaiError::ToolExecution("crash".into());
        assert_eq!(e.to_string(), "tool execution error: crash");
    }

    #[test]
    fn sandbox_display() {
        let e = AgnosaiError::Sandbox("oom".into());
        assert_eq!(e.to_string(), "sandbox error: oom");
    }

    #[test]
    fn fleet_display() {
        let e = AgnosaiError::Fleet("unreachable".into());
        assert_eq!(e.to_string(), "fleet error: unreachable");
    }

    #[test]
    fn ipc_display() {
        let e = AgnosaiError::Ipc("broken pipe".into());
        assert_eq!(e.to_string(), "IPC error: broken pipe");
    }

    #[test]
    fn timeout_display() {
        let e = AgnosaiError::Timeout(std::time::Duration::from_secs(30));
        assert_eq!(e.to_string(), "timeout after 30s");
    }

    #[test]
    fn other_display() {
        let e = AgnosaiError::Other("something else".into());
        assert_eq!(e.to_string(), "something else");
    }

    #[test]
    fn from_serde_json_error() {
        let serde_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let e: AgnosaiError = serde_err.into();
        assert!(matches!(e, AgnosaiError::Serialization(_)));
        assert!(e.to_string().starts_with("serialization error:"));
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let e: AgnosaiError = io_err.into();
        assert!(matches!(e, AgnosaiError::Io(_)));
        assert!(e.to_string().starts_with("I/O error:"));
    }

    #[test]
    fn error_is_display() {
        // Ensure AgnosaiError implements Display (compile-time check via usage)
        let e = AgnosaiError::CyclicDAG;
        let _s: String = format!("{}", e);
    }
}
