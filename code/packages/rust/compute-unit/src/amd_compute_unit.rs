//! AMDComputeUnit -- AMD Compute Unit (GCN/RDNA) simulator.
//!
//! # How AMD CUs Differ from NVIDIA SMs
//!
//! While NVIDIA and AMD GPUs look similar from the outside, their internal
//! organization is quite different:
//!
//! ```text
//! NVIDIA SM:                          AMD CU (GCN):
//! ---------                           --------------
//! 4 warp schedulers                   4 SIMD units (16-wide each)
//! Each issues 1 warp (32 threads)     Each runs 1 wavefront (64 lanes)
//! Total: 128 threads/cycle            Total: 64 lanes x 4 = 256 lanes/cycle
//!
//! Register file: unified              Register file: per-SIMD VGPR
//! Shared memory: explicit             LDS: explicit (similar to shared mem)
//! Warp scheduling: hardware           Wavefront scheduling: hardware
//! Scalar unit: per-thread             Scalar unit: SHARED by wavefront
//! ```
//!
//! # The Scalar Unit -- AMD's Key Innovation
//!
//! The scalar unit executes operations that are the SAME across all lanes:
//! - Address computation (base_addr + offset)
//! - Loop counters (i++)
//! - Branch conditions (if i < N)
//! - Constants (pi, epsilon, etc.)
//!
//! Instead of doing this 64 times (once per lane), AMD does it ONCE in the
//! scalar unit and broadcasts the result. This saves power and register space.
//!
//! # Architecture Diagram
//!
//! ```text
//! AMDComputeUnit (GCN-style)
//! +---------------------------------------------------------------+
//! |  Wavefront Scheduler                                           |
//! |  +-----------------------------------------------------------+ |
//! |  | wf0: READY  wf1: STALLED  wf2: READY  wf3: READY ...     | |
//! |  +-----------------------------------------------------------+ |
//! |                                                                |
//! |  +------------------+ +------------------+                     |
//! |  | SIMD Unit 0      | | SIMD Unit 1      |                     |
//! |  | 16-wide ALU      | | 16-wide ALU      |                     |
//! |  | VGPR: 256        | | VGPR: 256        |                     |
//! |  +------------------+ +------------------+                     |
//! |                                                                |
//! |  +------------------+                                          |
//! |  | Scalar Unit      |  <- executes once for all lanes          |
//! |  | SGPR: 104        |  (address computation, flow control)     |
//! |  +------------------+                                          |
//! |                                                                |
//! |  Shared Resources:                                             |
//! |  +------------------------------------------------------------+|
//! |  | LDS (Local Data Share): 64 KB                               ||
//! |  | L1 Vector Cache: 16 KB                                      ||
//! |  +------------------------------------------------------------+|
//! +---------------------------------------------------------------+
//! ```

use std::collections::HashMap;
use std::fmt;

use parallel_execution_engine::{EngineTrace, WavefrontConfig, WavefrontEngine};
use parallel_execution_engine::protocols::ParallelExecutionEngine;
use fp_arithmetic::FP32;

use crate::protocols::{
    Architecture, ComputeUnit, ComputeUnitTrace, ResourceError,
    SchedulingPolicy, SharedMemory, WarpState, WorkItem,
};

// ---------------------------------------------------------------------------
// AMDCUConfig -- configuration for an AMD-style Compute Unit
// ---------------------------------------------------------------------------

