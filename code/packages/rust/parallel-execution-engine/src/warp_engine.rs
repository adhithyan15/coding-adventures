//! WarpEngine -- SIMT parallel execution (NVIDIA CUDA / ARM Mali style).
//!
//! # What is SIMT?
//!
//! SIMT stands for "Single Instruction, Multiple Threads." NVIDIA invented this
//! term to describe how their GPU cores work. It's a hybrid between two older
//! concepts:
//!
//! ```text
//! SISD (one instruction, one datum):
//!     Like a single CPU core. Our gpu-core package at Layer 9.
//!
//! SIMD (one instruction, multiple data):
//!     Like AMD wavefronts. One instruction operates on a wide vector.
//!     There are no "threads" -- just lanes in a vector ALU.
//!
//! SIMT (one instruction, multiple threads):
//!     Like NVIDIA warps. Multiple threads, each with its own registers
//!     and (logically) its own program counter. They USUALLY execute
//!     the same instruction, but CAN diverge.
//! ```
//!
//! The key difference between SIMD and SIMT:
//!
//! ```text
//! SIMD: "I have one wide ALU that processes 32 numbers at once."
//! SIMT: "I have 32 tiny threads that happen to execute in lockstep."
//! ```
//!
//! This distinction matters when threads need to take different paths (branches).
//! In SIMD, you just mask off lanes. In SIMT, the hardware manages a divergence
//! stack to serialize the paths and then reconverge.
//!
//! # How a Warp Works
//!
//! A warp is a group of threads (32 for NVIDIA, 16 for ARM Mali) that the
//! hardware schedules together. On each clock cycle:
//!
//! 1. The warp scheduler picks one instruction (at the warp's PC).
//! 2. That instruction is issued to ALL active threads simultaneously.
//! 3. Each thread executes the instruction on its OWN registers.
//! 4. If the instruction is a branch, threads may diverge.
//!
//! ```text
//! +-----------------------------------------------------+
//! |  Warp (32 threads)                                   |
//! |                                                      |
//! |  Active Mask: [1,1,1,1,1,1,1,1,...,1,1,1,1]          |
//! |  PC: 0x004                                           |
//! |                                                      |
//! |  +------+ +------+ +------+       +------+           |
//! |  | T0   | | T1   | | T2   |  ...  | T31  |           |
//! |  |R0=1.0| |R0=2.0| |R0=3.0|       |R0=32.|           |
//! |  |R1=0.5| |R1=0.5| |R1=0.5|       |R1=0.5|           |
//! |  +------+ +------+ +------+       +------+           |
//! |                                                      |
//! |  Instruction: FMUL R2, R0, R1                        |
//! |  Result: T0.R2=0.5, T1.R2=1.0, T2.R2=1.5, ...       |
//! +-----------------------------------------------------+
//! ```
//!
//! # Divergence: The Price of Flexibility
//!
//! When threads in a warp encounter a branch and disagree on which way to go,
//! the warp "diverges." The hardware serializes the paths:
//!
//! ```text
//! Step 1: Evaluate the branch condition for ALL threads.
//! Step 2: Threads that go "true" -> execute first (others masked off).
//! Step 3: Push (reconvergence_pc, other_mask) onto the divergence stack.
//! Step 4: When "true" path finishes, pop the stack.
//! Step 5: Execute the "false" path (first group masked off).
//! Step 6: At the reconvergence point, all threads are active again.
//! ```
//!
//! Example with 4 threads:
//!
//! ```text
//! if (thread_id < 2):    Mask: [1,1,0,0]  <- threads 0,1 take true path
//!     path_A()           Only threads 0,1 execute
//! else:                  Mask: [0,0,1,1]  <- threads 2,3 take false path
//!     path_B()           Only threads 2,3 execute
//! // reconverge          Mask: [1,1,1,1]  <- all 4 threads active again
//! ```
//!
//! This means divergent branches effectively halve your throughput -- the warp
//! runs both paths sequentially instead of simultaneously.

use std::collections::HashMap;

use gpu_core::{GPUCore, GenericISA, Instruction, ProcessingElement};
use fp_arithmetic::{FloatFormat, FP32};

