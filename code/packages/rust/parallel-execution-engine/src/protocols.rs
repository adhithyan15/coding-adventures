//! Protocols -- the unified interface for all parallel execution engines.
//!
//! # What is a Parallel Execution Engine?
//!
//! At Layer 9 (gpu-core), we built a single processing element -- one tiny
//! compute unit that executes one instruction at a time. Useful for learning,
//! but real accelerators never run just ONE core. They run THOUSANDS in parallel.
//!
//! Layer 8 is where parallelism happens. It takes many Layer 9 cores (or
//! simpler processing elements) and orchestrates them to execute together.
//! But HOW they're orchestrated differs fundamentally across architectures:
//!
//! ```text
//! NVIDIA GPU:   32 threads in a "warp" -- each has its own registers,
//!               but they execute the same instruction (SIMT).
//!
//! AMD GPU:      32/64 "lanes" in a "wavefront" -- one instruction stream,
//!               one wide vector ALU, explicit execution mask (SIMD).
//!
//! Google TPU:   NxN grid of multiply-accumulate units -- data FLOWS
//!               through the array, no instructions at all (Systolic).
//!
//! Apple NPU:    Array of MACs driven by a compiler-generated schedule --
//!               no runtime scheduler, just a fixed plan (Scheduled MAC).
//!
//! Intel GPU:    SIMD8 execution units with multiple hardware threads --
//!               a hybrid of SIMD and multi-threading (Subslice).
//! ```
//!
//! Despite these radical differences, ALL of them share a common interface:
//! "advance one clock cycle, tell me what happened, report utilization."
//! That common interface is the [`ParallelExecutionEngine`] trait.
//!
//! # Flynn's Taxonomy -- A Quick Refresher
//!
//! In 1966, Michael Flynn classified computer architectures:
//!
//! ```text
//! +-------------------+-----------------+---------------------+
//! |                   | Single Data     | Multiple Data        |
//! +-------------------+-----------------+---------------------+
//! | Single Instr.     | SISD (old CPU)  | SIMD (vector proc.) |
//! | Multiple Instr.   | MISD (rare)     | MIMD (multi-core)   |
//! +-------------------+-----------------+---------------------+
//! ```
//!
//! Modern accelerators don't fit neatly into these boxes:
//! - NVIDIA coined "SIMT" because warps are neither pure SIMD nor pure MIMD.
//! - Systolic arrays don't have "instructions" at all.
//! - NPU scheduled arrays are driven by static compiler schedules.
//!
//! Our [`ExecutionModel`] enum captures these real-world execution models.

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// ExecutionModel -- the five parallel execution paradigms
// ---------------------------------------------------------------------------

/// The five parallel execution models supported by this package.
///
/// Each model represents a fundamentally different way to organize parallel
/// computation. They are NOT interchangeable -- each has different properties
/// around divergence, synchronization, and data movement.
///
/// Think of these as "architectural philosophies":
///
/// ```text
/// SIMT:          "Give every thread its own identity, execute together"
/// SIMD:          "One instruction, wide ALU, explicit masking"
/// Systolic:      "Data flows through a grid -- no instructions needed"
/// ScheduledMac:  "Compiler decides everything -- hardware just executes"
/// VLIW:          "Pack multiple ops into one wide instruction word"
/// ```
///
/// Comparison table:
///
/// ```text
/// Model          | Has PC? | Has threads? | Divergence?     | Used by
/// ---------------+---------+--------------+-----------------+----------
/// SIMT           | Yes*    | Yes          | HW-managed      | NVIDIA
/// SIMD           | Yes     | No (lanes)   | Explicit mask   | AMD
/// Systolic       | No      | No           | N/A             | Google TPU
/// ScheduledMac   | No      | No           | Compile-time    | Apple NPU
/// VLIW           | Yes     | No           | Predicated      | Qualcomm
///
/// * SIMT: each thread logically has its own PC, but they usually share one.
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionModel {
    /// NVIDIA CUDA / ARM Mali style: threads with independent PCs in lockstep.
    Simt,
    /// AMD GCN/RDNA / Intel Xe style: one instruction over wide vector lanes.
    Simd,
    /// Google TPU style: data flows through a grid, no instructions at all.
    Systolic,
    /// Apple ANE / Qualcomm Hexagon style: compiler-scheduled MAC arrays.
    ScheduledMac,
    /// Qualcomm Hexagon / TI C6x style: multiple ops in one wide instruction word.
    Vliw,
}

