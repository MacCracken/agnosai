//! Topic-based pub/sub with wildcard matching.
//!
//! From Agnosticos — supports patterns like:
//! - `"task.*"` matches `"task.completed"`, `"task.failed"`
//! - `"agent.#"` matches `"agent.assigned"`, `"agent.status.changed"`
//!
//! Used for decoupled inter-agent event communication within a single node.
//! For cross-node pub/sub, see `agnosai-fleet/relay.rs`.

// TODO: Port from Agnosticos pubsub.rs
