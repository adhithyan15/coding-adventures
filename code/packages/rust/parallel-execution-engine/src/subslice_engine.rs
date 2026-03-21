//! SubsliceEngine -- Intel Xe hybrid SIMD execution engine.
//!
//! # What is a Subslice?
//!
//! Intel's GPU architecture uses a hierarchical organization that's different
//! from both NVIDIA's SIMT warps and AMD's SIMD wavefronts. The basic unit
//! is the "subslice" (also called "sub-slice" or "dual sub-slice" in newer
//! architectures).
//!
//! A subslice contains:
//! - Multiple Execution Units (EUs), typically 8
//! - Each EU runs multiple hardware threads, typically 7
//! - Each thread processes SIMD8 (8-wide vector) instructions
//!
//! ```text
//! +------------------------------------------------------+
//! |  Subslice                                            |
//! |                                                      |
//! |  +----------------------+  +----------------------+  |
//! |  |  EU 0                |  |  EU 1                |  |
//! |  |  +----------------+  |  |  +----------------+  |  |
//! |  |  | Thread 0: SIMD8|  |  |  | Thread 0: SIMD8|  |  |
//! |  |  | Thread 1: SIMD8|  |  |  | Thread 1: SIMD8|  |  |
//! |  |  | ...            |  |  |  | ...            |  |  |
//! |  |  | Thread 6: SIMD8|  |  |  | Thread 6: SIMD8|  |  |
//! |  |  +----------------+  |  |  +----------------+  |  |
//! |  |  Thread Arbiter      |  |  Thread Arbiter      |  |
//! |  +----------------------+  +----------------------+  |
//! |                                                      |
//! |  ... (EU 2 through EU 7, same structure) ...         |
//! |                                                      |
//! |  Shared Local Memory (SLM): 64 KB                    |
//! |  Instruction Cache                                   |
//! |  Thread Dispatcher                                   |
//! +------------------------------------------------------+
//! ```
//!
//! # Why Multiple Threads Per EU?
//!
//! This is Intel's approach to latency hiding. When one thread is stalled
//! (waiting for memory, for example), the EU's thread arbiter switches to
//! another ready thread. This keeps the SIMD ALU busy even when individual
//! threads are blocked.
//!
//! ```text
//! EU Thread Arbiter timeline:
//!
//! Cycle 1: Thread 0 executes SIMD8 add    <- thread 0 is ready
//! Cycle 2: Thread 0 stalls (cache miss)   <- thread 0 blocked
//! Cycle 3: Thread 3 executes SIMD8 mul    <- switch to thread 3
//! Cycle 4: Thread 3 executes SIMD8 add    <- thread 3 still ready
//! Cycle 5: Thread 0 data arrives          <- thread 0 ready again
//! Cycle 6: Thread 0 executes SIMD8 store
//! ```
//!
//! # Total Parallelism
//!
//! One subslice: 8 EUs x 7 threads x 8 SIMD lanes = 448 operations per cycle.
//! That's a LOT of parallelism from a single subslice.

use std::collections::HashMap;

use gpu_core::{GPUCore, GenericISA, Instruction, ProcessingElement};
use fp_arithmetic::{FloatFormat, FP32};

use crate::protocols::{EngineTrace, ExecutionModel, ParallelExecutionEngine};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for an Intel Xe-style SIMD subslice.
///
/// Real-world reference values:
///
/// ```text
/// Architecture   | EUs/subslice | Threads/EU | SIMD Width | GRF
/// ---------------+--------------+------------+------------+-----
/// Intel Xe-LP    | 16           | 7          | 8          | 128
/// Intel Xe-HPG   | 16           | 8          | 8/16       | 128
/// Intel Xe-HPC   | 16           | 8          | 8/16/32    | 128
/// Our default    | 8            | 7          | 8          | 128
/// ```
#[derive(Debug, Clone)]
pub struct SubsliceConfig {
    /// Number of execution units in the subslice.
    pub num_eus: usize,
    /// Hardware threads per EU (for latency hiding).
    pub threads_per_eu: usize,
    /// SIMD vector width (8 for SIMD8, 16 for SIMD16).
    pub simd_width: usize,
    /// General register file entries per EU.
    pub grf_size: usize,
    /// Shared local memory size in bytes.
    pub slm_size: usize,
    /// FP format for computations.
    pub float_format: FloatFormat,
}

