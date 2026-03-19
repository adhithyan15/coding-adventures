/// Two-bit saturating counter predictor -- the classic, used in most textbooks.
///
/// The two-bit predictor improves on the one-bit predictor by adding hysteresis.
/// Instead of flipping the prediction on every misprediction, it takes TWO
/// consecutive mispredictions to change the predicted direction. This is achieved
/// with a 2-bit saturating counter -- a counter that counts up to 3 and down to 0,
/// but never wraps around (it "saturates" at the boundaries).
///
/// The four states and their meanings:
///
/// ```text
///     STRONGLY      WEAKLY        WEAKLY        STRONGLY
///     NOT TAKEN     NOT TAKEN     TAKEN         TAKEN
///       (00)          (01)         (10)          (11)
///
///     Predict:      Predict:      Predict:      Predict:
///     NOT TAKEN     NOT TAKEN     TAKEN         TAKEN
/// ```
///
/// State transition diagram:
///
/// ```text
///     taken                taken               taken               taken
///     ------>              ------>              ------>              ------>
///     (sat)   SNT <-------- WNT <-------- WT <-------- ST   (sat)
///             ------>              ------>              ------>
///           not taken          not taken           not taken
/// ```
///
/// Why this solves the double-misprediction problem:
///     Consider a loop running 10 times. After the loop body saturates the
///     counter to StronglyTaken, the single not-taken at loop exit only
///     moves it to WeaklyTaken -- which still predicts taken on re-entry.
///     Result: 1 misprediction per loop invocation instead of 2.
///
/// Historical usage:
///     - Alpha 21064: 2-bit counters with 2048 entries
///     - Intel Pentium: 2-bit counters with 256 entries
///     - Early ARM (ARM7): 2-bit counters with 64 entries
use std::collections::HashMap;

use crate::prediction::{BranchPredictor, Prediction};
use crate::stats::PredictionStats;

/// The 4 states of a 2-bit saturating counter.
///
/// We use explicit integer values (0-3) that correspond to the 2-bit counter
/// value. This makes the increment/decrement logic natural:
///   taken -> min(state + 1, 3)
///   not taken -> max(state - 1, 0)
///
/// The "saturating" part means we clamp at the boundaries rather than wrapping.
/// In hardware, this is implemented with a simple 2-bit adder and saturation
/// logic -- about 4 gates per entry.
///
/// # Example
/// ```
/// use branch_predictor::TwoBitState;
///
/// let state = TwoBitState::WeaklyNotTaken;
/// assert!(!state.predicts_taken());
///
/// let state = state.taken_outcome();
/// assert_eq!(state, TwoBitState::WeaklyTaken);
/// assert!(state.predicts_taken());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwoBitState {
    /// 00 -- high confidence NOT taken.
    StronglyNotTaken = 0,
    /// 01 -- low confidence NOT taken.
    WeaklyNotTaken = 1,
    /// 10 -- low confidence TAKEN.
    WeaklyTaken = 2,
    /// 11 -- high confidence TAKEN.
    StronglyTaken = 3,
}

impl TwoBitState {
    /// Transition on a "taken" branch outcome (increment, saturate at 3).
    ///
    /// # Example
    /// ```
    /// use branch_predictor::TwoBitState;
    ///
    /// let s = TwoBitState::WeaklyNotTaken.taken_outcome();
    /// assert_eq!(s, TwoBitState::WeaklyTaken);
    ///
    /// let s = TwoBitState::StronglyTaken.taken_outcome();
    /// assert_eq!(s, TwoBitState::StronglyTaken); // saturated!
    /// ```
    pub fn taken_outcome(self) -> TwoBitState {
        match self {
            TwoBitState::StronglyNotTaken => TwoBitState::WeaklyNotTaken,
            TwoBitState::WeaklyNotTaken => TwoBitState::WeaklyTaken,
            TwoBitState::WeaklyTaken => TwoBitState::StronglyTaken,
            TwoBitState::StronglyTaken => TwoBitState::StronglyTaken, // saturated
        }
    }

