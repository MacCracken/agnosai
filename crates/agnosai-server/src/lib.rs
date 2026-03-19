pub mod routes;
pub mod sse;
pub mod state;

pub use state::{AppState, SharedState};

use axum::routing::{get, post};
use axum::Router;

/// Build the complete application router with all routes.
pub fn router(state: SharedState) -> Router {
    let api_v1 = Router::new()
        .route("/crews", post(routes::crews::create_crew))
        .route("/crews/{id}", get(routes::crews::get_crew))
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
        .nest("/api/v1", api_v1)
}
