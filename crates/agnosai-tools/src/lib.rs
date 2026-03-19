//! Tool registry and execution — native Rust, WASM, and Python bridge.
//!
//! Tools are the actions agents can perform. Every tool implements the
//! [`NativeTool`] trait and is registered in a thread-safe [`ToolRegistry`].
//!
//! # Built-in Tools
//!
//! - `echo` — echo input (testing)
//! - `json_transform` — extract JSON fields
//! - 9 AGNOS ecosystem tools (Synapse, Mneme, Delta)

pub mod builtin;
pub mod native;
pub mod python_tool;
pub mod registry;
pub mod wasm_tool;

// Re-export key types for convenience.
pub use native::{NativeTool, ParameterSchema, ToolInput, ToolOutput, ToolSchema};
pub use registry::ToolRegistry;
