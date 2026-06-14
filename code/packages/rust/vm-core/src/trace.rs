//! # VMTrace — per-instruction execution trace records.
//!
//! `VMTrace` captures a snapshot of the interpreter state around each
//! instruction dispatch.  The opt-in `VMCore::execute_traced()` path fills
//! a `Vec<VMTrace>` that callers can inspect for debugging, test assertions,
//! and reproducer generation.
//!
//! ## Overhead
//!
//! Each trace record clones the full register file twice (before and after).
//! This is intentionally expensive — `execute_traced` is for debug/test
//! tooling only.  Production `execute()` pays zero overhead.
//!
//! ## Relationship to `jit-profiling-insights` (LANG11)
//!
//! LANG11 reads `IIRInstr.observed_slot` (the rolling type feedback) to
//! produce higher-level JIT hints.  `VMTrace` provides a linear time-ordered
//! record of what actually happened during one run — complementary but
//! distinct: LANG11 is cumulative; `VMTrace` is per-execution.

use crate::value::Value;
use interpreter_ir::IIRInstr;

/// A snapshot of the interpreter state around one instruction dispatch.
///
/// Each record corresponds to one iteration of the dispatch loop.
/// Records are produced in execution order (trace[0] = first instruction
/// executed, trace[N-1] = last).
#[derive(Debug, Clone)]
pub struct VMTrace {
    /// Nesting depth of the call stack when this instruction executed.
    ///
    /// Depth 0 = the entry function; depth N = N levels of `call` deep.
    pub frame_depth: usize,

    /// Name of the function being executed.
    pub fn_name: String,

    /// Index of the instruction within the function's instruction list.
    pub ip: usize,

    /// A clone of the instruction that was dispatched.
    pub instr: IIRInstr,

    /// Register file values immediately **before** the instruction ran.
    ///
    /// Registers are stored by index (register 0 = registers_before[0]).
    /// The length equals the number of live registers at this point.
    pub registers_before: Vec<Value>,

    /// Register file values immediately **after** the instruction ran.
    pub registers_after: Vec<Value>,
}

impl VMTrace {
    /// Human-readable one-line summary of this trace record.
    ///
    /// Format: `<depth>:<fn_name>@<ip>  <opcode> [<dest>] <- [<srcs>]`
    pub fn summary(&self) -> String {
        let srcs: Vec<String> = self.instr.srcs.iter()
            .map(|op| format!("{op:?}"))
            .collect();
        let dest = self.instr.dest.as_deref().unwrap_or("_");
        format!(
            "{}:{}@{}  {} {} <- [{}]",
            self.frame_depth,
            self.fn_name,
            self.ip,
            self.instr.op,
            dest,
            srcs.join(", "),
        )
    }
}
