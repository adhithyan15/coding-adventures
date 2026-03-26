/// Message Queue -- structured, typed message passing between processes.
///
/// While pipes transmit raw bytes (the reader must know how to parse them),
/// message queues transmit discrete, typed **messages**. Each message carries:
///
/// ```text
///   +----------+-----------------------------+
///   | msg_type | body (up to 4096 bytes)     |
///   +----------+-----------------------------+
/// ```
///
/// The type tag allows selective receiving: "give me only messages of type 3"
/// while leaving other message types in the queue for someone else.
///
/// ## Analogy
///
/// Think of a shared mailbox in an apartment building's lobby. Anyone can drop
/// off an envelope (send), and anyone can pick one up (receive). Each envelope
/// has a label ("type") -- you might only care about envelopes labeled "rent"
/// and ignore the ones labeled "newsletter."
///
/// ## FIFO Ordering
///
/// Messages follow FIFO (First-In, First-Out) order. When receiving with a
/// type filter, the queue returns the OLDEST message matching that type:
///
/// ```text
///   Queue contents (front -> back):
///     [type=1, "apple"] -> [type=2, "banana"] -> [type=1, "cherry"]
///
///   receive(0)   -> [type=1, "apple"]   (any type, oldest overall)
///   receive(2)   -> [type=2, "banana"]  (oldest type-2 message)
///   receive(1)   -> [type=1, "apple"]   (oldest type-1, skips type-2)
/// ```
///
/// ## Capacity Limits
///
/// ```text
///   +-------------------+-------+------------------------------------------+
///   | Limit             | Value | What happens when exceeded               |
///   +-------------------+-------+------------------------------------------+
///   | max_messages      |  256  | send() returns Err(QueueFull)            |
///   | max_message_size  | 4096  | send() returns Err(MessageTooLarge)      |
///   +-------------------+-------+------------------------------------------+
/// ```

use std::collections::VecDeque;
use crate::IpcError;

/// Default limits matching System V IPC conventions.
pub const DEFAULT_MAX_MESSAGES: usize = 256;
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 4096;

/// A single message in the queue: a (type, body) pair.
///
/// The type is a positive integer that senders and receivers agree on.
/// For example, a client-server protocol might use:
///   type=1 -> request
///   type=2 -> response
///   type=3 -> heartbeat
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub msg_type: u32,
    pub body: Vec<u8>,
}

pub struct MessageQueue {
    /// The internal FIFO storage. We use `VecDeque` for efficient push-back
    /// and pop-front operations -- both are O(1) amortized.
    messages: VecDeque<Message>,

    /// Maximum number of messages the queue can hold.
    max_messages: usize,

    /// Maximum size of a single message body in bytes.
    max_message_size: usize,
}

impl MessageQueue {
    /// Create a new empty message queue with the given limits.
    pub fn new(max_messages: usize, max_message_size: usize) -> Self {
        MessageQueue {
            messages: VecDeque::new(),
            max_messages,
            max_message_size,
        }
    }

    /// Send a typed message to the queue.
    ///
    /// Returns `Ok(())` if the message was enqueued, or an error if:
    /// - The queue is full (`QueueFull`)
    /// - The message body exceeds `max_message_size` (`MessageTooLarge`)
    /// - The message type is 0 (`InvalidMessageType`)
    pub fn send(&mut self, msg_type: u32, body: &[u8]) -> Result<(), IpcError> {
        // Validate: type must be a positive integer (> 0).
        if msg_type == 0 {
            return Err(IpcError::InvalidMessageType);
        }

        // Validate: body must not exceed the size limit.
        if body.len() > self.max_message_size {
            return Err(IpcError::MessageTooLarge {
                actual: body.len(),
                max: self.max_message_size,
            });
        }

        // Validate: queue must not be full.
        if self.messages.len() >= self.max_messages {
            return Err(IpcError::QueueFull);
        }

        self.messages.push_back(Message {
            msg_type,
            body: body.to_vec(),
        });
        Ok(())
    }

    /// Receive a message from the queue.
    ///
    /// - `msg_type == 0`: dequeue the oldest message of any type.
    /// - `msg_type > 0`: dequeue the oldest message of that specific type,
    ///   leaving non-matching messages in place.
    ///
    /// Returns `None` if no matching message is found.
    pub fn receive(&mut self, msg_type: u32) -> Option<Message> {
        if msg_type == 0 {
            // Any type: dequeue the front of the FIFO.
            self.messages.pop_front()
        } else {
            // Specific type: find the first matching message.
            let index = self.messages.iter().position(|m| m.msg_type == msg_type)?;
            self.messages.remove(index)
        }
    }

    /// How many messages are currently in the queue?
    pub fn count(&self) -> usize {
        self.messages.len()
    }

    /// Is the queue full?
    pub fn is_full(&self) -> bool {
        self.messages.len() >= self.max_messages
    }

    /// Is the queue empty?
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the max_messages limit.
    pub fn max_messages(&self) -> usize {
        self.max_messages
    }

