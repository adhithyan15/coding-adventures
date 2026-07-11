//! Runtime errors. Anything the v0.1 runtime does not yet model surfaces as a
//! descriptive [`RuntimeError`] — never as silently wrong output.

use std::fmt;

/// An error raised while reading or executing a COBOL program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// The source failed to lex or parse (message from the frontend).
    Parse(String),
    /// A referenced data-name is not defined in WORKING-STORAGE.
    UndefinedName(String),
    /// Two data items share a name (qualification is not yet supported).
    DuplicateName(String),
    /// A PICTURE string uses a feature the v0.1 runtime does not yet model.
    UnsupportedPicture(String),
    /// A statement or construct the v0.1 runtime does not yet execute.
    Unsupported(String),
    /// `DIVIDE` by zero with no `ON SIZE ERROR` clause to catch it.
    DivideByZero,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Parse(m) => write!(f, "parse error: {m}"),
            RuntimeError::UndefinedName(n) => write!(f, "undefined data-name: {n}"),
            RuntimeError::DuplicateName(n) => {
                write!(f, "duplicate data-name (qualification not yet supported): {n}")
            }
            RuntimeError::UnsupportedPicture(p) => {
                write!(f, "unsupported PICTURE in runtime v0.1: {p}")
            }
            RuntimeError::Unsupported(m) => write!(f, "unsupported in runtime v0.1: {m}"),
            RuntimeError::DivideByZero => write!(f, "divide by zero"),
        }
    }
}

impl std::error::Error for RuntimeError {}
