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
//!   ├── DivisionByZero           — integer division by zero
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
