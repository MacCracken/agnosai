//! Core types, traits, and error handling for AgnosAI.
//!
//! This crate provides the foundational types shared across all AgnosAI crates:
//! agents, tasks, crews, messages, resources, and errors. It has no I/O
//! dependencies — all types are pure data with serde support.
//!
//! # Example
//!
//! ```
//! use agnosai::core::{AgentDefinition, Task, CrewSpec};
//!
//! let agent = AgentDefinition::from_json(r#"{
//!     "agent_key": "analyst",
//!     "name": "Analyst",
//!     "role": "data analyst",
//!     "goal": "analyze data"
//! }"#).unwrap();
//!
//! let task = Task::new("Analyze quarterly revenue trends");
//! let crew = CrewSpec::new("analysis-crew");
//! ```

pub mod agent;
pub mod crew;
pub mod error;
pub mod message;
pub mod resource;
pub mod task;

pub use agent::{AgentDefinition, AgentId, AgentState};
pub use crew::{CrewSpec, CrewState, CrewStatus};
pub use error::{AgnosaiError, Result};
pub use message::Message;
pub use resource::{
    AcceleratorFamily, AcceleratorType, ComputeDevice, HardwareInventory, HardwareRequirement,
    ResourceBudget,
};
#[cfg(feature = "hwaccel")]
pub use resource::{
    HwAccelFamily, HwAccelRequirement, HwAccelType, MemoryEstimate, ModelShard, QuantizationLevel,
    ShardingPlan, ShardingStrategy, TrainingMemoryEstimate, TrainingMethod, TrainingTarget,
};
pub use task::{ProcessMode, Task, TaskDAG, TaskPriority, TaskResult, TaskStatus};