impl Default for SubsliceConfig {
    fn default() -> Self {
        Self {
            num_eus: 8,
            threads_per_eu: 7,
            simd_width: 8,
            grf_size: 128,
            slm_size: 65536,
            float_format: FP32,
        }
    }
}

// ---------------------------------------------------------------------------
// Execution Unit -- manages multiple hardware threads
// ---------------------------------------------------------------------------

/// One Execution Unit (EU) in the subslice.
///
/// Each EU has multiple hardware threads and a thread arbiter that picks
/// one ready thread to execute per cycle. Each thread runs SIMD8
/// instructions, which we simulate with one GPUCore per SIMD lane.
///
/// # Thread Arbitration
///
/// The arbiter's job is to keep the SIMD ALU busy. On each cycle, it:
/// 1. Scans all threads to find which are "ready" (not stalled).
/// 2. Picks one ready thread (round-robin among ready threads).
/// 3. Issues that thread's next SIMD8 instruction.
///
/// This is how Intel hides memory latency -- while one thread waits for
/// data, another thread runs. With 7 threads per EU, the ALU can stay
/// busy even with high-latency memory operations.
pub struct ExecutionUnit {
    pub eu_id: usize,
    config: SubsliceConfig,
    current_thread: usize,
    /// _threads[thread_id] = list of GPUCore (one per SIMD lane)
    threads: Vec<Vec<GPUCore>>,
    thread_active: Vec<bool>,
    program: Vec<Instruction>,
}

impl ExecutionUnit {
    /// Create a new Execution Unit.
    pub fn new(eu_id: usize, config: &SubsliceConfig) -> Self {
        // Each thread has `simd_width` SIMD lanes, each backed by a GPUCore.
        let threads: Vec<Vec<GPUCore>> = (0..config.threads_per_eu)
            .map(|_| {
                (0..config.simd_width)
                    .map(|_| {
                        GPUCore::with_config(
                            Box::new(GenericISA),
                            config.float_format,
                            config.grf_size.min(256),
                            config.slm_size / config.threads_per_eu.max(1),
                        )
                    })
                    .collect()
            })
            .collect();

        let thread_active = vec![false; config.threads_per_eu];

        Self {
            eu_id,
            config: config.clone(),
            current_thread: 0,
            threads,
            thread_active,
            program: Vec::new(),
        }
    }

    /// Access to thread SIMD lanes.
    pub fn threads(&self) -> &Vec<Vec<GPUCore>> {
        &self.threads
    }

    /// Load a program into all threads of this EU.
    pub fn load_program(&mut self, program: Vec<Instruction>) {
        self.program = program.clone();
        for thread_id in 0..self.config.threads_per_eu {
            for lane_core in &mut self.threads[thread_id] {
                lane_core.load_program(program.clone());
            }
            self.thread_active[thread_id] = true;
        }
        self.current_thread = 0;
    }

    /// Set a register value for a specific lane of a specific thread.
    pub fn set_thread_lane_register(
        &mut self,
        thread_id: usize,
        lane: usize,
        reg: usize,
        value: f64,
    ) {
        self.threads[thread_id][lane].registers.write_float(reg, value);
    }

    /// Execute one cycle using the thread arbiter.
    ///
    /// The arbiter selects one ready thread and executes its SIMD8
    /// instruction across all lanes.
    pub fn step(&mut self) -> HashMap<usize, String> {
        let mut traces: HashMap<usize, String> = HashMap::new();

        // Find a ready thread using round-robin
        let thread_id = match self.find_ready_thread() {
            Some(id) => id,
            None => return traces,
        };

        // Execute SIMD8 instruction on all lanes of the selected thread
        let mut lane_descriptions: Vec<String> = Vec::new();
        for lane_core in &mut self.threads[thread_id] {
            if !lane_core.halted() {
                match lane_core.step() {
                    Ok(trace) => {
                        lane_descriptions.push(trace.description);
                    }
                    Err(_) => {
                        lane_descriptions.push("(error)".to_string());
                    }
                }
            }
        }

        // Check if all lanes of this thread are halted
        if self.threads[thread_id].iter().all(|c| c.halted()) {
            self.thread_active[thread_id] = false;
        }

        if !lane_descriptions.is_empty() {
            traces.insert(
                thread_id,
                format!(
                    "Thread {}: SIMD{} -- {}",
                    thread_id, self.config.simd_width, lane_descriptions[0]
                ),
            );
        }

        traces
    }

