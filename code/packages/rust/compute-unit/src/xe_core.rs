//! XeCore -- Intel Xe Core simulator.
//!
//! # What is an Xe Core?
//!
//! Intel's Xe Core is a hybrid: it combines SIMD execution units (like AMD)
//! with hardware threads (like NVIDIA), wrapped in a unique organizational
//! structure. It's the building block of Intel's Arc GPUs and Data Center
//! GPUs (Ponte Vecchio, Flex series).
//!
//! # Architecture
//!
//! An Xe Core contains:
//! - **Execution Units (EUs)**: 8-16 per Xe Core, each with its own ALU
//! - **Hardware threads**: 7 threads per EU for latency hiding
//! - **SIMD width**: SIMD8 (or SIMD16/32 on newer architectures)
//! - **SLM (Shared Local Memory)**: 64 KB, similar to NVIDIA's shared memory
//! - **Thread dispatcher**: distributes work to EU threads
//!
//! ```text
//! XeCore
//! +---------------------------------------------------------------+
//! |  Thread Dispatcher                                             |
//! |                                                                |
//! |  +------------------+ +------------------+                     |
//! |  | EU 0             | | EU 1             |                     |
//! |  | Thread 0: SIMD8  | | Thread 0: SIMD8  |                     |
//! |  | Thread 1: SIMD8  | | Thread 1: SIMD8  |                     |
//! |  | ...              | | ...              |                     |
//! |  | Thread 6: SIMD8  | | Thread 6: SIMD8  |                     |
//! |  | Thread Arbiter   | | Thread Arbiter   |                     |
//! |  +------------------+ +------------------+                     |
//! |  ... (EU 2 through EU 15)                                      |
//! |                                                                |
//! |  Shared Local Memory (SLM): 64 KB                              |
//! |  L1 Cache: 192 KB                                              |
//! +---------------------------------------------------------------+
//! ```
//!
//! # How Xe Differs from NVIDIA and AMD
//!
//! ```text
//! NVIDIA SM:  4 schedulers, each manages many warps
//! AMD CU:     4 SIMD units, each runs wavefronts
//! Intel Xe:   8-16 EUs, each has 7 threads, each thread does SIMD8
//! ```
//!
//! The key insight: Intel puts the thread-level parallelism INSIDE each EU
//! (7 threads per EU), while NVIDIA puts it across warps (64 warps per SM)
//! and AMD puts it across wavefronts (40 wavefronts per CU).
//!
//! Total parallelism:
//! ```text
//! NVIDIA SM: 64 warps x 32 threads = 2048 threads
//! AMD CU:    40 wavefronts x 64 lanes = 2560 lanes
//! Intel Xe:  16 EUs x 7 threads x 8 SIMD = 896 lanes
//! ```

use std::collections::HashMap;
use std::fmt;

use fp_arithmetic::FP32;
use parallel_execution_engine::{SubsliceConfig, SubsliceEngine};
use parallel_execution_engine::protocols::ParallelExecutionEngine;

use crate::protocols::{
    Architecture, ComputeUnit, ComputeUnitTrace, ResourceError,
    SchedulingPolicy, SharedMemory, WorkItem,
};

// ---------------------------------------------------------------------------
// XeCoreConfig -- configuration for an Intel Xe Core
// ---------------------------------------------------------------------------

/// Configuration for an Intel Xe Core.
///
/// ```text
/// Parameter           | Xe-LP (iGPU) | Xe-HPG (Arc)  | Xe-HPC
/// --------------------+--------------+---------------+--------
/// EUs per Xe Core     | 16           | 16            | 16
/// Threads per EU      | 7            | 8             | 8
/// SIMD width          | 8            | 8 (or 16)     | 8/16/32
/// GRF per EU          | 128          | 128           | 128
/// SLM size            | 64 KB        | 64 KB         | 128 KB
/// L1 cache            | 192 KB       | 192 KB        | 384 KB
/// ```
#[derive(Debug, Clone)]
pub struct XeCoreConfig {
    /// Execution Units per Xe Core.
    pub num_eus: usize,
    /// Hardware threads per EU.
    pub threads_per_eu: usize,
    /// SIMD vector width.
    pub simd_width: usize,
    /// General Register File entries per EU.
    pub grf_per_eu: usize,
    /// Shared Local Memory size in bytes.
    pub slm_size: usize,
    /// L1 cache in bytes.
    pub l1_cache_size: usize,
    /// Instruction cache in bytes.
    pub instruction_cache_size: usize,
    /// Thread dispatcher scheduling policy.
    pub scheduling_policy: SchedulingPolicy,
    /// Cycles for a global memory access.
    pub memory_latency_cycles: usize,
}

