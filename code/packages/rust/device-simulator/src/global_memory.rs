//! Global Memory -- device-wide VRAM / HBM simulator.
//!
//! # What is Global Memory?
//!
//! Global memory is the large, high-bandwidth memory that serves the entire
//! accelerator device. Every compute unit can read from and write to global
//! memory, making it the shared data store for all parallel computation.
//!
//! ```text
//! NVIDIA: HBM3 (High Bandwidth Memory) -- 80 GB on H100
//! AMD:    GDDR6 -- 24 GB on RX 7900 XTX
//! Google: HBM2e -- 32 GB per TPU v4 chip
//! Intel:  GDDR6 -- 16 GB on Arc A770
//! Apple:  Unified LPDDR5 -- shared with CPU/GPU, up to 192 GB
//! ```
//!
//! # Key Properties
//!
//! 1. **High bandwidth**: 1-3 TB/s (wide buses: 4096-bit HBM vs 64-bit DDR).
//! 2. **High latency**: ~400-800 cycles per request.
//! 3. **Shared**: ALL compute units share global memory.
//! 4. **Coalescing**: The memory controller merges contiguous thread requests.
//! 5. **Partitioned**: Memory is split across channels/stacks.
//!
//! # Sparse Memory Representation
//!
//! Real devices have 16-80 GB of VRAM. We use a sparse HashMap: only addresses
//! that have been written to consume actual memory. Reads to uninitialized
//! addresses return zeros (matching real hardware behavior after memset).

use std::collections::HashMap;

use crate::protocols::MemoryTransaction;

// =========================================================================
// GlobalMemoryStats -- tracks memory access patterns and efficiency
// =========================================================================

/// Tracks memory access patterns and efficiency.
///
/// Memory access patterns are the #1 performance bottleneck on GPUs.
/// A kernel that achieves perfect coalescing uses 32x less bandwidth than
/// one with fully scattered access.
///
/// Key metric: **coalescing_efficiency** = total_requests / total_transactions.
/// Ideal = 1.0 per thread (e.g., 32.0 for 32 threads coalescing into 1 txn).
#[derive(Debug, Clone)]
pub struct GlobalMemoryStats {
    /// Number of read operations.
    pub total_reads: u64,
    /// Number of write operations.
    pub total_writes: u64,
    /// Memory transactions after coalescing.
    pub total_transactions: u64,
    /// Memory requests before coalescing (per-thread).
    pub total_requests: u64,
    /// Total bytes moved through the memory system.
    pub bytes_transferred: u64,
    /// requests / transactions (higher = better coalescing).
    pub coalescing_efficiency: f64,
    /// Times multiple requests hit same memory channel.
    pub partition_conflicts: u64,
    /// Bytes copied from CPU to device.
    pub host_to_device_bytes: u64,
    /// Bytes copied from device to CPU.
    pub device_to_host_bytes: u64,
    /// Total cycles spent on host transfers.
    pub host_transfer_cycles: u64,
}

impl Default for GlobalMemoryStats {
    fn default() -> Self {
        Self {
            total_reads: 0,
            total_writes: 0,
            total_transactions: 0,
            total_requests: 0,
            bytes_transferred: 0,
            coalescing_efficiency: 0.0,
            partition_conflicts: 0,
            host_to_device_bytes: 0,
            device_to_host_bytes: 0,
            host_transfer_cycles: 0,
        }
    }
}

impl GlobalMemoryStats {
    /// Recalculate coalescing efficiency from current counts.
    pub fn update_efficiency(&mut self) {
        if self.total_transactions > 0 {
            self.coalescing_efficiency =
                self.total_requests as f64 / self.total_transactions as f64;
        }
    }
}

// =========================================================================
// SimpleGlobalMemory -- sparse VRAM/HBM model
// =========================================================================

/// Global memory implementation with coalescing and partitioning.
///
/// This models the device-wide memory (VRAM/HBM) that all compute units
/// share. It tracks access patterns, coalescing efficiency, and partition
/// conflicts to help identify memory bottlenecks.
///
/// # Usage
///
/// ```
/// use device_simulator::global_memory::SimpleGlobalMemory;
///
/// let mut mem = SimpleGlobalMemory::new(1024 * 1024, 1000.0, 400, 4, 128, 64.0, 1000, false);
///
/// // Allocate space
/// let addr = mem.allocate(256, 256).unwrap();
///
/// // Write and read
/// mem.write(addr, &[0x41, 0x42, 0x43, 0x44]).unwrap();
/// let data = mem.read(addr, 4).unwrap();
/// assert_eq!(data, vec![0x41, 0x42, 0x43, 0x44]);
/// ```
#[derive(Debug, Clone)]
pub struct SimpleGlobalMemory {
    /// Total memory capacity in bytes.
    capacity: u64,
    /// Peak bandwidth in bytes per cycle.
    bandwidth: f64,
    /// Access latency in cycles.
    _latency: usize,
    /// Number of memory partitions/channels.
    channels: usize,
    /// Width of a single memory transaction in bytes.
    transaction_size: u64,
    /// Host (PCIe/NVLink) bandwidth in bytes per cycle.
    host_bandwidth: f64,
    /// Initial latency for host transfers in cycles.
    host_latency: u64,
    /// If true, host transfers are zero-cost (Apple unified memory).
    unified: bool,

