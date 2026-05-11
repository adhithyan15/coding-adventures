//! # error — `TwigToBeamError`
//!
//! The unified error type for the end-to-end Twig → BEAM pipeline.
//!
//! ## Why one error type?
//!
//! Each stage in the pipeline has its own error type:
//!
//! | Stage               | Crate error type      |
//! |---------------------|-----------------------|
//! | Lex + parse + compile | `TwigCompileError`  |
//! | BEAM lowering       | `IIRBeamError`        |
//!
//! Callers of `compile_twig_to_beam` should not need to import all four
//! crates just to match on errors.  `TwigToBeamError` wraps all of them
//! under one roof with a `stage` tag so the caller knows which step failed.
//!
//! ## Stage ordering
//!
//! ```text
//! 1. CompileError   — Twig → IIR failed (syntax, unbound name, etc.)
//! 2. BeamError      — IIR → BEAM lowering failed
//! ```
//!
//! Note: the type-checker and builtin-lowering passes are currently
//! infallible for Twig input (the type-checker returns a report, not a
//! `Result`; the builtin lowerer never hard-fails).  If either becomes
//! fallible in the future, a new variant can be added here without breaking
//! existing match arms (Rust non-exhaustive enums via `#[non_exhaustive]`).

use std::fmt;

use iir_to_beam::IIRBeamError;
use twig_ir_compiler::TwigCompileError;

// ---------------------------------------------------------------------------
// TwigToBeamError
// ---------------------------------------------------------------------------

/// Pipeline error for `compile_twig_to_beam`.
///
/// Each variant corresponds to exactly one stage in the pipeline.
/// The stage names are stable strings, usable in error messages.
///
/// ```rust
/// use twig_to_beam::{compile_twig_to_beam, TwigToBeamError};
///
/// match compile_twig_to_beam("(bad syntax (((", "bad") {
///     Err(TwigToBeamError::CompileError(e)) => {
///         eprintln!("Twig compile error: {e}");
///     }
///     Err(e) => eprintln!("other error: {e}"),
///     Ok(_)  => unreachable!(),
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum TwigToBeamError {
    /// Stage 1 failed: Twig lex/parse/compile could not produce an IIR module.
    ///
    /// This covers syntax errors, unbound variable references, and any other
    /// error the Twig frontend can emit.
    CompileError(TwigCompileError),

    /// Stage 4 failed: IIR → BEAM lowering failed.
    ///
    /// This typically means the IIR module contained an operation or type that
    /// the BEAM backend does not support (e.g. float arithmetic — BEAM uses
    /// tagged integers; floats require a separate heap object).
    BeamError(IIRBeamError),
}

// ── From impls for the `?` operator ──────────────────────────────────────────

impl From<TwigCompileError> for TwigToBeamError {
    fn from(e: TwigCompileError) -> Self {
        TwigToBeamError::CompileError(e)
    }
}

impl From<IIRBeamError> for TwigToBeamError {
    fn from(e: IIRBeamError) -> Self {
        TwigToBeamError::BeamError(e)
    }
}

// ── Display ──────────────────────────────────────────────────────────────────

impl fmt::Display for TwigToBeamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwigToBeamError::CompileError(e) => {
                write!(f, "twig compile error: {e}")
            }
            TwigToBeamError::BeamError(e) => {
                write!(f, "BEAM lowering error: {e}")
            }
        }
    }
}

impl std::error::Error for TwigToBeamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TwigToBeamError::CompileError(e) => Some(e),
            TwigToBeamError::BeamError(e) => Some(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beam_error_display_contains_stage_prefix() {
        // Build a ValidationFailed error to test the Display wrapping.
        let e = TwigToBeamError::BeamError(IIRBeamError::ValidationFailed(vec![
            "EmptyModule".into(),
        ]));
        let s = format!("{e}");
        assert!(s.contains("BEAM"), "Display should mention stage; got: {s}");
        assert!(s.contains("EmptyModule"));
    }

    #[test]
    fn error_implements_std_error() {
        let e = TwigToBeamError::BeamError(IIRBeamError::ValidationFailed(vec![]));
        // Must be usable as `&dyn std::error::Error`.
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn from_iir_beam_error() {
        let inner = IIRBeamError::ValidationFailed(vec![]);
        let wrapped: TwigToBeamError = inner.into();
        assert!(matches!(wrapped, TwigToBeamError::BeamError(_)));
    }
}
