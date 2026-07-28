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

/// Human-in-the-loop approval gates for task results.
pub mod approval;
/// Token and cost budget enforcement.
pub mod budget;
/// Crew lifecycle runner (assemble, execute, aggregate).
pub mod crew_runner;
/// Durable crew state persistence and recovery.
pub mod durable_state;
/// Hierarchical process mode — manager-driven task delegation.
pub mod hierarchical;
/// Inter-process communication utilities.
pub mod ipc;
/// Multi-turn conversation memory for agents.
pub mod memory;
/// Multi-tenancy with per-tenant budget enforcement.
pub mod multi_tenant;
#[allow(clippy::module_inception)]
/// Top-level orchestrator struct and crew execution.
pub mod orchestrator;
/// Structured output validation and retry logic.
pub mod output_validation;
/// Plan caching for repeated crew executions.
pub mod plan_cache;
/// Topic-based publish/subscribe with wildcard matching.
pub mod pubsub;
/// Priority queue and DAG topological sort.
pub mod scheduler;
/// Agent-task scoring and ranking.
pub mod scoring;

pub use orchestrator::Orchestrator;
