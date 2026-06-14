//! `AOTError` — the error type for ahead-of-time compilation failures.
//!
//! AOT compilation can fail for two broad reasons:
//!
//! 1. **Backend failure** — the pluggable backend (e.g. `NullBackend`) returned
//!    `None` from its `compile()` method, meaning it could not translate the
//!    CIR instruction sequence.
//!
//! 2. **Snapshot failure** — reading a `.aot` binary with `snapshot::read()`
//!    encountered a bad magic number, a truncated header, or a version mismatch.
//!
//! All error variants carry a human-readable `message` string so callers can
//! display meaningful diagnostics without needing to pattern-match every case.
//!
//! # Example
//!
//! ```
//! use aot_core::errors::AOTError;
//!
//! let e = AOTError::backend("NullBackend returned None");
//! assert!(e.to_string().contains("NullBackend"));
//!
//! let e2 = AOTError::snapshot("bad magic: expected AOT\\0");
//! assert!(e2.to_string().contains("bad magic"));
//! ```

use std::fmt;

// ---------------------------------------------------------------------------
// AOTError
// ---------------------------------------------------------------------------

/// An error that occurred during ahead-of-time compilation or snapshot I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AOTError {
    /// The backend failed to compile a CIR instruction sequence.
    ///
    /// `message` describes what the backend did not support, e.g.
    /// `"NullBackend returned None"` or `"unsupported opcode mul_f64"`.
    Backend { message: String },

    /// A `.aot` binary was malformed: bad magic, truncated header, or
    /// unexpected version number.
    ///
    /// `message` contains the specific read failure reason.
    Snapshot { message: String },
}

impl AOTError {
    /// Construct a `Backend` error with the given message.
    pub fn backend(message: impl Into<String>) -> Self {
        AOTError::Backend { message: message.into() }
    }

    /// Construct a `Snapshot` error with the given message.
    pub fn snapshot(message: impl Into<String>) -> Self {
        AOTError::Snapshot { message: message.into() }
    }

    /// Return the error message regardless of variant.
    pub fn message(&self) -> &str {
        match self {
            AOTError::Backend { message } => message,
            AOTError::Snapshot { message } => message,
        }
    }
}

impl fmt::Display for AOTError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AOTError::Backend { message } => write!(f, "AOT backend error: {}", message),
            AOTError::Snapshot { message } => write!(f, "AOT snapshot error: {}", message),
        }
    }
}

impl std::error::Error for AOTError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_error_message() {
        let e = AOTError::backend("compile failed");
        assert_eq!(e.message(), "compile failed");
    }

    #[test]
    fn snapshot_error_message() {
        let e = AOTError::snapshot("bad magic");
        assert_eq!(e.message(), "bad magic");
    }

    #[test]
    fn backend_display() {
        let e = AOTError::backend("unsupported op");
        assert!(e.to_string().contains("backend"));
        assert!(e.to_string().contains("unsupported op"));
    }

    #[test]
    fn snapshot_display() {
        let e = AOTError::snapshot("truncated");
        assert!(e.to_string().contains("snapshot"));
        assert!(e.to_string().contains("truncated"));
    }

    #[test]
    fn backend_eq() {
        assert_eq!(AOTError::backend("x"), AOTError::backend("x"));
        assert_ne!(AOTError::backend("x"), AOTError::backend("y"));
    }

    #[test]
    fn snapshot_ne_backend() {
        assert_ne!(AOTError::backend("x"), AOTError::snapshot("x"));
    }

    #[test]
    fn implements_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(AOTError::backend("test"));
        assert!(e.to_string().contains("test"));
    }

    #[test]
    fn clone() {
        let e = AOTError::snapshot("msg");
        assert_eq!(e.clone(), e);
    }
}
