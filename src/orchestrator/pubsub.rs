//! Topic-based pub/sub with wildcard matching.
//!
//! From Agnosticos — supports patterns like:
//! - `"task.*"` matches `"task.completed"`, `"task.failed"`
//! - `"agent.#"` matches `"agent.assigned"`, `"agent.status.changed"`
//!
//! Used for decoupled inter-agent event communication within a single node.
//! For cross-node pub/sub, see `agnosai-fleet/relay.rs`.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::broadcast;

/// Default broadcast channel capacity per subscription pattern.
const CHANNEL_CAPACITY: usize = 256;

/// Maximum number of subscription patterns to prevent unbounded memory growth.
const MAX_SUBSCRIPTION_PATTERNS: usize = 10_000;

/// A message delivered to subscribers.
#[derive(Debug, Clone)]
pub struct TopicMessage {
    /// The concrete topic the message was published to (e.g. `"task.completed"`).
    pub topic: String,
    /// Arbitrary JSON payload.
    pub payload: Value,
    /// Server-side timestamp at publish time.
    pub timestamp: DateTime<Utc>,
}

/// Thread-safe, topic-based publish/subscribe hub.
///
/// Subscriptions are keyed by pattern strings (which may contain `*` and `#`
/// wildcards). Publishing fans out to every pattern whose wildcard expansion
/// matches the concrete topic.
pub struct PubSub {
    /// pattern -> broadcast sender
    subscriptions: DashMap<String, broadcast::Sender<TopicMessage>>,
}

impl PubSub {
    /// Create a new, empty pub/sub hub.
    pub fn new() -> Self {
        Self {
            subscriptions: DashMap::new(),
        }
    }

    /// Subscribe to a topic pattern and return a receiver.
    ///
    /// If the same pattern is subscribed to more than once, each call returns
    /// an independent receiver attached to the *same* underlying broadcast
    /// channel, so every receiver sees every matching message.
    ///
    /// Returns `None` if the maximum number of subscription patterns has been
    /// reached and the pattern is new.
    pub fn subscribe(&self, pattern: &str) -> Option<broadcast::Receiver<TopicMessage>> {
        // Check if pattern already exists (no capacity issue).
        if let Some(entry) = self.subscriptions.get(pattern) {
            return Some(entry.subscribe());
        }
        // Enforce max patterns for new subscriptions.
        if self.subscriptions.len() >= MAX_SUBSCRIPTION_PATTERNS {
            tracing::warn!(
                pattern,
                limit = MAX_SUBSCRIPTION_PATTERNS,
                "pubsub subscription rejected: pattern limit reached"
            );
            return None;
        }
        Some(
            self.subscriptions
                .entry(pattern.to_owned())
                .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0)
                .subscribe(),
        )
    }

    /// Publish a message to a concrete topic.
    ///
    /// The message is delivered to all subscribers whose pattern matches `topic`.
    /// Subscribers that have been dropped or whose channel is full are silently
    /// skipped (broadcast send errors are non-fatal).
    pub fn publish(&self, topic: &str, payload: Value) {
        let msg = TopicMessage {
            topic: topic.to_owned(),
            payload,
            timestamp: Utc::now(),
        };

        for entry in self.subscriptions.iter() {
            let pattern = entry.key();
            if matches_pattern(pattern, topic) {
                // Ignore SendError — it just means no active receivers.
                let _ = entry.value().send(msg.clone());
            }
        }
    }

    /// Remove all senders for a given pattern.
    ///
    /// Existing receivers will start returning `RecvError::Closed`.
    pub fn unsubscribe_all(&self, pattern: &str) {
        self.subscriptions.remove(pattern);
    }

    /// Return the number of active subscription patterns.
    #[must_use]
    pub fn pattern_count(&self) -> usize {
        self.subscriptions.len()
    }
}

