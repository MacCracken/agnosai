//! HTTP API server for AgnosAI — REST endpoints for crew execution and management.
//!
//! Built on axum. Provides health probes, crew execution, tool listing,
//! agent definition management, A2A delegation, and SSE streaming.

pub mod auth;
pub mod routes;
pub mod sse;
pub mod state;

pub use auth::AuthConfig;
pub use state::{AppState, SharedState};

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

/// Maximum request body size: 10 MiB.
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Build the complete application router with all routes.
pub fn router(state: SharedState) -> Router {
    let api_v1 = Router::new()
        .route("/crews", post(routes::crews::create_crew))
        .route("/crews/{id}", get(routes::crews::get_crew))
        .route("/crews/{id}/stream", get(routes::sse::crew_stream))
        .route("/a2a/receive", post(routes::a2a::receive))
        .route("/a2a/status", post(routes::a2a::status))
        .route(
            "/agents/definitions",
            get(routes::agents::list_definitions).post(routes::agents::create_definition),
        )
        .route("/tools", get(routes::tools::list_tools))
        .route("/presets", get(routes::definitions::list_presets))
        .with_state(state.clone());

    Router::new()
        .route("/health", get(routes::health::health))
        .route("/ready", get(routes::health::ready))
        .route("/mcp", post(routes::mcp::mcp_handler))
        .nest("/api/v1", api_v1)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}
