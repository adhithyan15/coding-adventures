/// One-bit branch predictor -- one flip-flop per branch.
///
/// The one-bit predictor is the simplest dynamic predictor. Unlike static
/// predictors (AlwaysTaken, BTFNT), it actually learns from the branch's
/// history. Each branch address maps to a single bit of state that records
/// the last outcome:
///
/// ```text
///     bit = 0 -> predict NOT TAKEN
///     bit = 1 -> predict TAKEN
/// ```
///
/// After each branch resolves, the bit is updated to match the actual outcome.
/// This means the predictor always predicts "whatever happened last time."
///
/// Hardware implementation:
///     A small SRAM table indexed by the lower bits of the PC.
///     Each entry is a single flip-flop (1 bit of storage).
///     Total storage: table_size x 1 bit.
///     For a 1024-entry table: 1024 bits = 128 bytes.
///
/// The aliasing problem:
///     Since the table is indexed by (pc % table_size), two different branches
///     can map to the same entry. This is called "aliasing" or "interference."
///     When branches alias, they corrupt each other's predictions.
///
/// The double-misprediction problem:
///     Consider a loop that runs N times then exits. The one-bit predictor
///     mispredicts TWICE per loop invocation: once at the first iteration
///     (it remembered "not taken" from the last exit) and once at the last
///     iteration (it remembered "taken" from the loop body). The two-bit
///     predictor solves this with hysteresis.
use std::collections::HashMap;

use crate::prediction::{BranchPredictor, Prediction};
use crate::stats::PredictionStats;

/// 1-bit predictor -- one flip-flop per branch address.
///
/// Maintains a table of 1-bit entries indexed by `pc % table_size`.
/// Each entry remembers the LAST outcome of that branch.
///
/// The state diagram is trivially simple:
///
/// ```text
///     +-----------------+     taken      +-----------------+
///     | Predict NOT     | ------------> |  Predict TAKEN   |
///     |  TAKEN (bit=0)  | <------------ |    (bit=1)       |
///     +-----------------+   not taken   +-----------------+
/// ```
///
/// Every misprediction flips the bit. This is too aggressive -- a single
/// anomalous outcome changes the prediction. The 2-bit predictor adds
/// hysteresis to fix this.
///
/// # Example
/// ```
/// use branch_predictor::{OneBitPredictor, BranchPredictor};
///
/// let mut pred = OneBitPredictor::new(1024);
///
/// // First encounter -- cold start, defaults to NOT TAKEN
/// let p = pred.predict(0x100);
/// assert!(!p.taken);
///
/// // Update with actual outcome: branch was taken
/// pred.update(0x100, true, None);
///
/// // Now predicts TAKEN (remembers last outcome)
/// let p = pred.predict(0x100);
/// assert!(p.taken);
/// ```
pub struct OneBitPredictor {
    /// Number of entries in the prediction table.
    table_size: usize,
    /// Maps (index) -> last_outcome. We use a `HashMap` rather than a `Vec`
    /// to avoid pre-allocating memory for entries that are never accessed.
    /// In hardware, all entries exist physically but start at 0 (not-taken).
    table: HashMap<u64, bool>,
    /// Statistics tracker.
    stats: PredictionStats,
}

impl OneBitPredictor {
    /// Create a new one-bit predictor with the given table size.
    ///
    /// # Arguments
    /// * `table_size` - Number of entries in the prediction table.
    ///   Common sizes: 256, 512, 1024, 2048, 4096.
    pub fn new(table_size: usize) -> Self {
        Self {
            table_size,
            table: HashMap::new(),
            stats: PredictionStats::new(),
        }
    }

    /// Compute the table index for a given PC.
    ///
    /// In hardware, this is just the lower log2(table_size) bits of the PC.
    /// Using modulo achieves the same result in software.
    fn index(&self, pc: u64) -> u64 {
        pc % self.table_size as u64
    }
}