impl ExecutionModel {
    /// Return the human-readable name of this execution model.
    ///
    /// Matches the Python implementation's string values for compatibility.
    pub fn name(&self) -> &'static str {
        match self {
            ExecutionModel::Simt => "SIMT",
            ExecutionModel::Simd => "SIMD",
            ExecutionModel::Systolic => "SYSTOLIC",
            ExecutionModel::ScheduledMac => "SCHEDULED_MAC",
            ExecutionModel::Vliw => "VLIW",
        }
    }
}

impl fmt::Display for ExecutionModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ---------------------------------------------------------------------------
// DivergenceInfo -- tracking branch divergence (SIMT/SIMD only)
// ---------------------------------------------------------------------------

/// Information about branch divergence during one execution step.
///
/// # What is Divergence?
///
/// When a group of threads/lanes encounters a branch (if/else), some may
/// take the "true" path and others the "false" path. This is called
/// "divergence" -- the threads are no longer executing in lockstep.
///
/// ```text
/// Before branch:    All 8 threads active: [1, 1, 1, 1, 1, 1, 1, 1]
/// Branch condition:  thread_id < 4?
/// After branch:     Only 4 active:        [1, 1, 1, 1, 0, 0, 0, 0]
///                   The other 4 will run later.
/// ```
///
/// Divergence is the enemy of GPU performance. When half the threads are
/// masked off, you're wasting half your hardware. Real GPU code tries to
/// minimize divergence by ensuring threads in the same warp/wavefront
/// take the same path.
///
/// # Fields
///
/// - `active_mask_before`: Which units were active BEFORE the branch.
/// - `active_mask_after`: Which units are active AFTER the branch.
/// - `reconvergence_pc`: The instruction address where all units rejoin.
///    -1 if not applicable (e.g., SIMD explicit mask).
/// - `divergence_depth`: How many nested divergent branches we're inside.
///    0 means no divergence. Higher = more serialization.
#[derive(Debug, Clone)]
pub struct DivergenceInfo {
    pub active_mask_before: Vec<bool>,
    pub active_mask_after: Vec<bool>,
    pub reconvergence_pc: i64,
    pub divergence_depth: usize,
}

// ---------------------------------------------------------------------------
// DataflowInfo -- tracking data movement (Systolic only)
// ---------------------------------------------------------------------------

/// Information about data flow in a systolic array.
///
/// # What is Dataflow Execution?
///
/// In a systolic array, there are no instructions. Instead, data "flows"
/// through a grid of processing elements, like water flowing through pipes.
/// Each PE does a multiply-accumulate and passes data to its neighbor.
///
/// This struct tracks the state of every PE in the grid so we can
/// visualize how data pulses through the array cycle by cycle.
///
/// # Fields
///
/// - `pe_states`: 2D grid of PE state descriptions.
///   `pe_states[row][col]` = "acc=3.14, in=2.0"
/// - `data_positions`: Where each input value currently is in the array.
///   Maps input_id to (row, col) position.
#[derive(Debug, Clone)]
pub struct DataflowInfo {
    pub pe_states: Vec<Vec<String>>,
    pub data_positions: HashMap<String, (usize, usize)>,
}

// ---------------------------------------------------------------------------
// EngineTrace -- the unified trace record for all engines
// ---------------------------------------------------------------------------

