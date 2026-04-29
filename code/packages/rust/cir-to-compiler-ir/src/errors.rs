//! # Errors — `CIRLoweringError`
//!
//! A single error type is used throughout the crate. Every function that can
//! fail returns `Result<_, CIRLoweringError>`.
//!
//! ## Why a dedicated error type?
//!
//! Rather than using a string or a generic `Box<dyn Error>`, a concrete struct
//! lets callers pattern-match on the error and inspect the message without
//! dynamic dispatch. It is also `Clone` and `PartialEq`, which makes it easy
//! to compare in tests.
//!
//! ## Example
//!
//! ```
//! use cir_to_compiler_ir::CIRLoweringError;
//!
//! let e = CIRLoweringError::new("mul not supported in v1 IR");
//! assert!(e.to_string().contains("mul"));
//! ```

use std::fmt;

// ===========================================================================
// CIRLoweringError
// ===========================================================================

/// The error returned when a CIR instruction cannot be lowered to `IrProgram`.
///
/// This arises when:
/// - The CIR op requires an `IrOp` that does not yet exist (`MUL`, `DIV`, `OR`,
///   `XOR`, `NOT`, float ops).
/// - The CIR instruction references runtime-only services (`call_runtime`,
///   `io_in`, `io_out`) that have no IR equivalent in v1.
/// - The instruction list is structurally invalid (missing operands, etc.).
///
/// # Stability
///
/// The `message` field is human-readable and subject to change. Do not
/// pattern-match on its contents in production code — pattern-match on the
/// type itself instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CIRLoweringError {
    /// Human-readable description of what went wrong.
    pub message: String,
}

impl CIRLoweringError {
    /// Create a new `CIRLoweringError` with the given message.
    ///
    /// # Example
    ///
    /// ```
    /// use cir_to_compiler_ir::CIRLoweringError;
    /// let e = CIRLoweringError::new("example error");
    /// assert_eq!(e.message, "example error");
    /// ```
    pub fn new(message: impl Into<String>) -> Self {
        CIRLoweringError {
            message: message.into(),
        }
    }
}

impl fmt::Display for CIRLoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CIRLoweringError: {}", self.message)
    }
}

impl std::error::Error for CIRLoweringError {}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stores_message() {
        let e = CIRLoweringError::new("test message");
        assert_eq!(e.message, "test message");
    }

    #[test]
    fn test_display_contains_message() {
        let e = CIRLoweringError::new("float ops unsupported");
        assert!(e.to_string().contains("float ops unsupported"));
        assert!(e.to_string().contains("CIRLoweringError"));
    }

    #[test]
    fn test_clone_is_equal() {
        let e1 = CIRLoweringError::new("x");
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_different_messages_are_not_equal() {
        let e1 = CIRLoweringError::new("a");
        let e2 = CIRLoweringError::new("b");
        assert_ne!(e1, e2);
    }

    #[test]
    fn test_error_trait_impl() {
        let e = CIRLoweringError::new("err");
        // std::error::Error is object-safe — we can use it as a trait object.
        let _boxed: Box<dyn std::error::Error> = Box::new(e);
    }

    #[test]
    fn test_accepts_string_or_str() {
        let _e1 = CIRLoweringError::new("str literal");
        let _e2 = CIRLoweringError::new("string".to_string());
    }
}
