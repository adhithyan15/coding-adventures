//! Protocols -- shared types for all compute unit simulators.
//!
//! # What is a Compute Unit?
//!
//! A compute unit is the organizational structure that wraps execution engines
//! (Layer 8) with scheduling, shared memory, register files, and caches to form
//! a complete computational building block. Think of it as the "factory floor":
//!
//! ```text
//! Workers         = execution engines (warps, wavefronts, systolic arrays)
//! Floor manager   = warp/wavefront scheduler
//! Shared toolbox  = shared memory / LDS (data accessible to all teams)
//! Supply closet   = L1 cache (recent data kept nearby)
//! Filing cabinets = register file (massive, partitioned among teams)
//! Work orders     = thread blocks / work groups queued for execution
//! ```
//!
//! Every vendor has a different name for this level of the hierarchy:
//!
//! ```text
//! NVIDIA:   Streaming Multiprocessor (SM)
//! AMD:      Compute Unit (CU) / Work Group Processor (WGP in RDNA)
//! Intel:    Xe Core (or Subslice in older gen)
//! Google:   Matrix Multiply Unit (MXU) + Vector/Scalar units
//! Apple:    Neural Engine Core
//! ```
//!
//! Despite the naming differences, they all serve the same purpose: take
//! execution engines, add scheduling and shared resources, and present a
//! coherent compute unit to the device layer above.
//!
//! # Protocol-Based Design
//!
//! Just like Layer 8 (parallel-execution-engine), we use a Rust trait to
//! define a common interface that all compute units implement. This allows
//! higher layers to drive any compute unit uniformly, regardless of vendor.
//!
//! A trait is Rust's version of an "interface" or "protocol" -- any struct
//! that implements the right methods satisfies the trait.

use std::collections::HashMap;
use std::fmt;

use parallel_execution_engine::EngineTrace;
use gpu_core::Instruction;

// ---------------------------------------------------------------------------
// Architecture -- which vendor's compute unit this is
// ---------------------------------------------------------------------------

/// Vendor architectures supported at the compute unit level.
///
/// Each architecture represents a fundamentally different approach to
/// organizing parallel computation. They are NOT interchangeable -- each
/// has unique scheduling strategies, memory hierarchies, and execution
/// models.
///
/// ```text
/// Architecture      | Scheduling    | Memory Model  | Execution
/// ------------------+---------------+---------------+--------------
/// NVIDIA SM         | Warp sched.   | Shared mem    | SIMT warps
/// AMD CU            | Wave sched.   | LDS           | SIMD wavefronts
/// Google MXU        | Compile-time  | Weight buffer | Systolic array
/// Intel Xe Core     | Thread disp.  | SLM           | SIMD + threads
/// Apple ANE Core    | Compiler      | SRAM + DMA    | Scheduled MAC
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Architecture {
    /// NVIDIA Streaming Multiprocessor (Volta, Ampere, Hopper).
    NvidiaSm,
    /// AMD Compute Unit (GCN) / Work Group Processor (RDNA).
    AmdCu,
    /// Google TPU Matrix Multiply Unit.
    GoogleMxu,
    /// Intel Xe Core (Arc, Data Center GPU).
    IntelXeCore,
    /// Apple Neural Engine Core.
    AppleAneCore,
}