/// Record of one parallel execution step across ALL parallel units.
///
/// # Why a Unified Trace?
///
/// Every engine -- warp, wavefront, systolic, MAC array -- produces one
/// `EngineTrace` per clock cycle. This lets higher layers (and tests, and
/// visualization tools) treat all engines uniformly.
///
/// The trace captures:
/// 1. WHAT happened (description, per-unit details)
/// 2. WHO was active (active_mask, utilization)
/// 3. HOW efficient it was (active_count / total_count)
/// 4. Engine-specific details (divergence for SIMT, dataflow for systolic)
///
/// # Example trace from a 4-thread warp
///
/// ```text
/// EngineTrace {
///     cycle: 3,
///     engine_name: "WarpEngine",
///     execution_model: ExecutionModel::Simt,
///     description: "FADD R2, R0, R1 -- 3/4 threads active",
///     unit_traces: {0: "R2 = 1.0 + 2.0 = 3.0", 1: "R2 = 3.0 + 4.0 = 7.0", ...},
///     active_mask: [true, true, false, true],
///     active_count: 3,
///     total_count: 4,
///     utilization: 0.75,
/// }
/// ```
#[derive(Debug, Clone)]
pub struct EngineTrace {
    /// Clock cycle number.
    pub cycle: u64,
    /// Which engine produced this trace.
    pub engine_name: String,
    /// The parallel execution model (SIMT, SIMD, etc.).
    pub execution_model: ExecutionModel,
    /// Human-readable summary of what happened.
    pub description: String,
    /// Per-unit descriptions (thread/lane/PE/MAC index -> str).
    pub unit_traces: HashMap<usize, String>,
    /// Which units were active this cycle.
    pub active_mask: Vec<bool>,
    /// How many units did useful work.
    pub active_count: usize,
    /// Total units available.
    pub total_count: usize,
    /// active_count / total_count (0.0 to 1.0).
    pub utilization: f64,
    /// Branch divergence details (SIMT/SIMD only).
    pub divergence_info: Option<DivergenceInfo>,
    /// Data flow state (systolic only).
    pub dataflow_info: Option<DataflowInfo>,
}

impl EngineTrace {
    /// Pretty-print the trace for educational display.
    ///
    /// Returns a multi-line string showing the cycle, engine, utilization,
    /// and per-unit details. Example output:
    ///
    /// ```text
    /// [Cycle 3] WarpEngine (SIMT) -- 75.0% utilization (3/4 active)
    ///   FADD R2, R0, R1 -- 3/4 threads active
    ///   Unit 0: R2 = 1.0 + 2.0 = 3.0
    ///   Unit 1: R2 = 3.0 + 4.0 = 7.0
    ///   Unit 2: (masked -- diverged)
    ///   Unit 3: R2 = 5.0 + 6.0 = 11.0
    /// ```
    pub fn format(&self) -> String {
        let pct = format!("{:.1}%", self.utilization * 100.0);
        let mut lines = vec![format!(
            "[Cycle {}] {} ({}) -- {} utilization ({}/{} active)",
            self.cycle, self.engine_name, self.execution_model.name(),
            pct, self.active_count, self.total_count,
        )];
        lines.push(format!("  {}", self.description));

        // Sort unit traces by ID for deterministic output.
        let mut sorted_ids: Vec<usize> = self.unit_traces.keys().copied().collect();
        sorted_ids.sort();
        for unit_id in sorted_ids {
            lines.push(format!(
                "  Unit {}: {}",
                unit_id, self.unit_traces[&unit_id]
            ));
        }

        if let Some(ref di) = self.divergence_info {
            lines.push(format!(
                "  Divergence: depth={}, reconvergence_pc={}",
                di.divergence_depth, di.reconvergence_pc
            ));
        }

        lines.join("\n")
    }
}

impl fmt::Display for EngineTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

// ---------------------------------------------------------------------------
// ParallelExecutionEngine -- the trait all engines implement
// ---------------------------------------------------------------------------

/// The common interface for all parallel execution engines.
///
/// # Trait Design
///
/// This trait captures the minimal shared behavior of ALL parallel
/// execution engines, regardless of execution model:
///
/// 1. `name()` -- identify which engine this is
/// 2. `width()` -- how many parallel units (threads, lanes, PEs, MACs)
/// 3. `execution_model()` -- which paradigm (SIMT, SIMD, systolic, etc.)
/// 4. `step()` -- advance one clock cycle
/// 5. `halted()` -- is all work complete?
/// 6. `reset()` -- return to initial state
///
/// Any type that implements this trait can be driven uniformly by
/// Layer 7 (the compute unit) or by tests and visualization tools.
///
/// # Why so minimal?
///
/// Different engines have radically different APIs:
/// - `WarpEngine` has `load_program()`, `set_thread_register()`
/// - `SystolicArray` has `load_weights()`, `feed_input()`
/// - `MACArrayEngine` has `load_schedule()`, `load_inputs()`
///
/// Those are engine-specific. The trait only captures what they ALL share,
/// so that Layer 7 (the compute unit) can drive any engine uniformly.
pub trait ParallelExecutionEngine {
    /// Engine name: "WarpEngine", "WavefrontEngine", etc.
    fn name(&self) -> &str;

    /// Parallelism width (threads, lanes, PEs, MACs).
    fn width(&self) -> usize;

