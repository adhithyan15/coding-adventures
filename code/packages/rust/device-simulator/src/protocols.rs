//! Protocols -- shared types for all device simulators.
//!
//! # What is a Device Simulator?
//!
//! A device simulator models a **complete accelerator** -- not just one compute
//! unit, but the entire chip with all its compute units, global memory, caches,
//! and the work distributor that ties them together.
//!
//! Think of it as the difference between simulating one factory floor (Layer 7)
//! versus simulating the entire factory complex:
//!
//! ```text
//! Layer 7 (Compute Unit):    One SM / CU / MXU -- a single factory floor
//! Layer 6 (Device):          The whole factory -- all floors + warehouse +
//!                            shipping dock + floor manager's office
//! ```
//!
//! The device layer adds four new concepts:
//!
//! 1. **Global Memory (VRAM)** -- the large device-wide memory (the warehouse).
//!    All compute units share it. High bandwidth but high latency (~400 cycles).
//!
//! 2. **L2 Cache** -- sits between compute units and global memory. Reduces the
//!    average latency for frequently-accessed data.
//!
//! 3. **Work Distributor** -- takes kernel launches (work orders) and assigns
//!    thread blocks to compute units that have available resources.
//!
//! 4. **Host Interface** -- the connection to the CPU. Data must be copied from
//!    CPU memory to device memory before the GPU can use it (except on Apple's
//!    unified memory, where it's zero-copy).
//!
//! # Memory Hierarchy at the Device Level
//!
//! ```text
//!             +----------------+
//! CPU RAM --> | Host Interface | --> PCIe / NVLink / unified
//!             +-------+--------+
//!                     |
//!             +-------+--------+
//!             | Global Memory  |  24-80 GB, ~400 cycle latency
//!             |  (HBM/GDDR)   |  1-3 TB/s bandwidth
//!             +-------+--------+
//!                     |
//!             +-------+--------+
//!             |   L2 Cache     |  4-96 MB, ~200 cycle latency
//!             |  (shared)      |
//!             +--+---+---+-----+
//!                |   |   |
//!              CU 0 CU 1 ... CU N   (each with local shared memory)
//! ```

use compute_unit::ComputeUnitTrace;
use gpu_core::Instruction;

use crate::global_memory::GlobalMemoryStats;

// =========================================================================
// MemoryTransaction -- a single wide memory access after coalescing
// =========================================================================

/// A single wide memory transaction after coalescing.
///
/// When 32 threads in a warp each request 4 bytes, those 128 bytes of
/// requests might coalesce into a single 128-byte transaction (best case)
/// or 32 separate transactions (worst case -- scattered access).
///
/// # Coalescing Visual
///
/// ```text
/// Best case (1 transaction):
///     Thread  0  1  2  3  4  ...  31
///     Addr   [0][4][8][12][16]...[124]
///            +------------------------+
///              One 128B transaction
///
/// Worst case (32 transactions):
///     Thread  0     1      2      3
///     Addr   [0]  [512]  [1024]  [1536]  ...
///             |      |      |      |
///             v      v      v      v
///          Trans 1 Trans 2 Trans 3 Trans 4
/// ```
#[derive(Debug, Clone)]
pub struct MemoryTransaction {
    /// Aligned start address of the transaction.
    pub address: u64,
    /// Transaction size in bytes (32, 64, or 128).
    pub size: u64,
    /// Bitmask of which threads are served by this transaction.
    pub thread_mask: u64,
}

// =========================================================================
// KernelDescriptor -- what gets launched on the device
// =========================================================================

/// Describes a kernel launch (GPU) or operation (TPU/NPU).
///
/// # Two Worlds
///
/// GPU-style devices (NVIDIA, AMD, Intel) receive a **program** with grid
/// and block dimensions -- "run this code on this many threads."
///
/// Dataflow-style devices (TPU, NPU) receive an **operation** with input
/// and weight data -- "multiply these matrices" or "apply this activation."
///
/// The same KernelDescriptor handles both by having fields for each style.
/// GPU devices use the program/grid/block fields. Dataflow devices use the
/// operation/input/weight fields.
#[derive(Debug, Clone)]
pub struct KernelDescriptor {
    /// Human-readable kernel name.
    pub name: String,
    /// Unique kernel identifier.
    pub kernel_id: u64,