impl Architecture {
    /// Return the string value matching the Python enum.
    pub fn value(&self) -> &'static str {
        match self {
            Architecture::NvidiaSm => "nvidia_sm",
            Architecture::AmdCu => "amd_cu",
            Architecture::GoogleMxu => "google_mxu",
            Architecture::IntelXeCore => "intel_xe_core",
            Architecture::AppleAneCore => "apple_ane_core",
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}

// ---------------------------------------------------------------------------
// WarpState -- possible states of a warp in the scheduler
// ---------------------------------------------------------------------------

/// Possible states of a warp (or wavefront, or thread) in the scheduler.
///
/// A warp moves through these states during its lifetime:
///
/// ```text
/// READY --> RUNNING --> READY (if more instructions)
///   |                     |
///   |       +-------------+
///   |       |
///   +-> STALLED_MEMORY --> READY (when data arrives)
///   +-> STALLED_BARRIER --> READY (when all warps reach barrier)
///   +-> STALLED_DEPENDENCY --> READY (when register available)
///   +-> COMPLETED
/// ```
///
/// The scheduler's job is to find a READY warp and issue it to an engine.
/// When a warp stalls (e.g., on a memory access), the scheduler switches
/// to another READY warp -- this is how GPUs hide latency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpState {
    /// Warp has an instruction ready to issue. Can be scheduled.
    Ready,
    /// Warp is currently executing on an engine this cycle.
    Running,
    /// Warp is waiting for a memory operation to complete.
    ///
    /// Memory accesses to global (off-chip) memory take ~200-400 cycles on a
    /// real GPU. During this time, the warp cannot execute and the scheduler
    /// must find another warp to keep the hardware busy.
    StalledMemory,
    /// Warp is waiting at a `__syncthreads()` barrier.
    ///
    /// Thread block synchronization requires ALL warps in the block to reach
    /// the barrier before any can proceed.
    StalledBarrier,
    /// Warp is waiting for a register dependency to resolve.
    StalledDependency,
    /// Warp has executed its HALT instruction. Done.
    Completed,
}

// ---------------------------------------------------------------------------
// SchedulingPolicy -- how the scheduler picks which warp to issue
// ---------------------------------------------------------------------------

/// How the warp scheduler picks which warp to issue next.
///
/// Real GPUs use sophisticated scheduling policies that balance throughput,
/// fairness, and latency hiding:
///
/// ```text
/// Policy       | Strategy              | Used by
/// -------------+-----------------------+--------------
/// ROUND_ROBIN  | Fair rotation         | Teaching, some AMD
/// GREEDY       | Most-ready-first      | Throughput-focused
/// OLDEST_FIRST | Longest-waiting-first | Fairness-focused
/// GTO          | Same warp til stall   | NVIDIA (common)
/// LRR          | Skip-stalled rotation | AMD (common)
/// ```
///
/// GTO (Greedy-Then-Oldest) is particularly interesting: it keeps issuing
/// from the same warp until it stalls, then switches to the oldest ready
/// warp. This reduces context-switch overhead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Simple rotation: warp 0, 1, 2, ..., wrap around.
    RoundRobin,
    /// Always pick the warp with the most ready instructions.
    Greedy,
    /// Pick the warp that has been waiting longest.
    OldestFirst,
    /// Greedy-Then-Oldest: issue from same warp until it stalls,
    /// then switch to the oldest ready warp. NVIDIA's common choice.
    Gto,
    /// Loose Round Robin: like round-robin but skips stalled warps.
    /// AMD's common choice.
    Lrr,
}

// ---------------------------------------------------------------------------
// WorkItem -- a unit of parallel work dispatched to a compute unit
// ---------------------------------------------------------------------------

/// A unit of parallel work dispatched to a compute unit.
///
/// In CUDA terms, this is a **thread block** (cooperative thread array).
/// In OpenCL terms, this is a **work group**.
/// In TPU terms, this is a **tile** of a matrix operation.
/// In NPU terms, this is an **inference tile**.
///
/// # Thread Block Decomposition (NVIDIA example)
///
/// A WorkItem with `thread_count=256` on an NVIDIA SM:
///
/// ```text
/// WorkItem(thread_count=256)
/// +-- Warp 0:  threads 0-31
/// +-- Warp 1:  threads 32-63
/// +-- ...
/// +-- Warp 7:  threads 224-255
/// ```
///
/// All 8 warps share the same shared memory and can synchronize with
/// `__syncthreads()`.
#[derive(Debug, Clone)]
pub struct WorkItem {
    /// Unique identifier for this work item.
    pub work_id: usize,
    /// Instruction list for instruction-stream architectures.
    pub program: Option<Vec<Instruction>>,
    /// Number of parallel threads/lanes in this block.
    pub thread_count: usize,
    /// Per-thread initial register values: `per_thread_data[thread_id][reg] = value`.
    pub per_thread_data: HashMap<usize, HashMap<usize, f64>>,
    /// Activation matrix for dataflow architectures (TPU/NPU).
    pub input_data: Option<Vec<Vec<f64>>>,
    /// Weight matrix for dataflow architectures.
    pub weight_data: Option<Vec<Vec<f64>>>,
    /// MAC schedule for NPU-style architectures.
    pub schedule: Option<Vec<Vec<f64>>>,
    /// Shared memory requested by this work item (bytes).
    pub shared_mem_bytes: usize,
    /// Registers needed per thread (for occupancy calculation).
    pub registers_per_thread: usize,
}