    /// Transition on a "not taken" branch outcome (decrement, saturate at 0).
    ///
    /// # Example
    /// ```
    /// use branch_predictor::TwoBitState;
    ///
    /// let s = TwoBitState::WeaklyTaken.not_taken_outcome();
    /// assert_eq!(s, TwoBitState::WeaklyNotTaken);
    ///
    /// let s = TwoBitState::StronglyNotTaken.not_taken_outcome();
    /// assert_eq!(s, TwoBitState::StronglyNotTaken); // saturated!
    /// ```
    pub fn not_taken_outcome(self) -> TwoBitState {
        match self {
            TwoBitState::StronglyNotTaken => TwoBitState::StronglyNotTaken, // saturated
            TwoBitState::WeaklyNotTaken => TwoBitState::StronglyNotTaken,
            TwoBitState::WeaklyTaken => TwoBitState::WeaklyNotTaken,
            TwoBitState::StronglyTaken => TwoBitState::WeaklyTaken,
        }
    }

    /// Whether this state predicts "taken".
    ///
    /// The threshold is at WeaklyTaken (2). States 2 and 3 predict taken;
    /// states 0 and 1 predict not-taken. In hardware, this is just bit 1
    /// of the 2-bit counter -- a single wire, zero logic.
    pub fn predicts_taken(self) -> bool {
        (self as u8) >= (TwoBitState::WeaklyTaken as u8)
    }
}

/// 2-bit saturating counter predictor -- the textbook classic.
///
/// This was used in real processors: Alpha 21064, early MIPS, early ARM.
/// Modern CPUs use more sophisticated predictors (TAGE, perceptron) but
/// the 2-bit counter is the foundation that all advanced predictors build on.
///
/// # Example
/// ```
/// use branch_predictor::{TwoBitPredictor, BranchPredictor, TwoBitState};
///
/// let mut pred = TwoBitPredictor::new(256, TwoBitState::WeaklyNotTaken);
///
/// // First encounter -- starts at WeaklyNotTaken -> predicts NOT TAKEN
/// let p = pred.predict(0x100);
/// assert!(!p.taken);
///
/// // After one 'taken' outcome -> moves to WeaklyTaken -> predicts TAKEN
/// pred.update(0x100, true, None);
/// let p = pred.predict(0x100);
/// assert!(p.taken);
/// ```
pub struct TwoBitPredictor {
    /// Number of entries in the prediction table.
    table_size: usize,
    /// Initial state for counter entries that haven't been seen yet.
    initial_state: TwoBitState,
    /// Maps (index) -> TwoBitState. Entries start at initial_state.
    /// We use a HashMap and fill on first access (lazy initialization).
    ///
    /// In Rust, `HashMap` requires its keys to implement `Hash + Eq`.
    /// We use `u64` keys (the table index), which satisfies both traits.
    table: HashMap<u64, TwoBitState>,
    /// Statistics tracker.
    stats: PredictionStats,
}

impl TwoBitPredictor {
    /// Create a new two-bit predictor.
    ///
    /// # Arguments
    /// * `table_size` - Number of entries in the prediction table.
    /// * `initial_state` - Starting state for all counter entries.
    pub fn new(table_size: usize, initial_state: TwoBitState) -> Self {
        Self {
            table_size,
            initial_state,
            table: HashMap::new(),
            stats: PredictionStats::new(),
        }
    }

    /// Compute the table index for a given PC.
    fn index(&self, pc: u64) -> u64 {
        pc % self.table_size as u64
    }

    /// Get the state for a table entry, initializing if needed.
    fn get_state(&self, index: u64) -> TwoBitState {
        *self.table.get(&index).unwrap_or(&self.initial_state)
    }

