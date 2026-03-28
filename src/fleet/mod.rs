//! Distributed fleet coordination, GPU scheduling, and multi-node orchestration.
//!
//! Provides node registry with heartbeat-based liveness, 5 placement policies,
//! GPU VRAM tracking and allocation, distributed crew state with barrier sync,
//! and fleet coordination with failover.

pub mod coordinator;
/// Cost-aware crew planning and model selection.
pub mod cost_planning;
/// Multi-node fleet discovery backends.
pub mod discovery;
/// Container/VM environment detection and resource limits.
pub mod environment;
pub mod federation;
pub mod gpu;
pub mod placement;
pub mod registry;
pub mod relay;
pub mod state;
/// Topology-aware scheduling — NVLink/XGMI-aware placement.
pub mod topology;

pub use coordinator::FleetCoordinator;
pub use federation::FederationManager;
pub use placement::{PlacementPolicy, PlacementRequest, PlacementResult, place, rank_nodes};
pub use registry::{NodeId, NodeInfo, NodeRegistry, NodeStatus};
pub use relay::Relay;
pub use state::CrewRunId;
