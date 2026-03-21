//! Pipeline snapshots and execution statistics.
//!
//! # Snapshots: Photographs of the Assembly Line
//!
//! A [`PipelineSnapshot`] captures the full state of the pipeline at a single
//! point in time (one clock cycle). Think of it as a photograph of the
//! assembly line: you can see what instruction is at each station.
//!
//! Snapshots are used for:
//!   - Debugging: "What was in the EX stage at cycle 7?"
//!   - Visualization: drawing pipeline diagrams
//!   - Testing: verifying that the pipeline behaves correctly
//!
//! # Statistics: How Efficiently Is the Pipeline Being Used?
//!
//! [`PipelineStats`] tracks performance statistics across the pipeline's
//! execution. These are the same metrics that hardware performance counters
//! measure in real CPUs.

use std::collections::HashMap;
use std::fmt;

use crate::token::PipelineToken;

// =========================================================================
// PipelineSnapshot -- the complete state of the pipeline at one moment
// =========================================================================

/// Captures the full state of the pipeline at a single point in time.
///
/// # Example
///
/// ```text
/// Cycle 7:
///   IF:  instr@28  (fetching instruction at PC=28)
///   ID:  ADD@24    (decoding an ADD instruction)
///   EX:  SUB@20    (executing a SUB)
///   MEM: ---       (bubble -- pipeline was stalled here)
///   WB:  LDR@12    (writing back a load result)
/// ```
#[derive(Debug, Clone)]
pub struct PipelineSnapshot {
    /// The clock cycle number when this snapshot was taken.
    /// Cycles count from 1 (the first call to step() is cycle 1).
    pub cycle: i64,

    /// Maps stage name to the token currently occupying that stage.
    /// A token with is_bubble=true means the stage holds a bubble/NOP.
    pub stages: HashMap<String, PipelineToken>,

    /// True if the pipeline was stalled during this cycle.
    pub stalled: bool,

    /// True if a pipeline flush occurred during this cycle.
    pub flushing: bool,

    /// The current program counter (address of next fetch).
    pub pc: i64,
}

impl fmt::Display for PipelineSnapshot {
    /// Returns a compact representation of the pipeline state.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[cycle {}] PC={} stalled={} flushing={}",
            self.cycle, self.pc, self.stalled, self.flushing
        )
    }
}

// =========================================================================
// PipelineStats -- execution statistics
// =========================================================================

/// Tracks performance statistics across the pipeline's execution.
///
/// # Key Metrics
///
/// **IPC (Instructions Per Cycle)**: The most important pipeline metric.
///
/// ```text
/// IPC = instructions_completed / total_cycles
///
/// Ideal:     IPC = 1.0 (one instruction completes every cycle)
/// With stalls: IPC < 1.0 (some cycles are wasted)
/// Superscalar: IPC > 1.0 (multiple instructions per cycle)
/// ```
///
/// **CPI (Cycles Per Instruction)**: The inverse of IPC.
///
/// ```text
/// CPI = total_cycles / instructions_completed
///
/// Ideal:     CPI = 1.0
/// Typical:   CPI = 1.2-2.0 for real workloads
/// ```
///
/// # Breakdown of Wasted Cycles
///
/// ```text
/// Total cycles = Useful cycles + Stall cycles + Flush cycles + Bubble cycles
///
/// Stall cycles:  Caused by data hazards (load-use dependencies)
/// Flush cycles:  Caused by branch mispredictions
/// Bubble cycles: Cycles where at least one stage held a bubble
/// ```
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Number of clock cycles the pipeline has executed.
    pub total_cycles: i64,

    /// Number of non-bubble instructions that have reached the final
    /// (writeback) stage.
    pub instructions_completed: i64,

    /// Number of cycles where the pipeline was stalled.
    pub stall_cycles: i64,

    /// Number of cycles where a flush occurred.
    pub flush_cycles: i64,

    /// Total number of stage-cycles occupied by bubbles.
    /// For example, if 3 stages hold bubbles for 1 cycle, that
    /// contributes 3 to bubble_cycles.
    pub bubble_cycles: i64,
}

impl PipelineStats {
    /// Returns the instructions per cycle.
    ///
    /// IPC is the primary measure of pipeline efficiency:
    ///   - IPC = 1.0: perfect pipeline utilization (ideal)
    ///   - IPC < 1.0: some cycles are wasted (stalls, flushes)
    ///   - IPC > 1.0: superscalar execution (multiple instructions per cycle)
    ///
    /// Returns 0.0 if no cycles have been executed (avoids division by zero).
    pub fn ipc(&self) -> f64 {
        if self.total_cycles == 0 {
            return 0.0;
        }
        self.instructions_completed as f64 / self.total_cycles as f64
    }

    /// Returns cycles per instruction (inverse of IPC).
    ///
    /// Returns 0.0 if no instructions have completed (avoids division by zero).
    pub fn cpi(&self) -> f64 {
        if self.instructions_completed == 0 {
            return 0.0;
        }
        self.total_cycles as f64 / self.instructions_completed as f64
    }
}

impl fmt::Display for PipelineStats {
    /// Returns a formatted summary of pipeline statistics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PipelineStats{{cycles={}, completed={}, IPC={:.3}, CPI={:.3}, stalls={}, flushes={}, bubbles={}}}",
            self.total_cycles,
            self.instructions_completed,
            self.ipc(),
            self.cpi(),
            self.stall_cycles,
            self.flush_cycles,
            self.bubble_cycles,
        )
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_calculation() {
        let stats = PipelineStats {
            total_cycles: 100,
            instructions_completed: 80,
            ..Default::default()
        };
        assert!((stats.ipc() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_cpi_calculation() {
        let stats = PipelineStats {
            total_cycles: 120,
            instructions_completed: 100,
            ..Default::default()
        };
        assert!((stats.cpi() - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_ipc_zero_cycles() {
        let stats = PipelineStats::default();
        assert_eq!(stats.ipc(), 0.0);
    }

    #[test]
    fn test_cpi_zero_instructions() {
        let stats = PipelineStats {
            total_cycles: 10,
            ..Default::default()
        };
        assert_eq!(stats.cpi(), 0.0);
    }

    #[test]
    fn test_stats_string() {
        let stats = PipelineStats {
            total_cycles: 100,
            instructions_completed: 80,
            stall_cycles: 5,
            flush_cycles: 3,
            bubble_cycles: 10,
        };
        let s = stats.to_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_snapshot_string() {
        let snap = PipelineSnapshot {
            cycle: 7,
            pc: 28,
            stalled: true,
            flushing: false,
            stages: HashMap::new(),
        };
        let s = snap.to_string();
        assert!(!s.is_empty());
    }
}
