//! Multi-turn conversation memory for agent context management.
//!
//! Provides a `ConversationBuffer` that accumulates messages across turns
//! within a task, with configurable strategies for managing context window size.

use crate::llm::{Message, Role};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Strategy for managing conversation history when it exceeds capacity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MemoryStrategy {
    /// Keep all messages (no eviction). May exceed token limits.
    #[default]
    Full,
    /// Keep the most recent N message pairs (sliding window).
    SlidingWindow,
    /// Keep the system message + first message + last N messages.
    /// Middle messages are dropped.
    HeadTail,
}

/// Per-agent conversation buffer that accumulates messages across turns.
///
/// Used within a single task execution to maintain multi-turn context
/// when the agent needs iterative LLM interactions (e.g. tool use loops,
/// clarification, or chain-of-thought refinement).
#[non_exhaustive]
pub struct ConversationBuffer {
    /// All messages in the conversation.
    messages: Vec<Message>,
    /// Maximum number of messages to retain (0 = unlimited).
    max_messages: usize,
    /// Strategy for trimming when at capacity.
    strategy: MemoryStrategy,
    /// Agent ID for logging.
    agent_id: String,
}

impl ConversationBuffer {
    /// Create a new conversation buffer with unlimited capacity.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            max_messages: 0,
            strategy: MemoryStrategy::Full,
            agent_id: agent_id.into(),
        }
    }

    /// Create a buffer with a sliding window strategy.
    pub fn with_sliding_window(agent_id: impl Into<String>, max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            strategy: MemoryStrategy::SlidingWindow,
            agent_id: agent_id.into(),
        }
    }

    /// Create a buffer with head-tail strategy.
    pub fn with_head_tail(agent_id: impl Into<String>, max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            strategy: MemoryStrategy::HeadTail,
            agent_id: agent_id.into(),
        }
    }

    /// Add a user message to the buffer.
    pub fn add_user(&mut self, content: impl Into<String>) {
        self.push(Message::new(Role::User, content));
    }

    /// Add an assistant message to the buffer.
    pub fn add_assistant(&mut self, content: impl Into<String>) {
        self.push(Message::new(Role::Assistant, content));
    }

    /// Add a message and apply the eviction strategy if needed.
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
        self.trim();
    }

    /// Get the current messages as a slice for inclusion in an inference request.
    #[must_use]
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get the messages as a Vec (for constructing inference requests).
    #[must_use]
    pub fn to_vec(&self) -> Vec<Message> {
        self.messages.clone()
    }

    /// Number of messages in the buffer.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Apply the eviction strategy.
    fn trim(&mut self) {
        if self.max_messages == 0 || self.messages.len() <= self.max_messages {
            return;
        }

        match self.strategy {
            MemoryStrategy::Full => {} // No trimming.
            MemoryStrategy::SlidingWindow => {
                let excess = self.messages.len() - self.max_messages;
                debug!(
                    agent_id = %self.agent_id,
                    evicted = excess,
                    "sliding window: evicting oldest messages"
                );
                self.messages.drain(..excess);
            }
            MemoryStrategy::HeadTail => {
                // Keep first message + last (max_messages - 1) messages.
                if self.messages.len() > self.max_messages && self.max_messages >= 2 {
                    let keep_tail = self.max_messages - 1;
                    let remove_start = 1;
                    let remove_end = self.messages.len() - keep_tail;
                    debug!(
                        agent_id = %self.agent_id,
                        evicted = remove_end - remove_start,
                        "head-tail: evicting middle messages"
                    );
                    self.messages.drain(remove_start..remove_end);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_strategy_keeps_all() {
        let mut buf = ConversationBuffer::new("agent-1");
        for i in 0..100 {
            buf.add_user(format!("msg {i}"));
        }
        assert_eq!(buf.len(), 100);
    }

    #[test]
    fn sliding_window_evicts_oldest() {
        let mut buf = ConversationBuffer::with_sliding_window("agent-1", 4);
        buf.add_user("a");
        buf.add_assistant("b");
        buf.add_user("c");
        buf.add_assistant("d");
        assert_eq!(buf.len(), 4);

        buf.add_user("e");
        assert_eq!(buf.len(), 4);
        // "a" should have been evicted.
        assert_eq!(buf.messages()[0].content, "b");
        assert_eq!(buf.messages()[3].content, "e");
    }

    #[test]
    fn head_tail_keeps_first_and_last() {
        let mut buf = ConversationBuffer::with_head_tail("agent-1", 4);
        for i in 0..10 {
            buf.add_user(format!("msg-{i}"));
        }
        assert_eq!(buf.len(), 4);
        // First message preserved.
        assert_eq!(buf.messages()[0].content, "msg-0");
        // Last 3 messages preserved.
        assert_eq!(buf.messages()[1].content, "msg-7");
        assert_eq!(buf.messages()[2].content, "msg-8");
        assert_eq!(buf.messages()[3].content, "msg-9");
    }

    #[test]
    fn empty_buffer() {
        let buf = ConversationBuffer::new("agent-1");
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(buf.messages().is_empty());
    }

    #[test]
    fn clear_removes_all() {
        let mut buf = ConversationBuffer::new("agent-1");
        buf.add_user("hello");
        buf.add_assistant("hi");
        assert_eq!(buf.len(), 2);
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn to_vec_clones_messages() {
        let mut buf = ConversationBuffer::new("agent-1");
        buf.add_user("hello");
        let vec = buf.to_vec();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec[0].content, "hello");
    }

    #[test]
    fn sliding_window_no_trim_under_limit() {
        let mut buf = ConversationBuffer::with_sliding_window("agent-1", 10);
        buf.add_user("a");
        buf.add_assistant("b");
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn strategy_serde_roundtrip() {
        let strategies = [
            MemoryStrategy::Full,
            MemoryStrategy::SlidingWindow,
            MemoryStrategy::HeadTail,
        ];
        for s in &strategies {
            let json = serde_json::to_string(s).unwrap();
            let restored: MemoryStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, restored);
        }
    }
}
