//! StreamingMultiprocessor -- NVIDIA SM simulator.
//!
//! # What is a Streaming Multiprocessor?
//!
//! The SM is the heart of NVIDIA's GPU architecture. Every NVIDIA GPU -- from
//! the GeForce in your laptop to the H100 in a data center -- is built from
//! SMs. Each SM is a self-contained compute unit that can independently execute
//! work without coordination with other SMs.
//!
//! An SM contains:
//! - **Warp schedulers** (4 on modern GPUs) that pick ready warps to execute
//! - **WarpEngines** (one per scheduler) that execute 32-thread warps
//! - **Register file** (256 KB, 65536 registers) partitioned among warps
//! - **Shared memory** (up to 228 KB) for inter-thread communication
//! - **L1 cache** (often shares capacity with shared memory)
//!
//! # The Key Innovation: Latency Hiding
//!
//! CPUs hide latency with deep pipelines, out-of-order execution, and branch
//! prediction -- complex hardware that's expensive in transistors and power.
//!
//! GPUs take the opposite approach: have MANY warps, and when one stalls,
//! switch to another. A single SM can have 48-64 warps resident. When warp 0
//! stalls on a memory access (~400 cycles), the scheduler instantly switches
//! to warp 1. By the time it has cycled through enough warps, warp 0's data
//! has arrived.
//!
//! ```text
//! CPU strategy:  Make one thread FAST (deep pipeline, speculation, OoO)
//! GPU strategy:  Have MANY threads, switch instantly to hide latency
//! ```
//!
//! # Architecture Diagram
//!
//! ```text
//! StreamingMultiprocessor
//! +---------------------------------------------------------------+
//! |  Warp Scheduler 0        Warp Scheduler 1                      |
//! |  +-----------------+    +-----------------+                    |
//! |  | w0: READY       |    | w1: STALLED     |                    |
//! |  | w4: READY       |    | w5: READY       |                    |
//! |  +--------+--------+    +--------+--------+                    |
//! |           |                      |                             |
//! |           v                      v                             |
//! |  +-----------------+    +-----------------+                    |
//! |  | WarpEngine 0    |    | WarpEngine 1    |                    |
//! |  | (32 threads)    |    | (32 threads)    |                    |
//! |  +-----------------+    +-----------------+                    |
//! |                                                                |
//! |  Shared Resources:                                             |
//! |  +------------------------------------------------------------+|
//! |  | Register File: 256 KB (65,536 x 32-bit registers)          ||
//! |  | Shared Memory: 96 KB (configurable split with L1 cache)     ||
//! |  +------------------------------------------------------------+|
//! +---------------------------------------------------------------+
//! ```

use std::collections::HashMap;

use parallel_execution_engine::{EngineTrace, WarpConfig, WarpEngine};
use parallel_execution_engine::protocols::ParallelExecutionEngine;
use fp_arithmetic::FP32;

use crate::protocols::{
    Architecture, ComputeUnit, ComputeUnitTrace, ResourceError,
    SchedulingPolicy, SharedMemory, WarpState, WorkItem,
};

// ---------------------------------------------------------------------------
// SMConfig -- all tunable parameters for an NVIDIA-style SM
// ---------------------------------------------------------------------------