use crate::protocols::{
    DivergenceInfo, EngineTrace, ExecutionModel, ParallelExecutionEngine,
};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a SIMT warp engine.
///
/// Real-world reference values:
///
/// ```text
/// Vendor      | Warp Width | Registers | Memory     | Max Divergence
/// ------------+------------+-----------+------------+---------------
/// NVIDIA      | 32         | 255       | 512 KB     | 32+ levels
/// ARM Mali    | 16         | 64        | varies     | 16+ levels
/// Our default | 32         | 32        | 1024 B     | 32 levels
/// ```
#[derive(Debug, Clone)]
pub struct WarpConfig {
    /// Number of threads in the warp (32 for NVIDIA, 16 for ARM Mali).
    pub warp_width: usize,
    /// Registers per thread (our generic ISA uses 32).
    pub num_registers: usize,
    /// Local memory per thread in bytes.
    pub memory_per_thread: usize,
    /// FP format for registers (FP32, FP16, BF16).
    pub float_format: FloatFormat,
    /// Maximum nesting of divergent branches.
    pub max_divergence_depth: usize,
}

impl Default for WarpConfig {
    fn default() -> Self {
        Self {
            warp_width: 32,
            num_registers: 32,
            memory_per_thread: 1024,
            float_format: FP32,
            max_divergence_depth: 32,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-thread context
// ---------------------------------------------------------------------------

/// Per-thread execution context in a SIMT warp.
///
/// Each thread in the warp has:
/// - `thread_id`: its position in the warp (0 to warp_width-1)
/// - `core`: a full GPUCore instance with its own registers and memory
/// - `active`: whether this thread is currently executing (false = masked off)
/// - `pc`: per-thread program counter (used in independent scheduling mode)
///
/// In NVIDIA hardware, each CUDA thread has 255 registers. In our simulator,
/// each thread gets a full GPUCore instance, which is heavier but lets us
/// reuse all the existing instruction execution infrastructure.
pub struct ThreadContext {
    pub thread_id: usize,
    pub core: GPUCore,
    pub active: bool,
    pub pc: usize,
}

// ---------------------------------------------------------------------------
// Divergence stack entry
// ---------------------------------------------------------------------------

/// One entry on the divergence stack.
///
/// When threads diverge at a branch, we push an entry recording:
/// - `reconvergence_pc`: where threads should rejoin
/// - `saved_mask`: which threads took the OTHER path (will run later)
///
/// This is the pre-Volta divergence handling mechanism. The stack allows
/// nested divergence -- if threads diverge again while already diverged,
/// another entry is pushed.
///
/// ```text
/// Divergence stack example (4 threads, nested branches):
///
///     Stack (top -> bottom):
///     +--------------------------------------------+
///     | reconvergence_pc=10, saved_mask=[0,0,1,0]  |  <- inner branch
///     +--------------------------------------------+
///     | reconvergence_pc=20, saved_mask=[0,0,0,1]  |  <- outer branch
///     +--------------------------------------------+
/// ```
#[derive(Debug, Clone)]
struct DivergenceStackEntry {
    reconvergence_pc: usize,
    saved_mask: Vec<bool>,
}

// ---------------------------------------------------------------------------
// WarpEngine -- the SIMT parallel execution engine
// ---------------------------------------------------------------------------

/// SIMT warp execution engine (NVIDIA CUDA / ARM Mali style).
///
/// Manages N threads executing in lockstep with hardware divergence support.
/// Each thread is backed by a real GPUCore instance from the gpu-core package.
///
/// # Usage Pattern
///
/// 1. Create engine with config
/// 2. Load program (same program goes to all threads)
/// 3. Set per-thread register values (give each thread different data)
/// 4. Step or run (engine issues instructions to all active threads)
/// 5. Read results from per-thread registers
///
/// # Example
///
/// ```
/// use gpu_core::opcodes::{limm, fmul, halt};
/// use parallel_execution_engine::warp_engine::{WarpEngine, WarpConfig};
///
/// let mut config = WarpConfig::default();
/// config.warp_width = 4;
/// let mut engine = WarpEngine::new(config);
/// engine.load_program(vec![limm(0, 2.0), limm(1, 3.0), fmul(2, 0, 1), halt()]);
/// let traces = engine.run(10000).unwrap();
/// assert_eq!(engine.threads()[0].core.registers.read_float(2), 6.0);
/// ```
pub struct WarpEngine {
    config: WarpConfig,
    cycle: u64,
    program: Vec<Instruction>,
    threads: Vec<ThreadContext>,
    divergence_stack: Vec<DivergenceStackEntry>,
    all_halted: bool,
}

impl WarpEngine {
    /// Create a new WarpEngine with the given configuration.
    pub fn new(config: WarpConfig) -> Self {
        // Create one GPUCore per thread. Each thread is an independent
        // processing element with its own registers and local memory.
        let threads: Vec<ThreadContext> = (0..config.warp_width)
            .map(|i| ThreadContext {
                thread_id: i,
                core: GPUCore::with_config(
                    Box::new(GenericISA),
                    config.float_format,
                    config.num_registers,
                    config.memory_per_thread,
                ),
                active: true,
                pc: 0,
            })
            .collect();

        Self {
            config,
            cycle: 0,
            program: Vec::new(),
            threads,
            divergence_stack: Vec::new(),
            all_halted: false,
        }
    }

    /// Access to per-thread contexts (for reading results).
    pub fn threads(&self) -> &[ThreadContext] {
        &self.threads
    }

    /// Mutable access to per-thread contexts.
    pub fn threads_mut(&mut self) -> &mut Vec<ThreadContext> {
        &mut self.threads
    }

    /// Which threads are currently active (not masked off).
    pub fn active_mask(&self) -> Vec<bool> {
        self.threads.iter().map(|t| t.active).collect()
    }

    /// The configuration this engine was created with.
    pub fn config(&self) -> &WarpConfig {
        &self.config
    }

    /// Load the same program into all threads.
    ///
    /// In real NVIDIA hardware, all threads in a warp share the same
    /// instruction memory. We simulate this by loading the same program
    /// into each thread's GPUCore.
    pub fn load_program(&mut self, program: Vec<Instruction>) {
        self.program = program.clone();
        for thread in &mut self.threads {
            thread.core.load_program(program.clone());
            thread.active = true;
            thread.pc = 0;
        }
        self.all_halted = false;
        self.cycle = 0;
        self.divergence_stack.clear();
    }

    /// Set a register value for a specific thread.
    ///
    /// This is how you give each thread different data to work on.
    /// In a real GPU kernel, each thread would compute its global index
    /// and use it to load different data from memory. In our simulator,
    /// we pre-load the data into registers.
    ///
    /// # Panics
    ///
    /// Panics if `thread_id` is out of range.
    pub fn set_thread_register(&mut self, thread_id: usize, reg: usize, value: f64) {
        assert!(
            thread_id < self.config.warp_width,
            "Thread ID {} out of range [0, {})",
            thread_id,
            self.config.warp_width
        );
        self.threads[thread_id].core.registers.write_float(reg, value);
    }

    /// Run until all threads halt or max_cycles reached.
    ///
    /// Creates clock edges internally to drive execution. Each cycle
    /// produces one EngineTrace.
    ///
    /// # Errors
    ///
    /// Returns an error if max_cycles is exceeded (likely an infinite loop).
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
                "WarpEngine: max_cycles ({}) reached",
                max_cycles
            ));
        }
        Ok(traces)
    }

    // --- Divergence handling (private) ---

    /// Handle a divergent branch by pushing onto the divergence stack.
    ///
    /// When some threads take a branch and others don't:
    /// 1. Find the reconvergence point (the max PC among all active threads).
    /// 2. Push the "not taken" threads onto the stack with the reconvergence PC.
    /// 3. Mask off the "not taken" threads so only "taken" threads execute.
    fn handle_divergence(
        &mut self,
        taken_threads: &[usize],
        not_taken_threads: &[usize],
        mask_before: &[bool],
    ) -> DivergenceInfo {
        // The reconvergence PC is the maximum PC among all active threads
        // after the branch. This is a simplified heuristic -- real hardware
        // uses the immediate post-dominator in the control flow graph.
        let all_pcs: Vec<usize> = taken_threads
            .iter()
            .chain(not_taken_threads.iter())
            .map(|&tid| self.threads[tid].core.pc)
            .collect();
        let reconvergence_pc = *all_pcs.iter().max().unwrap_or(&0);

        // Build the saved mask: threads that took the "not taken" path
        let mut saved_mask = vec![false; self.config.warp_width];
        for &tid in not_taken_threads {
            saved_mask[tid] = true;
            self.threads[tid].active = false;
        }

        // Push onto the divergence stack
        if self.divergence_stack.len() < self.config.max_divergence_depth {
            self.divergence_stack.push(DivergenceStackEntry {
                reconvergence_pc,
                saved_mask,
            });
        }

        let mask_after: Vec<bool> = self.threads.iter().map(|t| t.active).collect();

        DivergenceInfo {
            active_mask_before: mask_before.to_vec(),
            active_mask_after: mask_after,
            reconvergence_pc: reconvergence_pc as i64,
            divergence_depth: self.divergence_stack.len(),
        }
    }

    /// Check if active threads have reached a reconvergence point.
    ///
    /// If the divergence stack is non-empty and all active threads are
    /// at or past the reconvergence PC, pop the stack and reactivate
    /// the saved threads.
    fn check_reconvergence(&mut self) {
        if self.divergence_stack.is_empty() {
            return;
        }

        let entry = self.divergence_stack.last().unwrap();
        let reconvergence_pc = entry.reconvergence_pc;

        let active_threads: Vec<usize> = self
            .threads
            .iter()
            .filter(|t| t.active && !t.core.halted())
            .map(|t| t.thread_id)
            .collect();

        if active_threads.is_empty() {
            return;
        }

        // Check if all active threads have reached the reconvergence PC
        let all_at_reconvergence = active_threads
            .iter()
            .all(|&tid| self.threads[tid].core.pc >= reconvergence_pc);

        if all_at_reconvergence {
            let entry = self.divergence_stack.pop().unwrap();
            // Reactivate the saved threads
            for (tid, &should_activate) in entry.saved_mask.iter().enumerate() {
                if should_activate && !self.threads[tid].core.halted() {
                    self.threads[tid].active = true;
                }
            }
        }
    }

    /// Pop the divergence stack and produce a trace for the switch.
    ///
    /// Called when all currently active threads are halted/masked but
    /// there are still entries on the divergence stack (meaning some
    /// threads are waiting to execute the other branch path).
    fn pop_divergence_and_trace(&mut self) -> EngineTrace {
        let entry = self.divergence_stack.pop().unwrap();

        // Reactivate saved threads
        for (tid, &should_activate) in entry.saved_mask.iter().enumerate() {
            if should_activate && !self.threads[tid].core.halted() {
                self.threads[tid].active = true;
            }
        }

        let current_mask: Vec<bool> = self
            .threads
            .iter()
            .map(|t| t.active && !t.core.halted())
            .collect();
        let active_count = current_mask.iter().filter(|&&m| m).count();

        let mut unit_traces = HashMap::new();
        for t in &self.threads {
            let desc = if entry.saved_mask[t.thread_id] {
                "reactivated".to_string()
            } else {
                "(waiting)".to_string()
            };
            unit_traces.insert(t.thread_id, desc);
        }

        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simt,
            description: format!(
                "Divergence stack pop -- reactivated {} threads",
                active_count
            ),
            unit_traces,
            active_mask: current_mask,
            active_count,
            total_count: self.config.warp_width,
            utilization: if self.config.warp_width > 0 {
                active_count as f64 / self.config.warp_width as f64
            } else {
                0.0
            },
            divergence_info: None,
            dataflow_info: None,
        }
    }

    /// Produce a trace for when all threads are halted.
    fn make_halted_trace(&self) -> EngineTrace {
        let mut unit_traces = HashMap::new();
        for t in &self.threads {
            unit_traces.insert(t.thread_id, "(halted)".to_string());
        }
        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simt,
            description: "All threads halted".to_string(),
            unit_traces,
            active_mask: vec![false; self.config.warp_width],
            active_count: 0,
            total_count: self.config.warp_width,
            utilization: 0.0,
            divergence_info: None,
            dataflow_info: None,
        }
    }
}

