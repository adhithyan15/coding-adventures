//! Error types for the IIR codegen adapter layer.
//!
//! [`IIRAdapterError`] is the single error type returned by [`crate::compile_iir`].
//! It consolidates the four different backend error types into one clean variant
//! set that callers can pattern-match on.
//!
//! # Variant overview
//!
//! | Variant | When raised |
//! |---------|------------|
//! | `UnknownBackend` | The caller supplied a name not in the registry |
//! | `ValidationFailed` | The module failed the backend's pre-flight checks |
//! | `LoweringFailed` | The lowering step returned an unexpected error |
//!
//! # Example
//!
//! ```rust
//! use iir_codegen_adapters::{compile_iir, IIRAdapterError};
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
//!
//! // An empty module fails validation (EmptyModule) on every backend.
//! let empty = IIRModule { name: "e".into(), functions: vec![], entry_point: None, language: "t".into() };
//!
//! match compile_iir(&empty, "iir-wasm") {
//!     Err(IIRAdapterError::ValidationFailed { backend, errors }) => {
//!         assert_eq!(backend, "iir-wasm");
//!         assert!(!errors.is_empty());
//!     }
//!     other => panic!("unexpected: {:?}", other),
//! }
//! ```

use std::fmt;

// ===========================================================================
// IIRAdapterError
// ===========================================================================

/// Error returned by [`crate::compile_iir`] when compilation cannot complete.
#[derive(Debug, Clone, PartialEq)]
pub enum IIRAdapterError {
    /// The requested backend name is not registered.
    ///
    /// This means the caller passed a string like `"x86"` or `"llvm"` that
    /// has no entry in the IIR codegen registry.  Check `list_iir_backends()`
    /// for the accepted names.
    UnknownBackend {
        /// The name the caller provided.
        requested: String,
        /// The names that *are* registered, sorted alphabetically.
        available: Vec<String>,
    },

    /// The `IIRModule` failed pre-flight validation for the chosen backend.
    ///
    /// Each backend's `validate()` method returns a list of human-readable
    /// error strings.  All of them are collected here so the caller can
    /// present them together rather than stopping at the first one.
    ValidationFailed {
        /// The backend identifier (e.g. `"iir-wasm"`).
        backend: String,
        /// One error string per failing check, in the order they were found.
        errors: Vec<String>,
    },

    /// The lowering step returned an error after validation passed.
    ///
    /// This should be rare — `validate()` is supposed to catch everything —
    /// but can happen if the module contains a combination of features that
    /// validation does not cover (a bug in the validator) or if an internal
    /// limit is hit.
    LoweringFailed {
        /// The backend identifier.
        backend: String,
        /// The raw error message from the backend.
        detail: String,
    },
}

impl fmt::Display for IIRAdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // ── UnknownBackend ────────────────────────────────────────────────
            //
            // List all known backends in the message so the caller doesn't have
            // to make a second call to find out what names are valid.
            IIRAdapterError::UnknownBackend { requested, available } => {
                let avail = if available.is_empty() {
                    "<none registered>".to_string()
                } else {
                    available.join(", ")
                };
                write!(
                    f,
                    "Unknown IIR backend {:?}. Available backends: {}",
                    requested, avail
                )
            }

            // ── ValidationFailed ─────────────────────────────────────────────
            //
            // Show all errors on numbered lines so they are easy to read in a
            // terminal or log file.
            IIRAdapterError::ValidationFailed { backend, errors } => {
                write!(
                    f,
                    "IIRModule failed validation for backend {:?} ({} error{}):",
                    backend,
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" }
                )?;
                for (i, e) in errors.iter().enumerate() {
                    write!(f, "\n  {}. {}", i + 1, e)?;
                }
                Ok(())
            }

            // ── LoweringFailed ────────────────────────────────────────────────
            IIRAdapterError::LoweringFailed { backend, detail } => {
                write!(
                    f,
                    "Lowering to backend {:?} failed unexpectedly: {}",
                    backend, detail
                )
            }
        }
    }
}

impl std::error::Error for IIRAdapterError {}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_backend_display_lists_available() {
        let e = IIRAdapterError::UnknownBackend {
            requested: "x86".into(),
            available: vec!["iir-beam".into(), "iir-wasm".into()],
        };
        let s = e.to_string();
        assert!(s.contains("x86"), "should mention the requested name");
        assert!(s.contains("iir-beam"), "should list available backends");
        assert!(s.contains("iir-wasm"), "should list available backends");
    }

    #[test]
    fn unknown_backend_display_empty_registry() {
        let e = IIRAdapterError::UnknownBackend {
            requested: "foo".into(),
            available: vec![],
        };
        let s = e.to_string();
        assert!(s.contains("<none registered>"));
    }

    #[test]
    fn validation_failed_display_single_error() {
        let e = IIRAdapterError::ValidationFailed {
            backend: "iir-wasm".into(),
            errors: vec!["EmptyModule: module has no functions".into()],
        };
        let s = e.to_string();
        assert!(s.contains("iir-wasm"));
        assert!(s.contains("EmptyModule"));
        assert!(s.contains("1 error")); // singular
    }

    #[test]
    fn validation_failed_display_multiple_errors() {
        let e = IIRAdapterError::ValidationFailed {
            backend: "iir-jvm".into(),
            errors: vec!["err1".into(), "err2".into(), "err3".into()],
        };
        let s = e.to_string();
        assert!(s.contains("3 errors")); // plural
        assert!(s.contains("err1"));
        assert!(s.contains("err3"));
    }

    #[test]
    fn lowering_failed_display() {
        let e = IIRAdapterError::LoweringFailed {
            backend: "iir-clr".into(),
            detail: "method token overflow".into(),
        };
        let s = e.to_string();
        assert!(s.contains("iir-clr"));
        assert!(s.contains("method token overflow"));
    }

    #[test]
    fn error_is_std_error() {
        // Confirm we implement std::error::Error (compile check).
        let e: Box<dyn std::error::Error> = Box::new(IIRAdapterError::UnknownBackend {
            requested: "x".into(),
            available: vec![],
        });
        let _ = e.to_string();
    }

    #[test]
    fn clone_and_eq() {
        let e = IIRAdapterError::ValidationFailed {
            backend: "iir-beam".into(),
            errors: vec!["UnsupportedType: float".into()],
        };
        assert_eq!(e.clone(), e);
    }
}
