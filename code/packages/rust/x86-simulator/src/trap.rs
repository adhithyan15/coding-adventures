//! Faults that stop execution.
//!
//! Anything that would, on real hardware, raise a fault or that this simulator
//! does not (yet) model is surfaced as a `Trap` and propagated out of `step`/
//! `run` — fail-closed. A guest program can only ever *trap*; it cannot escape
//! the sandbox or affect the host.

/// A fault that halts the simulator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trap {
    /// `ud2` (or any instruction the decoder maps to "illegal") executed — this
    /// is how an E5 out-of-bounds array access aborts (SIGILL analogue).
    IllegalInstruction(u64),
    /// A memory access outside the address space, or heap exhaustion.
    MemoryFault(u64),
    /// An opcode the decoder does not understand yet (carries the byte offset).
    /// Distinct from `IllegalInstruction` so "unimplemented" is never mistaken
    /// for a program-level trap.
    DecodeError { offset: u64, opcode: u8 },
    /// A `call` to an external symbol the harness did not provide a host shim for.
    UnresolvedExternal(String),
    /// Execution ran longer than the step budget (runaway-loop backstop).
    StepLimitExceeded,
    /// `div`/`idiv` by zero, or a quotient that overflows the destination — the
    /// `#DE` divide-error fault on real hardware (carries the instruction offset).
    DivideError(u64),
}

impl std::fmt::Display for Trap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Trap::IllegalInstruction(a) => write!(f, "illegal instruction (ud2) at {a:#x}"),
            Trap::MemoryFault(a) => write!(f, "memory fault at {a:#x}"),
            Trap::DecodeError { offset, opcode } =>
                write!(f, "undecoded opcode {opcode:#04x} at {offset:#x}"),
            Trap::UnresolvedExternal(s) => write!(f, "unresolved external symbol {s:?}"),
            Trap::StepLimitExceeded => write!(f, "step limit exceeded"),
            Trap::DivideError(a) => write!(f, "divide error (#DE) at {a:#x}"),
        }
    }
}

impl std::error::Error for Trap {}
