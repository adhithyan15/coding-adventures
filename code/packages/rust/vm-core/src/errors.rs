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
