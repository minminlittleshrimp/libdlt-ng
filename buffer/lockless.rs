// Lockless buffer module
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;

pub struct LocklessBuffer<T> {
    queue: Arc<ArrayQueue<T>>,
}

impl<T> LocklessBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        LocklessBuffer {
            queue: Arc::new(ArrayQueue::new(capacity)),
        }
    }

    pub fn push(&self, item: T) -> Result<(), T> {
        self.queue.push(item)
    }

    pub fn pop(&self) -> Option<T> {
        self.queue.pop()
    }

    pub fn clone_handle(&self) -> Self {
        LocklessBuffer {
            queue: self.queue.clone(),
        }
    }
}

impl<T> Clone for LocklessBuffer<T> {
    fn clone(&self) -> Self {
        self.clone_handle()
    }
}