    /// Find the next ready thread using round-robin arbitration.
    ///
    /// Scans threads starting from the last-executed thread + 1,
    /// wrapping around. Returns the first thread that is active and
    /// has non-halted lanes.
    fn find_ready_thread(&mut self) -> Option<usize> {
        for offset in 0..self.config.threads_per_eu {
            let tid = (self.current_thread + offset) % self.config.threads_per_eu;
            if self.thread_active[tid]
                && self.threads[tid].iter().any(|c| !c.halted())
            {
                self.current_thread = (tid + 1) % self.config.threads_per_eu;
                return Some(tid);
            }
        }
        None
    }

    /// True if all threads on this EU are done.
    pub fn all_halted(&self) -> bool {
        !self.thread_active.iter().any(|&a| a)
    }

    /// Reset all threads on this EU.
    pub fn reset(&mut self) {
        for thread_id in 0..self.config.threads_per_eu {
            for lane_core in &mut self.threads[thread_id] {
                lane_core.reset();
                if !self.program.is_empty() {
                    lane_core.load_program(self.program.clone());
                }
            }
            self.thread_active[thread_id] = !self.program.is_empty();
        }
        self.current_thread = 0;
    }
}

// ---------------------------------------------------------------------------
// SubsliceEngine -- the hybrid SIMD execution engine
// ---------------------------------------------------------------------------

/// Intel Xe-style subslice execution engine.
///
/// Manages multiple EUs, each with multiple hardware threads, each
/// processing SIMD8 vectors. The thread arbiter in each EU selects
/// one ready thread per cycle.
///
/// # Parallelism Hierarchy
///
/// ```text
/// Subslice (this engine)
/// +-- EU 0
/// |   +-- Thread 0: SIMD8 [lane0, lane1, ..., lane7]
/// |   +-- Thread 1: SIMD8 [lane0, lane1, ..., lane7]
/// |   +-- ... (threads_per_eu threads)
/// +-- EU 1
/// |   +-- Thread 0: SIMD8
/// |   +-- ...
/// +-- ... (num_eus EUs)
/// ```
///
/// Total parallelism = num_eus * threads_per_eu * simd_width
///
/// # Example
///
/// ```
/// use gpu_core::opcodes::{limm, fmul, halt};
/// use parallel_execution_engine::subslice_engine::{SubsliceEngine, SubsliceConfig};
///
/// let mut config = SubsliceConfig::default();
/// config.num_eus = 2;
/// config.threads_per_eu = 2;
/// config.simd_width = 4;
/// let mut engine = SubsliceEngine::new(config);
/// engine.load_program(vec![limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]);
/// let traces = engine.run(10000).unwrap();
/// ```
pub struct SubsliceEngine {
    config: SubsliceConfig,
    cycle: u64,
    program: Vec<Instruction>,
    /// The Execution Units.
    eus: Vec<ExecutionUnit>,
    all_halted: bool,
}

impl SubsliceEngine {
    /// Create a new SubsliceEngine with the given configuration.
    pub fn new(config: SubsliceConfig) -> Self {
        let eus: Vec<ExecutionUnit> = (0..config.num_eus)
            .map(|i| ExecutionUnit::new(i, &config))
            .collect();

        Self {
            config,
            cycle: 0,
            program: Vec::new(),
            eus,
            all_halted: false,
        }
    }

    /// The configuration this engine was created with.
    pub fn config(&self) -> &SubsliceConfig {
        &self.config
    }

    /// Access to the execution units.
    pub fn eus(&self) -> &[ExecutionUnit] {
        &self.eus
    }

