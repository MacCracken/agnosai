//! Inter-node messaging via Redis pub/sub.
//!
//! Provides ordered, deduplicated message passing between fleet nodes.
//! Each message carries a monotonic sequence number; receivers track the
//! last-seen sequence per sender to detect and discard duplicates.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use super::registry::NodeId;

/// A message sent between fleet nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RelayMessage {
    /// Monotonic sequence number from the sender.
    pub seq: u64,
    /// Sender node ID.
    pub from: NodeId,
    /// Target node ID (empty string = broadcast).
    pub to: String,
    /// Message topic for routing.
    pub topic: String,
    /// Payload (arbitrary JSON).
    pub payload: serde_json::Value,
    /// Wall-clock timestamp.
    pub timestamp: DateTime<Utc>,
}

impl RelayMessage {
    /// Create a new relay message.
    pub fn new(
        seq: u64,
        from: impl Into<NodeId>,
        to: impl Into<String>,
        topic: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            seq,
            from: from.into(),
            to: to.into(),
            topic: topic.into(),
            payload,
            timestamp: Utc::now(),
        }
    }
}

/// Incoming message after dedup.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IncomingMessage {
    pub message: RelayMessage,
    /// Whether this was a broadcast (to == "").
    pub is_broadcast: bool,
}

/// Stats about relay activity.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RelayStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub duplicates_dropped: u64,
}

/// Local relay that handles message sequencing, dedup, and fan-out.
///
/// In a full deployment this would be backed by Redis pub/sub or gRPC streams.
/// This implementation provides the in-process core that a Redis/gRPC adapter
/// wraps.
pub struct Relay {
    node_id: NodeId,
    next_seq: AtomicU64,
    /// Last-seen sequence per sender for dedup.
    seen: std::sync::Mutex<HashMap<NodeId, u64>>,
    /// Broadcast channel for incoming messages.
    tx: broadcast::Sender<IncomingMessage>,
    stats: std::sync::Mutex<RelayStats>,
}

