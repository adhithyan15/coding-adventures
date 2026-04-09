//! `SilentWaiting` — a no-op waiting implementation for testing and quiet use.
//!
//! # When to use `SilentWaiting`
//!
//! There are situations where you do *not* want any spinner or animation:
//!
//! - **Tests** — printing to stdout in tests is noisy and can interfere with
//!   test output capture. `SilentWaiting` lets tests drive the runner without
//!   any side effects.
//!
//! - **Piped / scripted input** — when the REPL is driven by a file or pipe
//!   rather than an interactive terminal, spinner characters are garbage in
//!   the output.
//!
//! - **Minimal CLIs** — some tools prefer a clean, no-animation aesthetic.
//!
//! # Implementation notes
//!
//! All methods are pure no-ops. The state type is `()` (unit), which has zero
//! runtime cost. The tick interval is 100 ms — a reasonable polling cadence
//! that doesn't spin-burn the CPU.
//!
//! Because `()` implements `std::any::Any`, boxing it as `Box<dyn Any + Send>`
//! works seamlessly with the `Waiting` trait's type-erased state contract.

use crate::waiting::Waiting;

/// A no-op [`Waiting`](crate::waiting::Waiting) implementation.
///
/// Performs no I/O and holds no state (`()`). Useful for tests and non-interactive
/// REPL sessions where a spinner would be inappropriate.
///
/// # Examples
///
/// ```
/// use repl::silent_waiting::SilentWaiting;
/// use repl::waiting::Waiting;
///
/// let w = SilentWaiting;
/// assert_eq!(w.tick_ms(), 100);
///
/// let state = w.start();       // returns Box<()>
/// let state = w.tick(state);   // no-op, returns Box<()>
/// w.stop(state);               // no-op
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SilentWaiting;

impl Waiting for SilentWaiting {
    /// Returns `Box::new(())` — zero-cost unit state.
    ///
    /// No I/O is performed.
    fn start(&self) -> Box<dyn std::any::Any + Send> {
        Box::new(())
    }

    /// Returns `Box::new(())` — discards the incoming state and returns a
    /// fresh unit state.
    ///
    /// No I/O is performed. The incoming state is dropped.
    fn tick(&self, _state: Box<dyn std::any::Any + Send>) -> Box<dyn std::any::Any + Send> {
        Box::new(())
    }

    /// Poll every 100 milliseconds.
    ///
    /// 100 ms is chosen as a balance: fast enough that a short-running eval
    /// doesn't wait an entire second for the result to be noticed, but slow
    /// enough to avoid burning CPU in a busy-wait loop.
    fn tick_ms(&self) -> u64 {
        100
    }

    /// No-op. The state is dropped.
    fn stop(&self, _state: Box<dyn std::any::Any + Send>) {
        // Nothing to clean up — SilentWaiting never writes to the terminal.
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waiting::Waiting;

    #[test]
    fn test_tick_ms() {
        assert_eq!(SilentWaiting.tick_ms(), 100);
    }

    #[test]
    fn test_start_tick_stop_do_not_panic() {
        let w = SilentWaiting;
        let s0 = w.start();
        let s1 = w.tick(s0);
        let s2 = w.tick(s1);
        w.stop(s2);
    }

    #[test]
    fn test_silent_waiting_is_copy() {
        let w = SilentWaiting;
        let _w2 = w;
        let _w3 = w; // Copy — still usable
    }
}