    /// Sparse storage -- only written addresses consume memory.
    /// Uses `HashMap<u64, u8>` so a 16 GB address space doesn't require
    /// 16 GB of RAM in the simulator.
    data: HashMap<u64, u8>,

    /// Simple bump allocator: next free address.
    next_free: u64,
    /// Active allocations: start_addr -> size.
    allocations: HashMap<u64, u64>,

    /// Access statistics.
    stats: GlobalMemoryStats,
}

impl SimpleGlobalMemory {
    /// Create a new global memory instance.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Total memory in bytes.
    /// * `bandwidth` - Peak bandwidth in bytes per cycle.
    /// * `latency` - Access latency in cycles.
    /// * `channels` - Number of memory partitions/channels.
    /// * `transaction_size` - Width of a single memory transaction (bytes).
    /// * `host_bandwidth` - PCIe/NVLink bandwidth in bytes per cycle.
    /// * `host_latency` - Initial latency for host transfers in cycles.
    /// * `unified` - If true, host transfers are zero-cost (Apple).
    pub fn new(
        capacity: u64,
        bandwidth: f64,
        latency: usize,
        channels: usize,
        transaction_size: u64,
        host_bandwidth: f64,
        host_latency: u64,
        unified: bool,
    ) -> Self {
        Self {
            capacity,
            bandwidth,
            _latency: latency,
            channels,
            transaction_size,
            host_bandwidth,
            host_latency,
            unified,
            data: HashMap::new(),
            next_free: 0,
            allocations: HashMap::new(),
            stats: GlobalMemoryStats::default(),
        }
    }

    /// Create with reasonable defaults for testing.
    pub fn with_capacity(capacity: u64) -> Self {
        Self::new(capacity, 1000.0, 400, 8, 128, 64.0, 1000, false)
    }

    // --- Properties ---