/// Configuration for an NVIDIA-style Streaming Multiprocessor.
///
/// Real-world SM configurations (for reference):
///
/// ```text
/// Parameter             | Volta (V100) | Ampere (A100) | Hopper (H100)
/// ----------------------+--------------+---------------+--------------
/// Warp schedulers       | 4            | 4             | 4
/// Max warps per SM      | 64           | 64            | 64
/// Max threads per SM    | 2048         | 2048          | 2048
/// CUDA cores (FP32)     | 64           | 64            | 128
/// Register file         | 256 KB       | 256 KB        | 256 KB
/// Shared memory         | 96 KB        | 164 KB        | 228 KB
/// ```
///
/// Our default configuration models a Volta-class SM with reduced sizes
/// for faster simulation.
#[derive(Debug, Clone)]
pub struct SMConfig {
    /// Number of warp schedulers (typically 4).
    pub num_schedulers: usize,
    /// Threads per warp (always 32 for NVIDIA).
    pub warp_width: usize,
    /// Maximum resident warps on this SM.
    pub max_warps: usize,
    /// Maximum resident thread blocks.
    pub max_blocks: usize,
    /// How the scheduler picks warps.
    pub scheduling_policy: SchedulingPolicy,
    /// Total 32-bit registers available.
    pub register_file_size: usize,
    /// Max registers a single thread can use.
    pub max_registers_per_thread: usize,
    /// Shared memory in bytes.
    pub shared_memory_size: usize,
    /// L1 cache in bytes.
    pub l1_cache_size: usize,
    /// Instruction cache in bytes.
    pub instruction_cache_size: usize,
    /// Cycles for a global memory access (stall duration).
    pub memory_latency_cycles: usize,
    /// Whether `__syncthreads()` is supported.
    pub barrier_enabled: bool,
}

