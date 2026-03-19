/// Static branch predictors -- the simplest strategies, requiring no learning.
///
/// Static predictors make the same prediction every time, regardless of history.
/// They require zero hardware (no tables, no counters, no state) and serve as
/// baselines against which dynamic predictors are measured.
///
/// Three strategies are implemented here:
///
/// 1. **AlwaysTakenPredictor** -- always predicts "taken"
///    Accuracy: ~60-70% on typical code. Why? Most branches are loop back-edges,
///    which are taken on every iteration except the last. A loop that runs 100
///    times has 100 branches: 99 taken + 1 not-taken = 99% accuracy on that loop.
///    The overall ~60% comes from mixing loops with if-else branches.
///
/// 2. **AlwaysNotTakenPredictor** -- always predicts "not taken"
///    Accuracy: ~30-40% on typical code. This is the worst reasonable strategy,
///    but it has a hardware advantage: the "not taken" path is just the next
///    sequential instruction, so the CPU doesn't need to compute a target address.
///    Early processors (Intel 8086) effectively used this because they had no
///    branch prediction unit -- they just fetched the next instruction.
///
/// 3. **BackwardTakenForwardNotTaken (BTFNT)** -- direction-based heuristic
///    Accuracy: ~65-75% on typical code. Backward branches (target < pc) are
///    usually loop back-edges, so predict taken. Forward branches (target > pc)
///    are usually if-else, so predict not-taken. This is what early MIPS R4000
///    and SPARC processors used.
use std::collections::HashMap;

use crate::prediction::{BranchPredictor, Prediction};
use crate::stats::PredictionStats;

// ─── AlwaysTakenPredictor ───────────────────────────────────────────────────
//
// The simplest "optimistic" predictor. Always bets that the branch will be
// taken (jump to the target address).
//
// Hardware cost: zero. No tables, no counters, no state at all.
// The prediction logic is just a wire tied to 1.
//
// When it works well:
//   - Tight loops (for i in 0..1000) -- 999/1000 correct
//   - Unconditional jumps -- 100% correct (they're always taken)
//
// When it fails:
//   - if x > 0 { ... } (random data) -- ~50% correct
//   - Early loop exits -- misses every exit

/// Always predicts "taken". Simple but surprisingly effective (~60% accurate).
///
/// # Example
/// ```
/// use branch_predictor::{AlwaysTakenPredictor, BranchPredictor};
///
/// let mut pred = AlwaysTakenPredictor::new();
/// let p = pred.predict(0x100);
/// assert!(p.taken);
/// ```
pub struct AlwaysTakenPredictor {
    stats: PredictionStats,
}

impl AlwaysTakenPredictor {
    /// Create a new always-taken predictor.
    pub fn new() -> Self {
        Self {
            stats: PredictionStats::new(),
        }
    }
}

impl Default for AlwaysTakenPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchPredictor for AlwaysTakenPredictor {
    /// Always predict taken, with zero confidence (it's just a guess).
    fn predict(&mut self, _pc: u64) -> Prediction {
        Prediction {
            taken: true,
            confidence: 0.0,
            target: None,
        }
    }

    /// Record whether the always-taken guess was correct.
    fn update(&mut self, _pc: u64, taken: bool, _target: Option<u64>) {
        // We predicted TAKEN, so we're correct when the branch IS taken
        self.stats.record(taken);
    }

    fn stats(&self) -> &PredictionStats {
        &self.stats
    }

    fn reset(&mut self) {
        self.stats.reset();
    }
}

// ─── AlwaysNotTakenPredictor ────────────────────────────────────────────────
//
// The simplest "pessimistic" predictor. Always bets the branch falls through
// to the next sequential instruction.
//
// Hardware advantage: the "next sequential instruction" is already being fetched
// by the instruction fetch unit. No target address computation needed.

/// Always predicts "not taken". The baseline against which others are measured.
///
/// # Example
/// ```
/// use branch_predictor::{AlwaysNotTakenPredictor, BranchPredictor};
///
/// let mut pred = AlwaysNotTakenPredictor::new();
/// let p = pred.predict(0x100);
/// assert!(!p.taken);
/// ```
pub struct AlwaysNotTakenPredictor {
    stats: PredictionStats,
}

impl AlwaysNotTakenPredictor {
    /// Create a new always-not-taken predictor.
    pub fn new() -> Self {
        Self {
            stats: PredictionStats::new(),
        }
    }
}

