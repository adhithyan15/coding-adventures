//! Stepping algorithms — step-over, step-in, step-out.
//!
//! Implements the three stepping algorithms from spec 05e §"Stepping
//! algorithms" as a pure state machine.  The state machine is exercised by
//! the [`DapServer`] event loop:
//!
//! 1. The handler for `next` / `stepIn` / `stepOut` calls
//!    [`StepController::start`] with the chosen [`StepMode`] and the current
//!    VM state (call-stack depth and the location to step from).
//! 2. The server then loops: `step_instruction()` on the VM, wait for the
//!    `stopped` event, call [`StepController::on_stopped`] with the new
//!    `(call_depth, location, source_line)` snapshot.
//! 3. `on_stopped` returns either [`StepDecision::Done`] (the user-visible
//!    step is complete; emit `stopped { reason: "step" }`) or
//!    [`StepDecision::Continue`] (still stepping; loop again).
//!
//! ## Algorithms
//!
//! ### Step-over (`StepMode::Over`)
//!
//! Goal: advance to the next source line in the current frame, but **don't
//! descend** into callees.
//!
//! ```text
//!  start: record line₀ and depth₀
//!  on_stopped(depth, line):
//!    if depth > depth₀          → Continue   (deeper — still inside callee)
//!    if depth < depth₀          → Done       (returned past start frame; stop here)
//!    if depth == depth₀
//!      and line != line₀        → Done
//!      else                     → Continue
//! ```
//!
//! ### Step-in (`StepMode::In`)
//!
//! Goal: advance to the next *different* source line, descending into
//! callees if a call is the first thing executed.
//!
//! ```text
//!  start: record line₀
//!  on_stopped(_, line):
//!    if line != line₀           → Done
//!    else                       → Continue
//! ```
//!
//! ### Step-out (`StepMode::Out`)
//!
//! Goal: run until the current frame returns.
//!
//! ```text
//!  start: record depth₀
//!  on_stopped(depth, _):
//!    if depth < depth₀          → Done
//!    else                       → Continue
//! ```
//!
//! Breakpoints are handled separately: if the VM stops with reason
//! `Breakpoint`, the server short-circuits the step and emits
//! `stopped { reason: "breakpoint" }`.

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Which stepping algorithm is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// `next` — advance to the next source line in the current frame.
    Over,
    /// `stepIn` — advance to the next source line, possibly into a callee.
    In,
    /// `stepOut` — run until the current frame returns.
    Out,
}

/// What the step controller wants to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDecision {
    /// The step is complete; emit `stopped { reason: "step" }`.
    Done,
    /// Keep stepping — issue another `step_instruction` to the VM.
    Continue,
}

/// State machine for one in-progress step.
///
/// `None` of the fields are public to keep the algorithm tractable —
/// constructed via [`StepController::new`], started via
/// [`StepController::start`], advanced via [`StepController::on_stopped`].
#[derive(Debug, Default)]
pub struct StepController {
    /// `Some(mode)` while a step is in progress; `None` when idle.
    mode: Option<StepMode>,
    /// Call-stack depth at the moment the step began.
    start_depth: usize,
    /// Source line at the moment the step began.  `0` when no source mapping.
    start_line: u32,
}

impl StepController {
    /// Build an idle controller.
    pub fn new() -> Self {
        StepController {
            mode: None,
            start_depth: 0,
            start_line: 0,
        }
    }

    /// Begin a step in `mode`.  Captures the starting `(depth, line)` so the
    /// algorithm can detect when the step is complete.
    pub fn start(&mut self, mode: StepMode, start_depth: usize, start_line: u32) {
        self.mode = Some(mode);
        self.start_depth = start_depth;
        self.start_line = start_line;
    }

    /// Notify the controller that the VM stopped after a `step_instruction`.
    ///
    /// `current_depth` and `current_line` describe the new VM state.  Returns
    /// [`StepDecision::Done`] if the user-visible step has completed,
    /// otherwise [`StepDecision::Continue`].
    ///
    /// Calling `on_stopped` when no step is in progress is a no-op that
    /// returns `Done` — the event loop will then process the stop normally.
    pub fn on_stopped(&mut self, current_depth: usize, current_line: u32) -> StepDecision {
        let mode = match self.mode {
            Some(m) => m,
            None    => return StepDecision::Done,
        };

        let decision = match mode {
            StepMode::Over => self.over_decision(current_depth, current_line),
            StepMode::In   => self.in_decision(current_line),
            StepMode::Out  => self.out_decision(current_depth),
        };

        if decision == StepDecision::Done {
            self.mode = None;
        }
        decision
    }

    /// True while a step is in progress.
    pub fn is_active(&self) -> bool {
        self.mode.is_some()
    }

    /// Cancel the current step (used when a breakpoint short-circuits it).
    pub fn cancel(&mut self) {
        self.mode = None;
    }