impl Default for PubSub {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether a dot-separated wildcard `pattern` matches a concrete `topic`.
///
/// Wildcard rules:
/// - `*` matches exactly one segment.
/// - `#` matches zero or more segments (greedy).
/// - All other segments must match literally.
///
/// Both pattern and topic are split on `'.'`.
#[must_use]
pub fn matches_pattern(pattern: &str, topic: &str) -> bool {
    // Use stack-allocated arrays for small segment counts (typical: 2-5 segments).
    // Only heap-allocate for unusually deep topic hierarchies.
    let mut pat_buf: [&str; 16] = [""; 16];
    let mut topic_buf: [&str; 16] = [""; 16];

    let pat_count = pattern.split('.').count();
    let topic_count = topic.split('.').count();

    if pat_count <= 16 && topic_count <= 16 {
        for (i, seg) in pattern.split('.').enumerate() {
            pat_buf[i] = seg;
        }
        for (i, seg) in topic.split('.').enumerate() {
            topic_buf[i] = seg;
        }
        matches_recursive(&pat_buf[..pat_count], &topic_buf[..topic_count])
    } else {
        let pat_segments: Vec<&str> = pattern.split('.').collect();
        let topic_segments: Vec<&str> = topic.split('.').collect();
        matches_recursive(&pat_segments, &topic_segments)
    }
}

/// Maximum recursion depth to prevent stack overflow on adversarial patterns.
const MAX_MATCH_DEPTH: usize = 32;

fn matches_recursive(pattern: &[&str], topic: &[&str]) -> bool {
    matches_recursive_inner(pattern, topic, 0)
}

fn matches_recursive_inner(pattern: &[&str], topic: &[&str], depth: usize) -> bool {
    if depth > MAX_MATCH_DEPTH {
        return false;
    }
    match (pattern.first(), topic.first()) {
        // Both exhausted — match.
        (None, None) => true,

        // Pattern exhausted but topic has remaining segments — no match.
        (None, Some(_)) => false,

        // `#` can match zero or more remaining segments.
        (Some(&"#"), _) => {
            let rest_pat = &pattern[1..];
            if rest_pat.is_empty() {
                // Trailing `#` matches everything remaining (including nothing).
                return true;
            }
            // Try consuming 0, 1, 2, ... topic segments.
            for i in 0..=topic.len() {
                if matches_recursive_inner(rest_pat, &topic[i..], depth + 1) {
                    return true;
                }
            }
            false
        }

        // Topic exhausted but pattern still has segments — only valid if all
        // remaining pattern segments are `#`.
        (Some(_), None) => pattern.iter().all(|&s| s == "#"),

        // `*` matches exactly one segment.
        (Some(&"*"), Some(_)) => matches_recursive_inner(&pattern[1..], &topic[1..], depth + 1),

        // Literal match.
        (Some(p), Some(t)) => {
            if *p == *t {
                matches_recursive_inner(&pattern[1..], &topic[1..], depth + 1)
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Pattern matching unit tests ──────────────────────────────────────

    #[test]
    fn exact_match() {
        assert!(matches_pattern("task.completed", "task.completed"));
    }

    #[test]
    fn exact_no_match() {
        assert!(!matches_pattern("task.completed", "task.failed"));
    }

    #[test]
    fn star_matches_one_segment() {
        assert!(matches_pattern("task.*", "task.completed"));
        assert!(matches_pattern("task.*", "task.failed"));
    }

    #[test]
    fn star_does_not_match_zero_segments() {
        assert!(!matches_pattern("task.*", "task"));
    }

    #[test]
    fn star_does_not_match_multiple_segments() {
        assert!(!matches_pattern("task.*", "task.sub.completed"));
    }

    #[test]
    fn star_in_middle() {
        assert!(matches_pattern("task.*.done", "task.build.done"));
        assert!(!matches_pattern("task.*.done", "task.build.not_done"));
        assert!(!matches_pattern("task.*.done", "task.a.b.done"));
    }

    #[test]
    fn hash_matches_zero_segments() {
        assert!(matches_pattern("agent.#", "agent"));
    }

    #[test]
    fn hash_matches_one_segment() {
        assert!(matches_pattern("agent.#", "agent.status"));
    }

    #[test]
    fn hash_matches_multiple_segments() {
        assert!(matches_pattern("agent.#", "agent.status.changed"));
        assert!(matches_pattern("agent.#", "agent.a.b.c.d"));
    }

    #[test]
    fn hash_alone_matches_everything() {
        assert!(matches_pattern("#", "anything"));
        assert!(matches_pattern("#", "a.b.c"));
        assert!(matches_pattern("#", ""));
    }

    #[test]
    fn hash_in_middle() {
        assert!(matches_pattern("task.#.done", "task.done"));
        assert!(matches_pattern("task.#.done", "task.build.done"));
        assert!(matches_pattern("task.#.done", "task.a.b.done"));
        assert!(!matches_pattern("task.#.done", "task.a.b.failed"));
    }

    #[test]
    fn mixed_wildcards() {
        assert!(matches_pattern("*.#.done", "task.a.b.done"));
        assert!(matches_pattern("*.status.#", "agent.status"));
        assert!(matches_pattern("*.status.#", "agent.status.changed"));
    }

    #[test]
    fn no_match_different_prefix() {
        assert!(!matches_pattern("task.*", "agent.completed"));
    }

    #[test]
    fn no_match_shorter_topic() {
        assert!(!matches_pattern("task.sub.*", "task"));
    }

    // ── PubSub integration tests ─────────────────────────────────────────

    #[tokio::test]
    async fn publish_reaches_exact_subscriber() {
        let ps = PubSub::new();
        let mut rx = ps.subscribe("task.completed").unwrap();

        ps.publish("task.completed", json!({"id": 1}));

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.topic, "task.completed");
        assert_eq!(msg.payload, json!({"id": 1}));
    }

    #[tokio::test]
    async fn publish_reaches_wildcard_subscriber() {
        let ps = PubSub::new();
        let mut rx = ps.subscribe("task.*").unwrap();

        ps.publish("task.completed", json!({"id": 2}));
        ps.publish("task.failed", json!({"id": 3}));

        let m1 = rx.recv().await.unwrap();
        assert_eq!(m1.topic, "task.completed");

        let m2 = rx.recv().await.unwrap();
        assert_eq!(m2.topic, "task.failed");
    }

    #[tokio::test]
    async fn publish_reaches_hash_subscriber() {
        let ps = PubSub::new();
        let mut rx = ps.subscribe("agent.#").unwrap();

        ps.publish("agent.status.changed", json!({"status": "idle"}));

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.topic, "agent.status.changed");
    }

    #[tokio::test]
    async fn non_matching_subscriber_gets_nothing() {
        let ps = PubSub::new();
        let mut rx = ps.subscribe("task.*").unwrap();

        ps.publish("agent.started", json!({}));

        // Nothing should be available — use try_recv.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn multiple_subscribers_same_pattern() {
        let ps = PubSub::new();
        let mut rx1 = ps.subscribe("task.*").unwrap();
        let mut rx2 = ps.subscribe("task.*").unwrap();

        ps.publish("task.done", json!({"ok": true}));

        let m1 = rx1.recv().await.unwrap();
        let m2 = rx2.recv().await.unwrap();
        assert_eq!(m1.topic, "task.done");
        assert_eq!(m2.topic, "task.done");
    }

    #[tokio::test]
    async fn unsubscribe_all_closes_receivers() {
        let ps = PubSub::new();
        let mut rx = ps.subscribe("task.*").unwrap();

        ps.unsubscribe_all("task.*");

        // Channel is now closed.
        assert!(rx.recv().await.is_err());
    }

    #[tokio::test]
    async fn message_has_timestamp() {
        let ps = PubSub::new();
        let mut rx = ps.subscribe("t").unwrap();
        let before = Utc::now();

        ps.publish("t", json!(null));

        let msg = rx.recv().await.unwrap();
        assert!(msg.timestamp >= before);
        assert!(msg.timestamp <= Utc::now());
    }
}
