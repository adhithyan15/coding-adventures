//! Stepping algorithms: step-over, step-in, step-out.
//!
//! ## Implementation plan (LS03 PR A)
//!
//! All three algorithms are specified in detail in spec 05e §"Stepping algorithms".
//! Summarised here for quick reference:
//!
//! ### step-over (DAP: "next")
//!
//! Goal: advance to the next source line, but don't descend into callees.
//!
//! 1. Record current_line = sidecar.offset_to_source(current_offset).line.
//! 2. Record call_depth = vm.get_call_stack().len().
//! 3. Loop:
//!    a. vm.step_instruction()
//!    b. new_depth = vm.get_call_stack().len()
//!    c. If new_depth < call_depth → we returned from a callee; advance past it.
//!       Continue until new_depth == call_depth AND new_line != current_line.
//!    d. new_line = sidecar.offset_to_source(vm.current_offset()).line
//!    e. If new_line != current_line AND new_depth == call_depth → done, emit "stopped".
//!    f. If we hit a user breakpoint → emit "stopped" with reason "breakpoint" instead.
//!
//! ### step-in (DAP: "stepIn")
//!
//! Goal: advance to the next source line OR into a callee.
//!
//! 1. Record current_line.
//! 2. Loop: step_instruction() until new_line != current_line → emit "stopped".
//!
//! ### step-out (DAP: "stepOut")
//!
//! Goal: run until returning from the current call frame.
//!
//! 1. Record call_depth = vm.get_call_stack().len().
//! 2. Loop: step_instruction() until call_depth decreases → emit "stopped".

/// Controls stepping state and algorithm execution.
///
/// ## TODO — implement (LS03 PR A)
pub struct StepController {
    // TODO: StepMode enum { Over, In, Out, None }
    //       current_depth: usize
}

impl StepController {
    /// Create a new step controller.
    pub fn new() -> Self {
        StepController {}
    }
}

impl Default for StepController {
    fn default() -> Self { Self::new() }
}
