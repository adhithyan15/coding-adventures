/// Base types for all branch predictors.
///
/// In CPU design, the branch predictor sits at the very front of the pipeline --
/// in the fetch stage. Before the CPU even knows what instruction it's looking at,
/// the predictor guesses whether the current PC points to a branch and, if so,
/// whether that branch will be taken.
///
/// Why is this necessary? Consider a 15-stage pipeline (like Intel's Skylake).
/// A branch instruction is resolved in stage ~10. Without prediction, the CPU
/// would have to stall for 10 cycles on EVERY branch -- roughly 20% of all
/// instructions. With prediction, the CPU speculatively fetches down the
/// predicted path. If the prediction is correct, there's zero cost. If wrong,
/// the pipeline flushes and restarts -- a 10-15 cycle penalty.
///
/// The math works out: even 90% accuracy is a huge win.
/// - Without prediction: 20% branches x 10 cycle stall = 2 cycles/instruction penalty
/// - With 90% prediction: 20% branches x 10% miss x 15 cycle flush = 0.3 cycles/instruction
use crate::stats::PredictionStats;

/// A branch prediction -- the predictor's guess before the branch executes.
///
/// A `Prediction` is the output of the `predict()` method. It bundles three
/// pieces of information:
///
/// 1. `taken` -- will the branch jump to its target? (the core question)
/// 2. `confidence` -- how sure is the predictor? (useful for hybrid predictors
///    that choose between sub-predictors based on confidence)
/// 3. `target` -- where does the branch go? (from the BTB, if available)
///
/// We derive `Clone` and `Copy` because predictions are small value types
/// (a bool, an f64, and an Option<u64> -- all stack-allocated). In Rust,
/// `Copy` means the value is bitwise-copied on assignment rather than moved,
/// which is appropriate for small, immutable data.
///
/// # Example
/// ```
/// use branch_predictor::Prediction;
///
/// // A confident prediction that the branch is taken, jumping to 0x400
/// let pred = Prediction { taken: true, confidence: 0.9, target: Some(0x400) };
/// assert!(pred.taken);
///
/// // A low-confidence prediction from a cold-start predictor
/// let pred = Prediction { taken: false, confidence: 0.0, target: None };
/// assert!(!pred.taken);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Prediction {
    /// The predictor's guess: will the branch be taken?
    pub taken: bool,
    /// Confidence level from 0.0 (no confidence) to 1.0 (certain).
    pub confidence: f64,
    /// Predicted target address (from BTB, if available).
    pub target: Option<u64>,
}

/// Interface that all branch predictors must implement.
///
/// The CPU core calls `predict()` before executing a branch.
/// After the branch executes, the core calls `update()` with the actual outcome.
/// This feedback loop is how the predictor learns.
///
/// # Why `&mut self`?
///
/// Even `predict()` sometimes needs to mutate internal state (e.g., the BTFNT
/// predictor reads from its target cache, which may trigger internal bookkeeping).
/// More importantly, `update()` always mutates the predictor's tables. Using
/// `&mut self` on both methods ensures the Rust compiler can verify that no
/// two threads access the predictor simultaneously without synchronization --
/// a real concern in multi-core CPU simulators.
///
/// # Design pattern: Strategy
///
/// Each predictor (AlwaysTaken, TwoBit, etc.) is a strategy that can be
/// swapped into any CPU core design. The core only depends on `BranchPredictor`,
/// never on a concrete predictor type.
///
/// The lifecycle of a branch prediction:
///
/// ```text
///   1. CPU fetches instruction at address `pc`
///   2. CPU calls predictor.predict(pc) -> gets a Prediction
///   3. CPU speculatively fetches from the predicted path
///   4. Several cycles later, the branch resolves
///   5. CPU calls predictor.update(pc, actual_taken, actual_target)
///   6. Predictor adjusts its internal state to learn from the outcome
/// ```
pub trait BranchPredictor {
    /// Predict whether the branch at address `pc` will be taken.
    fn predict(&mut self, pc: u64) -> Prediction;

    /// Update the predictor with the actual branch outcome.
    ///
    /// This is the learning step. After the branch resolves in the execute
    /// stage, the core feeds back the real outcome so the predictor can
    /// adjust its tables.
    fn update(&mut self, pc: u64, taken: bool, target: Option<u64>);

    /// Get prediction accuracy statistics.
    fn stats(&self) -> &PredictionStats;

    /// Reset all predictor state (for a new program).
    ///
    /// Clears the prediction table and resets statistics. Call this between
    /// benchmarks to ensure clean measurements.
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_creation() {
        let pred = Prediction {
            taken: true,
            confidence: 0.9,
            target: Some(0x400),
        };
        assert!(pred.taken);
        assert!((pred.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(pred.target, Some(0x400));
    }

    #[test]
    fn test_prediction_no_target() {
        let pred = Prediction {
            taken: false,
            confidence: 0.0,
            target: None,
        };
        assert!(!pred.taken);
        assert!(pred.target.is_none());
    }

    #[test]
    fn test_prediction_is_copy() {
        let pred = Prediction {
            taken: true,
            confidence: 0.5,
            target: None,
        };
        let pred2 = pred; // Copy, not move
        assert_eq!(pred.taken, pred2.taken); // both still valid
    }
}