    /// Inspect the current state for a branch address (for testing/debugging).
    pub fn get_branch_state(&self, pc: u64) -> TwoBitState {
        self.get_state(self.index(pc))
    }
}

impl BranchPredictor for TwoBitPredictor {
    /// Predict based on the 2-bit counter for this branch.
    ///
    /// Reads the counter state and returns taken/not-taken based on the
    /// threshold (states 2-3 -> taken, states 0-1 -> not-taken).
    fn predict(&mut self, pc: u64) -> Prediction {
        let idx = self.index(pc);
        let state = self.get_state(idx);

        // Confidence: strong states are more confident than weak states.
        let confidence = match state {
            TwoBitState::StronglyTaken | TwoBitState::StronglyNotTaken => 1.0,
            _ => 0.5,
        };

        Prediction {
            taken: state.predicts_taken(),
            confidence,
            target: None,
        }
    }

    /// Update the 2-bit counter based on the actual outcome.
    ///
    /// Increments on taken, decrements on not-taken, saturating at boundaries.
    fn update(&mut self, pc: u64, taken: bool, _target: Option<u64>) {
        let idx = self.index(pc);
        let state = self.get_state(idx);

        // Record accuracy BEFORE updating
        self.stats.record(state.predicts_taken() == taken);

        // Transition the state
        let new_state = if taken {
            state.taken_outcome()
        } else {
            state.not_taken_outcome()
        };
        self.table.insert(idx, new_state);
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

    // ── TwoBitState tests ───────────────────────────────────────────

    #[test]
    fn test_state_values() {
        assert_eq!(TwoBitState::StronglyNotTaken as u8, 0);
        assert_eq!(TwoBitState::WeaklyNotTaken as u8, 1);
        assert_eq!(TwoBitState::WeaklyTaken as u8, 2);
        assert_eq!(TwoBitState::StronglyTaken as u8, 3);
    }

    #[test]
    fn test_predicts_taken_threshold() {
        assert!(!TwoBitState::StronglyNotTaken.predicts_taken());
        assert!(!TwoBitState::WeaklyNotTaken.predicts_taken());
        assert!(TwoBitState::WeaklyTaken.predicts_taken());
        assert!(TwoBitState::StronglyTaken.predicts_taken());
    }

    #[test]
    fn test_taken_transitions() {
        assert_eq!(
            TwoBitState::StronglyNotTaken.taken_outcome(),
            TwoBitState::WeaklyNotTaken
        );
        assert_eq!(
            TwoBitState::WeaklyNotTaken.taken_outcome(),
            TwoBitState::WeaklyTaken
        );
        assert_eq!(
            TwoBitState::WeaklyTaken.taken_outcome(),
            TwoBitState::StronglyTaken
        );
        assert_eq!(
            TwoBitState::StronglyTaken.taken_outcome(),
            TwoBitState::StronglyTaken
        ); // saturated
    }

    #[test]
    fn test_not_taken_transitions() {
        assert_eq!(
            TwoBitState::StronglyNotTaken.not_taken_outcome(),
            TwoBitState::StronglyNotTaken
        ); // saturated
        assert_eq!(
            TwoBitState::WeaklyNotTaken.not_taken_outcome(),
            TwoBitState::StronglyNotTaken
        );
        assert_eq!(
            TwoBitState::WeaklyTaken.not_taken_outcome(),
            TwoBitState::WeaklyNotTaken
        );
        assert_eq!(
            TwoBitState::StronglyTaken.not_taken_outcome(),
            TwoBitState::WeaklyTaken
        );
    }

    // ── TwoBitPredictor tests ───────────────────────────────────────

    #[test]
    fn test_initial_state_weakly_not_taken() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
        let p = pred.predict(0x100);
        assert!(!p.taken);
        assert!((p.confidence - 0.5).abs() < f64::EPSILON); // weak state
    }

