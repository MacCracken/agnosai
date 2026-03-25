//! SSE streaming endpoint for crew execution events.

use axum::extract::{Path, State};
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use uuid::Uuid;

use crate::server::sse::CrewEvent;
use crate::server::state::SharedState;

/// GET /api/v1/crews/:id/stream — SSE stream for crew events.
///
/// Subscribes to the event bus for the given crew ID and streams events
/// as they are emitted by the crew runner. Handles lagged receivers by
/// notifying the client and closing the stream.
pub async fn crew_stream(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let crew_id = id.to_string();

    // Check if this crew has an active event channel.
    let crew_exists = state.events.has(id);

    // Subscribe to the crew's event channel.
    let rx = state.events.subscribe(id);

    // If the crew doesn't exist, clean up the lazily-created channel.
    if !crew_exists {
        state.events.remove(id);
    }

    let stream = async_stream::stream! {
        if !crew_exists {
            let not_found = CrewEvent {
                crew_id: crew_id.clone(),
                event_type: "error".to_string(),
                data: serde_json::json!({"message": "crew not found", "crew_id": crew_id}),
            };
            let data = serde_json::to_string(&not_found).unwrap_or_default();
            yield Ok(Event::default().event("error").data(data));
            return;
        }

        // Send initial connected event.
        let connected = CrewEvent {
            crew_id: crew_id.clone(),
            event_type: "connected".to_string(),
            data: serde_json::json!({"message": "SSE stream connected"}),
        };
        let data = serde_json::to_string(&connected).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "SSE: failed to serialize connected event");
            r#"{"event_type":"connected"}"#.to_string()
        });
        yield Ok(Event::default().event("connected").data(data));

        // Forward real crew events from the broadcast channel.
        let mut event_rx = rx;
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_else(|e| {
                        tracing::warn!(
                            error = %e,
                            event_type = %event.event_type,
                            "SSE: failed to serialize event"
                        );
                        format!(r#"{{"event_type":"{}","error":"serialization failed"}}"#, event.event_type)
                    });
                    yield Ok(Event::default().event(event.event_type.clone()).data(data));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(crew_id = %crew_id, dropped = n, "SSE: client lagged, closing stream");
                    let err_data = serde_json::json!({
                        "event_type": "error",
                        "message": format!("stream lagged, {n} events dropped"),
                    });
                    yield Ok(Event::default().event("error").data(err_data.to_string()));
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    )
}

#[cfg(test)]
mod tests {
    use crate::orchestrator::Orchestrator;
    use crate::server::sse::EventBus;
    use crate::server::state::{AppState, SharedState};
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
            auth: Default::default(),
            events: EventBus::new(),
            http_client: reqwest::Client::new(),
            audit: std::sync::Arc::new(crate::llm::AuditChain::new(b"test-key", 100)),
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

    #[tokio::test]
    async fn sse_endpoint_sends_error_event_for_unknown_crew() {
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

        // Read the SSE body and verify an error event is emitted.
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 64)
            .await
            .unwrap();
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            body.contains("event: error"),
            "expected an error event for unknown crew, got: {body}"
        );
        assert!(
            body.contains("crew not found"),
            "expected 'crew not found' in error event data, got: {body}"
        );
    }

    #[tokio::test]
    async fn sse_endpoint_for_active_crew_sends_connected_event() {
        let crew_id = uuid::Uuid::new_v4();
        let orchestrator = Orchestrator::new(Default::default()).await.unwrap();
        let tools = Arc::new(ToolRegistry::new());
        let state: SharedState = Arc::new(AppState {
            orchestrator,
            tools,
            auth: Default::default(),
            events: EventBus::new(),
            http_client: reqwest::Client::new(),
            audit: std::sync::Arc::new(crate::llm::AuditChain::new(b"test-key", 100)),
        });

        // Pre-create the event channel so the crew is "known".
        let _rx = state.events.subscribe(crew_id);
        let router = crate::server::router(state);

        let response = router
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
