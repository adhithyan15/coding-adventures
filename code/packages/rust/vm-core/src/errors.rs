//! Error types for `vm-core`.
//!
//! All errors implement `std::error::Error` and are variants of [`VMError`].
//! The hierarchy mirrors the Python vm-core exception hierarchy:
//!
//! ```text
//! VMError
//!   ├── UnknownOpcode(String)    — no handler for an opcode
//!   ├── FrameOverflow            — call stack depth exceeded
//!   ├── UndefinedVariable(String) — variable not in scope
//!   ├── TypeError { expected, actual, context } — type mismatch
//!   ├── DivisionByZero           — integer division by zero (legacy `div`/`mod`)
//!   ├── Trap { kind }            — a LANG83 trap (`trap`, `i64_div_floor`, …)
//!   ├── RegisterLimitExceeded { function, registers, limit }
//!   │                            — a function needs more registers than a frame holds
//!   ├── RegisterBudgetExceeded { function, slots, budget }
//!   │                            — the call stack's register files outgrew the VM budget
//!   └── Custom(String)           — for type_assert and other domain errors
//! ```

use std::fmt;

/// All errors that can be raised during VM execution.
#[derive(Debug)]
pub enum VMError {
    /// No handler registered for the given opcode mnemonic.
    UnknownOpcode(String),
    /// A `call` instruction would exceed the maximum call-stack depth.
    FrameOverflow {
        depth: usize,
        callee: String,
    },
    /// An instruction references a variable name that has no register slot.
    UndefinedVariable(String),
    /// A value has the wrong runtime type (raised by `type_assert` and casts).
    TypeError {
        expected: String,
        actual: String,
        context: String,
    },
    /// Integer division by zero.
    DivisionByZero,
    /// A LANG83 trap: the program executed `trap kind`, or a trapping standard
    /// operation hit its defined trap (`i64_div_floor` by zero → `Trap.DivZero`,
    /// `i64::MIN / -1` → `Trap.Overflow`). `kind` is a name from the closed table
    /// in `interpreter_ir::exception_kind`. Output written before the trap has
    /// already been flushed (every VM output builtin flushes as it writes).
    Trap {
        /// The trap kind, e.g. `"Trap.DivZero"`.
        kind: String,
    },
    /// Named label referenced by a branch does not exist in the function.
    UndefinedLabel { label: String, function: String },
    /// A function needs more registers than one frame may hold
    /// ([`crate::frame::VMFrame::MAX_FRAME_REGISTERS`]).
    ///
    /// Raised before the program runs: [`crate::core::VMCore::execute`] counts
    /// the distinct register names (parameters plus every instruction `dest`)
    /// in each function of the module and refuses the whole module if any
    /// function is over the limit. The frame also checks at run time, so a
    /// register a language opcode handler invents on the fly hits the same
    /// error instead of being dropped.
    ///
    /// Before this variant existed the frame silently ignored the 65,537th
    /// new register, and the program failed later, far from the cause, with
    /// `undefined variable "…"`.
    RegisterLimitExceeded {
        /// The function whose frame is too small.
        function: String,
        /// How many distinct registers it needs (at run time: at least this many).
        registers: usize,
        /// The per-frame limit, `VMFrame::MAX_FRAME_REGISTERS`.
        limit: usize,
    },
    /// The register files of every live frame together would outgrow the
    /// VM-wide register-slot budget (`VMCore::max_register_slots`).
    ///
    /// Raised at run time, when a frame's register file is about to grow —
    /// a new named register, a `store_reg` to a slot past the end, or a
    /// `call` pushing a callee frame.  The per-frame limit
    /// ([`Self::RegisterLimitExceeded`]) caps one frame; this caps the whole
    /// call stack, so the memory one `execute` can pin in register files is
    /// at most `budget × size_of::<Value>()` bytes.  A host whose programs
    /// legitimately need more raises `VMCore::max_register_slots` (the
    /// displayed message says so).
    ///
    /// Before it existed, a three-instruction recursive function
    /// (`store_reg 65535, 1; call r = f; ret r`) grew every one of its 512
    /// frames to 65,536 slots — about 768 MiB of register files — before
    /// `FrameOverflow` stopped it.
    RegisterBudgetExceeded {
        /// The function whose frame was growing (or being pushed).
        function: String,
        /// The total register slots of all live frames the growth would
        /// have reached.
        slots: usize,
        /// The VM-wide budget, `VMCore::max_register_slots`.
        budget: usize,
    },
    /// A custom error raised by language-specific opcode handlers.
    Custom(String),
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VMError::UnknownOpcode(op) => {
                write!(f, "no handler for opcode {op:?}")
            }
            VMError::FrameOverflow { depth, callee } => {
                write!(
                    f,
                    "call stack depth {depth} exceeded calling {callee:?}"
                )
            }
            VMError::UndefinedVariable(name) => {
                write!(f, "undefined variable {name:?}")
            }
            VMError::TypeError { expected, actual, context } => {
                write!(
                    f,
                    "type error in {context}: expected {expected:?}, got {actual:?}"
                )
            }
            VMError::DivisionByZero => write!(f, "division by zero"),
            VMError::Trap { kind } => write!(f, "trap: {kind}"),
            VMError::UndefinedLabel { label, function } => {
                write!(f, "undefined label {label:?} in function {function:?}")
            }
            VMError::RegisterLimitExceeded { function, registers, limit } => {
                write!(
                    f,
                    "function {function:?} needs {registers} registers, \
                     more than the {limit} a vm-core frame can hold"
                )
            }
            VMError::RegisterBudgetExceeded { function, slots, budget } => {
                write!(
                    f,
                    "function {function:?} would bring the call stack to {slots} register slots, \
                     more than the vm-core budget of {budget} (raise VMCore::max_register_slots)"
                )
            }
            VMError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for VMError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unknown_opcode() {
        let e = VMError::UnknownOpcode("tetrad.move".into());
        assert!(e.to_string().contains("tetrad.move"));
    }

    #[test]
    fn display_frame_overflow() {
        let e = VMError::FrameOverflow { depth: 512, callee: "foo".into() };
        assert!(e.to_string().contains("512"));
        assert!(e.to_string().contains("foo"));
    }

    #[test]
    fn display_trap_names_its_kind() {
        let e = VMError::Trap { kind: "Trap.DivZero".into() };
        assert_eq!(e.to_string(), "trap: Trap.DivZero");
    }

    #[test]
    fn display_register_limit_names_function_count_and_limit() {
        let e = VMError::RegisterLimitExceeded {
            function: "main".into(),
            registers: 65_537,
            limit: 65_536,
        };
        assert_eq!(
            e.to_string(),
            "function \"main\" needs 65537 registers, more than the 65536 a vm-core frame can hold"
        );
    }

    #[test]
    fn display_register_budget_names_function_slots_and_budget() {
        let e = VMError::RegisterBudgetExceeded {
            function: "f".into(),
            slots: 1_114_112,
            budget: 1_048_576,
        };
        assert_eq!(
            e.to_string(),
            "function \"f\" would bring the call stack to 1114112 register slots, \
             more than the vm-core budget of 1048576 (raise VMCore::max_register_slots)"
        );
    }

    #[test]
    fn display_type_error() {
        let e = VMError::TypeError {
            expected: "u8".into(),
            actual: "str".into(),
            context: "add".into(),
        };
        assert!(e.to_string().contains("u8"));
        assert!(e.to_string().contains("str"));
    }
}
