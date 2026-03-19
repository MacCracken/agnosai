//! Server-sent events for streaming crew execution progress.

use axum::response::sse::Event;
use futures::stream::Stream;
use std::convert::Infallible;
use tokio::sync::broadcast;

/// A crew execution event sent over SSE.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrewEvent {
    pub crew_id: String,
    pub event_type: String, // "task_started", "task_completed", "crew_completed", "error"
    pub data: serde_json::Value,
}

/// Create an SSE stream from a broadcast receiver.
///
/// Yields `Event`s until the sender is dropped or the channel is closed.
pub fn event_stream(
    mut rx: broadcast::Receiver<CrewEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> {
    async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            let data = serde_json::to_string(&event).unwrap_or_default();
            yield Ok(Event::default()
                .event(event.event_type.clone())
                .data(data));
        }
    }
}
