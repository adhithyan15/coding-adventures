//! `Backend` — the pluggable code-generation and execution interface for jit-core.
//!
//! A `Backend` translates a `list[CIRInstr]` into an opaque binary blob and
//! executes that blob against a slice of runtime [`Value`]s.  The binary
//! format is entirely backend-defined — it may be native machine code,
//! WASM, JVM bytecode, a compact bytecode for a mini-interpreter, etc.
//!
//! # Why a trait, not a generic parameter?
//!
//! `JITCore` holds a `Box<dyn Backend>` so it can be constructed without
//! knowing the concrete backend type at compile time.  This mirrors the
//! Python `BackendProtocol` structural type.
//!
//! # Implementing a backend
//!
//! ```rust,ignore
//! use jit_core::backend::Backend;
//! use jit_core::cir::CIRInstr;
//! use vm_core::value::Value;
//!
//! /// A trivial "do nothing" backend useful for testing.
//! pub struct NullBackend;
//!
//! impl Backend for NullBackend {
//!     fn name(&self) -> &str { "null" }
//!
//!     fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> {
//!         // Return a single 0-byte sentinel binary.
//!         Some(vec![0u8])
//!     }
//!
//!     fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
//!         Value::Null
//!     }
//! }
//! ```
//!
//! # Security
//!
//! The `Backend` trait makes no promises about sandboxing.  A backend that
//! executes native machine code (e.g. a future `x86-64` or `aarch64` backend)
//! must enforce its own memory-safety invariants.  The `jit-core` crate itself
//! is safe Rust throughout.

use crate::cir::CIRInstr;
use vm_core::value::Value;

// ---------------------------------------------------------------------------
// Backend trait
// ---------------------------------------------------------------------------

/// A pluggable compilation and execution backend for jit-core.
///
/// # Contract
///
/// - `compile` is **pure** — it must not mutate shared state.  The same IR
///   must always produce semantically equivalent binaries (though the bytes
///   may differ between calls if the backend uses non-deterministic code gen).
///
/// - `run` is **pure with respect to the JIT cache** — it must not mutate
///   any `JITCore` or `JITCache` state.  It may mutate external I/O or heap,
///   matching the semantics of the original program.
///
/// - If `compile` returns `None`, the function falls back to interpreted
///   execution permanently (the JIT cache entry is never created).
///
/// # Thread safety
///
/// `Backend` requires `Send + Sync` so the trait object can be held behind
/// an `Arc` and potentially used from multiple threads in future.
pub trait Backend: Send + Sync {
    /// A short human-readable name for this backend (e.g. `"wasm"`, `"interp"`).
    ///
    /// Stored in `JITCacheEntry.backend_name` for diagnostics and `cache_stats()`.
    fn name(&self) -> &str;

    /// Compile a CIR instruction sequence into an opaque binary.
    ///
    /// Returns `None` when compilation is not possible (unsupported opcodes,
    /// register-count too large, etc.).  `JITCore` treats `None` as a
    /// compilation failure and keeps the function on the interpreter tier.
    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>>;

    /// Execute a previously compiled binary with the given argument values.
    ///
    /// `binary` is the slice returned by the last successful `compile` call
    /// for this function.  `args` are the resolved parameter values in
    /// declaration order.
    ///
    /// Returns the function's return value, or `Value::Null` for void
    /// functions.
    fn run(&self, binary: &[u8], args: &[Value]) -> Value;

    /// Compile with full function context — name, parameters, return type.
    ///
    /// Native backends (e.g. an ARM64 backend) need this richer signature
    /// to lay out the AAPCS64 prologue (which registers carry which params)
    /// and to choose the right `ret_*` width.  IR-only backends (NullBackend,
    /// EchoBackend, WASM) typically ignore the context and just call
    /// [`Self::compile`] — that's the default implementation.
    ///
    /// Callers (jit-core, aot-core) should prefer this method over
    /// [`Self::compile`] when they have an `IIRFunction` in hand.
    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }
}

/// Read-only view of an `IIRFunction`'s shape passed to
/// [`Backend::compile_function`].
///
/// Lifetimes match the originating function so no cloning is needed in the
/// hot path — backends typically read these fields once during prologue
/// emission and discard them.
#[derive(Debug, Clone, Copy)]
pub struct FunctionContext<'a> {
    /// Function name (e.g. `"main"`, `"fib"`).  Used for label generation
    /// and any debug info the backend chooses to emit.
    pub name: &'a str,
    /// Parameter list — `(name, type_str)` in declaration order.  AAPCS64
    /// backends marshal these from `x0..x7` into virtual-register slots.
    pub params: &'a [(String, String)],
    /// Return type string (e.g. `"u8"`, `"void"`).  Determines whether the
    /// backend expects a `ret_*` mnemonic and how to package the return
    /// value into `x0`.
    pub return_type: &'a str,
}

// ---------------------------------------------------------------------------
// NullBackend — for testing
// ---------------------------------------------------------------------------

/// A no-op backend that always "compiles" (returns a 1-byte sentinel) and
/// always returns `Value::Null` when run.
///
/// Useful for unit tests that need a concrete `Backend` without writing
/// a real code generator.
///
/// # Example
///
/// ```
/// use jit_core::backend::{Backend, NullBackend};
/// use jit_core::cir::CIRInstr;
/// use vm_core::value::Value;
///
/// let b = NullBackend;
/// let binary = b.compile(&[]).unwrap();
/// assert_eq!(b.run(&binary, &[]), Value::Null);
/// assert_eq!(b.name(), "null");
/// ```
pub struct NullBackend;

impl Backend for NullBackend {
    fn name(&self) -> &str {
        "null"
    }

    fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> {
        // A single-byte sentinel; the run() method ignores the content.
        Some(vec![0u8])
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        Value::Null
    }
}

/// A backend that returns the first argument unchanged (identity function).
///
/// Useful for testing the full JIT pipeline end-to-end without needing a
/// real code generator: the compiled "binary" records the single argument
/// name; `run` just echoes the first argument.
pub struct EchoBackend;

impl Backend for EchoBackend {
    fn name(&self) -> &str {
        "echo"
    }

    fn compile(&self, _ir: &[CIRInstr]) -> Option<Vec<u8>> {
        Some(vec![0u8])
    }

    fn run(&self, _binary: &[u8], args: &[Value]) -> Value {
        args.first().cloned().unwrap_or(Value::Null)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_backend_name() {
        assert_eq!(NullBackend.name(), "null");
    }

    #[test]
    fn null_backend_compile_always_succeeds() {
        let binary = NullBackend.compile(&[]);
        assert!(binary.is_some());
        assert_eq!(binary.unwrap(), vec![0u8]);
    }

    #[test]
    fn null_backend_run_returns_null() {
        let result = NullBackend.run(&[0u8], &[Value::Int(42)]);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn echo_backend_name() {
        assert_eq!(EchoBackend.name(), "echo");
    }

    #[test]
    fn echo_backend_run_returns_first_arg() {
        let result = EchoBackend.run(&[0u8], &[Value::Int(7), Value::Int(99)]);
        assert_eq!(result, Value::Int(7));
    }

    #[test]
    fn echo_backend_run_no_args_returns_null() {
        let result = EchoBackend.run(&[0u8], &[]);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn backend_is_dyn_safe() {
        // Make sure Backend can be used as a trait object.
        let b: Box<dyn Backend> = Box::new(NullBackend);
        assert_eq!(b.name(), "null");
    }
}