    #[test]
    fn test_one_taken_flips_from_wnt_to_wt() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
        pred.update(0x100, true, None);
        assert_eq!(pred.get_branch_state(0x100), TwoBitState::WeaklyTaken);
        let p = pred.predict(0x100);
        assert!(p.taken);
    }

    #[test]
    fn test_two_taken_reaches_strongly_taken() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
        pred.update(0x100, true, None);
        pred.update(0x100, true, None);
        assert_eq!(pred.get_branch_state(0x100), TwoBitState::StronglyTaken);
        let p = pred.predict(0x100);
        assert!(p.taken);
        assert!((p.confidence - 1.0).abs() < f64::EPSILON); // strong state
    }

    #[test]
    fn test_hysteresis_on_loop_exit() {
        // The key advantage over 1-bit: a single not-taken doesn't flip
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);

        // Simulate loop: 9 taken iterations
        for _ in 0..9 {
            pred.update(0x100, true, None);
        }
        assert_eq!(pred.get_branch_state(0x100), TwoBitState::StronglyTaken);

        // Loop exit: one not-taken
        pred.update(0x100, false, None);
        assert_eq!(pred.get_branch_state(0x100), TwoBitState::WeaklyTaken);

        // Re-entry: still predicts taken! (unlike 1-bit which would flip)
        let p = pred.predict(0x100);
        assert!(p.taken);
    }

    #[test]
    fn test_loop_misprediction_count() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);

        // First invocation: 10 iterations
        // Iter 1: WNT -> predicts not-taken, actual taken -> WRONG
        pred.update(0x100, true, None);
        assert_eq!(pred.stats().incorrect, 1);

        // Iter 2-9: predicts taken after first update, all correct
        for _ in 1..9 {
            pred.update(0x100, true, None);
        }

        // Iter 10: predicts taken, actual not-taken -> WRONG
        pred.update(0x100, false, None);
        assert_eq!(pred.stats().incorrect, 2);

        // Second invocation, iter 1: WT predicts taken, actual taken -> CORRECT
        pred.update(0x100, true, None);
        // Only 2 mispredictions for the whole first run + correct re-entry
        assert_eq!(pred.stats().incorrect, 2);
    }

    #[test]
    fn test_strongly_not_taken_needs_two_taken_to_flip() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::StronglyNotTaken);
        // One taken: SNT -> WNT, still predicts not-taken
        pred.update(0x100, true, None);
        assert_eq!(pred.get_branch_state(0x100), TwoBitState::WeaklyNotTaken);
        let p = pred.predict(0x100);
        assert!(!p.taken);

        // Second taken: WNT -> WT, now predicts taken
        pred.update(0x100, true, None);
        assert_eq!(pred.get_branch_state(0x100), TwoBitState::WeaklyTaken);
        let p = pred.predict(0x100);
        assert!(p.taken);
    }

    #[test]
    fn test_confidence_levels() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::StronglyNotTaken);
        let p = pred.predict(0x100);
        assert!((p.confidence - 1.0).abs() < f64::EPSILON); // strong

        pred.update(0x100, true, None); // -> WNT
        let p = pred.predict(0x100);
        assert!((p.confidence - 0.5).abs() < f64::EPSILON); // weak
    }

    #[test]
    fn test_reset_clears_table() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
        pred.update(0x100, true, None);
        pred.update(0x100, true, None);
        pred.reset();

        assert_eq!(pred.stats().predictions, 0);
        assert_eq!(
            pred.get_branch_state(0x100),
            TwoBitState::WeaklyNotTaken
        );
    }

    #[test]
    fn test_independent_branches() {
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
        pred.update(0x100, true, None); // 0x100 -> WT
        pred.update(0x200, false, None); // 0x200 -> SNT

        assert_eq!(pred.get_branch_state(0x100), TwoBitState::WeaklyTaken);
        assert_eq!(
            pred.get_branch_state(0x200),
            TwoBitState::StronglyNotTaken
        );
    }
}
