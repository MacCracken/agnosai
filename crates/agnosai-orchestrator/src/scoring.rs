use agnosai_core::agent::AgentDefinition;
use agnosai_core::task::Task;

/// Score an agent's suitability for a given task.
///
/// Scoring factors (from Agnosticos):
/// - Capability match: do the agent's tools cover the task requirements?
/// - Complexity alignment: does the agent's complexity level match the task?
/// - GPU affinity: does the agent need/prefer GPU and is one available?
/// - Current load: is the agent idle or already overloaded?
pub fn score_agent(_agent: &AgentDefinition, _task: &Task) -> f64 {
    // TODO: Port scoring logic from Agnosticos scoring.rs
    0.0
}