impl Relay {
    /// Create a new relay for the given node.
    pub fn new(node_id: impl Into<NodeId>, capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            node_id: node_id.into(),
            next_seq: AtomicU64::new(1),
            seen: std::sync::Mutex::new(HashMap::new()),
            tx,
            stats: std::sync::Mutex::new(RelayStats::default()),
        }
    }

    /// Create a relay with default channel capacity (256).
    pub fn with_defaults(node_id: impl Into<NodeId>) -> Self {
        Self::new(node_id, 256)
    }

    /// Build and send a message to a specific node or broadcast.
    ///
    /// Returns the assigned sequence number.
    pub fn send(
        &self,
        to: impl Into<String>,
        topic: impl Into<String>,
        payload: serde_json::Value,
    ) -> u64 {
        let seq = self.next_seq.fetch_add(1, Ordering::AcqRel);
        let msg = RelayMessage {
            seq,
            from: self.node_id.clone(),
            to: to.into(),
            topic: topic.into(),
            payload,
            timestamp: Utc::now(),
        };

        debug!(seq, to = %msg.to, topic = %msg.topic, "relay: sending message");

        let incoming = IncomingMessage {
            is_broadcast: msg.to.is_empty(),
            message: msg,
        };

        // Ignore send errors (no receivers).
        let _ = self.tx.send(incoming);

        if let Ok(mut stats) = self.stats.lock() {
            stats.messages_sent += 1;
        }

        seq
    }

    /// Broadcast a message to all nodes.
    pub fn broadcast(&self, topic: impl Into<String>, payload: serde_json::Value) -> u64 {
        self.send("", topic, payload)
    }

    /// Subscribe to incoming messages.
    pub fn subscribe(&self) -> broadcast::Receiver<IncomingMessage> {
        self.tx.subscribe()
    }

    /// Process an incoming message, applying dedup.
    ///
    /// Returns `Some(msg)` if the message is new, `None` if it's a duplicate.
    pub fn receive(&self, msg: RelayMessage) -> Option<IncomingMessage> {
        // Skip our own messages.
        if msg.from == self.node_id {
            return None;
        }

        // Skip messages targeted at other nodes (unless broadcast).
        if !msg.to.is_empty() && msg.to != self.node_id {
            return None;
        }

        // Dedup by sequence number.
        let mut seen = self.seen.lock().unwrap_or_else(|e| {
            warn!("relay seen-map mutex was poisoned, resetting");
            let mut inner = e.into_inner();
            inner.clear(); // Reset to safe empty state rather than using corrupted data.
            inner
        });
        let last_seen = seen.entry(msg.from.clone()).or_insert(0);
        if msg.seq <= *last_seen {
            debug!(
                seq = msg.seq,
                from = %msg.from,
                "relay: dropping duplicate message"
            );
            if let Ok(mut stats) = self.stats.lock() {
                stats.duplicates_dropped += 1;
            }
            return None;
        }

        if msg.seq != *last_seen + 1 {
            warn!(
                expected = *last_seen + 1,
                got = msg.seq,
                from = %msg.from,
                "relay: sequence gap detected"
            );
        }

        *last_seen = msg.seq;

        if let Ok(mut stats) = self.stats.lock() {
            stats.messages_received += 1;
        }

        let incoming = IncomingMessage {
            is_broadcast: msg.to.is_empty(),
            message: msg,
        };

        // Fan out to local subscribers.
        let _ = self.tx.send(incoming.clone());

        Some(incoming)
    }

    /// Get the node ID of this relay.
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Current relay statistics.
    pub fn stats(&self) -> RelayStats {
        self.stats
            .lock()
            .unwrap_or_else(|e| {
                warn!("relay stats mutex was poisoned, recovering");
                e.into_inner()
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn send_increments_seq() {
        let relay = Relay::with_defaults("node-a");
        let s1 = relay.send("node-b", "task", json!({"id": 1}));
        let s2 = relay.send("node-b", "task", json!({"id": 2}));
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
    }

    #[test]
    fn broadcast_uses_empty_to() {
        let relay = Relay::with_defaults("node-a");
        let mut rx = relay.subscribe();
        relay.broadcast("heartbeat", json!({}));
        let msg = rx.try_recv().expect("should receive broadcast");
        assert!(msg.is_broadcast);
        assert!(msg.message.to.is_empty());
    }

    #[test]
    fn receive_dedup_drops_duplicate() {
        let relay = Relay::with_defaults("node-b");
        let msg1 = RelayMessage {
            seq: 1,
            from: "node-a".into(),
            to: "node-b".into(),
            topic: "test".into(),
            payload: json!({}),
            timestamp: Utc::now(),
        };
        let msg2 = msg1.clone(); // same seq

        assert!(relay.receive(msg1).is_some());
        assert!(relay.receive(msg2).is_none()); // duplicate

        let stats = relay.stats();
        assert_eq!(stats.messages_received, 1);
        assert_eq!(stats.duplicates_dropped, 1);
    }

    #[test]
    fn receive_skips_own_messages() {
        let relay = Relay::with_defaults("node-a");
        let msg = RelayMessage {
            seq: 1,
            from: "node-a".into(),
            to: "".into(),
            topic: "test".into(),
            payload: json!({}),
            timestamp: Utc::now(),
        };
        assert!(relay.receive(msg).is_none());
    }

    #[test]
    fn receive_skips_messages_for_other_nodes() {
        let relay = Relay::with_defaults("node-b");
        let msg = RelayMessage {
            seq: 1,
            from: "node-a".into(),
            to: "node-c".into(),
            topic: "test".into(),
            payload: json!({}),
            timestamp: Utc::now(),
        };
        assert!(relay.receive(msg).is_none());
    }

    #[test]
    fn receive_accepts_broadcast() {
        let relay = Relay::with_defaults("node-b");
        let msg = RelayMessage {
            seq: 1,
            from: "node-a".into(),
            to: "".into(),
            topic: "heartbeat".into(),
            payload: json!({"status": "ok"}),
            timestamp: Utc::now(),
        };
        let incoming = relay.receive(msg).expect("should accept broadcast");
        assert!(incoming.is_broadcast);
    }

    #[test]
    fn receive_detects_sequence_gap() {
        let relay = Relay::with_defaults("node-b");
        // Skip seq 1, send seq 2 directly.
        let msg = RelayMessage {
            seq: 2,
            from: "node-a".into(),
            to: "node-b".into(),
            topic: "test".into(),
            payload: json!({}),
            timestamp: Utc::now(),
        };
        // Should still accept (with a warning logged).
        assert!(relay.receive(msg).is_some());
    }

    #[test]
    fn stats_tracking() {
        let relay = Relay::with_defaults("node-a");
        relay.send("node-b", "t", json!({}));
        relay.send("node-b", "t", json!({}));
        let stats = relay.stats();
        assert_eq!(stats.messages_sent, 2);
    }

    #[test]
    fn subscriber_receives_sent_messages() {
        let relay = Relay::with_defaults("node-a");
        let mut rx = relay.subscribe();
        relay.send("node-b", "task", json!({"work": true}));
        let msg = rx.try_recv().expect("subscriber should receive message");
        assert_eq!(msg.message.topic, "task");
    }
}
