//! CoreStats -- aggregate statistics from all core sub-components.
//!
//! # Why Aggregate Statistics?
//!
//! Each sub-component tracks its own statistics independently:
//!   - Pipeline: stall cycles, flush cycles, completed instructions
//!   - Branch Predictor: accuracy, misprediction count
//!   - Hazard Unit: forwarding count, stall count
//!   - Cache: hit rate, miss rate, evictions
//!
//! CoreStats pulls all of these together into a single view, like the
//! dashboard of a car that shows speed, fuel level, and engine temperature.
//!
//! # Key Metrics
//!
//! IPC (Instructions Per Cycle): the most important performance metric.
//!
//! ```text
//!   IPC = InstructionsCompleted / TotalCycles
//!
//!   IPC = 1.0: every cycle produces a result (ideal for scalar pipeline)
//!   IPC < 1.0: stalls and flushes are wasting cycles
//!   IPC > 1.0: superscalar (not modeled yet)
//! ```

use std::collections::HashMap;
use std::fmt;

use branch_predictor::PredictionStats;
use cache::CacheStats;
use cpu_pipeline::PipelineStats;

/// Aggregate statistics from every sub-component of a Core.
#[derive(Debug, Clone)]
pub struct CoreStats {
    // --- Top-level metrics ---

    /// Number of instructions that reached WB.
    pub instructions_completed: i64,

    /// Total number of clock cycles elapsed.
    pub total_cycles: i64,

    // --- Sub-component statistics ---

    /// Pipeline statistics from the cpu-pipeline package.
    pub pipeline_stats: PipelineStats,

    /// Branch predictor statistics.
    pub predictor_stats: Option<PredictionStats>,

    /// Cache statistics, keyed by cache level name ("L1I", "L1D", "L2").
    pub cache_stats: HashMap<String, CacheStats>,

    // --- Hazard statistics ---

    /// Total number of forwarding operations.
    pub forward_count: i64,

    /// Total number of stall cycles.
    pub stall_count: i64,

    /// Total number of pipeline flush cycles.
    pub flush_count: i64,
}

impl CoreStats {
    /// Returns instructions per cycle.
    ///
    /// This is the primary measure of pipeline efficiency:
    ///   - 1.0 = perfect (every cycle retires an instruction)
    ///   - <1.0 = stalls/flushes wasting cycles
    ///   - 0.0 = no instructions completed or no cycles elapsed
    pub fn ipc(&self) -> f64 {
        if self.total_cycles == 0 {
            return 0.0;
        }
        self.instructions_completed as f64 / self.total_cycles as f64
    }

    /// Returns cycles per instruction.
    ///
    /// This is the inverse of IPC:
    ///   - 1.0 = one cycle per instruction (ideal)
    ///   - >1.0 = some cycles wasted
    ///   - 0.0 = no instructions completed
    pub fn cpi(&self) -> f64 {
        if self.instructions_completed == 0 {
            return 0.0;
        }
        self.total_cycles as f64 / self.instructions_completed as f64
    }
}

impl fmt::Display for CoreStats {
    /// Returns a formatted summary of all statistics.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Core Statistics:")?;
        writeln!(f, "  Instructions completed: {}", self.instructions_completed)?;
        writeln!(f, "  Total cycles:           {}", self.total_cycles)?;
        writeln!(f, "  IPC: {:.3}   CPI: {:.3}", self.ipc(), self.cpi())?;
        writeln!(f)?;

        writeln!(f, "Pipeline:")?;
        writeln!(f, "  Stall cycles:  {}", self.pipeline_stats.stall_cycles)?;
        writeln!(f, "  Flush cycles:  {}", self.pipeline_stats.flush_cycles)?;
        writeln!(f, "  Bubble cycles: {}", self.pipeline_stats.bubble_cycles)?;
        writeln!(f)?;

        if let Some(ref ps) = self.predictor_stats {
            writeln!(f, "Branch Prediction:")?;
            writeln!(f, "  Total branches:  {}", ps.predictions)?;
            writeln!(f, "  Correct:         {}", ps.correct)?;
            writeln!(f, "  Mispredictions:  {}", ps.incorrect)?;
            writeln!(f, "  Accuracy:        {:.1}%", ps.accuracy())?;
            writeln!(f)?;
        }

        if !self.cache_stats.is_empty() {
            writeln!(f, "Cache Performance:")?;
            for (name, stats) in &self.cache_stats {
                writeln!(
                    f,
                    "  {}: accesses={}, hit_rate={:.1}%",
                    name,
                    stats.total_accesses(),
                    stats.hit_rate() * 100.0
                )?;
            }
            writeln!(f)?;
        }

        writeln!(f, "Hazards:")?;
        writeln!(f, "  Forwards: {}", self.forward_count)?;
        writeln!(f, "  Stalls:   {}", self.stall_count)?;
        write!(f, "  Flushes:  {}", self.flush_count)?;

        Ok(())
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats(instructions: i64, cycles: i64) -> CoreStats {
        CoreStats {
            instructions_completed: instructions,
            total_cycles: cycles,
            pipeline_stats: PipelineStats::default(),
            predictor_stats: None,
            cache_stats: HashMap::new(),
            forward_count: 0,
            stall_count: 0,
            flush_count: 0,
        }
    }

    #[test]
    fn test_ipc() {
        let stats = make_stats(80, 100);
        assert!((stats.ipc() - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_cpi() {
        let stats = make_stats(100, 120);
        assert!((stats.cpi() - 1.2).abs() < 0.001);
    }

    #[test]
    fn test_ipc_zero_cycles() {
        let stats = make_stats(0, 0);
        assert_eq!(stats.ipc(), 0.0);
    }

    #[test]
    fn test_cpi_zero_instructions() {
        let stats = make_stats(0, 100);
        assert_eq!(stats.cpi(), 0.0);
    }

    #[test]
    fn test_display() {
        let stats = make_stats(80, 100);
        let s = format!("{}", stats);
        assert!(s.contains("Instructions completed: 80"));
        assert!(s.contains("Total cycles:           100"));
    }

    #[test]
    fn test_display_with_predictor_stats() {
        let mut stats = make_stats(100, 120);
        stats.predictor_stats = Some(PredictionStats {
            predictions: 10,
            correct: 8,
            incorrect: 2,
        });
        let s = format!("{}", stats);
        assert!(s.contains("Branch Prediction:"));
        assert!(s.contains("Total branches:  10"));
    }

    #[test]
    fn test_display_with_cache_stats() {
        let mut stats = make_stats(100, 120);
        let mut cs = CacheStats::new();
        cs.record_read(true);
        cs.record_read(false);
        stats.cache_stats.insert("L1D".to_string(), cs);
        let s = format!("{}", stats);
        assert!(s.contains("Cache Performance:"));
        assert!(s.contains("L1D"));
    }
}
