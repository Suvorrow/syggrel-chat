use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

const MAX_MESSAGES: usize = 256;
const MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024; // 16MB Limit

// ---- Shared state for the message buffer ----
struct SlidingWindowBuffer {
    messages: VecDeque<String>,
    total_bytes: usize,
    max_messages: usize,
    max_bytes: usize,
}

impl SlidingWindowBuffer {
    fn new(max_messages: usize, max_bytes: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            total_bytes: 0,
            max_messages,
            max_bytes,
        }
    }

    fn add_message(&mut self, msg: String) {
        let msg_bytes = msg.len();

        // Add new message
        self.messages.push_back(msg);
        self.total_bytes += msg_bytes;

        // Evict old messages if limits exceeded
        while (self.messages.len() > self.max_messages) || (self.total_bytes > self.max_bytes) {
            if let Some(oldest) = self.messages.pop_front() {
                self.total_bytes -= oldest.len();
            } else {
                break; // Safety check
            }
        }
    }

    fn get_next_n_messages(&mut self, count: usize) -> Vec<String> {
        let mut result = Vec::new();
        let count = std::cmp::min(count, self.messages.len());

        for _ in 0..count {
            if let Some(msg) = self.messages.pop_front() {
                self.total_bytes -= msg.len();
                result.push(msg);
            }
        }

        result

    }
}

struct MessageBuffer {
    buffer: Arc<Mutex<SlidingWindowBuffer>>,
}

impl MessageBuffer {
    fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(
                SlidingWindowBuffer::new(MAX_MESSAGES, MAX_TOTAL_BYTES)
            ))
        }
    } 

    // Add message to the buffer
    async fn push_message(&self, msg: String) {
        let mut buffer_guard = self.buffer.lock().await;
        buffer_guard.add_message(msg);
    }

    // Take messages from the buffer
    async fn take_messages(&self, count: usize) -> Vec<String> {
        let mut buffer_guard = self.buffer.lock().await;
        buffer_guard.get_next_n_messages(count)
    }
}
