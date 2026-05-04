//! # BranchStats — per-site conditional branch profiling.
//!
//! The dispatch loop calls `BranchStats::bump` after every `jmp_if_true`
//! and `jmp_if_false` instruction.  `VMCore` accumulates a nested map
//! `branch_stats: HashMap<fn_name, HashMap<ip, BranchStats>>` that callers
//! can query via `VMCore::branch_profile(fn_name, ip)`.
//!
//! ## Why per-ip rather than per-label?
//!
//! Labels are resolved at dispatch time to an instruction index.  The index
//! uniquely identifies a conditional branch within a function; using it as
//! the key keeps the collection O(1) — no label-table lookup needed at
//! count time.
//!
//! ## Taken-ratio semantics
//!
//! A ratio close to 1.0 means the branch is almost always taken — the JIT
//! should specialise on the taken path.  A ratio close to 0.0 means the
//! condition is rarely true — the fall-through path dominates.  A ratio near
//! 0.5 means the branch is unpredictable; the JIT should not specialise.
//!
//! These are the same semantics used by LLVM's `BranchProbabilityInfo` and
//! V8's branch-frequency feedback.

/// Taken / not-taken counts for one conditional branch instruction.
///
/// Keyed by `(fn_name, instruction_index)` in `VMCore::branch_stats`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchStats {
    /// Number of times the branch was taken (condition was true for
    /// `jmp_if_true`, or false for `jmp_if_false`).
    pub taken_count: u64,

    /// Number of times the branch was not taken.
    pub not_taken_count: u64,
}

impl BranchStats {
    /// Create a fresh zeroed counter.
    pub fn new() -> Self {
        BranchStats::default()
    }

    /// Record one branch observation.
    ///
    /// `taken` should be `true` when the jump was actually executed.
    #[inline]
    pub fn bump(&mut self, taken: bool) {
        if taken {
            self.taken_count += 1;
        } else {
            self.not_taken_count += 1;
        }
    }

    /// Total number of times this conditional instruction was reached.
    #[inline]
    pub fn total(&self) -> u64 {
        self.taken_count + self.not_taken_count
    }

    /// Fraction of observations where the branch was taken (0.0 – 1.0).
    ///
    /// Returns `0.0` if the branch has never been observed (avoids
    /// division by zero).
    pub fn taken_ratio(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            0.0
        } else {
            self.taken_count as f64 / total as f64
        }
    }

    /// `true` if the branch is strongly biased toward being taken
    /// (`taken_ratio >= threshold`).
    ///
    /// Typical threshold for "hot taken" JIT specialisation: 0.85.
    pub fn is_biased_taken(&self, threshold: f64) -> bool {
        self.taken_ratio() >= threshold
    }

    /// `true` if the branch is strongly biased toward not being taken.
    pub fn is_biased_not_taken(&self, threshold: f64) -> bool {
        let total = self.total();
        if total == 0 {
            return false;
        }
        (self.not_taken_count as f64 / total as f64) >= threshold
    }
}
