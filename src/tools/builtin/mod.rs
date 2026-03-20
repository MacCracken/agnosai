//! Built-in native tools.
//!
//! Includes simple utility tools (echo, json_transform) and HTTP-client tools
//! for optional AGNOS sibling services (Synapse, Mneme, Delta). The sibling
//! service tools are not hard dependencies — they fail gracefully when the
//! target service is unavailable.

pub mod delta;
pub mod echo;
pub mod json_transform;
pub mod mneme;
pub mod synapse;