    /// Total memory in bytes.
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Peak bandwidth in bytes per cycle.
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }

    /// Access statistics (with efficiency recalculated).
    pub fn stats(&self) -> GlobalMemoryStats {
        let mut s = self.stats.clone();
        s.update_efficiency();
        s
    }

    // --- Allocation ---

    /// Allocate memory. Returns the start address.
    ///
    /// Uses a simple bump allocator with alignment. Like cudaMalloc,
    /// this returns a device pointer that can be passed to kernels.
    ///
    /// Returns `Err` if not enough memory remains.
    pub fn allocate(&mut self, size: u64, alignment: u64) -> Result<u64, String> {
        // Align the next free pointer
        let aligned = (self.next_free + alignment - 1) & !(alignment - 1);

        if aligned + size > self.capacity {
            return Err(format!(
                "Out of device memory: requested {} bytes at {}, capacity {}",
                size, aligned, self.capacity
            ));
        }

        self.allocations.insert(aligned, size);
        self.next_free = aligned + size;
        Ok(aligned)
    }

    /// Free a previous allocation.
    ///
    /// Our simple bump allocator doesn't reclaim memory, but we track
    /// that the free was called.
    pub fn free(&mut self, address: u64) {
        self.allocations.remove(&address);
    }

    // --- Read / Write ---

    /// Read bytes from global memory.
    ///
    /// Uninitialized addresses return zeros (like cudaMemset(0)).
    pub fn read(&mut self, address: u64, size: usize) -> Result<Vec<u8>, String> {
        if address + size as u64 > self.capacity {
            return Err(format!(
                "Address {}+{} out of range [0, {})",
                address, size, self.capacity
            ));
        }

        self.stats.total_reads += 1;
        self.stats.bytes_transferred += size as u64;

        let mut result = vec![0u8; size];
        for i in 0..size {
            result[i] = *self.data.get(&(address + i as u64)).unwrap_or(&0);
        }
        Ok(result)
    }

    /// Write bytes to global memory.
    pub fn write(&mut self, address: u64, data: &[u8]) -> Result<(), String> {
        let size = data.len() as u64;
        if address + size > self.capacity {
            return Err(format!(
                "Address {}+{} out of range [0, {})",
                address, size, self.capacity
            ));
        }

        self.stats.total_writes += 1;
        self.stats.bytes_transferred += size;

        for (i, &byte_val) in data.iter().enumerate() {
            self.data.insert(address + i as u64, byte_val);
        }
        Ok(())
    }

    // --- Host transfers ---

    /// Copy from host (CPU) to device memory.
    ///
    /// For unified memory (Apple), this is zero-cost -- no actual data
    /// movement, just a page table remap.
    ///
    /// Returns the number of cycles consumed by the transfer.
    pub fn copy_from_host(&mut self, dst_addr: u64, data: &[u8]) -> u64 {
        // Write the data regardless
        let _ = self.write(dst_addr, data);

        let size = data.len() as u64;
        self.stats.host_to_device_bytes += size;

        if self.unified {
            return 0;
        }

        // Transfer time = latency + size / bandwidth
        let cycles = if self.host_bandwidth > 0.0 {
            self.host_latency + (size as f64 / self.host_bandwidth) as u64
        } else {
            0
        };
        self.stats.host_transfer_cycles += cycles;
        cycles
    }

    /// Copy from device memory to host (CPU).
    ///
    /// Returns (data, cycles_consumed).
    pub fn copy_to_host(&mut self, src_addr: u64, size: usize) -> (Vec<u8>, u64) {
        let data = self.read(src_addr, size).unwrap_or_else(|_| vec![0; size]);

        self.stats.device_to_host_bytes += size as u64;

        if self.unified {
            return (data, 0);
        }

        let cycles = if self.host_bandwidth > 0.0 {
            self.host_latency + (size as f64 / self.host_bandwidth) as u64
        } else {
            0
        };
        self.stats.host_transfer_cycles += cycles;
        (data, cycles)
    }

    // --- Coalescing ---

    /// Given per-thread addresses, merge into coalesced transactions.
    ///
    /// # Coalescing Algorithm
    ///
    /// 1. For each thread's address, compute which transaction-sized
    ///    aligned region it falls in.
    /// 2. Group threads by aligned region.
    /// 3. Each group becomes one transaction.
    ///
    /// The fewer transactions, the better -- ideal is 1 transaction
    /// for 32 threads (128 bytes of contiguous access).
    pub fn coalesce(&mut self, addresses: &[u64]) -> Vec<MemoryTransaction> {
        let ts = self.transaction_size;

        // Group threads by aligned transaction address
        let mut groups: HashMap<u64, u64> = HashMap::new();
        for (thread_idx, &addr) in addresses.iter().enumerate() {
            let aligned = (addr / ts) * ts;
            let mask = groups.entry(aligned).or_insert(0);
            *mask |= 1u64 << thread_idx;
        }

        let mut sorted_addrs: Vec<u64> = groups.keys().copied().collect();
        sorted_addrs.sort();

        let transactions: Vec<MemoryTransaction> = sorted_addrs
            .iter()
            .map(|&aligned| MemoryTransaction {
                address: aligned,
                size: ts,
                thread_mask: groups[&aligned],
            })
            .collect();

        // Track stats
        self.stats.total_requests += addresses.len() as u64;
        self.stats.total_transactions += transactions.len() as u64;

        // Check partition conflicts
        let mut channels_hit: HashMap<u64, u64> = HashMap::new();
        for txn in &transactions {
            let channel = (txn.address / ts) % self.channels as u64;
            *channels_hit.entry(channel).or_insert(0) += 1;
        }
        for &count in channels_hit.values() {
            if count > 1 {
                self.stats.partition_conflicts += count - 1;
            }
        }

        transactions
    }

    // --- Reset ---

    /// Clear all data, allocations, and statistics.
    pub fn reset(&mut self) {
        self.data.clear();
        self.next_free = 0;
        self.allocations.clear();
        self.stats = GlobalMemoryStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read_back() {
        let mut mem = SimpleGlobalMemory::with_capacity(1024);
        mem.write(0, &[0x41, 0x42, 0x43, 0x44]).unwrap();
        let data = mem.read(0, 4).unwrap();
        assert_eq!(data, vec![0x41, 0x42, 0x43, 0x44]);
    }

    #[test]
    fn test_read_uninitialized_returns_zeros() {
        let mut mem = SimpleGlobalMemory::with_capacity(1024);
        let data = mem.read(0, 8).unwrap();
        assert_eq!(data, vec![0; 8]);
    }

    #[test]
    fn test_read_out_of_range() {
        let mut mem = SimpleGlobalMemory::with_capacity(64);
        assert!(mem.read(60, 8).is_err());
    }

    #[test]
    fn test_write_out_of_range() {
        let mut mem = SimpleGlobalMemory::with_capacity(64);
        assert!(mem.write(60, &[0; 8]).is_err());
    }

    #[test]
    fn test_allocate_returns_aligned_address() {
        let mut mem = SimpleGlobalMemory::with_capacity(1024 * 1024);
        let addr = mem.allocate(256, 256).unwrap();
        assert_eq!(addr % 256, 0);
    }

    #[test]
    fn test_sequential_allocations_dont_overlap() {
        let mut mem = SimpleGlobalMemory::with_capacity(1024 * 1024);
        let a1 = mem.allocate(256, 256).unwrap();
        let a2 = mem.allocate(256, 256).unwrap();
        assert!(a2 >= a1 + 256);
    }

    #[test]
    fn test_allocate_out_of_memory() {
        let mut mem = SimpleGlobalMemory::with_capacity(512);
        mem.allocate(256, 256).unwrap();
        assert!(mem.allocate(512, 256).is_err());
    }

    #[test]
    fn test_unified_memory_zero_cost() {
        let mut mem = SimpleGlobalMemory::new(1024, 1000.0, 400, 8, 128, 64.0, 1000, true);
        let cycles = mem.copy_from_host(0, &[0x01; 256]);
        assert_eq!(cycles, 0);
        let (data, cycles) = mem.copy_to_host(0, 256);
        assert_eq!(cycles, 0);
        assert_eq!(data, vec![0x01; 256]);
    }

    #[test]
    fn test_discrete_transfer_has_cost() {
        let mut mem = SimpleGlobalMemory::new(1024, 1000.0, 400, 8, 128, 64.0, 100, false);
        let cycles = mem.copy_from_host(0, &[0x01; 128]);
        assert!(cycles > 0);
    }

    #[test]
    fn test_coalesce_fully_coalesced() {
        let mut mem = SimpleGlobalMemory::new(1024, 1000.0, 400, 8, 128, 64.0, 100, false);
        let addrs: Vec<u64> = (0..32).map(|i| i * 4).collect();
        let transactions = mem.coalesce(&addrs);
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].size, 128);
        assert_eq!(transactions[0].address, 0);
    }

    #[test]
    fn test_coalesce_scattered() {
        let mut mem = SimpleGlobalMemory::new(1024 * 1024, 1000.0, 400, 8, 128, 64.0, 100, false);
        let addrs: Vec<u64> = (0..4).map(|i| i * 512).collect();
        let transactions = mem.coalesce(&addrs);
        assert_eq!(transactions.len(), 4);
    }

    #[test]
    fn test_coalescing_stats() {
        let mut mem = SimpleGlobalMemory::new(1024, 1000.0, 400, 8, 128, 64.0, 100, false);
        let addrs: Vec<u64> = (0..32).map(|i| i * 4).collect();
        mem.coalesce(&addrs);
        let stats = mem.stats();
        assert_eq!(stats.total_requests, 32);
        assert_eq!(stats.total_transactions, 1);
        assert_eq!(stats.coalescing_efficiency, 32.0);
    }

    #[test]
    fn test_reset_clears_everything() {
        let mut mem = SimpleGlobalMemory::with_capacity(1024);
        mem.write(0, &[0xFF; 4]).unwrap();
        mem.allocate(512, 256).unwrap();
        mem.reset();
        let data = mem.read(0, 4).unwrap();
        assert_eq!(data, vec![0; 4]);
        // Can allocate from beginning again
        let addr = mem.allocate(512, 256).unwrap();
        assert_eq!(addr, 0);
    }

    #[test]
    fn test_capacity_and_bandwidth() {
        let mem = SimpleGlobalMemory::new(4096, 3350.0, 400, 8, 128, 64.0, 100, false);
        assert_eq!(mem.capacity(), 4096);
        assert_eq!(mem.bandwidth(), 3350.0);
    }

    #[test]
    fn test_host_transfer_stats() {
        let mut mem = SimpleGlobalMemory::new(1024, 1000.0, 400, 8, 128, 64.0, 10, false);
        mem.copy_from_host(0, &[0; 128]);
        let stats = mem.stats();
        assert_eq!(stats.host_to_device_bytes, 128);
        assert!(stats.host_transfer_cycles > 0);
    }

    #[test]
    fn test_device_to_host_stats() {
        let mut mem = SimpleGlobalMemory::new(1024, 1000.0, 400, 8, 128, 64.0, 10, false);
        mem.write(0, &[0; 64]).unwrap();
        mem.copy_to_host(0, 64);
        let stats = mem.stats();
        assert_eq!(stats.device_to_host_bytes, 64);
    }
}
