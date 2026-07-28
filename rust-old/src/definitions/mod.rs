//! Agent definition loading, crew assembly, and versioning.
//!
//! Handles the declarative side of AgnosAI: loading agent definitions from
//! JSON/YAML files, assembling crews from team specs, and tracking definition
//! versions with rollback support.

pub mod assembler;
/// Kubernetes CRD type definitions for crew/agent resources.
pub mod k8s_crd;
pub mod loader;
pub mod packaging;
pub mod versioning;

// Re-exports for convenience.
pub use loader::{PresetSpec, builtin_presets};
pub use packaging::AgnosPackage;
