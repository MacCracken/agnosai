//! Priority inference queue backed by majra.
//!
//! Provides a `InferenceQueue` that accepts inference requests at different
//! priority levels and dispatches them to the LLM client in priority order.
//! Background and batch inference requests wait until higher-priority work
//! is drained.

use std::sync::Arc;

use majra::queue::{ConcurrentPriorityQueue, Priority, QueueItem};
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::llm::{HooshClient, InferenceRequest, InferenceResponse};

/// An inference request enqueued for priority-ordered processing.
#[derive(Debug)]
pub struct QueuedInference {
    /// The inference request to execute.
    pub request: InferenceRequest,
    /// Channel to send the result back to the caller.
    pub reply: oneshot::Sender<Result<InferenceResponse, String>>,
    /// Optional label for logging.
    pub label: String,
}

/// Priority inference queue that schedules LLM requests.
///
/// Higher-priority requests (e.g. interactive crew tasks) are processed
/// before lower-priority ones (e.g. background summarization).
pub struct InferenceQueue {
    queue: Arc<ConcurrentPriorityQueue<QueuedInference>>,
}

impl InferenceQueue {
    /// Create a new inference queue.
    pub fn new() -> Self {
        Self {
            queue: Arc::new(ConcurrentPriorityQueue::new()),
        }
    }

    /// Enqueue an inference request at the given priority.
    ///
    /// Returns a receiver that will contain the inference result.
    pub fn enqueue(
        &self,
        request: InferenceRequest,
        priority: Priority,
        label: impl Into<String>,
    ) -> oneshot::Receiver<Result<InferenceResponse, String>> {
        let (tx, rx) = oneshot::channel();
        let label = label.into();
        debug!(priority = %priority, label = %label, "inference request enqueued");
        let queue = Arc::clone(&self.queue);
        let item = QueueItem::new(
            priority,
            QueuedInference {
                request,
                reply: tx,
                label,
            },
        );
        tokio::spawn(async move {
            queue.enqueue(item).await;
        });
        rx
    }

    /// Enqueue at normal priority (convenience method).
    pub fn enqueue_normal(
        &self,
        request: InferenceRequest,
        label: impl Into<String>,
    ) -> oneshot::Receiver<Result<InferenceResponse, String>> {
        self.enqueue(request, Priority::Normal, label)
    }

    /// Enqueue at background priority (convenience method).
    pub fn enqueue_background(
        &self,
        request: InferenceRequest,
        label: impl Into<String>,
    ) -> oneshot::Receiver<Result<InferenceResponse, String>> {
        self.enqueue(request, Priority::Background, label)
    }

    /// Number of pending items in the queue.
    pub async fn pending(&self) -> usize {
        self.queue.len().await
    }

    /// Whether the queue is empty.
    pub async fn is_empty(&self) -> bool {
        self.queue.is_empty().await
    }

    /// Spawn a worker loop that processes queued inference requests.
    ///
    /// The worker pops the highest-priority request, executes it via the
    /// LLM client, and sends the result back through the oneshot channel.
    /// Runs until the queue is dropped or the returned handle is aborted.
    pub fn spawn_worker(
        &self,
        client: Arc<HooshClient>,
        poll_interval_ms: u64,
    ) -> tokio::task::JoinHandle<()> {
        let queue = Arc::clone(&self.queue);
        tokio::spawn(async move {
            info!("inference queue worker started");
            loop {
                if let Some(item) = queue.dequeue().await {
                    let queued = item.payload;
                    debug!(
                        label = %queued.label,
                        priority = %item.priority,
                        "processing queued inference"
                    );
                    let result = client
                        .infer(&queued.request)
                        .await
                        .map_err(|e| e.to_string());
                    let _ = queued.reply.send(result);
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
                }
            }
        })
    }
}

impl Default for InferenceQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Map AgnosAI task priority to majra queue priority.
#[must_use]
pub fn map_priority(task_priority: crate::core::task::TaskPriority) -> Priority {
    match task_priority {
        crate::core::task::TaskPriority::Background => Priority::Background,
        crate::core::task::TaskPriority::Low => Priority::Low,
        crate::core::task::TaskPriority::Normal => Priority::Normal,
        crate::core::task::TaskPriority::High => Priority::High,
        crate::core::task::TaskPriority::Critical => Priority::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::task::TaskPriority;

    #[tokio::test]
    async fn queue_creation() {
        let q = InferenceQueue::new();
        assert!(q.is_empty().await);
        assert_eq!(q.pending().await, 0);
    }

    #[test]
    fn priority_mapping() {
        assert_eq!(map_priority(TaskPriority::Background), Priority::Background);
        assert_eq!(map_priority(TaskPriority::Low), Priority::Low);
        assert_eq!(map_priority(TaskPriority::Normal), Priority::Normal);
        assert_eq!(map_priority(TaskPriority::High), Priority::High);
        assert_eq!(map_priority(TaskPriority::Critical), Priority::Critical);
    }

    #[tokio::test]
    async fn enqueue_increments_pending() {
        let q = InferenceQueue::new();
        let req = InferenceRequest {
            model: "test".into(),
            prompt: "hello".into(),
            ..Default::default()
        };
        let _rx = q.enqueue_normal(req, "test");
        // Give the spawn a moment to enqueue.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(q.pending().await, 1);
        assert!(!q.is_empty().await);
    }

    #[tokio::test]
    async fn enqueue_background_works() {
        let q = InferenceQueue::new();
        let req = InferenceRequest {
            model: "test".into(),
            prompt: "bg task".into(),
            ..Default::default()
        };
        let _rx = q.enqueue_background(req, "bg");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(q.pending().await, 1);
    }
}