    /// Get the max_message_size limit.
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Basic send/receive --

    #[test]
    fn test_send_and_receive_single() {
        let mut mq = MessageQueue::new(256, 4096);
        mq.send(1, &[65, 66, 67]).unwrap();

        let msg = mq.receive(0).unwrap();
        assert_eq!(msg.msg_type, 1);
        assert_eq!(msg.body, vec![65, 66, 67]);
    }

    #[test]
    fn test_fifo_ordering() {
        let mut mq = MessageQueue::new(256, 4096);
        mq.send(1, &[1]).unwrap();
        mq.send(1, &[2]).unwrap();
        mq.send(1, &[3]).unwrap();

        assert_eq!(mq.receive(0).unwrap().body, vec![1]);
        assert_eq!(mq.receive(0).unwrap().body, vec![2]);
        assert_eq!(mq.receive(0).unwrap().body, vec![3]);
    }

    // -- Typed receive --

    #[test]
    fn test_receive_specific_type() {
        let mut mq = MessageQueue::new(256, 4096);
        mq.send(1, &[10]).unwrap();
        mq.send(2, &[20]).unwrap();
        mq.send(1, &[30]).unwrap();

        let msg = mq.receive(2).unwrap();
        assert_eq!(msg.msg_type, 2);
        assert_eq!(msg.body, vec![20]);

        // Two type-1 messages should remain.
        assert_eq!(mq.count(), 2);
    }

    #[test]
    fn test_receive_type_returns_oldest_matching() {
        let mut mq = MessageQueue::new(256, 4096);
        mq.send(1, &[10]).unwrap();
        mq.send(2, &[20]).unwrap();
        mq.send(1, &[30]).unwrap();

        let msg = mq.receive(1).unwrap();
        assert_eq!(msg.body, vec![10]); // oldest type-1
    }

    #[test]
    fn test_receive_type_not_found() {
        let mut mq = MessageQueue::new(256, 4096);
        mq.send(1, &[10]).unwrap();

        assert!(mq.receive(99).is_none());
    }

    #[test]
    fn test_receive_any_type() {
        let mut mq = MessageQueue::new(256, 4096);
        mq.send(5, &[50]).unwrap();

        let msg = mq.receive(0).unwrap();
        assert_eq!(msg.msg_type, 5);
    }

    // -- Queue full --

    #[test]
    fn test_queue_full() {
        let mut mq = MessageQueue::new(3, 4096);
        mq.send(1, &[1]).unwrap();
        mq.send(1, &[2]).unwrap();
        mq.send(1, &[3]).unwrap();

        let result = mq.send(1, &[4]);
        assert_eq!(result, Err(IpcError::QueueFull));
        assert!(mq.is_full());
    }

    // -- Oversized message --

    #[test]
    fn test_oversized_message_rejected() {
        let mut mq = MessageQueue::new(256, 10);
        let big_body = vec![0u8; 11];

        let result = mq.send(1, &big_body);
        assert_eq!(result, Err(IpcError::MessageTooLarge { actual: 11, max: 10 }));
        assert_eq!(mq.count(), 0);
    }

    #[test]
    fn test_message_at_max_size_accepted() {
        let mut mq = MessageQueue::new(256, 10);
        let body = vec![0u8; 10];

        mq.send(1, &body).unwrap();
        assert_eq!(mq.count(), 1);
    }

    // -- Invalid type --

    #[test]
    fn test_type_zero_rejected() {
        let mut mq = MessageQueue::new(256, 4096);
        let result = mq.send(0, &[1]);
        assert_eq!(result, Err(IpcError::InvalidMessageType));
    }

    // -- Empty queue --

    #[test]
    fn test_receive_from_empty() {
        let mut mq = MessageQueue::new(256, 4096);
        assert!(mq.receive(0).is_none());
    }

    // -- State tracking --

    #[test]
    fn test_count() {
        let mut mq = MessageQueue::new(256, 4096);
        assert_eq!(mq.count(), 0);

        mq.send(1, &[1]).unwrap();
        mq.send(2, &[2]).unwrap();
        assert_eq!(mq.count(), 2);

        mq.receive(0);
        assert_eq!(mq.count(), 1);
    }

    #[test]
    fn test_empty_and_full() {
        let mut mq = MessageQueue::new(2, 4096);
        assert!(mq.is_empty());
        assert!(!mq.is_full());

        mq.send(1, &[1]).unwrap();
        assert!(!mq.is_empty());
        assert!(!mq.is_full());

        mq.send(1, &[2]).unwrap();
        assert!(!mq.is_empty());
        assert!(mq.is_full());
    }

    #[test]
    fn test_default_limits() {
        let mq = MessageQueue::new(DEFAULT_MAX_MESSAGES, DEFAULT_MAX_MESSAGE_SIZE);
        assert_eq!(mq.max_messages(), 256);
        assert_eq!(mq.max_message_size(), 4096);
    }
}