    /// The mode of the current step, or `None` if idle.
    pub fn mode(&self) -> Option<StepMode> {
        self.mode
    }

    // ---- per-mode decision helpers -----------------------------------------

    fn over_decision(&self, depth: usize, line: u32) -> StepDecision {
        // Deeper than start → still inside a callee, keep stepping.
        if depth > self.start_depth { return StepDecision::Continue; }
        // Shallower than start → we returned past the start frame; stop here.
        if depth < self.start_depth { return StepDecision::Done; }
        // Same depth, different line → reached the next user-visible line.
        if line != self.start_line { return StepDecision::Done; }
        // Same depth, same line → still on the same source statement.
        StepDecision::Continue
    }

    fn in_decision(&self, line: u32) -> StepDecision {
        if line != self.start_line { StepDecision::Done } else { StepDecision::Continue }
    }

    fn out_decision(&self, depth: usize) -> StepDecision {
        if depth < self.start_depth { StepDecision::Done } else { StepDecision::Continue }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- step-over -------------------------------------------------------

    #[test]
    fn over_same_depth_new_line_is_done() {
        let mut s = StepController::new();
        s.start(StepMode::Over, 1, 5);
        assert_eq!(s.on_stopped(1, 6), StepDecision::Done);
        assert!(!s.is_active(), "controller resets after Done");
    }

    #[test]
    fn over_same_depth_same_line_continues() {
        let mut s = StepController::new();
        s.start(StepMode::Over, 1, 5);
        assert_eq!(s.on_stopped(1, 5), StepDecision::Continue);
        assert!(s.is_active());
    }

    #[test]
    fn over_deeper_continues() {
        // Stepped INTO a callee — keep going.
        let mut s = StepController::new();
        s.start(StepMode::Over, 1, 5);
        assert_eq!(s.on_stopped(2, 99), StepDecision::Continue);
    }

    #[test]
    fn over_shallower_is_done() {
        // Returned past the start frame — stop.
        let mut s = StepController::new();
        s.start(StepMode::Over, 2, 5);
        assert_eq!(s.on_stopped(1, 99), StepDecision::Done);
    }

    // ---- step-in ---------------------------------------------------------

    #[test]
    fn in_different_line_is_done() {
        let mut s = StepController::new();
        s.start(StepMode::In, 1, 5);
        assert_eq!(s.on_stopped(2, 6), StepDecision::Done);
    }

    #[test]
    fn in_same_line_continues() {
        let mut s = StepController::new();
        s.start(StepMode::In, 1, 5);
        assert_eq!(s.on_stopped(1, 5), StepDecision::Continue);
    }

    #[test]
    fn in_descends_into_callee_immediately() {
        // First instruction of the callee sits on a different line.
        let mut s = StepController::new();
        s.start(StepMode::In, 1, 5);
        assert_eq!(s.on_stopped(2, 99), StepDecision::Done);
    }

    // ---- step-out --------------------------------------------------------

    #[test]
    fn out_returns_when_depth_drops() {
        let mut s = StepController::new();
        s.start(StepMode::Out, 3, 0);
        assert_eq!(s.on_stopped(2, 0), StepDecision::Done);
    }

    #[test]
    fn out_same_depth_continues() {
        let mut s = StepController::new();
        s.start(StepMode::Out, 3, 0);
        assert_eq!(s.on_stopped(3, 99), StepDecision::Continue);
    }

    #[test]
    fn out_deeper_continues() {
        // We made another nested call before returning — still inside.
        let mut s = StepController::new();
        s.start(StepMode::Out, 3, 0);
        assert_eq!(s.on_stopped(4, 99), StepDecision::Continue);
    }

    // ---- meta ------------------------------------------------------------

    #[test]
    fn idle_on_stopped_returns_done_no_op() {
        let mut s = StepController::new();
        assert!(!s.is_active());
        assert_eq!(s.on_stopped(0, 0), StepDecision::Done);
    }

    #[test]
    fn cancel_resets_state() {
        let mut s = StepController::new();
        s.start(StepMode::Over, 1, 5);
        assert!(s.is_active());
        s.cancel();
        assert!(!s.is_active());
        assert_eq!(s.mode(), None);
    }

    #[test]
    fn mode_accessor_reports_active_mode() {
        let mut s = StepController::new();
        s.start(StepMode::In, 0, 0);
        assert_eq!(s.mode(), Some(StepMode::In));
    }

    #[test]
    fn over_then_in_then_out_can_run_in_sequence() {
        let mut s = StepController::new();

        s.start(StepMode::Over, 1, 5);
        assert_eq!(s.on_stopped(1, 6), StepDecision::Done);

        s.start(StepMode::In, 1, 6);
        assert_eq!(s.on_stopped(2, 99), StepDecision::Done);

        s.start(StepMode::Out, 2, 99);
        assert_eq!(s.on_stopped(1, 100), StepDecision::Done);
    }
}
