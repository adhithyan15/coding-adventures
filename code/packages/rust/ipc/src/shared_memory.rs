/// Shared Memory -- zero-copy communication via a shared data region.
///
/// Pipes and message queues both **copy** data: the sender writes bytes into
/// a kernel buffer, and the receiver copies them out. For large data transfers,
/// this double-copy is expensive.
///
/// Shared memory eliminates copying entirely. Two (or more) processes map the
/// **same physical pages** into their virtual address spaces. A write by one
/// process is immediately visible to the others -- no system call, no copy,
/// no kernel involvement after the initial setup.
///
/// ## How It Works (Real OS)
///
/// ```text
///   Process A's address space       Process B's address space
///   +-----------------------+       +-----------------------+
///   | 0x8000: Shared Region |       | 0xC000: Shared Region |
///   |   "Hello from A"     |  <--  |   "Hello from A"      |
///   +-----------------------+       +-----------------------+
///            |                                |
///            +----------+--------------------+
///                       |
///                Physical Page Frame #42
/// ```
///
/// Both virtual addresses map to the same physical page. When A writes
/// "Hello from A", B can read it instantly -- same bytes in RAM.
///
/// ## Our Simulation
///
/// We simulate shared memory with a named region backed by a `Vec<u8>`.
/// "Attaching" a process means recording its PID in a `HashSet`;
/// reading and writing access the shared `Vec` directly.
///
/// ## WARNING: No Synchronization
///
/// Shared memory has NO built-in synchronization. If process A writes while
/// process B reads, B may see partially-updated data. Real programs use
/// semaphores, mutexes, or atomic operations to coordinate access.

use std::collections::HashSet;
use crate::IpcError;

pub struct SharedMemoryRegion {
    /// A string identifier that unrelated processes use to find this region.
    name: String,

    /// Region size in bytes.
    size: usize,

    /// The shared data -- zero-initialized byte array representing the
    /// physical pages that back this region.
    data: Vec<u8>,

    /// Process ID of the creator (has special privileges like deletion).
    owner_pid: u32,

    /// Set of process IDs currently attached. Used for cleanup: when the
    /// last process detaches, the region can be freed.
    attached_pids: HashSet<u32>,
}

impl SharedMemoryRegion {
    /// Create a new shared memory region, zero-initialized.
    pub fn new(name: String, size: usize, owner_pid: u32) -> Self {
        SharedMemoryRegion {
            name,
            size,
            data: vec![0u8; size],
            owner_pid,
            attached_pids: HashSet::new(),
        }
    }

    /// Attach a process to this region (record its PID).
    ///
    /// In a real OS, this modifies the process's page table to map the
    /// shared physical pages into its virtual address space.
    pub fn attach(&mut self, pid: u32) {
        self.attached_pids.insert(pid);
    }

    /// Detach a process from this region.
    ///
    /// In a real OS, this unmaps the shared pages from the process's
    /// virtual address space.
    pub fn detach(&mut self, pid: u32) {
        self.attached_pids.remove(&pid);
    }

    /// Read `count` bytes starting at `offset`.
    ///
    /// Unlike pipes (sequential), shared memory supports **random access**.
    pub fn read(&self, offset: usize, count: usize) -> Result<Vec<u8>, IpcError> {
        self.validate_bounds(offset, count)?;
        Ok(self.data[offset..offset + count].to_vec())
    }

