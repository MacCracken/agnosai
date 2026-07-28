//! HTTP route handlers for the AgnosAI API.
//!
//! - `crews` — crew creation and execution (`/api/v1/crews`)
//! - `agents` — agent definition management (`/api/v1/agents/definitions`)
//! - `tools` — tool listing (`/api/v1/tools`)
//! - `definitions` — preset listing (`/api/v1/presets`)
//! - `sse` — SSE streaming (`/api/v1/crews/{id}/stream`)
//! - `a2a` — Agent-to-Agent delegation (`/api/v1/a2a`)
//! - `mcp` — Model Context Protocol JSON-RPC (`/mcp`)
//! - `health` — health and readiness probes (`/health`, `/ready`)

/// Agent-to-Agent task delegation.
pub mod a2a;
/// Agent definition management.
pub mod agents;
/// Human-in-the-loop approval endpoints.
pub mod approval;
/// Crew creation and execution.
pub mod crews;
/// Dashboard API — crew history and agent performance.
pub mod dashboard;
/// Preset listing.
pub mod definitions;
/// Health and readiness probes.
pub mod health;
/// Model Context Protocol (JSON-RPC).
pub mod mcp;
/// Server-sent events for crew streaming.
pub mod sse;
/// Tool listing.
pub mod tools;
