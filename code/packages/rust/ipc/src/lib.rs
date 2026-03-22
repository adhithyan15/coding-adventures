/// # IPC -- Inter-Process Communication
///
/// This crate implements three classic IPC mechanisms that operating systems
/// use to let isolated processes exchange data:
///
/// 1. **Pipes** -- unidirectional byte streams (like pneumatic tubes)
/// 2. **Message Queues** -- FIFO queues of typed messages (like a shared mailbox)
/// 3. **Shared Memory** -- memory regions visible to multiple processes
///    (like a shared whiteboard)
///
/// Plus an `IpcManager` that acts as the kernel component owning all IPC
/// resources.
///
/// ## Quick start
/// ```
/// use ipc::{Pipe, MessageQueue, SharedMemoryRegion, IpcManager};
///
/// let mut pipe = Pipe::new(4096);
/// pipe.write(&[72, 101, 108]).unwrap();
/// let data = pipe.read(3);
/// assert_eq!(data, vec![72, 101, 108]);
/// ```

pub mod pipe;
pub mod message_queue;
pub mod shared_memory;
pub mod ipc_manager;

// Re-export the main types at the crate root for convenient access.
pub use pipe::Pipe;
pub use message_queue::{Message, MessageQueue};
pub use shared_memory::SharedMemoryRegion;
pub use ipc_manager::IpcManager;

/// Errors that can occur during IPC operations.
///
/// Each variant corresponds to a well-known error condition in Unix IPC:
///
/// ```text
///   +---------------------+------------------------------------------+
///   | Variant             | Unix equivalent                          |
///   +---------------------+------------------------------------------+
///   | BrokenPipe          | EPIPE / SIGPIPE                          |
///   | OutOfBounds         | SIGSEGV (segfault on shared memory)      |
///   | QueueFull           | EAGAIN on msgsnd with IPC_NOWAIT         |
///   | MessageTooLarge     | EMSGSIZE                                 |
///   | InvalidMessageType  | EINVAL                                   |
///   +---------------------+------------------------------------------+
/// ```
#[derive(Debug, PartialEq)]
pub enum IpcError {
    /// Write attempted on a pipe with no readers. Named after the Unix
    /// EPIPE error -- writing data that nobody will ever read is pointless.
    BrokenPipe,

    /// Read or write on shared memory exceeded the region's bounds.
    /// In a real OS, this would trigger a segmentation fault (SIGSEGV).
    OutOfBounds {
        offset: usize,
        count: usize,
        size: usize,
    },

    /// Message queue is full (at max_messages capacity).
    QueueFull,

    /// Message body exceeds max_message_size.
    MessageTooLarge {
        actual: usize,
        max: usize,
    },

    /// Message type must be a positive integer (> 0).
    InvalidMessageType,
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::BrokenPipe => write!(f, "broken pipe: write to pipe with no readers (EPIPE)"),
            IpcError::OutOfBounds { offset, count, size } => {
                write!(f, "out of bounds: offset {} + count {} exceeds size {}", offset, count, size)
            }
            IpcError::QueueFull => write!(f, "message queue is full"),
            IpcError::MessageTooLarge { actual, max } => {
                write!(f, "message too large: {} bytes exceeds max {}", actual, max)
            }
            IpcError::InvalidMessageType => write!(f, "message type must be a positive integer"),
        }
    }
}