/// Configuration for an AMD-style Compute Unit.
///
/// Real-world CU configurations:
///
/// ```text
/// Parameter            | GCN (Vega)   | RDNA2 (RX 6000) | RDNA3
/// ---------------------+--------------+------------------+------
/// SIMD units           | 4            | 2 (per CU)       | 2
/// Wave width           | 64           | 32 (native)      | 32
/// Max wavefronts       | 40           | 32               | 32
/// VGPRs per SIMD       | 256          | 256              | 256
/// SGPRs                | 104          | 104              | 104
/// LDS size             | 64 KB        | 128 KB           | 128 KB
/// ```
#[derive(Debug, Clone)]
pub struct AMDCUConfig {
    /// Number of SIMD units (vector ALUs).
    pub num_simd_units: usize,
    /// Lanes per wavefront (64 for GCN, 32 for RDNA).
    pub wave_width: usize,
    /// Maximum resident wavefronts.
    pub max_wavefronts: usize,
    /// Maximum resident work groups.
    pub max_work_groups: usize,
    /// How the scheduler picks wavefronts.
    pub scheduling_policy: SchedulingPolicy,
    /// Vector GPRs per SIMD unit.
    pub vgpr_per_simd: usize,
    /// Scalar GPRs (shared across all wavefronts).
    pub sgpr_count: usize,
    /// Local Data Share size in bytes.
    pub lds_size: usize,
    /// L1 vector cache in bytes.
    pub l1_vector_cache: usize,
    /// L1 scalar cache in bytes.
    pub l1_scalar_cache: usize,
    /// L1 instruction cache in bytes.
    pub l1_instruction_cache: usize,
    /// Cycles for a global memory access.
    pub memory_latency_cycles: usize,
}

impl Default for AMDCUConfig {
    fn default() -> Self {
        Self {
            num_simd_units: 4,
            wave_width: 64,
            max_wavefronts: 40,
            max_work_groups: 16,
            scheduling_policy: SchedulingPolicy::Lrr,
            vgpr_per_simd: 256,
            sgpr_count: 104,
            lds_size: 65536,
            l1_vector_cache: 16384,
            l1_scalar_cache: 16384,
            l1_instruction_cache: 32768,
            memory_latency_cycles: 200,
        }
    }
}

// ---------------------------------------------------------------------------
// WavefrontSlot -- tracks one wavefront's state
// ---------------------------------------------------------------------------

/// One wavefront in the AMD CU's scheduler.
///
/// Similar to WarpSlot in the NVIDIA SM, but for AMD wavefronts.
pub struct WavefrontSlot {
    /// Unique identifier for this wavefront.
    pub wave_id: usize,
    /// Which WorkItem this wavefront belongs to.
    pub work_id: usize,
    /// Current state.
    pub state: WarpState,
    /// Which SIMD unit this wavefront is assigned to.
    pub simd_unit: usize,
    /// The WavefrontEngine executing this wavefront.
    pub engine: WavefrontEngine,
    /// Cycles remaining until stall resolves.
    pub stall_counter: usize,
    /// Cycles since last issued (for scheduling).
    pub age: usize,
    /// VGPRs allocated for this wavefront.
    pub vgprs_used: usize,
}

// ---------------------------------------------------------------------------
// AMDComputeUnit -- the main CU simulator
// ---------------------------------------------------------------------------

/// AMD Compute Unit (GCN/RDNA) simulator.
///
/// Manages wavefronts across SIMD units, with scalar unit support,
/// LDS (Local Data Share), and wavefront scheduling.
///
/// # Key Differences from StreamingMultiprocessor
///
/// 1. **SIMD units instead of warp schedulers**: Each SIMD unit is a
///    16-wide vector ALU.
/// 2. **Scalar unit**: Operations common to all lanes execute once.
/// 3. **LDS instead of shared memory**: Functionally similar but different
///    banking.
/// 4. **LRR scheduling**: AMD uses Loose Round Robin by default.
pub struct AMDComputeUnit {
    config: AMDCUConfig,
    cycle: u64,
    lds: SharedMemory,
    lds_used: usize,
    wavefront_slots: Vec<WavefrontSlot>,
    next_wave_id: usize,
    vgpr_allocated: Vec<usize>,
}