impl Default for AlwaysNotTakenPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchPredictor for AlwaysNotTakenPredictor {
    /// Always predict not taken, with zero confidence.
    fn predict(&mut self, _pc: u64) -> Prediction {
        Prediction {
            taken: false,
            confidence: 0.0,
            target: None,
        }
    }

    /// Record whether the always-not-taken guess was correct.
    fn update(&mut self, _pc: u64, taken: bool, _target: Option<u64>) {
        // We predicted NOT taken, so we're correct when the branch is NOT taken
        self.stats.record(!taken);
    }

    fn stats(&self) -> &PredictionStats {
        &self.stats
    }

    fn reset(&mut self) {
        self.stats.reset();
    }
}

// ─── BackwardTakenForwardNotTaken (BTFNT) ───────────────────────────────────
//
// A direction-based heuristic that uses the branch's target address relative
// to its own PC to make the prediction:
//
//   - Backward branch (target <= pc) -> predict TAKEN
//     These are almost always loop back-edges.
//
//   - Forward branch (target > pc) -> predict NOT TAKEN
//     These are usually if-then-else.
//
// This predictor needs to know the target address at prediction time, which
// means it stores the most recently known target for each branch.
//
// Used in: MIPS R4000, SPARC V8, some early ARM processors.

/// BTFNT -- predicts taken for backward branches, not-taken for forward.
///
/// On the first encounter of a branch (cold start), it defaults to predicting
/// NOT taken, since we don't yet know the target direction.
///
/// # Example
/// ```
/// use branch_predictor::{BackwardTakenForwardNotTaken, BranchPredictor};
///
/// let mut pred = BackwardTakenForwardNotTaken::new();
///
/// // First encounter -- cold start, predicts not-taken
/// let p = pred.predict(0x108);
/// assert!(!p.taken);
///
/// // After learning the target is backward
/// pred.update(0x108, true, Some(0x100));
/// let p = pred.predict(0x108);
/// assert!(p.taken); // backward -> taken
/// ```
pub struct BackwardTakenForwardNotTaken {
    stats: PredictionStats,
    /// Maps PC -> last known target address. We need this because predict()
    /// is called before decode, so we rely on previous updates to know the
    /// branch direction.
    ///
    /// We use `HashMap` here because the number of unique branch PCs is
    /// not known at compile time. In Rust, `HashMap` provides O(1) average
    /// lookup, matching the Python `dict` in the original implementation.
    targets: HashMap<u64, u64>,
}

impl BackwardTakenForwardNotTaken {
    /// Create a new BTFNT predictor with no known targets.
    pub fn new() -> Self {
        Self {
            stats: PredictionStats::new(),
            targets: HashMap::new(),
        }
    }
}

impl Default for BackwardTakenForwardNotTaken {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchPredictor for BackwardTakenForwardNotTaken {
    /// Predict based on branch direction: backward=taken, forward=not-taken.
    ///
    /// If we haven't seen this branch before (no known target), we default
    /// to NOT taken -- the safe choice that doesn't require a target address.
    fn predict(&mut self, pc: u64) -> Prediction {
        match self.targets.get(&pc) {
            None => {
                // Cold start -- don't know the target direction yet
                Prediction {
                    taken: false,
                    confidence: 0.0,
                    target: None,
                }
            }
            Some(&target) => {
                // Backward branch (target <= pc) -> taken (loop back-edge)
                // Forward branch (target > pc)  -> not taken (if-else)
                let taken = target <= pc;
                Prediction {
                    taken,
                    confidence: 0.5,
                    target: Some(target),
                }
            }
        }
    }

    /// Record the branch outcome and learn the target address.
    fn update(&mut self, pc: u64, taken: bool, target: Option<u64>) {
        // Store the target so we can use it for future direction-based predictions
        if let Some(t) = target {
            self.targets.insert(pc, t);
        }

        // Determine what we would have predicted, accounting for cold starts
        let predicted_taken = match self.targets.get(&pc) {
            None => false,
            Some(&t) => t <= pc,
        };

        self.stats.record(predicted_taken == taken);
    }

    fn stats(&self) -> &PredictionStats {
        &self.stats
    }