    /// Which parallel execution model this engine uses.
    fn execution_model(&self) -> ExecutionModel;

    /// Advance one clock cycle. Returns a trace of what happened.
    fn step(&mut self) -> EngineTrace;

    /// True if all work is complete.
    fn halted(&self) -> bool;

    /// Reset to initial state.
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_model_names() {
        assert_eq!(ExecutionModel::Simt.name(), "SIMT");
        assert_eq!(ExecutionModel::Simd.name(), "SIMD");
        assert_eq!(ExecutionModel::Systolic.name(), "SYSTOLIC");
        assert_eq!(ExecutionModel::ScheduledMac.name(), "SCHEDULED_MAC");
        assert_eq!(ExecutionModel::Vliw.name(), "VLIW");
    }

    #[test]
    fn test_execution_model_display() {
        assert_eq!(format!("{}", ExecutionModel::Simt), "SIMT");
        assert_eq!(format!("{}", ExecutionModel::Systolic), "SYSTOLIC");
    }

    #[test]
    fn test_execution_model_equality() {
        assert_eq!(ExecutionModel::Simt, ExecutionModel::Simt);
        assert_ne!(ExecutionModel::Simt, ExecutionModel::Simd);
    }

    #[test]
    fn test_engine_trace_format() {
        let mut unit_traces = HashMap::new();
        unit_traces.insert(0, "R2 = 1.0 + 2.0 = 3.0".to_string());
        unit_traces.insert(1, "(masked)".to_string());

        let trace = EngineTrace {
            cycle: 3,
            engine_name: "WarpEngine".to_string(),
            execution_model: ExecutionModel::Simt,
            description: "FADD -- 1/2 active".to_string(),
            unit_traces,
            active_mask: vec![true, false],
            active_count: 1,
            total_count: 2,
            utilization: 0.5,
            divergence_info: None,
            dataflow_info: None,
        };

        let formatted = trace.format();
        assert!(formatted.contains("[Cycle 3]"));
        assert!(formatted.contains("WarpEngine"));
        assert!(formatted.contains("SIMT"));
        assert!(formatted.contains("50.0%"));
        assert!(formatted.contains("Unit 0:"));
        assert!(formatted.contains("Unit 1:"));
    }

    #[test]
    fn test_engine_trace_format_with_divergence() {
        let trace = EngineTrace {
            cycle: 5,
            engine_name: "WarpEngine".to_string(),
            execution_model: ExecutionModel::Simt,
            description: "branch divergence".to_string(),
            unit_traces: HashMap::new(),
            active_mask: vec![true, false],
            active_count: 1,
            total_count: 2,
            utilization: 0.5,
            divergence_info: Some(DivergenceInfo {
                active_mask_before: vec![true, true],
                active_mask_after: vec![true, false],
                reconvergence_pc: 10,
                divergence_depth: 1,
            }),
            dataflow_info: None,
        };

        let formatted = trace.format();
        assert!(formatted.contains("Divergence: depth=1"));
        assert!(formatted.contains("reconvergence_pc=10"));
    }

    #[test]
    fn test_engine_trace_display() {
        let trace = EngineTrace {
            cycle: 1,
            engine_name: "Test".to_string(),
            execution_model: ExecutionModel::Simd,
            description: "test".to_string(),
            unit_traces: HashMap::new(),
            active_mask: vec![],
            active_count: 0,
            total_count: 0,
            utilization: 0.0,
            divergence_info: None,
            dataflow_info: None,
        };
        let s = format!("{}", trace);
        assert!(s.contains("[Cycle 1]"));
    }

    #[test]
    fn test_divergence_info() {
        let di = DivergenceInfo {
            active_mask_before: vec![true, true, true, true],
            active_mask_after: vec![true, true, false, false],
            reconvergence_pc: 20,
            divergence_depth: 1,
        };
        assert_eq!(di.divergence_depth, 1);
        assert_eq!(di.reconvergence_pc, 20);
        assert_eq!(di.active_mask_before.len(), 4);
    }

    #[test]
    fn test_dataflow_info() {
        let df = DataflowInfo {
            pe_states: vec![vec!["acc=0".to_string()]],
            data_positions: HashMap::new(),
        };
        assert_eq!(df.pe_states.len(), 1);
        assert!(df.data_positions.is_empty());
    }
}