    // GPU-style fields
    /// The program (list of instructions) to run on each thread.
    pub program: Option<Vec<Instruction>>,
    /// Grid dimensions (blocks in x, y, z).
    pub grid_dim: (usize, usize, usize),
    /// Block dimensions (threads per block in x, y, z).
    pub block_dim: (usize, usize, usize),
    /// Shared memory bytes requested per block.
    pub shared_mem_bytes: usize,
    /// Registers needed per thread.
    pub registers_per_thread: usize,

    // Dataflow-style fields (TPU/NPU)
    /// Operation name ("matmul", "add", "relu", etc.).
    pub operation: String,
    /// Input data matrix for dataflow devices.
    pub input_data: Option<Vec<Vec<f64>>>,
    /// Weight data matrix for dataflow devices.
    pub weight_data: Option<Vec<Vec<f64>>>,
    /// Output address in global memory.
    pub output_address: u64,
}

impl Default for KernelDescriptor {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            kernel_id: 0,
            program: None,
            grid_dim: (1, 1, 1),
            block_dim: (32, 1, 1),
            shared_mem_bytes: 0,
            registers_per_thread: 32,
            operation: String::new(),
            input_data: None,
            weight_data: None,
            output_address: 0,
        }
    }
}

impl KernelDescriptor {
    /// Total number of threads across all blocks.
    pub fn total_threads(&self) -> usize {
        let (gx, gy, gz) = self.grid_dim;
        let (bx, by, bz) = self.block_dim;
        gx * gy * gz * bx * by * bz
    }

    /// Total number of thread blocks in the grid.
    pub fn total_blocks(&self) -> usize {
        let (gx, gy, gz) = self.grid_dim;
        gx * gy * gz
    }

    /// Number of threads in each block.
    pub fn threads_per_block(&self) -> usize {
        let (bx, by, bz) = self.block_dim;
        bx * by * bz
    }
}

// =========================================================================
// DeviceConfig -- full device specification
// =========================================================================

/// Complete device specification.
///
/// Every accelerator is characterized by how many compute units it has,
/// how much and how fast its memory is, how it connects to the CPU,
/// and how it distributes work.
///
/// # Memory Hierarchy Parameters
///
/// ```text
/// Host RAM --[host_bandwidth]--> Global Memory (VRAM)
///                                       |
///                               [global_memory_bandwidth]
///                                       |
///                                  L2 Cache
///                                       |
///                               Compute Units (shared memory)
///                                       |
///                                  Registers
/// ```
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Device name ("NVIDIA H100", "AMD RX 7900 XTX", etc.).
    pub name: String,
    /// Architecture identifier ("nvidia_sm", "amd_cu", etc.).
    pub architecture: String,

    // Compute
    /// Number of compute units (SMs, CUs, Xe-Cores, etc.).
    pub num_compute_units: usize,

    // Memory hierarchy
    /// L2 cache size in bytes.
    pub l2_cache_size: usize,
    /// L2 cache access latency in cycles.
    pub l2_cache_latency: usize,
    /// L2 cache associativity.
    pub l2_cache_associativity: usize,
    /// L2 cache line size in bytes.
    pub l2_cache_line_size: usize,

    /// Global memory size in bytes.
    pub global_memory_size: u64,
    /// Global memory peak bandwidth in bytes per cycle.
    pub global_memory_bandwidth: f64,
    /// Global memory access latency in cycles.
    pub global_memory_latency: usize,
    /// Number of memory channels/partitions.
    pub memory_channels: usize,

    // Host interface
    /// Host (PCIe/NVLink) bandwidth in bytes per cycle.
    pub host_bandwidth: f64,
    /// Host transfer initial latency in cycles.
    pub host_latency: usize,
    /// True if device shares memory with host (Apple unified memory).
    pub unified_memory: bool,

    // Scheduling
    /// Maximum kernels that can execute concurrently.
    pub max_concurrent_kernels: usize,
    /// Work distribution policy ("round_robin", "fill_first", "least_loaded").
    pub work_distribution_policy: String,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            name: "Generic Accelerator".to_string(),
            architecture: "generic".to_string(),
            num_compute_units: 4,
            l2_cache_size: 4 * 1024 * 1024,
            l2_cache_latency: 200,
            l2_cache_associativity: 16,
            l2_cache_line_size: 128,
            global_memory_size: 16 * 1024 * 1024,
            global_memory_bandwidth: 1000.0,
            global_memory_latency: 400,
            memory_channels: 8,
            host_bandwidth: 64.0,
            host_latency: 1000,
            unified_memory: false,
            max_concurrent_kernels: 1,
            work_distribution_policy: "round_robin".to_string(),
        }
    }
}

// =========================================================================
// DeviceTrace -- cycle-by-cycle visibility into the whole device
// =========================================================================

