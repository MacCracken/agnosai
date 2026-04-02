//! HTTP API server for AgnosAI — REST endpoints for crew execution and management.
//!
//! Built on axum. Provides health probes, crew execution, tool listing,
//! agent definition management, A2A delegation, and SSE streaming.

/// Authentication and authorization middleware.
pub mod auth;
/// Hot-reload configuration via watch channel.
pub mod hot_config;
/// Sensitive information output filter for LLM responses.
pub mod output_filter;
/// Prometheus-compatible metrics collection.
pub mod prometheus;
/// Prompt injection detection and input sanitization.
pub mod prompt_guard;
/// Per-endpoint rate limiting backed by majra.
#[cfg(feature = "majra")]
pub mod rate_limit;
/// HTTP route handlers.
pub mod routes;
/// Server-sent event streaming.
pub mod sse;
/// SSRF protection utilities.
pub mod ssrf;
/// Shared application state.
pub mod state;

pub use auth::AuthConfig;
pub use state::{AppState, SharedState};

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;
use axum::routing::{get, post};

use auth::auth_middleware;

/// Maximum request body size: 10 MiB.
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

/// Maximum concurrent requests per route (simple rate limiter).
const MAX_CONCURRENT_REQUESTS: usize = 100;

/// Build the complete application router with all routes.
///
/// Auth middleware is applied to `/api/v1/*` and `/mcp`. Health and readiness
/// probes (`/health`, `/ready`) are always public.
pub fn router(state: SharedState) -> Router {
    let auth_config = state.auth.clone();

    let api_v1 = Router::new()
        .route("/crews", post(routes::crews::create_crew))
        .route("/crews/{id}", get(routes::crews::get_crew))
        .route("/crews/{id}/cancel", post(routes::crews::cancel_crew))
        .route("/crews/{id}/stream", get(routes::sse::crew_stream))
        .route("/a2a/receive", post(routes::a2a::receive))
        .route("/a2a/status", post(routes::a2a::status))
        .route(
            "/agents/definitions",
            get(routes::agents::list_definitions).post(routes::agents::create_definition),
        )
        .route("/tools", get(routes::tools::list_tools))
        .route(
            "/tools/{name}",
            axum::routing::delete(routes::tools::remove_tool),
        )
        .route(
            "/approvals",
            get(routes::approval::list_pending).post(routes::approval::submit_approval),
        )
        .route("/presets", get(routes::definitions::list_presets))
        .route("/dashboard/crews", get(routes::dashboard::crew_history))
        .route(
            "/dashboard/agents",
            get(routes::dashboard::agent_performance),
        )
        .with_state(state.clone());

    // Protected routes: /api/v1/* and /mcp require auth (when enabled).
    let protected = Router::new()
        .route("/mcp", post(routes::mcp::mcp_handler))
        .nest("/api/v1", api_v1)
        .layer(middleware::from_fn(move |req, next| {
            let cfg = auth_config.clone();
            async move { auth_middleware(cfg, req, next).await }
        }))
        .with_state(state.clone());

    // Public routes: health probes and metrics are always accessible.
    Router::new()
        .route("/health", get(routes::health::health))
        .route("/ready", get(routes::health::ready))
        .route("/metrics", get(routes::health::metrics))
        .merge(protected)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(tower::limit::ConcurrencyLimitLayer::new(
            MAX_CONCURRENT_REQUESTS,
        ))
        .with_state(state)
}
