//! Built-in native tools.
//!
//! Includes simple utility tools (echo, json_transform), HTTP-client tools
//! for optional AGNOS sibling services (Synapse, Mneme, Delta), and ported
//! high-value tools from the Agnostic Python platform (load_testing,
//! security_audit).

pub mod delta;
pub mod echo;
pub mod json_transform;
pub mod load_testing;
pub mod mneme;
pub mod security_audit;
pub mod synapse;