/// One cycle of device-wide activity.
///
/// At the device level, we need to see all compute units simultaneously,
/// plus the memory system and work distributor. This is the information
/// that tools like NVIDIA Nsight Systems show -- the big picture of device
/// utilization.
///
/// Key questions a DeviceTrace answers:
/// - How many compute units are busy vs idle?
/// - Is the memory system a bottleneck?
/// - Is the work distributor keeping up?
/// - What's the overall device occupancy?
#[derive(Debug, Clone)]
pub struct DeviceTrace {
    /// Clock cycle number.
    pub cycle: u64,
    /// Device name.
    pub device_name: String,

    // Work distribution
    /// Actions the distributor took this cycle (e.g., "Block 42 -> SM 7").
    pub distributor_actions: Vec<String>,
    /// Number of blocks still waiting to be assigned.
    pub pending_blocks: usize,
    /// Number of blocks actively executing.
    pub active_blocks: usize,

    // Per-CU traces
    /// Traces from each compute unit this cycle.
    pub cu_traces: Vec<ComputeUnitTrace>,

    // Memory system
    /// L2 cache hits this cycle.
    pub l2_hits: usize,
    /// L2 cache misses this cycle.
    pub l2_misses: usize,
    /// Memory transactions issued this cycle.
    pub memory_transactions: usize,
    /// Fraction of peak bandwidth used (0.0 to 1.0).
    pub memory_bandwidth_used: f64,

    // Aggregate metrics
    /// Total active warps across all CUs.
    pub total_active_warps: usize,
    /// Device-wide occupancy (0.0 to 1.0).
    pub device_occupancy: f64,
    /// Floating-point operations this cycle.
    pub flops_this_cycle: usize,
}

impl DeviceTrace {
    /// Human-readable summary of this cycle.
    ///
    /// Example output:
    ///
    /// ```text
    /// [Cycle 10] NVIDIA H100 -- 45.2% occupancy
    ///   Distributor: Block 42 -> SM 7, Block 43 -> SM 12
    ///   Pending: 890 blocks, Active: 1056 blocks
    ///   L2: 342 hits, 12 misses (96.6% hit rate)
    ///   Memory: 8 transactions, 45.2% bandwidth
    ///   Active warps: 4234
    /// ```
    pub fn format(&self) -> String {
        let mut lines = vec![format!(
            "[Cycle {}] {} -- {:.1}% occupancy",
            self.cycle, self.device_name, self.device_occupancy * 100.0,
        )];

        if !self.distributor_actions.is_empty() {
            let actions_str = self.distributor_actions.join(", ");
            lines.push(format!("  Distributor: {}", actions_str));
        }

        lines.push(format!(
            "  Pending: {} blocks, Active: {} blocks",
            self.pending_blocks, self.active_blocks,
        ));

        let total_l2 = self.l2_hits + self.l2_misses;
        if total_l2 > 0 {
            let hit_rate = self.l2_hits as f64 / total_l2 as f64 * 100.0;
            lines.push(format!(
                "  L2: {} hits, {} misses ({:.1}% hit rate)",
                self.l2_hits, self.l2_misses, hit_rate,
            ));
        }

        lines.push(format!(
            "  Memory: {} transactions, {:.1}% bandwidth",
            self.memory_transactions, self.memory_bandwidth_used * 100.0,
        ));

        lines.push(format!("  Active warps: {}", self.total_active_warps));

        lines.join("\n")
    }
}

// =========================================================================
// DeviceStats -- aggregate metrics across the entire simulation
// =========================================================================

/// Device-wide aggregate statistics.
///
/// These stats answer the key performance questions:
///
/// 1. **Compute utilization**: Are the compute units busy or sitting idle?
/// 2. **Memory bandwidth utilization**: Is the memory system saturated?
/// 3. **Load imbalance**: Are some CUs doing more work than others?
/// 4. **L2 effectiveness**: Is the cache helping?
#[derive(Debug, Clone)]
pub struct DeviceStats {
    /// Total simulation cycles.
    pub total_cycles: u64,
    /// Cycles where at least one CU was active.
    pub active_cycles: u64,
    /// Cycles where all CUs were idle.
    pub idle_cycles: u64,

    // Compute
    /// Total floating-point operations executed.
    pub total_flops: u64,

    // Memory
    /// Global memory access statistics.
    pub global_memory_stats: GlobalMemoryStats,

    // Work distribution
    /// Total kernel launches.
    pub total_kernels_launched: u64,
    /// Total thread blocks dispatched.
    pub total_blocks_dispatched: u64,
}

