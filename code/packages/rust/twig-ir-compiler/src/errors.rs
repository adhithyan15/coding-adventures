//! Compile-time errors raised by the Twig → IIR compiler.
//!
//! These are language-level errors — typos, references to unbound names,
//! lambda captures that don't resolve to anything in scope.  Parse errors
//! and lexer errors propagate via `From<TwigParseError>` so callers only
//! ever need to handle one error type at the public entry point.

use std::fmt;

use twig_parser::TwigParseError;

/// Compile-time error.
///
/// Source positions (`line` / `column`) are 1-indexed and match the
/// position the parser AST node carried.  Callers can format these into
/// LSP-style diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwigCompileError {
    /// Human-readable description.
    pub message: String,
    /// 1-indexed source line.
    pub line: usize,
    /// 1-indexed source column.
    pub column: usize,
}

impl fmt::Display for TwigCompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TwigCompileError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for TwigCompileError {}

impl From<TwigParseError> for TwigCompileError {
    fn from(e: TwigParseError) -> Self {
        TwigCompileError {
            message: format!("parse error: {}", e.message),
            line: e.line,
            column: e.column,
        }
    }
}
