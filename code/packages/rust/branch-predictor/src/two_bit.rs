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
use std::collections::{HashMap, HashSet};

use state_machine::DFA;

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

    /// Convert a DFA state name string to a `TwoBitState` enum variant.
    ///
    /// The DFA uses human-readable string names for states. This function
    /// maps them back to our efficient enum representation. This is the
    /// bridge between the formal automata world (strings) and the hardware
    /// simulation world (enums).
    ///
    /// # Panics
    /// Panics if the state name is not one of the four known names.
    pub fn from_dfa_name(name: &str) -> TwoBitState {
        match name {
            "SNT" => TwoBitState::StronglyNotTaken,
            "WNT" => TwoBitState::WeaklyNotTaken,
            "WT" => TwoBitState::WeaklyTaken,
            "ST" => TwoBitState::StronglyTaken,
            _ => panic!("Unknown two-bit DFA state name: '{}'", name),
        }
    }

    /// Convert this `TwoBitState` to the DFA state name string.
    ///
    /// This is the inverse of `from_dfa_name`. The names are short
    /// abbreviations matching the state diagram in the module docs.
    pub fn to_dfa_name(self) -> &'static str {
        match self {
            TwoBitState::StronglyNotTaken => "SNT",
            TwoBitState::WeaklyNotTaken => "WNT",
            TwoBitState::WeaklyTaken => "WT",
            TwoBitState::StronglyTaken => "ST",
        }
    }
}

