//! Error types for jit-core.
//!
//! # Error hierarchy
//!
//! ```text
//! JITError
//!   ├── DeoptimizerError  — function deopt rate exceeded the 10 % threshold
//!   └── UnspecializableError — compile() called on a permanently invalidated fn
//! ```
//!
//! All errors implement `std::error::Error` so they compose naturally with
//! the `?` operator and crates like `anyhow`.
//!
//! # When each error fires
//!
//! | Error | When |
//! |---|---|
//! | `DeoptimizerError` | Internally when deopt_count / exec_count > 0.1; callers fall back to interpreter |
//! | `UnspecializableError` | When [`JITCore::compile`] is called on a function that has been permanently invalidated |

use std::fmt;

// ---------------------------------------------------------------------------
// JITError — base variant
// ---------------------------------------------------------------------------

/// Base error type for all jit-core failures.
///
/// Use pattern matching or `downcast_ref` to distinguish between the two
/// concrete variants.
#[derive(Debug)]
pub enum JITError {
    /// A compiled function deopted too frequently and was invalidated.
    Deoptimizer(DeoptimizerError),
    /// Compilation was attempted on a permanently invalidated function.
    Unspecializable(UnspecializableError),
    /// A generic JIT failure (compilation returned `None`, invalid IR, etc.).
    CompilationFailed(String),
}

impl fmt::Display for JITError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JITError::Deoptimizer(e) => write!(f, "deoptimizer: {e}"),
            JITError::Unspecializable(e) => write!(f, "unspecializable: {e}"),
            JITError::CompilationFailed(msg) => write!(f, "compilation failed: {msg}"),
        }
    }
}

impl std::error::Error for JITError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JITError::Deoptimizer(e) => Some(e),
            JITError::Unspecializable(e) => Some(e),
            JITError::CompilationFailed(_) => None,
        }
    }
}

impl From<DeoptimizerError> for JITError {
    fn from(e: DeoptimizerError) -> Self {
        JITError::Deoptimizer(e)
    }
}

impl From<UnspecializableError> for JITError {
    fn from(e: UnspecializableError) -> Self {
        JITError::Unspecializable(e)
    }
}

// ---------------------------------------------------------------------------
// DeoptimizerError
// ---------------------------------------------------------------------------

/// Raised when a compiled function deopts too frequently.
///
/// The JIT permanently invalidates a function when
/// `deopt_count / exec_count > 0.1` (10 %).  After that point the function
/// runs interpreted forever — recompiling a chronically deopting function
/// wastes CPU without improving throughput.
///
/// This error is mostly internal; callers of [`JITCore::execute_with_jit`]
/// observe the fallback to the interpreter rather than this error directly.
#[derive(Debug, Clone)]
pub struct DeoptimizerError {
    /// Name of the function that was invalidated.
    pub fn_name: String,
    /// Deopt count at the time of invalidation.
    pub deopt_count: u64,
    /// Execution count at the time of invalidation.
    pub exec_count: u64,
}

impl DeoptimizerError {
    /// Construct a new `DeoptimizerError`.
    pub fn new(fn_name: impl Into<String>, deopt_count: u64, exec_count: u64) -> Self {
        DeoptimizerError {
            fn_name: fn_name.into(),
            deopt_count,
            exec_count,
        }
    }

    /// The computed deopt rate at the time of invalidation.
    pub fn deopt_rate(&self) -> f64 {
        if self.exec_count == 0 {
            0.0
        } else {
            self.deopt_count as f64 / self.exec_count as f64
        }
    }
}

impl fmt::Display for DeoptimizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "function {:?} invalidated: deopt_rate={:.1}% ({}/{} executions)",
            self.fn_name,
            self.deopt_rate() * 100.0,
            self.deopt_count,
            self.exec_count,
        )
    }
}

impl std::error::Error for DeoptimizerError {}

// ---------------------------------------------------------------------------
// UnspecializableError
// ---------------------------------------------------------------------------

/// Raised when [`JITCore::compile`] is called on a permanently invalidated
/// function.
///
/// A function becomes unspecializable after its deopt rate exceeds 10 %.
/// Attempting to compile it again would just repeat the deoptimization cycle.
#[derive(Debug, Clone)]
pub struct UnspecializableError {
    /// Name of the function that cannot be (re-)compiled.
    pub fn_name: String,
}

impl UnspecializableError {
    /// Construct a new `UnspecializableError`.
    pub fn new(fn_name: impl Into<String>) -> Self {
        UnspecializableError {
            fn_name: fn_name.into(),
        }
    }
}

impl fmt::Display for UnspecializableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "function {:?} is marked unspecializable (deopt rate exceeded)",
            self.fn_name,
        )
    }
}

impl std::error::Error for UnspecializableError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn deoptimizer_error_display() {
        let e = DeoptimizerError::new("add", 5, 40);
        let s = e.to_string();
        assert!(s.contains("add"), "function name should appear in message");
        assert!(s.contains("12.5"), "deopt rate should appear: {s}");
    }

    #[test]
    fn deoptimizer_error_rate() {
        let e = DeoptimizerError::new("f", 1, 4);
        assert!((e.deopt_rate() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn deoptimizer_error_rate_zero_exec() {
        let e = DeoptimizerError::new("f", 0, 0);
        assert_eq!(e.deopt_rate(), 0.0);
    }

    #[test]
    fn unspecializable_display() {
        let e = UnspecializableError::new("hot_fn");
        assert!(e.to_string().contains("hot_fn"));
        assert!(e.to_string().contains("unspecializable"));
    }

    #[test]
    fn jit_error_from_deoptimizer() {
        let inner = DeoptimizerError::new("g", 2, 10);
        let e: JITError = inner.into();
        let s = e.to_string();
        assert!(s.starts_with("deoptimizer:"), "got: {s}");
    }

    #[test]
    fn jit_error_from_unspecializable() {
        let inner = UnspecializableError::new("g");
        let e: JITError = inner.into();
        let s = e.to_string();
        assert!(s.starts_with("unspecializable:"), "got: {s}");
    }

    #[test]
    fn jit_error_compilation_failed() {
        let e = JITError::CompilationFailed("backend returned None".into());
        assert!(e.to_string().contains("compilation failed"));
    }

    #[test]
    fn jit_error_has_source() {
        let e = JITError::Deoptimizer(DeoptimizerError::new("f", 1, 5));
        assert!(e.source().is_some());
    }

    #[test]
    fn jit_error_compilation_failed_no_source() {
        let e = JITError::CompilationFailed("x".into());
        assert!(e.source().is_none());
    }
}
