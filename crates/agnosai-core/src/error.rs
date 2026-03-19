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