/// Build a DFA that models the two-bit saturating counter.
///
/// This constructs a formal DFA from the `state_machine` crate whose
/// transition function is exactly the two-bit saturating counter logic:
///
/// ```text
///     States:      {SNT, WNT, WT, ST}
///     Alphabet:    {taken, not_taken}
///     Transitions: (SNT, taken)     -> WNT
///                  (SNT, not_taken) -> SNT   (saturated)
///                  (WNT, taken)     -> WT
///                  (WNT, not_taken) -> SNT
///                  (WT,  taken)     -> ST
///                  (WT,  not_taken) -> WNT
///                  (ST,  taken)     -> ST    (saturated)
///                  (ST,  not_taken) -> WT
///     Initial:     WNT (the common textbook default)
///     Accepting:   {WT, ST}  (states that predict "taken")
/// ```
///
/// The accepting states are the "taken" states (WT and ST), so
/// `dfa.accepts(sequence)` answers: "after this branch history,
/// would the predictor predict taken?"
///
/// # Example
/// ```
/// use branch_predictor::two_bit::two_bit_dfa;
///
/// let dfa = two_bit_dfa();
/// // Starting at WNT, one "taken" moves to WT (an accepting state)
/// assert!(dfa.accepts(&["taken"]));
/// // Starting at WNT, one "not_taken" moves to SNT (not accepting)
/// assert!(!dfa.accepts(&["not_taken"]));
/// ```
pub fn two_bit_dfa() -> DFA {
    let states: HashSet<String> = HashSet::from([
        "SNT".to_string(),
        "WNT".to_string(),
        "WT".to_string(),
        "ST".to_string(),
    ]);

    let alphabet: HashSet<String> = HashSet::from([
        "taken".to_string(),
        "not_taken".to_string(),
    ]);

    let transitions: HashMap<(String, String), String> = HashMap::from([
        // SNT transitions
        (("SNT".to_string(), "taken".to_string()), "WNT".to_string()),
        (("SNT".to_string(), "not_taken".to_string()), "SNT".to_string()),
        // WNT transitions
        (("WNT".to_string(), "taken".to_string()), "WT".to_string()),
        (("WNT".to_string(), "not_taken".to_string()), "SNT".to_string()),
        // WT transitions
        (("WT".to_string(), "taken".to_string()), "ST".to_string()),
        (("WT".to_string(), "not_taken".to_string()), "WNT".to_string()),
        // ST transitions
        (("ST".to_string(), "taken".to_string()), "ST".to_string()),
        (("ST".to_string(), "not_taken".to_string()), "WT".to_string()),
    ]);

    // Initial state: WNT (the common textbook default).
    // Accepting states: WT and ST (states that predict "taken").
    let accepting: HashSet<String> = HashSet::from([
        "WT".to_string(),
        "ST".to_string(),
    ]);

    DFA::new(states, alphabet, transitions, "WNT".to_string(), accepting)
        .expect("two_bit_dfa: DFA construction must not fail for known-good inputs")
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

    // ── DFA equivalence tests ─────────────────────────────────────

    #[test]
    fn test_dfa_construction() {
        let dfa = two_bit_dfa();
        assert_eq!(dfa.states().len(), 4);
        assert_eq!(dfa.alphabet().len(), 2);
        assert_eq!(dfa.initial(), "WNT");
        assert!(dfa.accepting().contains("WT"));
        assert!(dfa.accepting().contains("ST"));
        assert!(!dfa.accepting().contains("SNT"));
        assert!(!dfa.accepting().contains("WNT"));
    }

    #[test]
    fn test_dfa_is_complete() {
        let dfa = two_bit_dfa();
        assert!(dfa.is_complete());
    }

    #[test]
    fn test_dfa_no_warnings() {
        let dfa = two_bit_dfa();
        assert!(dfa.validate().is_empty());
    }

    #[test]
    fn test_dfa_taken_transitions_match_enum() {
        // Verify that every "taken" transition in the DFA matches
        // the TwoBitState::taken_outcome() method.
        let all_states = [
            TwoBitState::StronglyNotTaken,
            TwoBitState::WeaklyNotTaken,
            TwoBitState::WeaklyTaken,
            TwoBitState::StronglyTaken,
        ];
        let dfa = two_bit_dfa();
        for state in &all_states {
            let dfa_name = state.to_dfa_name();
            let expected = state.taken_outcome().to_dfa_name();
            let key = (dfa_name.to_string(), "taken".to_string());
            let actual = dfa.transitions().get(&key).expect("transition must exist");
            assert_eq!(
                actual, expected,
                "DFA taken transition from {} should be {} but was {}",
                dfa_name, expected, actual
            );
        }
    }

    #[test]
    fn test_dfa_not_taken_transitions_match_enum() {
        // Verify that every "not_taken" transition in the DFA matches
        // the TwoBitState::not_taken_outcome() method.
        let all_states = [
            TwoBitState::StronglyNotTaken,
            TwoBitState::WeaklyNotTaken,
            TwoBitState::WeaklyTaken,
            TwoBitState::StronglyTaken,
        ];
        let dfa = two_bit_dfa();
        for state in &all_states {
            let dfa_name = state.to_dfa_name();
            let expected = state.not_taken_outcome().to_dfa_name();
            let key = (dfa_name.to_string(), "not_taken".to_string());
            let actual = dfa.transitions().get(&key).expect("transition must exist");
            assert_eq!(
                actual, expected,
                "DFA not_taken transition from {} should be {} but was {}",
                dfa_name, expected, actual
            );
        }
    }

    #[test]
    fn test_dfa_accepts_predicts_taken() {
        let dfa = two_bit_dfa();
        // From WNT, one taken -> WT (accepting, predicts taken)
        assert!(dfa.accepts(&["taken"]));
        // From WNT, two taken -> ST (accepting)
        assert!(dfa.accepts(&["taken", "taken"]));
        // From WNT, not_taken -> SNT (not accepting)
        assert!(!dfa.accepts(&["not_taken"]));
        // From WNT, taken then not_taken -> WNT (not accepting)
        assert!(!dfa.accepts(&["taken", "not_taken"]));
    }

    #[test]
    fn test_dfa_saturation() {
        let dfa = two_bit_dfa();
        // Many taken inputs should stay in ST (accepting)
        assert!(dfa.accepts(&["taken", "taken", "taken", "taken", "taken"]));
        // Many not_taken inputs should stay in SNT (not accepting)
        assert!(!dfa.accepts(&["not_taken", "not_taken", "not_taken", "not_taken"]));
    }

    #[test]
    fn test_dfa_loop_hysteresis() {
        let dfa = two_bit_dfa();
        // Simulate a loop: many taken, then one not_taken.
        // Starting at WNT: T->WT, T->ST, T->ST, T->ST, NT->WT
        // WT is accepting, so the predictor still predicts taken after one NT.
        assert!(dfa.accepts(&["taken", "taken", "taken", "taken", "not_taken"]));
    }

    #[test]
    fn test_dfa_name_roundtrip() {
        let all_states = [
            TwoBitState::StronglyNotTaken,
            TwoBitState::WeaklyNotTaken,
            TwoBitState::WeaklyTaken,
            TwoBitState::StronglyTaken,
        ];
        for state in &all_states {
            let name = state.to_dfa_name();
            let recovered = TwoBitState::from_dfa_name(name);
            assert_eq!(*state, recovered);
        }
    }

    #[test]
    #[should_panic(expected = "Unknown two-bit DFA state name")]
    fn test_dfa_name_unknown_panics() {
        TwoBitState::from_dfa_name("BOGUS");
    }

    #[test]
    fn test_dfa_process_matches_predictor() {
        // Walk the DFA and the predictor in lock-step and verify they agree.
        let mut dfa = two_bit_dfa();
        let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
        let sequence = [true, true, false, true, false, false, true, true, true, false];

        for &taken in &sequence {
            // Before update, check prediction matches DFA accepting
            let dfa_predicts_taken = dfa.accepting().contains(dfa.current_state());
            let pred_predicts_taken = pred.get_branch_state(0x100).predicts_taken();
            assert_eq!(
                dfa_predicts_taken, pred_predicts_taken,
                "DFA state {} (accepts={}) disagrees with predictor state {:?} (predicts_taken={})",
                dfa.current_state(), dfa_predicts_taken,
                pred.get_branch_state(0x100), pred_predicts_taken
            );

            // Transition both
            let event = if taken { "taken" } else { "not_taken" };
            dfa.process(event).unwrap();
            pred.update(0x100, taken, None);

            // After update, verify DFA state matches predictor state
            let dfa_state = TwoBitState::from_dfa_name(dfa.current_state());
            let pred_state = pred.get_branch_state(0x100);
            assert_eq!(dfa_state, pred_state);
        }
    }
}