impl AMDComputeUnit {
    /// Create a new AMD Compute Unit with the given configuration.
    pub fn new(config: AMDCUConfig) -> Self {
        let lds = SharedMemory::with_size(config.lds_size);
        let vgpr_allocated = vec![0; config.num_simd_units];
        Self {
            config,
            cycle: 0,
            lds,
            lds_used: 0,
            wavefront_slots: Vec::new(),
            next_wave_id: 0,
            vgpr_allocated,
        }
    }

    /// Current occupancy: active wavefronts / max wavefronts.
    pub fn occupancy(&self) -> f64 {
        if self.config.max_wavefronts == 0 {
            return 0.0;
        }
        let active = self
            .wavefront_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();
        active as f64 / self.config.max_wavefronts as f64
    }

    /// Access to the CU configuration.
    pub fn config(&self) -> &AMDCUConfig {
        &self.config
    }

    /// Access to the Local Data Share.
    pub fn lds(&self) -> &SharedMemory {
        &self.lds
    }

    /// All wavefront slots (for inspection).
    pub fn wavefront_slots(&self) -> &[WavefrontSlot] {
        &self.wavefront_slots
    }

    /// Check if an engine trace indicates a memory instruction.
    fn is_memory_instruction(trace: &EngineTrace) -> bool {
        let desc = trace.description.to_uppercase();
        desc.contains("LOAD") || desc.contains("STORE")
    }
}

impl ComputeUnit for AMDComputeUnit {
    fn name(&self) -> &str {
        "CU"
    }

    fn architecture(&self) -> Architecture {
        Architecture::AmdCu
    }

    fn idle(&self) -> bool {
        self.wavefront_slots.is_empty()
            || self
                .wavefront_slots
                .iter()
                .all(|w| w.state == WarpState::Completed)
    }

    fn dispatch(&mut self, work: WorkItem) -> Result<(), ResourceError> {
        let num_waves =
            (work.thread_count + self.config.wave_width - 1) / self.config.wave_width;

        let current_active = self
            .wavefront_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();

        if current_active + num_waves > self.config.max_wavefronts {
            return Err(ResourceError {
                message: format!(
                    "Not enough wavefront slots: need {}, available {}",
                    num_waves,
                    self.config.max_wavefronts - current_active,
                ),
            });
        }

        let smem_needed = work.shared_mem_bytes;
        if self.lds_used + smem_needed > self.config.lds_size {
            return Err(ResourceError {
                message: format!(
                    "Not enough LDS: need {}, available {}",
                    smem_needed,
                    self.config.lds_size - self.lds_used,
                ),
            });
        }

        self.lds_used += smem_needed;

        for wave_idx in 0..num_waves {
            let wave_id = self.next_wave_id;
            self.next_wave_id += 1;

            let thread_start = wave_idx * self.config.wave_width;
            let thread_end = (thread_start + self.config.wave_width).min(work.thread_count);
            let actual_lanes = thread_end - thread_start;

            // Assign to a SIMD unit round-robin.
            let simd_unit = wave_idx % self.config.num_simd_units;

            // Create WavefrontEngine.
            let wf_config = WavefrontConfig {
                wave_width: actual_lanes,
                num_vgprs: self.config.vgpr_per_simd.min(256),
                num_sgprs: self.config.sgpr_count,
                lds_size: self.config.lds_size,
                float_format: FP32,
            };

            let mut engine = WavefrontEngine::new(wf_config);

            if let Some(ref program) = work.program {
                engine.load_program(program.clone());
            }

            // Set per-lane data.
            for lane_offset in 0..actual_lanes {
                let global_tid = thread_start + lane_offset;
                if let Some(regs) = work.per_thread_data.get(&global_tid) {
                    for (&reg, &val) in regs {
                        engine.set_lane_register(lane_offset, reg, val);
                    }
                }
            }

            let slot = WavefrontSlot {
                wave_id,
                work_id: work.work_id,
                state: WarpState::Ready,
                simd_unit,
                engine,
                stall_counter: 0,
                age: 0,
                vgprs_used: self.config.vgpr_per_simd.min(256),
            };
            self.wavefront_slots.push(slot);
        }

        Ok(())
    }

