//! Offline queue for messages that cannot be forwarded while the network
//! is unavailable.  Messages are held in memory and can be drained once
//! connectivity is restored.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// A single message that has been enqueued while offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub message: String,
    pub domain_hint: String,
    /// ISO-8601 timestamp recorded at enqueue time.
    pub timestamp: String,
}

/// In-memory offline queue.
#[derive(Debug, Default)]
pub struct OfflineQueue {
    queue: Vec<QueuedMessage>,
}

impl OfflineQueue {
    /// Create a new, empty queue.
    #[must_use]
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Push a message onto the back of the queue.
    pub fn enqueue(&mut self, message: &str, domain_hint: &str) {
        self.queue.push(QueuedMessage {
            message: message.to_owned(),
            domain_hint: domain_hint.to_owned(),
            timestamp: Utc::now().to_rfc3339(),
        });
    }

    /// Remove and return all queued messages in FIFO order.
    pub fn drain(&mut self) -> Vec<QueuedMessage> {
        std::mem::take(&mut self.queue)
    }

    /// Number of messages currently in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns `true` when the queue contains no messages.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_drain() {
        let mut q = OfflineQueue::new();
        assert!(q.is_empty());

        q.enqueue("hello", "general");
        q.enqueue("world", "music");
        assert_eq!(q.len(), 2);

        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].message, "hello");
        assert_eq!(drained[1].domain_hint, "music");

        // Queue should be empty after drain.
        assert!(q.is_empty());
    }

    #[test]
    fn double_drain_is_idempotent() {
        let mut q = OfflineQueue::new();
        q.enqueue("msg", "dev");
        let _ = q.drain();
        let second = q.drain();
        assert!(second.is_empty());
    }
}
