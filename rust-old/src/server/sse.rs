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

/// Maximum number of concurrent crew event channels.
const MAX_EVENT_CHANNELS: usize = 10_000;

/// A crew execution event sent over SSE.
#[derive(Debug, Clone, serde::Serialize)]
#[non_exhaustive]
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
    ///
    /// If the channel limit has been reached and this is a new crew ID,
    /// orphan channels are cleaned up first. If still at capacity, the
    /// channel is created anyway (the orchestrator needs it), but a warning
    /// is logged.
    pub fn sender(&self, crew_id: Uuid) -> broadcast::Sender<CrewEvent> {
        if let Some(entry) = self.channels.get(&crew_id) {
            return entry.clone();
        }
        if self.channels.len() >= MAX_EVENT_CHANNELS {
            self.cleanup_orphans();
            if self.channels.len() >= MAX_EVENT_CHANNELS {
                tracing::warn!(
                    channels = self.channels.len(),
                    limit = MAX_EVENT_CHANNELS,
                    "event bus at capacity after orphan cleanup"
                );
            }
        }
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
    #[must_use]
    pub fn has(&self, crew_id: Uuid) -> bool {
        self.channels.contains_key(&crew_id)
    }

    /// Number of active channels.
    #[must_use]
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// Whether the event bus has no channels.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
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
            let data = match serde_json::to_string(&event) {
                Ok(json) => json,
                Err(e) => {
                    tracing::warn!(error = %e, "SSE event serialization failed");
                    "{\"error\":\"serialization failed\"}".to_string()
                }
            };
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

    #[test]
    fn cleanup_orphans_removes_channels_with_no_receivers() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();

        // Create a channel by calling sender, then subscribe so the channel exists.
        let tx = bus.sender(id);
        let rx = tx.subscribe();

        // Drop the only receiver — channel is now orphaned.
        drop(rx);
        assert!(bus.has(id));

        bus.cleanup_orphans();
        assert!(
            !bus.has(id),
            "orphan channel should be removed after cleanup"
        );
    }

    #[test]
    fn channel_capacity_boundary() {
        let bus = EventBus::new();
        let ids: Vec<Uuid> = (0..100).map(|_| Uuid::new_v4()).collect();

        for &id in &ids {
            let _tx = bus.sender(id);
            // Subscribe so the channel stays alive during cleanup.
            let _rx = bus.subscribe(id);
        }
        assert_eq!(bus.len(), 100);

        for &id in &ids {
            bus.remove(id);
        }
        assert!(
            bus.is_empty(),
            "bus should be empty after removing all channels"
        );
    }

    #[test]
    fn broadcast_channel_overflow_causes_lagged_error() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();
        let mut rx = bus.subscribe(id);
        let tx = bus.sender(id);

        // Send CHANNEL_CAPACITY + 1 events without receiving.
        for i in 0..=CHANNEL_CAPACITY {
            let _ = tx.send(CrewEvent {
                crew_id: id.to_string(),
                event_type: format!("evt_{i}"),
                data: serde_json::json!(null),
            });
        }

        // The first recv should report Lagged because the oldest messages were overwritten.
        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Lagged(_)) => {} // expected
            other => panic!("expected Lagged error, got {other:?}"),
        }
    }

    #[test]
    fn concurrent_subscribers_same_crew() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();

        let mut rx1 = bus.subscribe(id);
        let mut rx2 = bus.subscribe(id);
        let mut rx3 = bus.subscribe(id);

        bus.sender(id)
            .send(CrewEvent {
                crew_id: id.to_string(),
                event_type: "shared".into(),
                data: serde_json::json!({"v": 1}),
            })
            .unwrap();

        assert_eq!(rx1.try_recv().unwrap().event_type, "shared");
        assert_eq!(rx2.try_recv().unwrap().event_type, "shared");
        assert_eq!(rx3.try_recv().unwrap().event_type, "shared");
    }

    #[test]
    fn event_isolation_between_crews() {
        let bus = EventBus::new();
        let crew_a = Uuid::new_v4();
        let crew_b = Uuid::new_v4();

        // Subscribe to both crews so that sends succeed.
        let _rx_a = bus.subscribe(crew_a);
        let mut rx_b = bus.subscribe(crew_b);

        bus.sender(crew_a)
            .send(CrewEvent {
                crew_id: crew_a.to_string(),
                event_type: "only_a".into(),
                data: serde_json::json!(null),
            })
            .unwrap();

        assert!(
            rx_b.try_recv().is_err(),
            "crew B subscriber must not receive crew A events"
        );
    }

    #[test]
    fn sender_idempotent_for_same_crew() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();

        let tx1 = bus.sender(id);
        let tx2 = bus.sender(id);

        // Both senders should point to the same underlying channel.
        // Sending on tx1 should be observable by a subscriber created via tx2.
        let mut rx = tx2.subscribe();
        tx1.send(CrewEvent {
            crew_id: id.to_string(),
            event_type: "dup".into(),
            data: serde_json::json!(null),
        })
        .unwrap();

        assert_eq!(rx.try_recv().unwrap().event_type, "dup");
        // Only one channel should exist, not two.
        assert_eq!(bus.len(), 1);
    }

    #[test]
    fn has_returns_false_after_remove() {
        let bus = EventBus::new();
        let id = Uuid::new_v4();

        let _tx = bus.sender(id);
        assert!(bus.has(id), "channel should exist after sender()");

        bus.remove(id);
        assert!(!bus.has(id), "channel should not exist after remove()");
    }
}