    /// Load a program into all EUs and all threads.
    ///
    /// Every thread on every EU gets the same program.
    pub fn load_program(&mut self, program: Vec<Instruction>) {
        self.program = program.clone();
        for eu in &mut self.eus {
            eu.load_program(program.clone());
        }
        self.all_halted = false;
        self.cycle = 0;
    }

    /// Set a register for a specific lane of a specific thread on a specific EU.
    pub fn set_eu_thread_lane_register(
        &mut self,
        eu_id: usize,
        thread_id: usize,
        lane: usize,
        reg: usize,
        value: f64,
    ) {
        self.eus[eu_id].set_thread_lane_register(thread_id, lane, reg, value);
    }

    /// Run until all EUs are done or max_cycles reached.
    pub fn run(&mut self, max_cycles: usize) -> Result<Vec<EngineTrace>, String> {
        let mut traces = Vec::new();
        for _ in 0..max_cycles {
            let trace = self.step();
            traces.push(trace);
            if self.all_halted {
                break;
            }
        }
        if !self.all_halted {
            return Err(format!(
                "SubsliceEngine: max_cycles ({}) reached",
                max_cycles
            ));
        }
        Ok(traces)
    }

    /// Produce a trace for when all EUs are halted.
    fn make_halted_trace(&self) -> EngineTrace {
        let total = self.width();
        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simd,
            description: "All EUs halted".to_string(),
            unit_traces: HashMap::new(),
            active_mask: vec![false; total],
            active_count: 0,
            total_count: total,
            utilization: 0.0,
            divergence_info: None,
            dataflow_info: None,
        }
    }
}

impl ParallelExecutionEngine for SubsliceEngine {
    fn name(&self) -> &str {
        "SubsliceEngine"
    }

    fn width(&self) -> usize {
        self.config.num_eus * self.config.threads_per_eu * self.config.simd_width
    }

    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::Simd
    }

    /// Execute one cycle: each EU's arbiter picks one thread.
    ///
    /// On each cycle, every EU independently selects one ready thread
    /// and executes its SIMD instruction. This means up to num_eus
    /// threads can execute simultaneously (one per EU).
    fn step(&mut self) -> EngineTrace {
        self.cycle += 1;

        if self.all_halted {
            return self.make_halted_trace();
        }

        let mut all_traces: HashMap<usize, String> = HashMap::new();
        let mut active_count = 0;

        for eu in &mut self.eus {
            if !eu.all_halted() {
                let eu_traces = eu.step();
                for (thread_id, desc) in eu_traces {
                    let flat_id =
                        eu.eu_id * self.config.threads_per_eu + thread_id;
                    all_traces.insert(flat_id, format!("EU{}/{}", eu.eu_id, desc));
                    active_count += self.config.simd_width;
                }
            }
        }

        // Check if all EUs are done
        if self.eus.iter().all(|eu| eu.all_halted()) {
            self.all_halted = true;
        }

        let total = self.width();

        // Build active mask (simplified: active threads * simd_width)
        let mut active_mask = vec![false; total];
        for i in 0..active_count.min(total) {
            active_mask[i] = true;
        }

        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simd,
            description: format!(
                "Subslice step -- {}/{} lanes active across {} EUs",
                active_count, total, self.config.num_eus
            ),
            unit_traces: all_traces,
            active_mask,
            active_count,
            total_count: total,
            utilization: if total > 0 {
                active_count as f64 / total as f64
            } else {
                0.0
            },
            divergence_info: None,
            dataflow_info: None,
        }
    }

    fn halted(&self) -> bool {
        self.all_halted
    }

    /// Reset all EUs to initial state.
    fn reset(&mut self) {
        for eu in &mut self.eus {
            eu.reset();
        }
        self.all_halted = false;
        self.cycle = 0;
    }
}