    fn step(&mut self) -> ComputeUnitTrace {
        self.cycle += 1;

        // Tick stall counters.
        for slot in &mut self.wavefront_slots {
            if slot.stall_counter > 0 {
                slot.stall_counter -= 1;
                if slot.stall_counter == 0 && slot.state == WarpState::StalledMemory {
                    slot.state = WarpState::Ready;
                }
            }
            if !matches!(slot.state, WarpState::Completed | WarpState::Running) {
                slot.age += 1;
            }
        }

        // Schedule: pick up to num_simd_units wavefronts (one per SIMD unit).
        let mut engine_traces: HashMap<usize, EngineTrace> = HashMap::new();
        let mut scheduler_actions: Vec<String> = Vec::new();

        for simd_id in 0..self.config.num_simd_units {
            // Find ready wavefronts assigned to this SIMD unit.
            let ready_indices: Vec<usize> = self
                .wavefront_slots
                .iter()
                .enumerate()
                .filter(|(_, w)| w.state == WarpState::Ready && w.simd_unit == simd_id)
                .map(|(i, _)| i)
                .collect();

            if ready_indices.is_empty() {
                continue;
            }

            // LRR: pick oldest ready wavefront.
            let picked_idx = *ready_indices
                .iter()
                .max_by_key(|&&i| self.wavefront_slots[i].age)
                .unwrap();

            self.wavefront_slots[picked_idx].state = WarpState::Running;

            let trace = self.wavefront_slots[picked_idx].engine.step();
            let wave_id = self.wavefront_slots[picked_idx].wave_id;
            engine_traces.insert(wave_id, trace.clone());

            scheduler_actions
                .push(format!("SIMD{}: issued wave {}", simd_id, wave_id));
            self.wavefront_slots[picked_idx].age = 0;

            // Update state after execution.
            if self.wavefront_slots[picked_idx].engine.halted() {
                self.wavefront_slots[picked_idx].state = WarpState::Completed;
            } else if Self::is_memory_instruction(&trace) {
                self.wavefront_slots[picked_idx].state = WarpState::StalledMemory;
                self.wavefront_slots[picked_idx].stall_counter =
                    self.config.memory_latency_cycles;
            } else {
                self.wavefront_slots[picked_idx].state = WarpState::Ready;
            }
        }

        if scheduler_actions.is_empty() {
            scheduler_actions.push("all wavefronts stalled or completed".to_string());
        }

        let active_waves = self
            .wavefront_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();

        let total_vgprs = self.config.vgpr_per_simd * self.config.num_simd_units;

        ComputeUnitTrace {
            cycle: self.cycle,
            unit_name: self.name().to_string(),
            architecture: self.architecture(),
            scheduler_action: scheduler_actions.join("; "),
            active_warps: active_waves,
            total_warps: self.config.max_wavefronts,
            engine_traces,
            shared_memory_used: self.lds_used,
            shared_memory_total: self.config.lds_size,
            register_file_used: self.vgpr_allocated.iter().sum(),
            register_file_total: total_vgprs,
            occupancy: if self.config.max_wavefronts > 0 {
                active_waves as f64 / self.config.max_wavefronts as f64
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
        self.wavefront_slots.clear();
        self.lds.reset();
        self.lds_used = 0;
        self.vgpr_allocated = vec![0; self.config.num_simd_units];
        self.next_wave_id = 0;
        self.cycle = 0;
    }
}

impl fmt::Display for AMDComputeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let active = self
            .wavefront_slots
            .iter()
            .filter(|w| w.state != WarpState::Completed)
            .count();
        write!(
            f,
            "AMDComputeUnit(waves={}/{}, occupancy={:.1}%)",
            active,
            self.config.max_wavefronts,
            self.occupancy() * 100.0,
        )
    }
}