impl Default for WorkItem {
    fn default() -> Self {
        Self {
            work_id: 0,
            program: None,
            thread_count: 32,
            per_thread_data: HashMap::new(),
            input_data: None,
            weight_data: None,
            schedule: None,
            shared_mem_bytes: 0,
            registers_per_thread: 32,
        }
    }
}

// ---------------------------------------------------------------------------
// ComputeUnitTrace -- record of one clock cycle across the compute unit
// ---------------------------------------------------------------------------

/// Record of one clock cycle across the entire compute unit.
///
/// Captures scheduler decisions, engine activity, memory accesses, and
/// resource utilization -- everything needed to understand what the compute
/// unit did in one cycle.
///
/// # Why Trace Everything?
///
/// Tracing is how you learn what GPUs actually do. Without traces, a GPU
/// is a black box. With traces, you can see:
///
/// - Which warp the scheduler picked and why
/// - How many warps are stalled on memory
/// - What occupancy looks like cycle by cycle
/// - Where bank conflicts happen in shared memory
#[derive(Debug, Clone)]
pub struct ComputeUnitTrace {
    /// Clock cycle number.
    pub cycle: u64,
    /// Which compute unit produced this trace.
    pub unit_name: String,
    /// Which vendor architecture.
    pub architecture: Architecture,
    /// What the scheduler decided this cycle.
    pub scheduler_action: String,
    /// How many warps/wavefronts are currently active.
    pub active_warps: usize,
    /// Maximum warps this unit can hold.
    pub total_warps: usize,
    /// Per-engine traces (engine_id -> EngineTrace from Layer 8).
    pub engine_traces: HashMap<usize, EngineTrace>,
    /// Bytes of shared memory in use.
    pub shared_memory_used: usize,
    /// Total shared memory available.
    pub shared_memory_total: usize,
    /// Registers currently allocated.
    pub register_file_used: usize,
    /// Total registers available.
    pub register_file_total: usize,
    /// active_warps / max_warps (0.0 to 1.0).
    pub occupancy: f64,
    /// L1 cache hits this cycle.
    pub l1_hits: usize,
    /// L1 cache misses this cycle.
    pub l1_misses: usize,
}

impl ComputeUnitTrace {
    /// Pretty-print the trace for educational display.
    ///
    /// Example output:
    ///
    /// ```text
    /// [Cycle 5] SM (nvidia_sm) -- 75.0% occupancy (48/64 warps)
    ///   Scheduler: issued warp 3 (GTO policy)
    ///   Shared memory: 49152/98304 bytes (50.0%)
    ///   Registers: 32768/65536 (50.0%)
    ///   Engine 0: FMUL R2, R0, R1 -- 32/32 threads active
    /// ```
    pub fn format(&self) -> String {
        let occ_pct = format!("{:.1}%", self.occupancy * 100.0);
        let mut lines = vec![format!(
            "[Cycle {}] {} ({}) -- {} occupancy ({}/{} warps)",
            self.cycle, self.unit_name, self.architecture.value(),
            occ_pct, self.active_warps, self.total_warps,
        )];
        lines.push(format!("  Scheduler: {}", self.scheduler_action));

        if self.shared_memory_total > 0 {
            let smem_pct =
                self.shared_memory_used as f64 / self.shared_memory_total as f64 * 100.0;
            lines.push(format!(
                "  Shared memory: {}/{} bytes ({:.1}%)",
                self.shared_memory_used, self.shared_memory_total, smem_pct,
            ));
        }

        if self.register_file_total > 0 {
            let reg_pct =
                self.register_file_used as f64 / self.register_file_total as f64 * 100.0;
            lines.push(format!(
                "  Registers: {}/{} ({:.1}%)",
                self.register_file_used, self.register_file_total, reg_pct,
            ));
        }

        let mut sorted_ids: Vec<usize> = self.engine_traces.keys().copied().collect();
        sorted_ids.sort();
        for eid in sorted_ids {
            lines.push(format!(
                "  Engine {}: {}",
                eid, self.engine_traces[&eid].description,
            ));
        }

        lines.join("\n")
    }
}