    fn reset(&mut self) {
        self.targets.clear();
        self.stats.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AlwaysTaken tests ───────────────────────────────────────────

    #[test]
    fn test_always_taken_predicts_taken() {
        let mut pred = AlwaysTakenPredictor::new();
        let p = pred.predict(0x100);
        assert!(p.taken);
        assert!((p.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_always_taken_correct_on_taken() {
        let mut pred = AlwaysTakenPredictor::new();
        pred.predict(0x100);
        pred.update(0x100, true, None);
        assert_eq!(pred.stats().correct, 1);
    }

    #[test]
    fn test_always_taken_wrong_on_not_taken() {
        let mut pred = AlwaysTakenPredictor::new();
        pred.predict(0x100);
        pred.update(0x100, false, None);
        assert_eq!(pred.stats().incorrect, 1);
    }

    #[test]
    fn test_always_taken_loop_accuracy() {
        let mut pred = AlwaysTakenPredictor::new();
        // Simulate a loop: 9 taken, 1 not-taken
        for i in 0..10 {
            pred.predict(0x100);
            pred.update(0x100, i < 9, Some(0x50));
        }
        assert_eq!(pred.stats().correct, 9);
        assert_eq!(pred.stats().incorrect, 1);
        assert!((pred.stats().accuracy() - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_always_taken_reset() {
        let mut pred = AlwaysTakenPredictor::new();
        pred.update(0x100, true, None);
        pred.reset();
        assert_eq!(pred.stats().predictions, 0);
    }

    // ── AlwaysNotTaken tests ────────────────────────────────────────

    #[test]
    fn test_always_not_taken_predicts_not_taken() {
        let mut pred = AlwaysNotTakenPredictor::new();
        let p = pred.predict(0x100);
        assert!(!p.taken);
    }

    #[test]
    fn test_always_not_taken_correct_on_not_taken() {
        let mut pred = AlwaysNotTakenPredictor::new();
        pred.predict(0x100);
        pred.update(0x100, false, None);
        assert_eq!(pred.stats().correct, 1);
    }

    #[test]
    fn test_always_not_taken_wrong_on_taken() {
        let mut pred = AlwaysNotTakenPredictor::new();
        pred.predict(0x100);
        pred.update(0x100, true, None);
        assert_eq!(pred.stats().incorrect, 1);
    }

    // ── BTFNT tests ─────────────────────────────────────────────────

    #[test]
    fn test_btfnt_cold_start_predicts_not_taken() {
        let mut pred = BackwardTakenForwardNotTaken::new();
        let p = pred.predict(0x108);
        assert!(!p.taken);
        assert!((p.confidence - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_btfnt_backward_predicts_taken() {
        let mut pred = BackwardTakenForwardNotTaken::new();
        // Teach the predictor the target
        pred.update(0x108, true, Some(0x100));
        // Now it knows it's a backward branch
        let p = pred.predict(0x108);
        assert!(p.taken);
        assert_eq!(p.target, Some(0x100));
    }

    #[test]
    fn test_btfnt_forward_predicts_not_taken() {
        let mut pred = BackwardTakenForwardNotTaken::new();
        // Teach the predictor a forward target
        pred.update(0x200, false, Some(0x300));
        // Forward branch -> not taken
        let p = pred.predict(0x200);
        assert!(!p.taken);
    }

    #[test]
    fn test_btfnt_equal_predicts_taken() {
        let mut pred = BackwardTakenForwardNotTaken::new();
        // Target == PC (degenerate case: infinite loop)
        pred.update(0x100, true, Some(0x100));
        let p = pred.predict(0x100);
        assert!(p.taken); // target <= pc -> taken
    }

    #[test]
    fn test_btfnt_loop_pattern() {
        let mut pred = BackwardTakenForwardNotTaken::new();
        // Simulate a loop at 0x108 with back-edge to 0x100
        // First iteration: cold start, predicts not-taken
        let p = pred.predict(0x108);
        assert!(!p.taken);
        pred.update(0x108, true, Some(0x100));

        // Subsequent iterations: knows it's backward, predicts taken
        for _ in 0..8 {
            let p = pred.predict(0x108);
            assert!(p.taken);
            pred.update(0x108, true, Some(0x100));
        }

        // Last iteration: still predicts taken, but branch is not taken
        let p = pred.predict(0x108);
        assert!(p.taken);
        pred.update(0x108, false, Some(0x100));
    }

    #[test]
    fn test_btfnt_reset() {
        let mut pred = BackwardTakenForwardNotTaken::new();
        pred.update(0x108, true, Some(0x100));
        pred.reset();
        assert_eq!(pred.stats().predictions, 0);
        // After reset, target cache should be cleared too
        let p = pred.predict(0x108);
        assert!(!p.taken); // cold start again
    }
}