impl BranchPredictor for OneBitPredictor {
    /// Predict based on the last outcome of this branch.
    ///
    /// On a cold start (branch not yet seen), defaults to NOT TAKEN.
    fn predict(&mut self, pc: u64) -> Prediction {
        let idx = self.index(pc);
        let taken = *self.table.get(&idx).unwrap_or(&false);
        // Confidence: 0.5 because we only have 1 bit of history
        Prediction {
            taken,
            confidence: 0.5,
            target: None,
        }
    }

    /// Update the prediction table with the actual outcome.
    ///
    /// Simply sets the bit to match the actual outcome. This is the "flip"
    /// that gives the 1-bit predictor its characteristic behavior.
    fn update(&mut self, pc: u64, taken: bool, _target: Option<u64>) {
        let idx = self.index(pc);
        // Record accuracy BEFORE updating the table
        let predicted = *self.table.get(&idx).unwrap_or(&false);
        self.stats.record(predicted == taken);
        // Now update the table to remember this outcome
        self.table.insert(idx, taken);
    }

    fn stats(&self) -> &PredictionStats {
        &self.stats
    }

    fn reset(&mut self) {
        self.table.clear();
        self.stats.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cold_start_predicts_not_taken() {
        let mut pred = OneBitPredictor::new(1024);
        let p = pred.predict(0x100);
        assert!(!p.taken);
    }

    #[test]
    fn test_learns_last_outcome() {
        let mut pred = OneBitPredictor::new(1024);
        // Taken outcome -> should predict taken next time
        pred.update(0x100, true, None);
        let p = pred.predict(0x100);
        assert!(p.taken);

        // Not-taken outcome -> should predict not-taken next time
        pred.update(0x100, false, None);
        let p = pred.predict(0x100);
        assert!(!p.taken);
    }

    #[test]
    fn test_double_misprediction_on_loop() {
        let mut pred = OneBitPredictor::new(1024);

        // First invocation of a loop with 10 iterations
        // Iter 1: cold (not-taken) vs actual taken -> WRONG
        pred.update(0x100, true, None);
        assert_eq!(pred.stats().incorrect, 1);

        // Iter 2-9: predicts taken, actual taken -> correct
        for _ in 1..9 {
            pred.update(0x100, true, None);
        }
        assert_eq!(pred.stats().correct, 8);

        // Iter 10: predicts taken, actual not-taken -> WRONG
        pred.update(0x100, false, None);
        assert_eq!(pred.stats().incorrect, 2);

        // Second invocation, iter 1: predicts not-taken (from exit), actual taken -> WRONG
        pred.update(0x100, true, None);
        assert_eq!(pred.stats().incorrect, 3); // the double-misprediction!
    }

    #[test]
    fn test_aliasing() {
        let mut pred = OneBitPredictor::new(4); // tiny table for aliasing
        // 0x100 and 0x104 both map to index 0 (0x100 % 4 = 0, 0x104 % 4 = 0)
        pred.update(0x100, true, None); // index 0 = taken
        // 0x104 should see the aliased entry
        let p = pred.predict(0x104);
        // Depends on 0x104 % 4 = 0, so yes, it sees the aliased "taken"
        assert!(p.taken);
    }

    #[test]
    fn test_different_branches_independent() {
        let mut pred = OneBitPredictor::new(1024);
        pred.update(0x100, true, None);
        pred.update(0x200, false, None);

        let p1 = pred.predict(0x100);
        let p2 = pred.predict(0x200);
        assert!(p1.taken);
        assert!(!p2.taken);
    }

    #[test]
    fn test_reset() {
        let mut pred = OneBitPredictor::new(1024);
        pred.update(0x100, true, None);
        pred.reset();

        assert_eq!(pred.stats().predictions, 0);
        let p = pred.predict(0x100);
        assert!(!p.taken); // back to cold start
    }

    #[test]
    fn test_confidence_is_half() {
        let mut pred = OneBitPredictor::new(1024);
        let p = pred.predict(0x100);
        assert!((p.confidence - 0.5).abs() < f64::EPSILON);
    }
}