    /// Write `data` starting at `offset`.
    ///
    /// WARNING: No synchronization. Concurrent writes from multiple processes
    /// can cause data races.
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), IpcError> {
        self.validate_bounds(offset, data.len())?;
        for (i, &byte) in data.iter().enumerate() {
            self.data[offset + i] = byte;
        }
        Ok(())
    }

    /// Get the region name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the region size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the owner PID.
    pub fn owner_pid(&self) -> u32 {
        self.owner_pid
    }

    /// Get the set of attached PIDs.
    pub fn attached_pids(&self) -> &HashSet<u32> {
        &self.attached_pids
    }

    /// Check that offset + count stays within bounds.
    fn validate_bounds(&self, offset: usize, count: usize) -> Result<(), IpcError> {
        if offset + count > self.size {
            return Err(IpcError::OutOfBounds {
                offset,
                count,
                size: self.size,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Creation --

    #[test]
    fn test_creation() {
        let shm = SharedMemoryRegion::new("test_region".to_string(), 1024, 1);
        assert_eq!(shm.name(), "test_region");
        assert_eq!(shm.size(), 1024);
        assert_eq!(shm.owner_pid(), 1);
        assert!(shm.attached_pids().is_empty());
    }

    #[test]
    fn test_initial_data_is_zeroed() {
        let shm = SharedMemoryRegion::new("test".to_string(), 8, 1);
        let result = shm.read(0, 8).unwrap();
        assert_eq!(result, vec![0u8; 8]);
    }

    // -- Attach/Detach --

    #[test]
    fn test_attach_adds_pid() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 64, 1);
        shm.attach(1);
        shm.attach(2);

        assert!(shm.attached_pids().contains(&1));
        assert!(shm.attached_pids().contains(&2));
        assert_eq!(shm.attached_pids().len(), 2);
    }

    #[test]
    fn test_detach_removes_pid() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 64, 1);
        shm.attach(1);
        shm.attach(2);
        shm.detach(1);

        assert!(!shm.attached_pids().contains(&1));
        assert!(shm.attached_pids().contains(&2));
    }

    #[test]
    fn test_attach_idempotent() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 64, 1);
        shm.attach(1);
        shm.attach(1);
        assert_eq!(shm.attached_pids().len(), 1);
    }

    // -- Read/Write --

    #[test]
    fn test_write_and_read() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 64, 1);
        shm.write(0, &[72, 101, 108, 108, 111]).unwrap();
        let result = shm.read(0, 5).unwrap();
        assert_eq!(result, vec![72, 101, 108, 108, 111]);
    }

    #[test]
    fn test_write_at_offset() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 64, 1);
        shm.write(10, &[1, 2, 3]).unwrap();

        assert_eq!(shm.read(10, 3).unwrap(), vec![1, 2, 3]);
        assert_eq!(shm.read(0, 3).unwrap(), vec![0, 0, 0]); // before offset still zero
    }

    #[test]
    fn test_overwrite() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 64, 1);
        shm.write(0, &[1, 2, 3]).unwrap();
        shm.write(0, &[4, 5, 6]).unwrap();
        assert_eq!(shm.read(0, 3).unwrap(), vec![4, 5, 6]);
    }

    // -- Bounds checking --

    #[test]
    fn test_read_out_of_bounds() {
        let shm = SharedMemoryRegion::new("test".to_string(), 8, 1);
        let result = shm.read(5, 10);
        assert_eq!(result, Err(IpcError::OutOfBounds { offset: 5, count: 10, size: 8 }));
    }

    #[test]
    fn test_write_out_of_bounds() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 8, 1);
        let result = shm.write(5, &[1, 2, 3, 4, 5]);
        assert_eq!(result, Err(IpcError::OutOfBounds { offset: 5, count: 5, size: 8 }));
    }

    #[test]
    fn test_read_at_exact_boundary() {
        let mut shm = SharedMemoryRegion::new("test".to_string(), 8, 1);
        shm.write(0, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let result = shm.read(0, 8).unwrap();
        assert_eq!(result, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    // -- Multi-process visibility --

    #[test]
    fn test_shared_data_visible() {
        let mut shm = SharedMemoryRegion::new("shared".to_string(), 64, 1);
        shm.attach(1);
        shm.attach(2);

        // "Process 1" writes.
        shm.write(0, &[42, 43, 44]).unwrap();

        // "Process 2" reads.
        assert_eq!(shm.read(0, 3).unwrap(), vec![42, 43, 44]);
    }
}
