/// IPC Manager -- the kernel component that owns all IPC resources.
///
/// In a real OS kernel, IPC resources (pipes, message queues, shared memory
/// segments) are global kernel objects managed by a central authority. The
/// `IpcManager` is that authority.
///
/// When a process calls `pipe()`, `msgget()`, or `shmget()`, the kernel
/// dispatches to the IpcManager, which creates the resource and returns an
/// identifier.
///
/// ## Resource Lifecycle
///
/// ```text
///   +----------+     create_pipe()      +--------+
///   | Process  | ---------------------> | Pipe   |
///   |          | <-- (pipe_id, r, w) -- |        |
///   +----------+                        +--------+
///        |
///        | close_pipe_read(pipe_id)
///        v
///   Reader count decremented. If both hit 0, pipe is eligible for cleanup.
/// ```
///
/// ## Identifier Scheme
///
/// Each resource type has its own ID counter starting from 0. Pipe IDs,
/// queue names, and shared memory names are independent.

use std::collections::HashMap;
use crate::pipe::Pipe;
use crate::message_queue::MessageQueue;
use crate::shared_memory::SharedMemoryRegion;

pub struct IpcManager {
    /// Pipes stored by integer ID.
    pipes: HashMap<u32, Pipe>,
    next_pipe_id: u32,

    /// File descriptor table: maps fd -> (pipe_id, mode).
    next_fd: u32,

    /// Message queues stored by name.
    message_queues: HashMap<String, MessageQueue>,

    /// Shared memory regions stored by name.
    shared_memory: HashMap<String, SharedMemoryRegion>,
}

impl IpcManager {
    /// Create a new IPC manager with no resources.
    pub fn new() -> Self {
        IpcManager {
            pipes: HashMap::new(),
            next_pipe_id: 0,
            next_fd: 0,
            message_queues: HashMap::new(),
            shared_memory: HashMap::new(),
        }
    }

    // ---------------------------------------------------------------
    // Pipe operations
    // ---------------------------------------------------------------

    /// Create a new pipe and return (pipe_id, read_fd, write_fd).
    ///
    /// This is the kernel's implementation of the `pipe()` system call.
    pub fn create_pipe(&mut self, capacity: usize) -> (u32, u32, u32) {
        let pipe_id = self.next_pipe_id;
        self.next_pipe_id += 1;

        let pipe = Pipe::new(capacity);
        self.pipes.insert(pipe_id, pipe);

        let read_fd = self.next_fd;
        self.next_fd += 1;
        let write_fd = self.next_fd;
        self.next_fd += 1;

        (pipe_id, read_fd, write_fd)
    }

    /// Get a mutable reference to a pipe by ID.
    pub fn get_pipe(&mut self, pipe_id: u32) -> Option<&mut Pipe> {
        self.pipes.get_mut(&pipe_id)
    }

    /// Get an immutable reference to a pipe by ID.
    pub fn get_pipe_ref(&self, pipe_id: u32) -> Option<&Pipe> {
        self.pipes.get(&pipe_id)
    }

    /// Close the read end of a pipe.
    pub fn close_pipe_read(&mut self, pipe_id: u32) {
        if let Some(pipe) = self.pipes.get_mut(&pipe_id) {
            pipe.close_read();
        }
    }

    /// Close the write end of a pipe.
    pub fn close_pipe_write(&mut self, pipe_id: u32) {
        if let Some(pipe) = self.pipes.get_mut(&pipe_id) {
            pipe.close_write();
        }
    }

    /// List all active pipe IDs.
    pub fn list_pipes(&self) -> Vec<u32> {
        self.pipes.keys().copied().collect()
    }

    // ---------------------------------------------------------------
    // Message queue operations
    // ---------------------------------------------------------------

    /// Create (or retrieve) a message queue by name.
    ///
    /// If a queue with this name already exists, it is returned (idempotent).
    pub fn create_message_queue(
        &mut self,
        name: String,
        max_messages: usize,
        max_message_size: usize,
    ) -> &mut MessageQueue {
        self.message_queues.entry(name).or_insert_with(|| {
            MessageQueue::new(max_messages, max_message_size)
        })
    }

    /// Get a mutable reference to a message queue by name.
    pub fn get_message_queue(&mut self, name: &str) -> Option<&mut MessageQueue> {
        self.message_queues.get_mut(name)
    }

    /// Delete a message queue by name.
    pub fn delete_message_queue(&mut self, name: &str) {
        self.message_queues.remove(name);
    }

