//! HTTP route handlers for the AgnosAI API.
//!
//! - [`crews`] — crew creation and execution (`/api/v1/crews`)
//! - [`agents`] — agent definition management (`/api/v1/agents/definitions`)
//! - [`tools`] — tool listing (`/api/v1/tools`)
//! - [`definitions`] — preset listing (`/api/v1/presets`)
//! - [`sse`] — SSE streaming (`/api/v1/crews/{id}/stream`)
//! - [`a2a`] — Agent-to-Agent delegation (`/api/v1/a2a`)
//! - [`mcp`] — Model Context Protocol JSON-RPC (`/mcp`)
//! - [`health`] — health and readiness probes (`/health`, `/ready`)

pub mod a2a;
pub mod agents;
pub mod crews;
pub mod definitions;
pub mod health;
pub mod mcp;
pub mod sse;
pub mod tools;