impl fmt::Display for ComputeUnitTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

// ---------------------------------------------------------------------------
// SharedMemory -- programmer-visible scratchpad with bank conflict detection
// ---------------------------------------------------------------------------

/// Shared memory with bank conflict detection.
///
/// # What is Shared Memory?
///
/// Shared memory is a small, fast, programmer-managed scratchpad that's
/// visible to all threads in a thread block. It's the GPU equivalent of
/// a team whiteboard -- everyone on the team can read and write to it.
///
/// ```text
/// Memory Level      | Latency    | Bandwidth
/// ------------------+------------+----------
/// Registers         | 0 cycles   | unlimited
/// Shared memory     | ~1-4 cycles| ~10 TB/s
/// L1 cache          | ~30 cycles | ~2 TB/s
/// Global (VRAM)     | ~400 cycles| ~1 TB/s
/// ```
///
/// That's a 100x latency difference between shared memory and global memory.
///
/// # Bank Conflicts -- The Hidden Performance Trap
///
/// Shared memory is divided into **banks** (typically 32). Each bank can
/// serve one request per cycle. If two threads access the same bank but
/// at different addresses, they **serialize** -- this is a bank conflict.
///
/// ```text
/// Bank mapping (32 banks, 4 bytes per bank):
///   Address 0x00 -> Bank 0    Address 0x04 -> Bank 1    ...
///   Address 0x80 -> Bank 0    Address 0x84 -> Bank 1    ...
///
/// The bank for an address is: (address / bank_width) % num_banks
/// ```
pub struct SharedMemory {
    /// Total bytes of shared memory.
    pub size: usize,
    /// Number of memory banks (typically 32).
    pub num_banks: usize,
    /// Bytes per bank (typically 4).
    pub bank_width: usize,
    /// The raw data storage.
    data: Vec<u8>,
    /// Total read/write accesses.
    total_accesses: usize,
    /// Total bank conflicts detected.
    total_conflicts: usize,
}

impl SharedMemory {
    /// Create a new shared memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Total bytes of shared memory.
    /// * `num_banks` - Number of memory banks (typically 32).
    /// * `bank_width` - Bytes per bank (typically 4).
    pub fn new(size: usize, num_banks: usize, bank_width: usize) -> Self {
        Self {
            size,
            num_banks,
            bank_width,
            data: vec![0u8; size],
            total_accesses: 0,
            total_conflicts: 0,
        }
    }

    /// Create shared memory with default bank configuration (32 banks, 4 bytes/bank).
    pub fn with_size(size: usize) -> Self {
        Self::new(size, 32, 4)
    }

    /// Read a 4-byte float from shared memory.
    ///
    /// # Panics
    ///
    /// Panics if the address is out of range.
    pub fn read(&mut self, address: usize, _thread_id: usize) -> f64 {
        assert!(
            address + 4 <= self.size,
            "Shared memory address {} out of range [0, {})",
            address, self.size,
        );
        self.total_accesses += 1;
        let bytes: [u8; 4] = [
            self.data[address],
            self.data[address + 1],
            self.data[address + 2],
            self.data[address + 3],
        ];
        f32::from_le_bytes(bytes) as f64
    }