impl ParallelExecutionEngine for WarpEngine {
    fn name(&self) -> &str {
        "WarpEngine"
    }

    fn width(&self) -> usize {
        self.config.warp_width
    }

    fn execution_model(&self) -> ExecutionModel {
        ExecutionModel::Simt
    }

    /// Execute one cycle: issue one instruction to all active threads.
    ///
    /// On each rising clock edge:
    /// 1. Find the instruction at the current warp PC.
    /// 2. Issue it to all active (non-masked) threads.
    /// 3. Detect divergence on branch instructions.
    /// 4. Handle reconvergence when appropriate.
    /// 5. Build and return an EngineTrace.
    fn step(&mut self) -> EngineTrace {
        self.cycle += 1;

        // If all halted, produce a no-op trace
        if self.all_halted {
            return self.make_halted_trace();
        }

        // Check for reconvergence
        self.check_reconvergence();

        // Find active, non-halted threads
        let active_thread_ids: Vec<usize> = self
            .threads
            .iter()
            .filter(|t| t.active && !t.core.halted())
            .map(|t| t.thread_id)
            .collect();

        if active_thread_ids.is_empty() {
            // All threads are either halted or masked off.
            // Check if we need to pop the divergence stack.
            if !self.divergence_stack.is_empty() {
                return self.pop_divergence_and_trace();
            }
            self.all_halted = true;
            return self.make_halted_trace();
        }

        // Save pre-step mask for divergence tracking
        let mask_before: Vec<bool> = self.threads.iter().map(|t| t.active).collect();

        // Execute the instruction on all active, non-halted threads
        let mut unit_traces: HashMap<usize, String> = HashMap::new();
        let mut branch_taken_threads: Vec<usize> = Vec::new();
        let mut branch_not_taken_threads: Vec<usize> = Vec::new();

        for i in 0..self.threads.len() {
            let thread = &self.threads[i];
            if thread.active && !thread.core.halted() {
                match self.threads[i].core.step() {
                    Ok(trace) => {
                        let desc = trace.description.clone();

                        // Detect branch divergence
                        if trace.next_pc != trace.pc + 1 && !trace.halted {
                            branch_taken_threads.push(i);
                        } else if !trace.halted {
                            branch_not_taken_threads.push(i);
                        }

                        if trace.halted {
                            unit_traces.insert(i, "HALTED".to_string());
                        } else {
                            unit_traces.insert(i, desc);
                        }
                    }
                    Err(_) => {
                        self.threads[i].active = false;
                        unit_traces.insert(i, "(error -- deactivated)".to_string());
                    }
                }
            } else if thread.core.halted() {
                unit_traces.insert(i, "(halted)".to_string());
            } else {
                unit_traces.insert(i, "(masked off)".to_string());
            }
        }

        // Handle divergence: if some threads branched and others didn't,
        // we have divergence. Push the "not taken" threads onto the
        // divergence stack and continue with only the "taken" threads.
        let divergence_info = if !branch_taken_threads.is_empty()
            && !branch_not_taken_threads.is_empty()
        {
            Some(self.handle_divergence(
                &branch_taken_threads,
                &branch_not_taken_threads,
                &mask_before,
            ))
        } else {
            None
        };

        // Check if all threads are now halted
        if self.threads.iter().all(|t| t.core.halted()) {
            self.all_halted = true;
        }

        // Build the trace
        let current_mask: Vec<bool> = self
            .threads
            .iter()
            .map(|t| t.active && !t.core.halted())
            .collect();
        let active_count = current_mask.iter().filter(|&&m| m).count();
        let total = self.config.warp_width;

        // Get a description from the first active thread's trace
        let first_desc = self
            .threads
            .iter()
            .find_map(|t| {
                unit_traces.get(&t.thread_id).and_then(|desc| {
                    if desc != "(masked off)" && desc != "(halted)" && desc != "(error -- deactivated)" {
                        Some(desc.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| "no active threads".to_string());

        EngineTrace {
            cycle: self.cycle,
            engine_name: self.name().to_string(),
            execution_model: ExecutionModel::Simt,
            description: format!("{} -- {}/{} threads active", first_desc, active_count, total),
            unit_traces,
            active_mask: current_mask,
            active_count,
            total_count: total,
            utilization: if total > 0 {
                active_count as f64 / total as f64
            } else {
                0.0
            },
            divergence_info,
            dataflow_info: None,
        }
    }

    fn halted(&self) -> bool {
        self.all_halted
    }

    /// Reset the engine to its initial state.
    ///
    /// Resets all thread cores, reactivates all threads, clears the
    /// divergence stack, and reloads the program (if one was loaded).
    fn reset(&mut self) {
        for thread in &mut self.threads {
            thread.core.reset();
            thread.active = true;
            thread.pc = 0;
            if !self.program.is_empty() {
                thread.core.load_program(self.program.clone());
            }
        }
        self.divergence_stack.clear();
        self.all_halted = false;
        self.cycle = 0;
    }
}

impl std::fmt::Debug for WarpEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let active = self.threads.iter().filter(|t| t.active).count();
        let halted_threads = self.threads.iter().filter(|t| t.core.halted()).count();
        write!(
            f,
            "WarpEngine(width={}, active={}, halted_threads={}, divergence_depth={})",
            self.config.warp_width,
            active,
            halted_threads,
            self.divergence_stack.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_core::opcodes::{fadd, fmul, halt, limm};

    #[test]
    fn test_warp_engine_creation() {
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let engine = WarpEngine::new(config);
        assert_eq!(engine.width(), 4);
        assert_eq!(engine.name(), "WarpEngine");
        assert_eq!(engine.execution_model(), ExecutionModel::Simt);
        assert!(!engine.halted());
    }

    #[test]
    fn test_warp_engine_simple_program() {
        // Load a program that computes R2 = 2.0 * 3.0 = 6.0
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let mut engine = WarpEngine::new(config);
        engine.load_program(vec![
            limm(0, 2.0),
            limm(1, 3.0),
            fmul(2, 0, 1),
            halt(),
        ]);

        let traces = engine.run(1000).unwrap();
        assert!(engine.halted());
        // All 4 threads should have computed R2 = 6.0
        for t in engine.threads() {
            assert_eq!(t.core.registers.read_float(2), 6.0);
        }
        // 4 instructions = 4 traces + possible halt trace
        assert!(traces.len() >= 4);
    }

    #[test]
    fn test_warp_engine_per_thread_registers() {
        // Give each thread different data, compute R2 = R0 + R1
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let mut engine = WarpEngine::new(config);

        engine.load_program(vec![fadd(2, 0, 1), halt()]);

        for t in 0..4 {
            engine.set_thread_register(t, 0, (t as f64) * 10.0);
            engine.set_thread_register(t, 1, 1.0);
        }

        let _traces = engine.run(1000).unwrap();
        assert!(engine.halted());

        // Thread 0: 0.0 + 1.0 = 1.0
        // Thread 1: 10.0 + 1.0 = 11.0
        // Thread 2: 20.0 + 1.0 = 21.0
        // Thread 3: 30.0 + 1.0 = 31.0
        assert_eq!(engine.threads()[0].core.registers.read_float(2), 1.0);
        assert_eq!(engine.threads()[1].core.registers.read_float(2), 11.0);
        assert_eq!(engine.threads()[2].core.registers.read_float(2), 21.0);
        assert_eq!(engine.threads()[3].core.registers.read_float(2), 31.0);
    }

    #[test]
    fn test_warp_engine_utilization() {
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let mut engine = WarpEngine::new(config);
        engine.load_program(vec![limm(0, 1.0), halt()]);

        let trace = engine.step();
        // All 4 threads should be active for the first instruction
        assert_eq!(trace.active_count, 4);
        assert_eq!(trace.total_count, 4);
        assert!((trace.utilization - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_warp_engine_reset() {
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let mut engine = WarpEngine::new(config);
        engine.load_program(vec![limm(0, 42.0), halt()]);
        engine.run(1000).unwrap();
        assert!(engine.halted());

        engine.reset();
        assert!(!engine.halted());
        // After reset, registers should be cleared
        assert_eq!(engine.threads()[0].core.registers.read_float(0), 0.0);
    }

    #[test]
    fn test_warp_engine_debug_format() {
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let engine = WarpEngine::new(config);
        let debug = format!("{:?}", engine);
        assert!(debug.contains("WarpEngine"));
        assert!(debug.contains("width=4"));
    }

    #[test]
    fn test_warp_engine_trace_format() {
        let mut config = WarpConfig::default();
        config.warp_width = 2;
        let mut engine = WarpEngine::new(config);
        engine.load_program(vec![limm(0, 1.0), halt()]);

        let trace = engine.step();
        let formatted = trace.format();
        assert!(formatted.contains("[Cycle 1]"));
        assert!(formatted.contains("WarpEngine"));
        assert!(formatted.contains("SIMT"));
    }

    #[test]
    #[should_panic(expected = "Thread ID 10 out of range")]
    fn test_warp_engine_set_register_out_of_range() {
        let mut config = WarpConfig::default();
        config.warp_width = 4;
        let mut engine = WarpEngine::new(config);
        engine.set_thread_register(10, 0, 1.0);
    }
}
