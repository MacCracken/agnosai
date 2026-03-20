//! SSE streaming endpoint for crew execution events.

use axum::extract::Path;
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use uuid::Uuid;

use crate::server::sse::CrewEvent;

/// GET /api/v1/crews/:id/stream — SSE stream for crew events.
///
/// For now, returns a simple stream that sends a "connected" event then closes.
/// Full integration with `CrewRunner` events is future work.
pub async fn crew_stream(
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let crew_id = id.to_string();

    let stream = async_stream::stream! {
        // Send initial connected event.
        let connected = CrewEvent {
            crew_id: crew_id.clone(),
            event_type: "connected".to_string(),
            data: serde_json::json!({"message": "SSE stream connected"}),
        };
        let data = serde_json::to_string(&connected).unwrap_or_default();
        yield Ok(Event::default().event("connected").data(data));

        // Future: subscribe to broadcast channel for real crew events.
        // For now the stream ends after the connected event.
    };

    Sse::new(stream)
}

#[cfg(test)]
mod tests {
    use crate::server::state::{AppState, SharedState};
    use crate::orchestrator::Orchestrator;
    use crate::tools::ToolRegistry;
    use axum::Router;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn test_app() -> Router {
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
        });
        crate::server::router(state)
    }

    #[tokio::test]
    async fn sse_endpoint_returns_event_stream_content_type() {
        let app = test_app().await;
        let crew_id = uuid::Uuid::new_v4();
        let response = app
            .oneshot(
                Request::get(format!("/api/v1/crews/{crew_id}/stream"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            content_type.contains("text/event-stream"),
            "expected text/event-stream, got {content_type}"
        );
    }
}
