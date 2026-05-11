//! Channel entry: per-channel state stored in the scheduler's channel table.
//!
//! A channel is a bounded FIFO queue with separate wait lists for senders
//! (blocked because the buffer is full) and receivers (blocked because the
//! buffer is empty).
//!
//! ## Capacity and rendezvous
//!
//! - `capacity == 0`: rendezvous mode.  A sender parks until a receiver is
//!   waiting, then the value transfers directly.
//! - `capacity > 0`: buffered mode.  Up to `capacity` values can sit in the
//!   buffer before senders park.
//!
//! ## Closure
//!
//! After `chan_close`:
//! - new sends return `ChannelClosed`.
//! - new receives drain the buffer first; once empty, return `RecvResult::Closed`.
//! - parked senders are woken and must observe the closure in their next dispatch.
//! - parked receivers are woken and will see `RecvResult::Closed` on next try.

use std::collections::VecDeque;

use crate::types::{TaskHandle, Value};

/// Per-channel data stored in the scheduler's channel table.
#[derive(Debug)]
pub struct ChannelEntry {
    /// Maximum number of buffered values (0 = rendezvous).
    pub capacity: usize,

    /// Buffered values waiting to be received.
    pub buffer: VecDeque<Value>,

    /// Whether the send-side has been closed.
    pub closed: bool,

    /// Tasks parked waiting to send a value (because the buffer is full).
    /// Each entry holds the sending task's handle and the value it wants to send.
    pub send_waiters: VecDeque<(TaskHandle, Value)>,

    /// Tasks parked waiting to receive a value (because the buffer is empty).
    pub recv_waiters: VecDeque<TaskHandle>,
}

impl ChannelEntry {
    /// Create a new `ChannelEntry` with the given capacity.
    pub fn new(capacity: usize) -> Self {
        ChannelEntry {
            capacity,
            buffer: VecDeque::new(),
            closed: false,
            send_waiters: VecDeque::new(),
            recv_waiters: VecDeque::new(),
        }
    }

    /// Returns `true` if the buffer is full (or rendezvous and no waiter).
    pub fn is_full(&self) -> bool {
        if self.capacity == 0 {
            // Rendezvous: "full" means there's no waiting receiver.
            self.recv_waiters.is_empty()
        } else {
            self.buffer.len() >= self.capacity
        }
    }

    /// Returns `true` if the buffer is empty and there are no rendezvous senders.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Attempt a non-blocking enqueue.
    ///
    /// Returns `true` and enqueues the value if space is available.
    /// Returns `false` if the channel is full, closed, or rendezvous with no receiver.
    ///
    /// If a receiver is parked in rendezvous mode, the value is transferred
    /// directly (buffer stays empty) and the parked receiver's handle is returned.
    pub fn try_enqueue(&mut self, value: Value) -> TryEnqueueResult {
        if self.closed {
            return TryEnqueueResult::Closed;
        }
        // If a receiver is waiting, deliver directly (even in buffered mode —
        // this is the "fast path" that avoids going through the buffer).
        if let Some(recv_handle) = self.recv_waiters.pop_front() {
            return TryEnqueueResult::DeliveredToWaiter(recv_handle, value);
        }
        // For rendezvous channels with no waiting receiver: block.
        if self.capacity == 0 {
            return TryEnqueueResult::Full;
        }
        // Buffered: enqueue if space available.
        if self.buffer.len() < self.capacity {
            self.buffer.push_back(value);
            TryEnqueueResult::Enqueued
        } else {
            TryEnqueueResult::Full
        }
    }

    /// Attempt a non-blocking dequeue.
    ///
    /// Returns a value if one is available.  If a send waiter is queued and
    /// we just freed space, returns its handle too so the caller can wake it.
    pub fn try_dequeue(&mut self) -> TryDequeueResult {
        if let Some(value) = self.buffer.pop_front() {
            // We freed a slot — if a send waiter was queued, move their value
            // into the buffer and wake them.
            if let Some((waiter, pending_value)) = self.send_waiters.pop_front() {
                self.buffer.push_back(pending_value);
                return TryDequeueResult::Received(value, Some(waiter));
            }
            return TryDequeueResult::Received(value, None);
        }
        // Buffer is empty.  Check for a rendezvous sender.
        if let Some((waiter, pending_value)) = self.send_waiters.pop_front() {
            // Rendezvous: take the value directly from the sender and wake them.
            return TryDequeueResult::Received(pending_value, Some(waiter));
        }
        if self.closed {
            TryDequeueResult::Closed
        } else {
            TryDequeueResult::Empty
        }
    }
}