impl Default for DeviceStats {
    fn default() -> Self {
        Self {
            total_cycles: 0,
            active_cycles: 0,
            idle_cycles: 0,
            total_flops: 0,
            global_memory_stats: GlobalMemoryStats::default(),
            total_kernels_launched: 0,
            total_blocks_dispatched: 0,
        }
    }
}

// =========================================================================
// AcceleratorDevice trait -- the unified device interface
// =========================================================================

/// The unified interface for all accelerator devices: GPU, TPU, NPU.
///
/// Despite radical differences between a GPU (thread-parallel, thousands of
/// cores) and a TPU (dataflow, one large matrix unit), they share a common
/// lifecycle:
///
/// 1. Allocate device memory
/// 2. Copy data from host to device
/// 3. Launch computation
/// 4. Wait for completion
/// 5. Copy results back to host
///
/// This trait captures that common lifecycle while leaving the
/// implementation details to each device type.
pub trait AcceleratorDevice {
    /// Device name ("NVIDIA H100", "Apple M3 Max ANE", etc.).
    fn name(&self) -> &str;

    /// Full device configuration.
    fn config(&self) -> &DeviceConfig;

    // --- Memory management ---

    /// Allocate device memory. Returns device pointer (address).
    fn malloc(&mut self, size: usize) -> u64;

    /// Free device memory allocation.
    fn free(&mut self, address: u64);

    /// Copy from host to device. Returns cycles consumed.
    fn memcpy_host_to_device(&mut self, dst: u64, data: &[u8]) -> u64;

    /// Copy from device to host. Returns (data, cycles).
    fn memcpy_device_to_host(&mut self, src: u64, size: usize) -> (Vec<u8>, u64);

    // --- Kernel launch ---

    /// Submit a kernel for execution.
    fn launch_kernel(&mut self, kernel: KernelDescriptor);

    // --- Simulation ---

    /// Advance the entire device by one clock cycle.
    fn step(&mut self) -> DeviceTrace;

    /// Run until all kernels complete or max_cycles reached.
    fn run(&mut self, max_cycles: usize) -> Vec<DeviceTrace>;

    /// True when all CUs are idle and no pending work remains.
    fn idle(&self) -> bool;

    /// Reset all state -- CUs, memory, caches, work queues.
    fn reset(&mut self);

    // --- Observability ---

    /// Aggregate statistics across all compute units and memory.
    fn stats(&self) -> DeviceStats;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_descriptor_default() {
        let k = KernelDescriptor::default();
        assert_eq!(k.name, "unnamed");
        assert_eq!(k.grid_dim, (1, 1, 1));
        assert_eq!(k.block_dim, (32, 1, 1));
        assert!(k.program.is_none());
    }

    #[test]
    fn test_kernel_total_threads() {
        let k = KernelDescriptor {
            grid_dim: (4, 2, 1),
            block_dim: (32, 1, 1),
            ..KernelDescriptor::default()
        };
        assert_eq!(k.total_threads(), 4 * 2 * 1 * 32);
    }

    #[test]
    fn test_kernel_total_blocks() {
        let k = KernelDescriptor {
            grid_dim: (4, 2, 3),
            ..KernelDescriptor::default()
        };
        assert_eq!(k.total_blocks(), 24);
    }

    #[test]
    fn test_device_config_default() {
        let c = DeviceConfig::default();
        assert_eq!(c.num_compute_units, 4);
        assert!(!c.unified_memory);
    }

    #[test]
    fn test_device_trace_format() {
        let trace = DeviceTrace {
            cycle: 10,
            device_name: "Test GPU".to_string(),
            distributor_actions: vec!["Block 0 -> SM 0".to_string()],
            pending_blocks: 5,
            active_blocks: 3,
            cu_traces: vec![],
            l2_hits: 10,
            l2_misses: 2,
            memory_transactions: 4,
            memory_bandwidth_used: 0.5,
            total_active_warps: 16,
            device_occupancy: 0.75,
            flops_this_cycle: 0,
        };
        let formatted = trace.format();
        assert!(formatted.contains("[Cycle 10]"));
        assert!(formatted.contains("Test GPU"));
        assert!(formatted.contains("75.0%"));
        assert!(formatted.contains("Block 0 -> SM 0"));
    }

    #[test]
    fn test_device_stats_default() {
        let s = DeviceStats::default();
        assert_eq!(s.total_cycles, 0);
        assert_eq!(s.total_kernels_launched, 0);
    }
}
