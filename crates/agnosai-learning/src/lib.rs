//! Adaptive learning and reinforcement learning for agent optimization.
//!
//! Pure Rust implementations of RL primitives — no Python ML libraries:
//!
//! - [`PerformanceProfile`] — success rate and duration tracking per agent
//! - [`Ucb1`] — multi-armed bandit for strategy selection
//! - [`ReplayBuffer`] — prioritized experience replay
//! - [`CapabilityScorer`] — dynamic confidence scoring with trend detection
//! - [`QLearner`] — tabular Q-learning

pub mod capability;
pub mod optimizer;
pub mod profile;
pub mod replay;
pub mod strategy;

// Re-export key types.
pub use capability::{CapabilityScore, CapabilityScorer, Trend};
pub use optimizer::QLearner;
pub use profile::{ActionRecord, PerformanceProfile};
pub use replay::{Experience, ReplayBuffer};
pub use strategy::{ArmStats, Ucb1};