    /// Write a 4-byte float to shared memory.
    ///
    /// # Panics
    ///
    /// Panics if the address is out of range.
    pub fn write(&mut self, address: usize, value: f64, _thread_id: usize) {
        assert!(
            address + 4 <= self.size,
            "Shared memory address {} out of range [0, {})",
            address, self.size,
        );
        self.total_accesses += 1;
        let bytes = (value as f32).to_le_bytes();
        self.data[address..address + 4].copy_from_slice(&bytes);
    }

    /// Detect bank conflicts for a set of simultaneous accesses.
    ///
    /// Given a list of addresses (one per thread), determine which accesses
    /// conflict (hit the same bank). Returns a list of conflict groups --
    /// each group is a list of thread indices that conflict. Groups of size 1
    /// (no conflict) are NOT included.
    ///
    /// # How Bank Conflict Detection Works
    ///
    /// 1. Compute the bank for each address: `bank = (address / bank_width) % num_banks`
    /// 2. Group threads by bank.
    /// 3. Any bank accessed by more than one thread is a conflict.
    ///
    /// ```text
    /// Example:
    ///   Threads 0 and 2 both hit bank 0 (addresses 0 and 128)
    ///   check_bank_conflicts([0, 4, 128, 12]) -> [[0, 2]]
    /// ```
    pub fn check_bank_conflicts(&mut self, addresses: &[usize]) -> Vec<Vec<usize>> {
        let mut bank_to_threads: HashMap<usize, Vec<usize>> = HashMap::new();
        for (thread_idx, &addr) in addresses.iter().enumerate() {
            let bank = (addr / self.bank_width) % self.num_banks;
            bank_to_threads.entry(bank).or_default().push(thread_idx);
        }

        let mut conflicts = Vec::new();
        for threads in bank_to_threads.values() {
            if threads.len() > 1 {
                conflicts.push(threads.clone());
                self.total_conflicts += threads.len() - 1;
            }
        }
        conflicts
    }

    /// Clear all data and reset statistics.
    pub fn reset(&mut self) {
        self.data = vec![0u8; self.size];
        self.total_accesses = 0;
        self.total_conflicts = 0;
    }

    /// Total number of read/write accesses.
    pub fn total_accesses(&self) -> usize {
        self.total_accesses
    }

    /// Total bank conflicts detected.
    pub fn total_conflicts(&self) -> usize {
        self.total_conflicts
    }
}

// ---------------------------------------------------------------------------
// ResourceError -- raised when dispatch fails due to resource limits
// ---------------------------------------------------------------------------

/// Error type for when a compute unit cannot accommodate a work item.
///
/// This happens when the SM doesn't have enough registers, shared memory,
/// or warp slots to fit the requested thread block.
#[derive(Debug, Clone)]
pub struct ResourceError {
    pub message: String,
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ResourceError: {}", self.message)
    }
}

impl std::error::Error for ResourceError {}

// ---------------------------------------------------------------------------
// ComputeUnit trait -- the unified interface
// ---------------------------------------------------------------------------

/// The unified interface for all compute units: SM, CU, MXU, Xe Core, ANE Core.
///
/// Despite radical differences between NVIDIA SMs, AMD CUs, and Google MXUs,
/// they all share this common interface:
///
/// 1. `dispatch(work)` -- accept work
/// 2. `step()` -- advance one cycle
/// 3. `run(max_cycles)` -- run until done
/// 4. `idle()` -- is all work complete?
/// 5. `reset()` -- clear all state
///
/// This lets the device layer above treat all compute units uniformly.
pub trait ComputeUnit {
    /// Unit name: "SM", "CU", "MXU", "XeCore", "ANECore".
    fn name(&self) -> &str;

    /// Which vendor architecture this compute unit belongs to.
    fn architecture(&self) -> Architecture;

    /// Accept a work item (thread block, work group, tile) for execution.
    fn dispatch(&mut self, work: WorkItem) -> Result<(), ResourceError>;

    /// Advance one clock cycle across all engines and the scheduler.
    fn step(&mut self) -> ComputeUnitTrace;