    /// List all message queue names.
    pub fn list_message_queues(&self) -> Vec<String> {
        self.message_queues.keys().cloned().collect()
    }

    // ---------------------------------------------------------------
    // Shared memory operations
    // ---------------------------------------------------------------

    /// Create (or retrieve) a shared memory region by name.
    ///
    /// If a region with this name already exists, it is returned (idempotent,
    /// ignoring the new size and owner_pid).
    pub fn create_shared_memory(
        &mut self,
        name: String,
        size: usize,
        owner_pid: u32,
    ) -> &mut SharedMemoryRegion {
        let n = name.clone();
        self.shared_memory.entry(name).or_insert_with(|| {
            SharedMemoryRegion::new(n, size, owner_pid)
        })
    }

    /// Get a mutable reference to a shared memory region by name.
    pub fn get_shared_memory(&mut self, name: &str) -> Option<&mut SharedMemoryRegion> {
        self.shared_memory.get_mut(name)
    }

    /// Delete a shared memory region by name.
    pub fn delete_shared_memory(&mut self, name: &str) {
        self.shared_memory.remove(name);
    }

    /// List all shared memory region names.
    pub fn list_shared_memory(&self) -> Vec<String> {
        self.shared_memory.keys().cloned().collect()
    }
}

impl Default for IpcManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipe::DEFAULT_PIPE_CAPACITY;

    // ---------------------------------------------------------------
    // Pipe operations
    // ---------------------------------------------------------------

    #[test]
    fn test_create_pipe_returns_triple() {
        let mut mgr = IpcManager::new();
        let (pipe_id, read_fd, write_fd) = mgr.create_pipe(DEFAULT_PIPE_CAPACITY);
        assert_ne!(read_fd, write_fd);
        assert!(mgr.get_pipe(pipe_id).is_some());
    }

    #[test]
    fn test_get_nonexistent_pipe() {
        let mgr = IpcManager::new();
        assert!(mgr.get_pipe_ref(999).is_none());
    }

    #[test]
    fn test_close_pipe_read() {
        let mut mgr = IpcManager::new();
        let (pipe_id, _, _) = mgr.create_pipe(DEFAULT_PIPE_CAPACITY);
        mgr.close_pipe_read(pipe_id);
        assert_eq!(mgr.get_pipe_ref(pipe_id).unwrap().reader_count(), 0);
    }

    #[test]
    fn test_close_pipe_write() {
        let mut mgr = IpcManager::new();
        let (pipe_id, _, _) = mgr.create_pipe(DEFAULT_PIPE_CAPACITY);
        mgr.close_pipe_write(pipe_id);
        assert_eq!(mgr.get_pipe_ref(pipe_id).unwrap().writer_count(), 0);
    }

    #[test]
    fn test_close_nonexistent_pipe_no_panic() {
        let mut mgr = IpcManager::new();
        mgr.close_pipe_read(999);
        mgr.close_pipe_write(999);
    }

    #[test]
    fn test_list_pipes() {
        let mut mgr = IpcManager::new();
        mgr.create_pipe(4096);
        mgr.create_pipe(4096);
        assert_eq!(mgr.list_pipes().len(), 2);
    }

    #[test]
    fn test_list_pipes_empty() {
        let mgr = IpcManager::new();
        assert!(mgr.list_pipes().is_empty());
    }

    #[test]
    fn test_multiple_pipes_unique_ids() {
        let mut mgr = IpcManager::new();
        let (id1, _, _) = mgr.create_pipe(4096);
        let (id2, _, _) = mgr.create_pipe(4096);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_pipe_write_read_through_manager() {
        let mut mgr = IpcManager::new();
        let (pipe_id, _, _) = mgr.create_pipe(4096);

        let pipe = mgr.get_pipe(pipe_id).unwrap();
        pipe.write(&[1, 2, 3]).unwrap();
        let result = pipe.read(3);
        assert_eq!(result, vec![1, 2, 3]);
    }

    // ---------------------------------------------------------------
    // Message queue operations
    // ---------------------------------------------------------------

    #[test]
    fn test_create_message_queue() {
        let mut mgr = IpcManager::new();
        let mq = mgr.create_message_queue("q1".to_string(), 256, 4096);
        assert!(mq.is_empty());
    }

    #[test]
    fn test_create_message_queue_idempotent() {
        let mut mgr = IpcManager::new();
        mgr.create_message_queue("q1".to_string(), 256, 4096);
        mgr.create_message_queue("q1".to_string(), 128, 2048); // different params, same name

        // Should still be the original queue (max_messages = 256).
        let mq = mgr.get_message_queue("q1").unwrap();
        assert_eq!(mq.max_messages(), 256);
    }

    #[test]
    fn test_get_nonexistent_queue() {
        let mut mgr = IpcManager::new();
        assert!(mgr.get_message_queue("nope").is_none());
    }

    #[test]
    fn test_delete_message_queue() {
        let mut mgr = IpcManager::new();
        mgr.create_message_queue("q1".to_string(), 256, 4096);
        mgr.delete_message_queue("q1");
        assert!(mgr.get_message_queue("q1").is_none());
    }

    #[test]
    fn test_delete_nonexistent_queue_no_panic() {
        let mut mgr = IpcManager::new();
        mgr.delete_message_queue("nope");
    }

    #[test]
    fn test_list_message_queues() {
        let mut mgr = IpcManager::new();
        mgr.create_message_queue("q1".to_string(), 256, 4096);
        mgr.create_message_queue("q2".to_string(), 256, 4096);

        let names = mgr.list_message_queues();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_list_message_queues_empty() {
        let mgr = IpcManager::new();
        assert!(mgr.list_message_queues().is_empty());
    }

    #[test]
    fn test_mq_send_receive_through_manager() {
        let mut mgr = IpcManager::new();
        mgr.create_message_queue("q1".to_string(), 256, 4096);

        let mq = mgr.get_message_queue("q1").unwrap();
        mq.send(1, &[10, 20]).unwrap();
        let msg = mq.receive(0).unwrap();
        assert_eq!(msg.msg_type, 1);
        assert_eq!(msg.body, vec![10, 20]);
    }

    // ---------------------------------------------------------------
    // Shared memory operations
    // ---------------------------------------------------------------

    #[test]
    fn test_create_shared_memory() {
        let mut mgr = IpcManager::new();
        let shm = mgr.create_shared_memory("r1".to_string(), 1024, 1);
        assert_eq!(shm.size(), 1024);
    }

    #[test]
    fn test_create_shared_memory_idempotent() {
        let mut mgr = IpcManager::new();
        mgr.create_shared_memory("r1".to_string(), 1024, 1);
        mgr.create_shared_memory("r1".to_string(), 2048, 2); // different params

        let shm = mgr.get_shared_memory("r1").unwrap();
        assert_eq!(shm.size(), 1024); // original size preserved
    }

    #[test]
    fn test_get_nonexistent_shared_memory() {
        let mut mgr = IpcManager::new();
        assert!(mgr.get_shared_memory("nope").is_none());
    }

    #[test]
    fn test_delete_shared_memory() {
        let mut mgr = IpcManager::new();
        mgr.create_shared_memory("r1".to_string(), 512, 1);
        mgr.delete_shared_memory("r1");
        assert!(mgr.get_shared_memory("r1").is_none());
    }

    #[test]
    fn test_delete_nonexistent_shared_memory_no_panic() {
        let mut mgr = IpcManager::new();
        mgr.delete_shared_memory("nope");
    }

    #[test]
    fn test_list_shared_memory() {
        let mut mgr = IpcManager::new();
        mgr.create_shared_memory("r1".to_string(), 100, 1);
        mgr.create_shared_memory("r2".to_string(), 200, 1);

        let names = mgr.list_shared_memory();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_list_shared_memory_empty() {
        let mgr = IpcManager::new();
        assert!(mgr.list_shared_memory().is_empty());
    }

    #[test]
    fn test_shm_write_read_through_manager() {
        let mut mgr = IpcManager::new();
        mgr.create_shared_memory("r1".to_string(), 64, 1);

        let shm = mgr.get_shared_memory("r1").unwrap();
        shm.attach(1);
        shm.write(0, &[42, 43, 44]).unwrap();
        assert_eq!(shm.read(0, 3).unwrap(), vec![42, 43, 44]);
    }

    // ---------------------------------------------------------------
    // Cross-type independence
    // ---------------------------------------------------------------

    #[test]
    fn test_resource_types_independent() {
        let mut mgr = IpcManager::new();
        mgr.create_pipe(4096);
        mgr.create_message_queue("q1".to_string(), 256, 4096);
        mgr.create_shared_memory("r1".to_string(), 64, 1);

        assert_eq!(mgr.list_pipes().len(), 1);
        assert_eq!(mgr.list_message_queues().len(), 1);
        assert_eq!(mgr.list_shared_memory().len(), 1);
    }

    #[test]
    fn test_default_impl() {
        let mgr = IpcManager::default();
        assert!(mgr.list_pipes().is_empty());
    }
}