/// Result of a `try_enqueue` call.
#[derive(Debug, PartialEq)]
pub enum TryEnqueueResult {
    /// Value was enqueued into the buffer.
    Enqueued,
    /// Value was delivered directly to a waiting receiver; the receiver's
    /// `TaskHandle` is returned so the caller can wake it.
    DeliveredToWaiter(TaskHandle, Value),
    /// Channel is full (or rendezvous with no receiver).
    Full,
    /// Channel is closed.
    Closed,
}

/// Result of a `try_dequeue` call.
#[derive(Debug, PartialEq)]
pub enum TryDequeueResult {
    /// Value received.  Optional `TaskHandle`: a parked sender was woken up.
    Received(Value, Option<TaskHandle>),
    /// Channel is empty (and open).
    Empty,
    /// Channel is closed and its buffer is empty.
    Closed,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    fn v(n: i64) -> Value {
        Value::Int(n)
    }

    #[test]
    fn buffered_channel_enqueue_dequeue() {
        let mut ch = ChannelEntry::new(2);
        assert!(matches!(ch.try_enqueue(v(1)), TryEnqueueResult::Enqueued));
        assert!(matches!(ch.try_enqueue(v(2)), TryEnqueueResult::Enqueued));
        assert!(matches!(ch.try_enqueue(v(3)), TryEnqueueResult::Full));
        assert!(matches!(ch.try_dequeue(), TryDequeueResult::Received(Value::Int(1), None)));
        assert!(matches!(ch.try_dequeue(), TryDequeueResult::Received(Value::Int(2), None)));
        assert!(matches!(ch.try_dequeue(), TryDequeueResult::Empty));
    }

    #[test]
    fn dequeue_wakes_send_waiter() {
        let mut ch = ChannelEntry::new(1);
        ch.try_enqueue(v(10)); // fill the buffer
        ch.send_waiters.push_back((TaskHandle(5), v(20)));
        let result = ch.try_dequeue();
        match result {
            TryDequeueResult::Received(Value::Int(10), Some(TaskHandle(5))) => {}
            other => panic!("unexpected: {:?}", other),
        }
        assert_eq!(ch.buffer.len(), 1); // waiter's value entered buffer
    }

    #[test]
    fn enqueue_delivers_to_recv_waiter() {
        let mut ch = ChannelEntry::new(1);
        ch.recv_waiters.push_back(TaskHandle(7));
        let result = ch.try_enqueue(v(42));
        match result {
            TryEnqueueResult::DeliveredToWaiter(TaskHandle(7), Value::Int(42)) => {}
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn rendezvous_full_without_receiver() {
        let mut ch = ChannelEntry::new(0);
        assert!(matches!(ch.try_enqueue(v(1)), TryEnqueueResult::Full));
    }

    #[test]
    fn closed_channel_send_blocked() {
        let mut ch = ChannelEntry::new(1);
        ch.closed = true;
        assert!(matches!(ch.try_enqueue(v(1)), TryEnqueueResult::Closed));
    }

    #[test]
    fn closed_channel_recv_empty() {
        let mut ch = ChannelEntry::new(1);
        ch.closed = true;
        assert!(matches!(ch.try_dequeue(), TryDequeueResult::Closed));
    }

    #[test]
    fn closed_channel_recv_drains_buffer_first() {
        let mut ch = ChannelEntry::new(2);
        ch.buffer.push_back(v(99));
        ch.closed = true;
        // Still has a value in the buffer
        assert!(matches!(
            ch.try_dequeue(),
            TryDequeueResult::Received(Value::Int(99), None)
        ));
        // Buffer now empty, channel closed
        assert!(matches!(ch.try_dequeue(), TryDequeueResult::Closed));
    }

    #[test]
    fn is_full_respects_capacity() {
        let mut ch = ChannelEntry::new(1);
        assert!(!ch.is_full());
        ch.buffer.push_back(v(1));
        assert!(ch.is_full());
    }
}
