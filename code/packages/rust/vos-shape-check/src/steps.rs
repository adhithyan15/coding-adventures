//! # steps: a deterministic measure of how much work a check did
//!
//! A test that wants to know "does the checker scan in linear time?" has two
//! ways to ask.  It can read a clock, or it can *count*.  A clock on a shared
//! CI runner is a poor witness: every run also pays a fixed cost (building the
//! grammar-driven lexer, seconds on a loaded runner) whose jitter is bigger
//! than the extra time a larger input costs, so a growth ratio of two timings
//! can come out as nonsense (one run measured 1276 against a limit of 10 on
//! code that is perfectly linear).  A counter has no fixed cost and no jitter:
//! the same input always counts the same number of steps.
//!
//! ## What is a step
//!
//! One step is one *advance of a scanning loop* (one iteration of a `while`
//! that walks tokens, one edge visited in the call graph) or one *token
//! predicate evaluation* (`Token::is_ident`, `Token::is_punct`), which is how
//! nearly every rescan looks at a token.  The point is not to measure seconds
//! but to grow in the same way the running time grows: if a scanner rescans
//! from the start of an item for every token inside it, the count grows with
//! the square of the input, exactly as the time would.
//!
//! ```text
//!   linear scan of n tokens:            steps ~ c * n       steps(4n) / steps(n) ~ 4
//!   quadratic rescan of n tokens:       steps ~ c * n * n   steps(4n) / steps(n) ~ 16
//! ```
//!
//! ## Why a thread-local
//!
//! The tests run in parallel, one check per thread; a global counter would mix
//! their work.  A `thread_local!` cell keeps each check's count its own, and
//! [`crate::check_crate_counted`] returns the difference it saw around one call.
//! (This is the checker's own bookkeeping, not component code: the
//! "no global state" rule binds the crates the checker *reads*.)

use std::cell::Cell;

thread_local! {
    static STEPS: Cell<u64> = const { Cell::new(0) };
}

/// Count one step of work.  Cheap: a thread-local increment.
#[inline]
pub fn tick() {
    STEPS.with(|s| s.set(s.get().wrapping_add(1)));
}

/// The number of steps this thread has counted so far.
pub fn steps() -> u64 {
    STEPS.with(Cell::get)
}
