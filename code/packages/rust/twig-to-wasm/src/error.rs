//! # error — `TwigToWasmError`
//!
//! The unified error type for the end-to-end Twig → WASM pipeline.
//!
//! ## Why one error type?
//!
//! Each stage in the pipeline has its own error type:
//!
//! | Stage               | Crate error type      |
//! |---------------------|-----------------------|
//! | Lex + parse + compile | `TwigCompileError`  |
//! | WASM lowering       | `IIRWasmError`        |
//!
//! Callers of `compile_twig_to_wasm` should not need to import all upstream
//! crates just to match on errors.  `TwigToWasmError` wraps them all under
//! one roof with a `stage` tag so the caller knows which step failed.
//!
//! ## Stage ordering
//!
//! ```text
//! 1. CompileError   — Twig → IIR failed (syntax, unbound name, etc.)
//! 2. WasmError      — IIR → WASM lowering or encoding failed
//! ```

use std::fmt;

use iir_to_wasm::IIRWasmError;
use twig_ir_compiler::TwigCompileError;

// ---------------------------------------------------------------------------
// TwigToWasmError
// ---------------------------------------------------------------------------

/// Pipeline error for `compile_twig_to_wasm`.
///
/// Each variant corresponds to exactly one stage in the pipeline.
///
/// ```rust
/// use twig_to_wasm::{compile_twig_to_wasm, TwigToWasmError};
///
/// match compile_twig_to_wasm("(bad syntax (((", "bad") {
///     Err(TwigToWasmError::CompileError(e)) => {
///         eprintln!("Twig compile error: {e}");
///     }
///     Err(e) => eprintln!("other error: {e}"),
///     Ok(_)  => unreachable!(),
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum TwigToWasmError {
    /// Stage 1 failed: Twig lex/parse/compile could not produce an IIR module.
    ///
    /// This covers syntax errors, unbound variable references, and any other
    /// error the Twig frontend can emit.
    CompileError(TwigCompileError),

    /// Stage 4 failed: IIR → WASM lowering or binary encoding failed.
    ///
    /// This typically means the IIR module contained a type hint or operation
    /// that the WASM backend does not support (e.g. `"str"` type, `call_builtin`
    /// that was not lowered by the builtin-lowering pass, `"any"` type hint on
    /// an instruction that the type checker could not resolve).
    WasmError(IIRWasmError),

    /// Stage 5 failed: WASM binary encoding failed.
    ///
    /// The `wasm-module-encoder` reports errors as `String` messages.  This
    /// variant wraps them so the caller does not need a separate match arm.
    EncodeError(String),
}

// ── From impls for the `?` operator ──────────────────────────────────────────

impl From<TwigCompileError> for TwigToWasmError {
    fn from(e: TwigCompileError) -> Self {
        TwigToWasmError::CompileError(e)
    }
}

impl From<IIRWasmError> for TwigToWasmError {
    fn from(e: IIRWasmError) -> Self {
        TwigToWasmError::WasmError(e)
    }
}

// ── Display ──────────────────────────────────────────────────────────────────

impl fmt::Display for TwigToWasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TwigToWasmError::CompileError(e) => {
                write!(f, "twig compile error: {e}")
            }
            TwigToWasmError::WasmError(e) => {
                write!(f, "WASM lowering error: {e}")
            }
            TwigToWasmError::EncodeError(msg) => {
                write!(f, "WASM encoding error: {msg}")
            }
        }
    }
}

impl std::error::Error for TwigToWasmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TwigToWasmError::CompileError(e) => Some(e),
            TwigToWasmError::WasmError(e) => Some(e),
            TwigToWasmError::EncodeError(_) => None,
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
    fn wasm_error_display_contains_stage_prefix() {
        let e = TwigToWasmError::WasmError(IIRWasmError::ValidationFailed(vec![
            "EmptyModule".into(),
        ]));
        let s = format!("{e}");
        assert!(s.contains("WASM"), "Display should mention stage; got: {s}");
        assert!(s.contains("EmptyModule"));
    }

    #[test]
    fn encode_error_display() {
        let e = TwigToWasmError::EncodeError("too many sections".into());
        let s = format!("{e}");
        assert!(s.contains("encoding"));
        assert!(s.contains("too many sections"));
    }

    #[test]
    fn error_implements_std_error() {
        let e = TwigToWasmError::WasmError(IIRWasmError::ValidationFailed(vec![]));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn from_iir_wasm_error() {
        let inner = IIRWasmError::ValidationFailed(vec![]);
        let wrapped: TwigToWasmError = inner.into();
        assert!(matches!(wrapped, TwigToWasmError::WasmError(_)));
    }
}