impl Default for SMConfig {
    fn default() -> Self {
        Self {
            num_schedulers: 4,
            warp_width: 32,
            max_warps: 48,
            max_blocks: 16,
            scheduling_policy: SchedulingPolicy::Gto,
            register_file_size: 65536,
            max_registers_per_thread: 255,
            shared_memory_size: 98304,
            l1_cache_size: 32768,
            instruction_cache_size: 131072,
            memory_latency_cycles: 200,
            barrier_enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// WarpSlot -- tracks one warp's state in the scheduler
// ---------------------------------------------------------------------------

/// One slot in the warp scheduler's table.
///
/// Each WarpSlot tracks the state of one warp -- whether it's ready to
/// execute, stalled waiting for memory, completed, etc.
///
/// # Warp Lifecycle
///
/// ```text
/// 1. dispatch() creates a WarpSlot in READY state
/// 2. Scheduler picks it -> RUNNING
/// 3. After execution:
///    - If LOAD/STORE: transition to STALLED_MEMORY for N cycles
///    - If HALT: transition to COMPLETED
///    - Otherwise: back to READY
/// 4. After stall countdown expires: back to READY
/// ```
pub struct WarpSlot {
    /// Unique identifier for this warp.
    pub warp_id: usize,
    /// Which WorkItem this warp belongs to.
    pub work_id: usize,
    /// Current state.
    pub state: WarpState,
    /// The WarpEngine executing this warp's threads.
    pub engine: WarpEngine,
    /// Cycles remaining until stall resolves (0 = not stalled).
    pub stall_counter: usize,
    /// How many cycles since this warp was last issued.
    pub age: usize,
    /// How many registers this warp occupies.
    pub registers_used: usize,
}

// ---------------------------------------------------------------------------
// WarpScheduler -- picks which warp to issue each cycle
// ---------------------------------------------------------------------------

/// Warp scheduler that implements multiple scheduling policies.
///
/// On each clock cycle, the scheduler:
/// 1. Scans all warp slots assigned to it
/// 2. Decrements stall counters for stalled warps
/// 3. Transitions warps whose stalls have resolved to READY
/// 4. Picks one READY warp according to the scheduling policy
/// 5. Returns that warp for execution
///
/// # Scheduling Policies
///
/// **ROUND_ROBIN**: Simply rotates through warps. Fair but doesn't optimize
/// for locality.
///
/// **GTO (Greedy-Then-Oldest)**: Keeps issuing from the same warp until it
/// stalls, then picks the oldest ready warp. This improves cache locality.
///
/// ```text
/// Cycle 1: Issue warp 3 (GTO stays with warp 3)
/// Cycle 2: Issue warp 3 (still ready, keep going)
/// Cycle 3: Warp 3 stalls (memory access)
/// Cycle 4: Switch to warp 7 (oldest ready warp)
/// ```
pub struct WarpScheduler {
    pub scheduler_id: usize,
    pub policy: SchedulingPolicy,
    /// Indices into the SM's warp_slots vector.
    warp_indices: Vec<usize>,
    rr_index: usize,
    last_issued: Option<usize>,
}

impl WarpScheduler {
    /// Create a new warp scheduler.
    pub fn new(scheduler_id: usize, policy: SchedulingPolicy) -> Self {
        Self {
            scheduler_id,
            policy,
            warp_indices: Vec::new(),
            rr_index: 0,
            last_issued: None,
        }
    }

    /// Add a warp (by its index into the SM's slot vec) to this scheduler.
    pub fn add_warp(&mut self, slot_index: usize) {
        self.warp_indices.push(slot_index);
    }

    /// Decrement stall counters and transition resolved warps to READY.
    pub fn tick_stalls(&self, slots: &mut [WarpSlot]) {
        for &idx in &self.warp_indices {
            let slot = &mut slots[idx];
            if slot.stall_counter > 0 {
                slot.stall_counter -= 1;
                if slot.stall_counter == 0
                    && matches!(
                        slot.state,
                        WarpState::StalledMemory | WarpState::StalledDependency
                    )
                {
                    slot.state = WarpState::Ready;
                }
            }
            if !matches!(slot.state, WarpState::Completed | WarpState::Running) {
                slot.age += 1;
            }
        }
    }

    /// Pick a ready warp according to the scheduling policy.
    /// Returns the slot index, or None if no warp is ready.
    pub fn pick_warp(&mut self, slots: &[WarpSlot]) -> Option<usize> {
        let ready: Vec<usize> = self
            .warp_indices
            .iter()
            .copied()
            .filter(|&idx| slots[idx].state == WarpState::Ready)
            .collect();

        if ready.is_empty() {
            return None;
        }

        match self.policy {
            SchedulingPolicy::RoundRobin | SchedulingPolicy::Lrr => {
                self.pick_round_robin(&ready, slots)
            }
            SchedulingPolicy::Gto => self.pick_gto(&ready, slots),
            SchedulingPolicy::OldestFirst | SchedulingPolicy::Greedy => {
                self.pick_oldest_first(&ready, slots)
            }
        }
    }

    fn pick_round_robin(&mut self, ready: &[usize], slots: &[WarpSlot]) -> Option<usize> {
        // Find the next ready warp starting from the current rr_index
        let all_ids: Vec<usize> = self
            .warp_indices
            .iter()
            .map(|&idx| slots[idx].warp_id)
            .collect();
        for i in 0..all_ids.len() {
            let idx_pos = (self.rr_index + i) % all_ids.len();
            let target_id = all_ids[idx_pos];
            for &slot_idx in ready {
                if slots[slot_idx].warp_id == target_id {
                    self.rr_index = (idx_pos + 1) % all_ids.len();
                    return Some(slot_idx);
                }
            }
        }
        Some(ready[0])
    }

    fn pick_gto(&mut self, ready: &[usize], slots: &[WarpSlot]) -> Option<usize> {
        // Try to continue with the last issued warp
        if let Some(last_warp_id) = self.last_issued {
            for &slot_idx in ready {
                if slots[slot_idx].warp_id == last_warp_id {
                    return Some(slot_idx);
                }
            }
        }
        // Last issued is not ready -- pick the oldest
        self.pick_oldest_first(ready, slots)
    }

    fn pick_oldest_first(&self, ready: &[usize], slots: &[WarpSlot]) -> Option<usize> {
        ready
            .iter()
            .copied()
            .max_by_key(|&idx| slots[idx].age)
    }

    /// Record that a warp was just issued (for GTO policy).
    pub fn mark_issued(&mut self, warp_id: usize, slots: &mut [WarpSlot]) {
        self.last_issued = Some(warp_id);
        for &idx in &self.warp_indices {
            if slots[idx].warp_id == warp_id {
                slots[idx].age = 0;
                break;
            }
        }
    }

    /// Clear all warps from this scheduler.
    pub fn reset(&mut self) {
        self.warp_indices.clear();
        self.rr_index = 0;
        self.last_issued = None;
    }
}

// ---------------------------------------------------------------------------
// StreamingMultiprocessor -- the main SM simulator
// ---------------------------------------------------------------------------

/// NVIDIA Streaming Multiprocessor simulator.
///
/// Manages multiple warps executing thread blocks, with a configurable
/// warp scheduler, shared memory, and register file partitioning.
///
/// # Usage Pattern
///
/// 1. Create SM with config
/// 2. Dispatch one or more WorkItems (thread blocks)
/// 3. Call `step()` or `run()` to simulate execution
/// 4. Read traces to understand what happened
///
/// # How dispatch() Works
///
/// When a thread block is dispatched to the SM:
///
/// 1. Check resources: enough registers? shared memory? warp slots?
/// 2. Decompose the block into warps (every 32 threads = 1 warp)
/// 3. Allocate registers for each warp
/// 4. Reserve shared memory for the block
/// 5. Create WarpEngine instances for each warp
/// 6. Add warp slots to the schedulers (round-robin distribution)
///
/// # How step() Works
///
/// On each clock cycle:
///
/// 1. Tick stall counters (memory latency countdown)
/// 2. Each scheduler picks one ready warp (using scheduling policy)
/// 3. Execute picked warps on their WarpEngines
/// 4. Check for memory instructions -> stall the warp
/// 5. Check for HALT -> mark warp as completed
/// 6. Build and return a ComputeUnitTrace
pub struct StreamingMultiprocessor {
    config: SMConfig,
    cycle: u64,
    shared_memory: SharedMemory,
    shared_memory_used: usize,
    registers_allocated: usize,
    schedulers: Vec<WarpScheduler>,
    warp_slots: Vec<WarpSlot>,
    next_warp_id: usize,
    active_blocks: Vec<usize>,
}

impl StreamingMultiprocessor {
    /// Create a new Streaming Multiprocessor with the given configuration.
    pub fn new(config: SMConfig) -> Self {
        let schedulers = (0..config.num_schedulers)
            .map(|i| WarpScheduler::new(i, config.scheduling_policy))
            .collect();
        let shared_memory = SharedMemory::with_size(config.shared_memory_size);
        Self {
            config,
            cycle: 0,
            shared_memory,
            shared_memory_used: 0,
            registers_allocated: 0,
            schedulers,
            warp_slots: Vec::new(),
            next_warp_id: 0,
            active_blocks: Vec::new(),
        }
    }

    /// Current occupancy: active (non-completed) warps / max warps.
    ///
    /// Occupancy is the key performance metric for GPU kernels. Low
    /// occupancy means the SM can't hide memory latency because there
    /// aren't enough warps to switch between when one stalls.
    pub fn occupancy(&self) -> f64 {
        if self.config.max_warps == 0 {
            return 0.0;
        }
        let active = self
            .warp_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();
        active as f64 / self.config.max_warps as f64
    }

    /// Access to the SM configuration.
    pub fn config(&self) -> &SMConfig {
        &self.config
    }

    /// Access to the shared memory instance.
    pub fn shared_memory(&self) -> &SharedMemory {
        &self.shared_memory
    }

    /// Mutable access to shared memory.
    pub fn shared_memory_mut(&mut self) -> &mut SharedMemory {
        &mut self.shared_memory
    }

    /// All warp slots (for inspection).
    pub fn warp_slots(&self) -> &[WarpSlot] {
        &self.warp_slots
    }

    /// Calculate theoretical occupancy for a kernel launch configuration.
    ///
    /// This is the STATIC occupancy calculation -- how full the SM could
    /// theoretically be, given the resource requirements of a kernel.
    ///
    /// Occupancy is limited by the tightest constraint among:
    ///
    /// 1. **Register pressure**: Each warp needs `registers_per_thread * 32` registers.
    /// 2. **Shared memory**: Each block needs `shared_mem_per_block` bytes.
    /// 3. **Hardware limit**: The SM simply can't hold more than `max_warps` warps.
    pub fn compute_occupancy(
        &self,
        registers_per_thread: usize,
        shared_mem_per_block: usize,
        threads_per_block: usize,
    ) -> f64 {
        let warp_w = self.config.warp_width;
        let warps_per_block = (threads_per_block + warp_w - 1) / warp_w;

        // Limit 1: register file
        let regs_per_warp = registers_per_thread * self.config.warp_width;
        let max_warps_by_regs = if regs_per_warp > 0 {
            self.config.register_file_size / regs_per_warp
        } else {
            self.config.max_warps
        };

        // Limit 2: shared memory
        let max_warps_by_smem = if shared_mem_per_block > 0 {
            let max_blocks_by_smem = self.config.shared_memory_size / shared_mem_per_block;
            max_blocks_by_smem * warps_per_block
        } else {
            self.config.max_warps
        };

        // Limit 3: hardware limit
        let max_warps_by_hw = self.config.max_warps;

        let active_warps = max_warps_by_regs.min(max_warps_by_smem).min(max_warps_by_hw);
        (active_warps as f64 / self.config.max_warps as f64).min(1.0)
    }

    /// Check if an engine trace indicates a memory instruction.
    fn is_memory_instruction(trace: &EngineTrace) -> bool {
        let desc = trace.description.to_uppercase();
        desc.contains("LOAD") || desc.contains("STORE")
    }
}

impl ComputeUnit for StreamingMultiprocessor {
    fn name(&self) -> &str {
        "SM"
    }

    fn architecture(&self) -> Architecture {
        Architecture::NvidiaSm
    }

    fn idle(&self) -> bool {
        self.warp_slots.is_empty()
            || self
                .warp_slots
                .iter()
                .all(|w| w.state == WarpState::Completed)
    }

    fn dispatch(&mut self, work: WorkItem) -> Result<(), ResourceError> {
        // Calculate resource requirements.
        let num_warps = (work.thread_count + self.config.warp_width - 1) / self.config.warp_width;
        let regs_needed = work.registers_per_thread * self.config.warp_width * num_warps;
        let smem_needed = work.shared_mem_bytes;

        // Check resource availability.
        let current_active = self
            .warp_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();

        if current_active + num_warps > self.config.max_warps {
            return Err(ResourceError {
                message: format!(
                    "Not enough warp slots: need {}, available {}",
                    num_warps,
                    self.config.max_warps - current_active,
                ),
            });
        }

        if self.registers_allocated + regs_needed > self.config.register_file_size {
            return Err(ResourceError {
                message: format!(
                    "Not enough registers: need {}, available {}",
                    regs_needed,
                    self.config.register_file_size - self.registers_allocated,
                ),
            });
        }

        if self.shared_memory_used + smem_needed > self.config.shared_memory_size {
            return Err(ResourceError {
                message: format!(
                    "Not enough shared memory: need {}, available {}",
                    smem_needed,
                    self.config.shared_memory_size - self.shared_memory_used,
                ),
            });
        }

        // Allocate resources.
        self.registers_allocated += regs_needed;
        self.shared_memory_used += smem_needed;
        self.active_blocks.push(work.work_id);

        // Create warps and distribute across schedulers.
        for warp_idx in 0..num_warps {
            let warp_id = self.next_warp_id;
            self.next_warp_id += 1;

            let thread_start = warp_idx * self.config.warp_width;
            let thread_end = (thread_start + self.config.warp_width).min(work.thread_count);
            let actual_threads = thread_end - thread_start;

            // Create a WarpEngine for this warp.
            let mut warp_config = WarpConfig::default();
            warp_config.warp_width = actual_threads;
            warp_config.num_registers = work.registers_per_thread;
            warp_config.float_format = FP32;

            let mut engine = WarpEngine::new(warp_config);

            // Load program if provided.
            if let Some(ref program) = work.program {
                engine.load_program(program.clone());
            }

            // Set per-thread data if provided.
            for t_offset in 0..actual_threads {
                let global_tid = thread_start + t_offset;
                if let Some(regs) = work.per_thread_data.get(&global_tid) {
                    for (&reg, &val) in regs {
                        engine.set_thread_register(t_offset, reg, val);
                    }
                }
            }

            let slot_index = self.warp_slots.len();
            let slot = WarpSlot {
                warp_id,
                work_id: work.work_id,
                state: WarpState::Ready,
                engine,
                stall_counter: 0,
                age: 0,
                registers_used: work.registers_per_thread * actual_threads,
            };
            self.warp_slots.push(slot);

            // Distribute to schedulers round-robin.
            let sched_idx = warp_idx % self.config.num_schedulers;
            self.schedulers[sched_idx].add_warp(slot_index);
        }

        Ok(())
    }

    fn step(&mut self) -> ComputeUnitTrace {
        self.cycle += 1;

        // Phase 1: Tick stall counters on all schedulers.
        for sched in &self.schedulers {
            sched.tick_stalls(&mut self.warp_slots);
        }

        // Phase 2: Each scheduler picks a warp and executes it.
        let mut engine_traces: HashMap<usize, EngineTrace> = HashMap::new();
        let mut scheduler_actions: Vec<String> = Vec::new();

        for sched_idx in 0..self.schedulers.len() {
            let picked_opt = self.schedulers[sched_idx].pick_warp(&self.warp_slots);

            let picked_slot_idx = match picked_opt {
                Some(idx) => idx,
                None => {
                    scheduler_actions.push(format!(
                        "S{}: no ready warp",
                        self.schedulers[sched_idx].scheduler_id,
                    ));
                    continue;
                }
            };

            // Mark as running.
            self.warp_slots[picked_slot_idx].state = WarpState::Running;

            // Execute one cycle on the warp's engine.
            let trace = self.warp_slots[picked_slot_idx].engine.step();
            let warp_id = self.warp_slots[picked_slot_idx].warp_id;
            engine_traces.insert(warp_id, trace.clone());

            // Record the scheduling decision.
            self.schedulers[sched_idx].mark_issued(warp_id, &mut self.warp_slots);
            scheduler_actions.push(format!(
                "S{}: issued warp {}",
                self.schedulers[sched_idx].scheduler_id, warp_id,
            ));

            // Phase 3: Check execution results and update warp state.
            if self.warp_slots[picked_slot_idx].engine.halted() {
                self.warp_slots[picked_slot_idx].state = WarpState::Completed;
            } else if Self::is_memory_instruction(&trace) {
                self.warp_slots[picked_slot_idx].state = WarpState::StalledMemory;
                self.warp_slots[picked_slot_idx].stall_counter =
                    self.config.memory_latency_cycles;
            } else {
                self.warp_slots[picked_slot_idx].state = WarpState::Ready;
            }
        }

        // Build the trace.
        let active_warps = self
            .warp_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();
        let total_warps = self.config.max_warps;

        ComputeUnitTrace {
            cycle: self.cycle,
            unit_name: self.name().to_string(),
            architecture: self.architecture(),
            scheduler_action: scheduler_actions.join("; "),
            active_warps,
            total_warps,
            engine_traces,
            shared_memory_used: self.shared_memory_used,
            shared_memory_total: self.config.shared_memory_size,
            register_file_used: self.registers_allocated,
            register_file_total: self.config.register_file_size,
            occupancy: if total_warps > 0 {
                active_warps as f64 / total_warps as f64
            } else {
                0.0
            },
            l1_hits: 0,
            l1_misses: 0,
        }
    }

    fn run(&mut self, max_cycles: usize) -> Vec<ComputeUnitTrace> {
        let mut traces = Vec::new();
        for _ in 0..max_cycles {
            let trace = self.step();
            traces.push(trace);
            if self.idle() {
                break;
            }
        }
        traces
    }

    fn reset(&mut self) {
        for sched in &mut self.schedulers {
            sched.reset();
        }
        self.warp_slots.clear();
        self.shared_memory.reset();
        self.shared_memory_used = 0;
        self.registers_allocated = 0;
        self.active_blocks.clear();
        self.next_warp_id = 0;
        self.cycle = 0;
    }
}

impl fmt::Display for StreamingMultiprocessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let active = self
            .warp_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();
        write!(
            f,
            "StreamingMultiprocessor(warps={}/{}, occupancy={:.1}%, policy={:?})",
            active,
            self.config.max_warps,
            self.occupancy() * 100.0,
            self.config.scheduling_policy,
        )
    }
}

use std::fmt;
