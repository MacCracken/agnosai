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
    /// ID of the crew that emitted the event.
    pub crew_id: String,
    /// Event type name (e.g. `"task_started"`, `"crew_completed"`).
    pub event_type: String,
    /// Arbitrary JSON payload for the event.
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

    /// Check whether a channel exists for the given crew.
    pub fn has(&self, crew_id: Uuid) -> bool {
        self.channels.contains_key(&crew_id)
    }

    /// Number of active channels.
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// Remove channels with no active receivers (orphan cleanup).
    pub fn cleanup_orphans(&self) {
        self.channels
            .retain(|_id, sender| sender.receiver_count() > 0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_bus_new_is_empty() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();
        // Subscribe creates the channel lazily.
        let _rx = bus.subscribe(id);
    }

    #[test]
    fn event_bus_sender_creates_channel() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();
        let tx = bus.sender(id);
        // Should be able to send without panic.
        let _ = tx.send(CrewEvent {
            crew_id: id.to_string(),
            event_type: "test".into(),
            data: serde_json::json!({}),
        });
    }

    #[test]
    fn event_bus_subscribe_receives_events() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();
        let mut rx = bus.subscribe(id);
        let tx = bus.sender(id);

        tx.send(CrewEvent {
            crew_id: id.to_string(),
            event_type: "task_started".into(),
            data: serde_json::json!({"task": "a"}),
        })
        .unwrap();

        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, "task_started");
        assert_eq!(event.crew_id, id.to_string());
    }

    #[test]
    fn event_bus_remove_cleans_channel() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();
        let _tx = bus.sender(id);
        bus.remove(id);

        // After remove, a new subscribe gets a fresh channel.
        let mut rx = bus.subscribe(id);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn event_bus_independent_crews() {
        let bus = EventBus::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let mut rx1 = bus.subscribe(id1);
        let mut rx2 = bus.subscribe(id2);

        bus.sender(id1)
            .send(CrewEvent {
                crew_id: id1.to_string(),
                event_type: "e1".into(),
                data: serde_json::json!(null),
            })
            .unwrap();

        // rx1 should receive, rx2 should not.
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_err());
    }

    #[test]
    fn crew_event_serializes() {
        let event = CrewEvent {
            crew_id: "abc".into(),
            event_type: "task_completed".into(),
            data: serde_json::json!({"status": "ok"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("task_completed"));
        assert!(json.contains("abc"));
    }
}