impl Default for XeCoreConfig {
    fn default() -> Self {
        Self {
            num_eus: 16,
            threads_per_eu: 7,
            simd_width: 8,
            grf_per_eu: 128,
            slm_size: 65536,
            l1_cache_size: 196608,
            instruction_cache_size: 65536,
            scheduling_policy: SchedulingPolicy::RoundRobin,
            memory_latency_cycles: 200,
        }
    }
}

// ---------------------------------------------------------------------------
// XeCore -- the main Intel Xe Core simulator
// ---------------------------------------------------------------------------

/// Intel Xe Core simulator.
///
/// Manages Execution Units (EUs) with hardware threads, SLM, and a
/// thread dispatcher that distributes work across EU threads.
///
/// # How Work Distribution Works
///
/// When a work group is dispatched to an Xe Core:
/// 1. The thread dispatcher calculates how many EU threads are needed
/// 2. Each thread gets a portion of the work (SIMD8 of the total)
/// 3. The EU's thread arbiter round-robins among active threads
/// 4. SLM is shared among all threads in the work group
///
/// # Latency Hiding in Xe
///
/// With 7 threads per EU, when one thread stalls on a memory access,
/// the EU arbiter switches to another ready thread on the NEXT cycle
/// (zero-penalty switching, just like NVIDIA warp switching).
pub struct XeCore {
    config: XeCoreConfig,
    cycle: u64,
    slm: SharedMemory,
    engine: SubsliceEngine,
    idle_flag: bool,
    work_items: Vec<WorkItem>,
}

impl XeCore {
    /// Create a new Xe Core with the given configuration.
    pub fn new(config: XeCoreConfig) -> Self {
        let slm = SharedMemory::with_size(config.slm_size);
        let engine = SubsliceEngine::new(SubsliceConfig {
            num_eus: config.num_eus,
            threads_per_eu: config.threads_per_eu,
            simd_width: config.simd_width,
            grf_size: config.grf_per_eu,
            slm_size: config.slm_size,
            float_format: FP32,
        });
        Self {
            config,
            cycle: 0,
            slm,
            engine,
            idle_flag: true,
            work_items: Vec::new(),
        }
    }

    /// Access to the Xe Core configuration.
    pub fn config(&self) -> &XeCoreConfig {
        &self.config
    }

    /// Access to Shared Local Memory.
    pub fn slm(&self) -> &SharedMemory {
        &self.slm
    }

    /// Access to the underlying SubsliceEngine.
    pub fn engine(&self) -> &SubsliceEngine {
        &self.engine
    }
}

impl ComputeUnit for XeCore {
    fn name(&self) -> &str {
        "XeCore"
    }

    fn architecture(&self) -> Architecture {
        Architecture::IntelXeCore
    }

    fn idle(&self) -> bool {
        if self.work_items.is_empty() && self.idle_flag {
            return true;
        }
        self.idle_flag && self.engine.halted()
    }

    fn dispatch(&mut self, work: WorkItem) -> Result<(), ResourceError> {
        self.work_items.push(work.clone());
        self.idle_flag = false;

        if let Some(ref program) = work.program {
            self.engine.load_program(program.clone());
        }

        // Set per-thread data across EUs.
        for (global_tid, regs) in &work.per_thread_data {
            let total_lanes = self.config.simd_width;
            let thread_total = total_lanes * self.config.threads_per_eu;
            let eu_id = global_tid / thread_total;
            let remainder = global_tid % thread_total;
            let thread_id = remainder / total_lanes;
            let lane = remainder % total_lanes;

            if eu_id < self.config.num_eus {
                for (&reg, &val) in regs {
                    self.engine
                        .set_eu_thread_lane_register(eu_id, thread_id, lane, reg, val);
                }
            }
        }

        Ok(())
    }

    fn step(&mut self) -> ComputeUnitTrace {
        self.cycle += 1;

        let engine_trace = self.engine.step();

        if self.engine.halted() {
            self.idle_flag = true;
        }

        let active = engine_trace.active_count;

        ComputeUnitTrace {
            cycle: self.cycle,
            unit_name: self.name().to_string(),
            architecture: self.architecture(),
            scheduler_action: engine_trace.description.clone(),
            active_warps: if active > 0 { 1 } else { 0 },
            total_warps: 1,
            engine_traces: {
                let mut m = HashMap::new();
                m.insert(0, engine_trace);
                m
            },
            shared_memory_used: 0,
            shared_memory_total: self.config.slm_size,
            register_file_used: self.config.grf_per_eu * self.config.num_eus,
            register_file_total: self.config.grf_per_eu * self.config.num_eus,
            occupancy: if active > 0 { 1.0 } else { 0.0 },
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
        self.engine.reset();
        self.slm.reset();
        self.work_items.clear();
        self.idle_flag = true;
        self.cycle = 0;
    }
}

impl fmt::Display for XeCore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "XeCore(eus={}, threads_per_eu={}, idle={})",
            self.config.num_eus, self.config.threads_per_eu, self.idle(),
        )
    }
}
