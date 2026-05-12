//! # error — `TwigToJvmError`
//!
//! The unified error type for the `twig-to-jvm` pipeline.
//!
//! ## Design rationale
//!
//! Each stage of the pipeline has its own error type:
//!
//! | Stage               | Error type          |
//! |---------------------|---------------------|
//! | `compile_source`    | `TwigCompileError`  |
//! | `infer_and_check`   | (TypeCheckReport — see note below) |
//! | `lower_builtins`    | (infallible)        |
//! | JVM backend         | `IIRJvmError`       |
//!
//! > **Note on type checking:** `infer_and_check` returns a `TypeCheckReport`
//! > rather than a `Result`.  The report carries a `Vec<TypeCheckError>` and an
//! > `ok()` method.  If the report is not ok, the pipeline aborts and wraps the
//! > first error string as `TwigToJvmError::TypeCheck`.
//!
//! `TwigToJvmError` is an enum so that callers can match on the specific failure
//! and either display a user-friendly message or propagate as-is.

use twig_ir_compiler::TwigCompileError;
use iir_to_jvm_class_file::IIRJvmError;

// ---------------------------------------------------------------------------
// TwigToJvmError
// ---------------------------------------------------------------------------

/// Unified error type for the `twig-to-jvm` compilation pipeline.
///
/// ## Variants
///
/// - `Compile` — the Twig source could not be lexed/parsed/compiled to IIR.
/// - `TypeCheck` — the type-checker found fatal errors in the IIR module.
///   The inner `Vec<String>` contains one error message per error.
/// - `JvmValidation` — the pre-flight validation before JVM lowering failed.
///   The inner `Vec<String>` contains the validation error messages.
/// - `JvmBackend` — the JVM lowering pass rejected the module (unsupported
///   opcode, invalid structure, etc.).
///
/// ## Example
///
/// ```rust,no_run
/// use twig_to_jvm::error::TwigToJvmError;
/// use twig_to_jvm::compile_twig_to_jvm;
///
/// // Passing broken Twig always returns a Compile error.
/// let bad = compile_twig_to_jvm("(+ 1", "broken");
/// assert!(matches!(bad, Err(TwigToJvmError::Compile(_))));
/// ```
#[derive(Debug)]
pub enum TwigToJvmError {
    /// The Twig frontend (lexer, parser, IIR compiler) rejected the source.
    ///
    /// The inner `TwigCompileError` carries the error message and optional
    /// source-location information.
    Compile(TwigCompileError),

    /// The IIR type-checker found fatal errors after inference.
    ///
    /// Each `String` in the inner vector is a formatted error message from
    /// `iir_type_checker::TypeCheckError`.  The pipeline aborts when
    /// `TypeCheckReport::ok()` returns `false`.
    TypeCheck(Vec<String>),

    /// The pre-flight JVM validation pass found errors before lowering.
    ///
    /// Each `String` in the inner vector is a validation error message from
    /// `iir_to_jvm_class_file::validate_for_jvm`.  These errors indicate
    /// that the IIR module contains instructions the JVM backend cannot
    /// handle (e.g. unsupported builtins that survived lowering, `"any"`
    /// type hints on arithmetic, etc.).
    JvmValidation(Vec<String>),

    /// The JVM lowering pass rejected the IIR module.
    ///
    /// `IIRJvmError` carries the specific reason (unsupported opcode, type
    /// mismatch, etc.).
    JvmBackend(IIRJvmError),
}

impl std::fmt::Display for TwigToJvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwigToJvmError::Compile(e) => {
                write!(f, "Twig compilation failed: {e}")
            }
            TwigToJvmError::TypeCheck(errs) => {
                write!(f, "Type check failed ({} error(s)): {}", errs.len(),
                    errs.first().map(String::as_str).unwrap_or("unknown"))
            }
            TwigToJvmError::JvmValidation(errs) => {
                write!(f, "JVM validation failed ({} error(s)): {}", errs.len(),
                    errs.first().map(String::as_str).unwrap_or("unknown"))
            }
            TwigToJvmError::JvmBackend(e) => {
                write!(f, "JVM backend error: {e}")
            }
        }
    }
}

impl std::error::Error for TwigToJvmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TwigToJvmError::Compile(e) => Some(e),
            TwigToJvmError::TypeCheck(_) => None,
            TwigToJvmError::JvmValidation(_) => None,
            TwigToJvmError::JvmBackend(e) => Some(e),
        }
    }
}

// Conversions from the individual stage error types.

impl From<TwigCompileError> for TwigToJvmError {
    fn from(e: TwigCompileError) -> Self {
        TwigToJvmError::Compile(e)
    }
}

impl From<IIRJvmError> for TwigToJvmError {
    fn from(e: IIRJvmError) -> Self {
        TwigToJvmError::JvmBackend(e)
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
        let e = TwigToJvmError::TypeCheck(vec![
            "instruction 'add' has type 'any'".into(),
            "instruction 'sub' has type 'any'".into(),
        ]);
        let s = format!("{e}");
        assert!(s.contains("2"), "display should mention error count");
        assert!(s.contains("Type check"), "display should identify stage");
    }

    #[test]
    fn compile_variant_display_mentions_stage() {
        use twig_ir_compiler::TwigCompileError;
        let inner = TwigCompileError { message: "test error".into(), line: 1, column: 1 };
        let e = TwigToJvmError::Compile(inner);
        let s = format!("{e}");
        assert!(s.contains("compilation"), "display should mention compilation");
    }

    #[test]
    fn from_twig_compile_error() {
        use twig_ir_compiler::TwigCompileError;
        let inner = TwigCompileError { message: "oops".into(), line: 1, column: 1 };
        let e: TwigToJvmError = inner.into();
        assert!(matches!(e, TwigToJvmError::Compile(_)));
    }
}
