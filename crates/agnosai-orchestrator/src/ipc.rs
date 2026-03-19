//! Unix socket IPC with length-prefixed framing.
//!
//! Protocol (from Agnosticos):
//! - 4-byte big-endian length prefix
//! - JSON payload
//! - Used for local agent-to-orchestrator communication
//!
//! For inter-node communication, see `agnosai-fleet/relay.rs`.

// TODO: Port from Agnosticos ipc.rs
