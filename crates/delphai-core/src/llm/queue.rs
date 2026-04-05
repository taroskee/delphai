use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Priority levels for inference requests. Higher = processed first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InferencePriority {
    /// Background/periodic updates.
    Low = 0,
    /// Event-driven conversations.
    Normal = 1,
    /// Player is watching / divine voice.
    High = 2,
}

/// A request queued for LLM inference.
#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub priority: InferencePriority,
    pub prompt: String,
    /// Opaque tag for the caller to identify the request (e.g. citizen name).
    pub tag: String,
}

impl PartialEq for InferenceRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for InferenceRequest {}

impl PartialOrd for InferenceRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InferenceRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

/// A priority queue that limits the number of inferences per turn.
#[derive(Debug)]
pub struct InferenceQueue {
    heap: BinaryHeap<InferenceRequest>,
    max_per_turn: usize,
}

impl InferenceQueue {
    pub fn new(max_per_turn: usize) -> Self {
        Self {
            heap: BinaryHeap::new(),
            max_per_turn,
        }
    }

    pub fn push(&mut self, request: InferenceRequest) {
        self.heap.push(request);
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn max_per_turn(&self) -> usize {
        self.max_per_turn
    }

    /// Drain up to `max_per_turn` requests, highest priority first.
    /// Remaining requests stay in the queue for the next turn.
    pub fn drain_turn(&mut self) -> Vec<InferenceRequest> {
        let n = self.max_per_turn.min(self.heap.len());
        let mut batch = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(req) = self.heap.pop() {
                batch.push(req);
            }
        }
        batch
    }

    /// Clear all pending requests.
    pub fn clear(&mut self) {
        self.heap.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(priority: InferencePriority, tag: &str) -> InferenceRequest {
        InferenceRequest {
            priority,
            prompt: format!("prompt for {tag}"),
            tag: tag.into(),
        }
    }

    #[test]
    fn empty_queue() {
        let q = InferenceQueue::new(3);
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn push_increases_len() {
        let mut q = InferenceQueue::new(3);
        q.push(req(InferencePriority::Normal, "a"));
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());
    }

    #[test]
    fn drain_respects_max_per_turn() {
        let mut q = InferenceQueue::new(2);
        q.push(req(InferencePriority::Low, "a"));
        q.push(req(InferencePriority::Low, "b"));
        q.push(req(InferencePriority::Low, "c"));

        let batch = q.drain_turn();
        assert_eq!(batch.len(), 2);
        assert_eq!(q.len(), 1, "one remains for next turn");
    }

    #[test]
    fn drain_returns_highest_priority_first() {
        let mut q = InferenceQueue::new(10);
        q.push(req(InferencePriority::Low, "low"));
        q.push(req(InferencePriority::High, "high"));
        q.push(req(InferencePriority::Normal, "normal"));

        let batch = q.drain_turn();
        assert_eq!(batch[0].tag, "high");
        assert_eq!(batch[1].tag, "normal");
        assert_eq!(batch[2].tag, "low");
    }

    #[test]
    fn drain_empty_queue_returns_empty() {
        let mut q = InferenceQueue::new(5);
        let batch = q.drain_turn();
        assert!(batch.is_empty());
    }

    #[test]
    fn drain_fewer_than_max() {
        let mut q = InferenceQueue::new(10);
        q.push(req(InferencePriority::Normal, "a"));
        let batch = q.drain_turn();
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn remaining_requests_survive_drain() {
        let mut q = InferenceQueue::new(1);
        q.push(req(InferencePriority::High, "high"));
        q.push(req(InferencePriority::Low, "low"));

        let first = q.drain_turn();
        assert_eq!(first[0].tag, "high");
        assert_eq!(q.len(), 1);

        let second = q.drain_turn();
        assert_eq!(second[0].tag, "low");
        assert!(q.is_empty());
    }

    #[test]
    fn clear_removes_all() {
        let mut q = InferenceQueue::new(5);
        q.push(req(InferencePriority::Normal, "a"));
        q.push(req(InferencePriority::Normal, "b"));
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn max_per_turn_is_accessible() {
        let q = InferenceQueue::new(7);
        assert_eq!(q.max_per_turn(), 7);
    }

    #[test]
    fn priority_ordering() {
        assert!(InferencePriority::High > InferencePriority::Normal);
        assert!(InferencePriority::Normal > InferencePriority::Low);
    }
}