    /// Run until all dispatched work is complete or max_cycles.
    fn run(&mut self, max_cycles: usize) -> Vec<ComputeUnitTrace>;

    /// True if no work remains and all engines are idle.
    fn idle(&self) -> bool;

    /// Reset all state: engines, scheduler, shared memory, caches.
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_value() {
        assert_eq!(Architecture::NvidiaSm.value(), "nvidia_sm");
        assert_eq!(Architecture::AmdCu.value(), "amd_cu");
        assert_eq!(Architecture::GoogleMxu.value(), "google_mxu");
        assert_eq!(Architecture::IntelXeCore.value(), "intel_xe_core");
        assert_eq!(Architecture::AppleAneCore.value(), "apple_ane_core");
    }

    #[test]
    fn test_architecture_display() {
        assert_eq!(format!("{}", Architecture::NvidiaSm), "nvidia_sm");
        assert_eq!(format!("{}", Architecture::AppleAneCore), "apple_ane_core");
    }

    #[test]
    fn test_work_item_default() {
        let work = WorkItem::default();
        assert_eq!(work.work_id, 0);
        assert_eq!(work.thread_count, 32);
        assert_eq!(work.registers_per_thread, 32);
        assert!(work.program.is_none());
        assert!(work.input_data.is_none());
    }

    #[test]
    fn test_shared_memory_read_write() {
        let mut smem = SharedMemory::with_size(1024);
        smem.write(0, 3.14, 0);
        let val = smem.read(0, 0);
        // f32 precision: 3.14 -> 3.140000104904175...
        assert!((val - 3.14).abs() < 0.001);
        assert_eq!(smem.total_accesses(), 2);
    }

    #[test]
    fn test_shared_memory_bank_conflicts() {
        let mut smem = SharedMemory::with_size(1024);
        // Addresses 0 and 128 both map to bank 0 (32 banks * 4 bytes = 128 byte stride)
        let conflicts = smem.check_bank_conflicts(&[0, 4, 128, 12]);
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].contains(&0));
        assert!(conflicts[0].contains(&2));
    }

    #[test]
    fn test_shared_memory_no_conflicts() {
        let mut smem = SharedMemory::with_size(1024);
        // Each thread hits a different bank
        let conflicts = smem.check_bank_conflicts(&[0, 4, 8, 12]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_shared_memory_reset() {
        let mut smem = SharedMemory::with_size(1024);
        smem.write(0, 42.0, 0);
        smem.reset();
        let val = smem.read(0, 0);
        assert_eq!(val, 0.0);
        // reset clears accesses, then we did one read
        assert_eq!(smem.total_accesses(), 1);
    }

    #[test]
    fn test_compute_unit_trace_format() {
        let trace = ComputeUnitTrace {
            cycle: 5,
            unit_name: "SM".to_string(),
            architecture: Architecture::NvidiaSm,
            scheduler_action: "issued warp 3".to_string(),
            active_warps: 4,
            total_warps: 48,
            engine_traces: HashMap::new(),
            shared_memory_used: 49152,
            shared_memory_total: 98304,
            register_file_used: 32768,
            register_file_total: 65536,
            occupancy: 4.0 / 48.0,
            l1_hits: 0,
            l1_misses: 0,
        };
        let formatted = trace.format();
        assert!(formatted.contains("[Cycle 5]"));
        assert!(formatted.contains("SM"));
        assert!(formatted.contains("nvidia_sm"));
        assert!(formatted.contains("issued warp 3"));
    }

    #[test]
    fn test_resource_error_display() {
        let err = ResourceError {
            message: "Not enough registers".to_string(),
        };
        let s = format!("{}", err);
        assert!(s.contains("Not enough registers"));
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_shared_memory_read_out_of_range() {
        let mut smem = SharedMemory::with_size(16);
        smem.read(16, 0); // out of range
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn test_shared_memory_write_out_of_range() {
        let mut smem = SharedMemory::with_size(16);
        smem.write(16, 1.0, 0); // out of range
    }
}
