//! Task scheduling, agent coordination, and crew execution.
//!
//! The orchestrator is the runtime core of AgnosAI. It manages crew lifecycles,
//! schedules tasks across priority tiers, resolves DAG dependencies, scores
//! agents for task assignment, and provides pub/sub event communication.
//!
//! # Key Components
//!
//! - [`Orchestrator`] — top-level entry point, wraps `Arc<RwLock<State>>`
//! - [`CrewRunner`](crew_runner::CrewRunner) — executes a crew through its full lifecycle
//! - [`Scheduler`](scheduler::Scheduler) — priority queue + DAG topological sort
//! - [`PubSub`](pubsub::PubSub) — topic-based pub/sub with wildcard matching

pub mod crew_runner;
pub mod ipc;
pub mod orchestrator;
pub mod pubsub;
pub mod scheduler;
pub mod scoring;

pub use orchestrator::Orchestrator;
