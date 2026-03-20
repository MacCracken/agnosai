//! Server-sent events for streaming crew execution progress.

use axum::response::sse::Event;
use dashmap::DashMap;
use futures::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Default broadcast channel capacity per crew.
const CHANNEL_CAPACITY: usize = 256;

/// A crew execution event sent over SSE.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrewEvent {
    pub crew_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Registry of per-crew event broadcast channels.
///
/// The orchestrator publishes events here; SSE endpoints subscribe.
#[derive(Clone, Default)]
pub struct EventBus {
    channels: Arc<DashMap<Uuid, broadcast::Sender<CrewEvent>>>,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
        }
    }

    /// Get or create a broadcast sender for a crew.
    pub fn sender(&self, crew_id: Uuid) -> broadcast::Sender<CrewEvent> {
        self.channels
            .entry(crew_id)
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
            .clone()
    }

    /// Subscribe to events for a specific crew.
    pub fn subscribe(&self, crew_id: Uuid) -> broadcast::Receiver<CrewEvent> {
        self.sender(crew_id).subscribe()
    }

    /// Remove a crew's channel (call after crew completes).
    pub fn remove(&self, crew_id: Uuid) {
        self.channels.remove(&crew_id);
    }
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
