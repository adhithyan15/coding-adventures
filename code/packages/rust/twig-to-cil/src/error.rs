//! # error — `TwigToCilError`
//!
//! The unified error type for the `twig-to-cil` pipeline.
//!
//! ## Pipeline stages and their error types
//!
//! | Stage                 | Error type      | Notes |
//! |-----------------------|-----------------|-------|
//! | `compile_source`      | `TwigCompileError` | Parse + IIR compile |
//! | `infer_and_check`     | `TypeCheckReport` (not `Result`) | Fatal errors surfaced via `.ok()` |
//! | `lower_builtins`      | (infallible)    | Always succeeds |
//! | `validate_iir_for_clr` | `Vec<String>`  | Pre-flight validation |
//! | `lower_iir_to_cil`    | `IIRClrError`   | Lowering errors |
//!
//! `TwigToCilError` wraps each stage's error type into a single enum so
//! callers match once and handle cleanly.

use twig_ir_compiler::TwigCompileError;
use iir_to_cil_bytecode::IIRClrError;

// ---------------------------------------------------------------------------
// TwigToCilError
// ---------------------------------------------------------------------------

/// Unified error type for the `twig-to-cil` compilation pipeline.
///
/// ## Example
///
/// ```rust,no_run
/// use twig_to_cil::{compile_twig_to_cil, error::TwigToCilError};
///
/// // Bad syntax always produces a Compile error.
/// let result = compile_twig_to_cil("(+ 1", "broken");
/// assert!(matches!(result, Err(TwigToCilError::Compile(_))));
/// ```
#[derive(Debug)]
pub enum TwigToCilError {
    /// The Twig frontend (lexer, parser, or IIR compiler) rejected the source.
    ///
    /// Carries the full `TwigCompileError` with message and source location.
    Compile(TwigCompileError),

    /// The IIR type-checker found fatal errors after inference.
    ///
    /// Each `String` is a formatted `TypeCheckError` message.  The pipeline
    /// aborts when `TypeCheckReport::ok()` returns `false`.
    TypeCheck(Vec<String>),

    /// The pre-flight CLR validation pass found errors.
    ///
    /// Each `String` is a message from `validate_iir_for_clr`.  These indicate
    /// that the IIR module contains instructions the CLR backend cannot handle.
    ClrValidation(Vec<String>),

    /// The CLR lowering pass (`lower_iir_to_cil`) returned an error.
    ///
    /// `IIRClrError` carries a structured description of the failure
    /// (unsupported op, undefined label, assembly error, etc.).
    ClrBackend(IIRClrError),
}

impl std::fmt::Display for TwigToCilError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwigToCilError::Compile(e) => {
                write!(f, "Twig compilation failed: {e}")
            }
            TwigToCilError::TypeCheck(errs) => {
                write!(
                    f,
                    "Type check failed ({} error(s)): {}",
                    errs.len(),
                    errs.first().map(String::as_str).unwrap_or("unknown")
                )
            }
            TwigToCilError::ClrValidation(errs) => {
                write!(
                    f,
                    "CLR validation failed ({} error(s)): {}",
                    errs.len(),
                    errs.first().map(String::as_str).unwrap_or("unknown")
                )
            }
            TwigToCilError::ClrBackend(e) => {
                write!(f, "CLR backend error: {e}")
            }
        }
    }
}

impl std::error::Error for TwigToCilError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TwigToCilError::Compile(e) => Some(e),
            TwigToCilError::TypeCheck(_) => None,
            TwigToCilError::ClrValidation(_) => None,
            TwigToCilError::ClrBackend(e) => Some(e),
        }
    }
}

impl From<TwigCompileError> for TwigToCilError {
    fn from(e: TwigCompileError) -> Self {
        TwigToCilError::Compile(e)
    }
}

impl From<IIRClrError> for TwigToCilError {
    fn from(e: IIRClrError) -> Self {
        TwigToCilError::ClrBackend(e)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_check_display_mentions_error_count() {
        let e = TwigToCilError::TypeCheck(vec![
            "instruction 'add' has type 'any'".into(),
            "instruction 'sub' has type 'any'".into(),
        ]);
        let s = format!("{e}");
        assert!(s.contains("2"), "display should mention error count");
        assert!(s.contains("Type check"), "display should identify stage");
    }

    #[test]
    fn clr_validation_display_mentions_count() {
        let e = TwigToCilError::ClrValidation(vec!["unsupported op: io_out".into()]);
        let s = format!("{e}");
        assert!(s.contains("1"), "should mention error count");
        assert!(s.contains("CLR validation"), "should identify stage");
    }

    #[test]
    fn compile_error_display_mentions_stage() {
        use twig_ir_compiler::TwigCompileError;
        let inner = TwigCompileError { message: "oops".into(), line: 1, column: 1 };
        let e = TwigToCilError::Compile(inner);
        let s = format!("{e}");
        assert!(s.contains("compilation"), "display should mention compilation");
    }

    #[test]
    fn from_twig_compile_error_conversion() {
        use twig_ir_compiler::TwigCompileError;
        let inner = TwigCompileError { message: "oops".into(), line: 1, column: 1 };
        let e: TwigToCilError = inner.into();
        assert!(matches!(e, TwigToCilError::Compile(_)));
    }

    #[test]
    fn from_iir_clr_error_conversion() {
        let inner = IIRClrError::UnsupportedOp {
            function: "f".into(),
            op: "io_in".into(),
        };
        let e: TwigToCilError = inner.into();
        assert!(matches!(e, TwigToCilError::ClrBackend(_)));
    }
}