impl std::fmt::Debug for SubsliceEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let active_eus = self.eus.iter().filter(|eu| !eu.all_halted()).count();
        write!(
            f,
            "SubsliceEngine(eus={}, active_eus={}, halted={})",
            self.config.num_eus, active_eus, self.all_halted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_core::opcodes::{fmul, halt, limm};

    #[test]
    fn test_subslice_creation() {
        let mut config = SubsliceConfig::default();
        config.num_eus = 2;
        config.threads_per_eu = 2;
        config.simd_width = 4;
        let engine = SubsliceEngine::new(config);
        // Total width = 2 * 2 * 4 = 16
        assert_eq!(engine.width(), 16);
        assert_eq!(engine.name(), "SubsliceEngine");
        assert_eq!(engine.execution_model(), ExecutionModel::Simd);
        assert!(!engine.halted());
    }

    #[test]
    fn test_subslice_simple_program() {
        let mut config = SubsliceConfig::default();
        config.num_eus = 2;
        config.threads_per_eu = 2;
        config.simd_width = 4;
        let mut engine = SubsliceEngine::new(config);
        engine.load_program(vec![
            limm(0, 2.0),
            limm(1, 3.0),
            fmul(2, 0, 1),
            halt(),
        ]);

        let traces = engine.run(10000).unwrap();
        assert!(engine.halted());
        assert!(!traces.is_empty());

        // All SIMD lanes should have computed R2 = 6.0
        for eu in engine.eus() {
            for thread in eu.threads() {
                for lane_core in thread {
                    assert_eq!(lane_core.registers.read_float(2), 6.0);
                }
            }
        }
    }

    #[test]
    fn test_subslice_utilization() {
        let mut config = SubsliceConfig::default();
        config.num_eus = 2;
        config.threads_per_eu = 2;
        config.simd_width = 4;
        let mut engine = SubsliceEngine::new(config);
        engine.load_program(vec![limm(0, 1.0), halt()]);

        let trace = engine.step();
        // Each EU selects one thread, so 2 threads * 4 SIMD lanes = 8 active
        // out of total 2*2*4=16
        assert!(trace.active_count > 0);
        assert_eq!(trace.total_count, 16);
    }

    #[test]
    fn test_subslice_reset() {
        let mut config = SubsliceConfig::default();
        config.num_eus = 2;
        config.threads_per_eu = 2;
        config.simd_width = 4;
        let mut engine = SubsliceEngine::new(config);
        engine.load_program(vec![limm(0, 42.0), halt()]);
        engine.run(10000).unwrap();
        assert!(engine.halted());

        engine.reset();
        assert!(!engine.halted());
    }

    #[test]
    fn test_subslice_debug_format() {
        let mut config = SubsliceConfig::default();
        config.num_eus = 2;
        config.threads_per_eu = 2;
        config.simd_width = 4;
        let engine = SubsliceEngine::new(config);
        let debug = format!("{:?}", engine);
        assert!(debug.contains("SubsliceEngine"));
        assert!(debug.contains("eus=2"));
    }

    #[test]
    fn test_eu_thread_arbitration() {
        // Test that the EU arbiter cycles through threads
        let config = SubsliceConfig {
            num_eus: 1,
            threads_per_eu: 3,
            simd_width: 2,
            grf_size: 32,
            slm_size: 4096,
            float_format: FP32,
        };
        let mut eu = ExecutionUnit::new(0, &config);
        eu.load_program(vec![limm(0, 1.0), limm(1, 2.0), halt()]);

        // First step should pick thread 0
        let traces1 = eu.step();
        assert!(traces1.contains_key(&0));

        // Second step should pick thread 1 (round-robin)
        let traces2 = eu.step();
        assert!(traces2.contains_key(&1));

        // Third step should pick thread 2
        let traces3 = eu.step();
        assert!(traces3.contains_key(&2));
    }

    #[test]
    fn test_set_eu_thread_lane_register() {
        let mut config = SubsliceConfig::default();
        config.num_eus = 1;
        config.threads_per_eu = 1;
        config.simd_width = 2;
        let mut engine = SubsliceEngine::new(config);
        engine.load_program(vec![halt()]);

        engine.set_eu_thread_lane_register(0, 0, 0, 0, 42.0);
        assert_eq!(
            engine.eus()[0].threads()[0][0].registers.read_float(0),
            42.0
        );
    }
}
