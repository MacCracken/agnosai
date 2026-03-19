//! Crew lifecycle: assemble → execute → aggregate.
//!
//! Replaces CrewAI's Crew class with a Rust-native implementation.
//!
//! Flow:
//! 1. Load agent definitions (JSON/YAML)
//! 2. Build task DAG from crew spec
//! 3. Topological sort → resolve execution order
//! 4. Score agents for each task
//! 5. Execute tasks (respecting concurrency limits and dependencies)
//! 6. Aggregate results into CrewState

// TODO: Implement crew runner
