//! Distributed fleet coordination, GPU scheduling, and multi-node orchestration.
//!
//! Provides node registry with heartbeat-based liveness, 5 placement policies,
//! GPU VRAM tracking and allocation, distributed crew state with barrier sync,
//! and fleet coordination with failover.

pub mod coordinator;
pub mod federation;
pub mod gpu;
pub mod placement;
pub mod registry;
pub mod relay;
pub mod state;
