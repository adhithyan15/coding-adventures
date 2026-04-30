//! # `twig-vm` — runtime wiring between Twig and the LANG-runtime.
//!
//! Implementation of LANG20 PRs 3 + 4 from the
//! [migration path](../../specs/LANG20-multilang-runtime.md).  This
//! crate is the bridge between:
//!
//! - the **Twig frontend** (`twig-lexer` → `twig-parser` →
//!   `twig-ir-compiler`) which produces an `IIRModule` from Twig
//!   source, and
//! - the **Lispy runtime** (`lispy-runtime`) which provides the
//!   value representation, builtins, and `LangBinding` impl.
//!
//! It exposes [`TwigVM`] — a thin facade that holds the runtime
//! state needed to compile and execute Twig programs.
//!
//! ## What this crate does (PRs 3 + 4)
//!
//! - **Compilation**: [`TwigVM::compile`] takes Twig source and
//!   returns an `IIRModule` ready for execution.
//! - **Execution**: [`TwigVM::run`] compiles + dispatches a Twig
//!   program end to end, returning the value of the synthesised
//!   `main` function.  See the [`dispatch`] module for the
//!   supported opcode subset.
//! - **Builtin resolution**: [`TwigVM::resolve_builtin`] proxies
//!   through `LispyBinding::resolve_builtin` so callers see the
//!   binding's resolved fn pointer.
//! - **Operand → Value conversion**: [`operand_to_value`] maps an
//!   IIR `Operand` (the universal-IR enum) into a `LispyValue`
//!   (the Lispy runtime's tagged-i64 representation).  This is
//!   the seam between the language-agnostic IIR and the
//!   per-language value model — small but crucial; the dispatcher
//!   hits it on every `call_builtin` argument and `ret` operand.
//!
//! ## What's NOT shipped yet (PR 5+)
//!
//! - **Closures**: `lambda`, `make_closure`, `apply_closure` —
//!   need closure heap layout and indirect dispatch.
//! - **Top-level value defines** (`global_set` / `global_get`) —
//!   need a per-process global table.
//! - **Quoted symbols** (`'foo`) — need a `Symbol` value
//!   constructor from a runtime-string operand.
//! - **Inline caches, send opcodes, JIT promotion, deopt** — full
//!   LANG20 §"Hot-path tactics".
//!
//! Programs using any of those compile (the IR compiler emits
//! valid IIR for them) but the dispatcher returns
//! [`RunError::UnsupportedOpcode`] — explicit "not yet" rather than
//! a silent miscompile.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  twig_lexer → twig_parser → twig_ir_compiler
//! IIRModule
//!     │
//!     ▼  twig-vm::dispatch (PR 4)
//! execution
//!     │
//!     ▼  LispyBinding (lispy-runtime)
//! LispyValue results
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod dispatch;
pub mod operand;

use interpreter_ir::IIRModule;
use lispy_runtime::LispyBinding;
use twig_ir_compiler::compile_source as compile_twig;

pub use dispatch::{run, RunError, MAX_DISPATCH_DEPTH, MAX_INSTRUCTIONS_PER_RUN, MAX_REGISTERS_PER_FRAME};
pub use operand::operand_to_value;

// Re-export the most-used types so callers don't need to depend
// on the upstream crates explicitly.
pub use lang_runtime_core::{BuiltinFn, LangBinding, RuntimeError};
pub use lispy_runtime::LispyValue;
pub use twig_ir_compiler::TwigCompileError;

/// The Twig VM facade — owns the integration between
/// `twig-ir-compiler` (compile-time) and `lispy-runtime` (runtime).
///
/// `TwigVM` is currently stateless; the binding itself
/// ([`LispyBinding`]) is a unit struct, the heap allocator is
/// process-global (PR 2's `Box::leak`), and the symbol intern
/// table is process-global too.  The struct exists so that PR 5+
/// can grow per-VM state (a JIT cache, a globals table, a
/// profile-feedback cache) without breaking callers.
///
/// # Lifecycle
///
/// 1. Construct via [`TwigVM::new`].
/// 2. Compile Twig source via [`TwigVM::compile`] — returns an
///    `IIRModule` ready for execution.
/// 3. Execute via [`TwigVM::run`] — compiles + dispatches in one
///    shot, returning the program's value.
///
/// # Example
///
/// ```
/// use twig_vm::TwigVM;
///
/// let vm = TwigVM::new();
/// let v = vm.run("(+ 1 2)").unwrap();
/// assert_eq!(v.as_int(), Some(3));
/// ```
#[derive(Debug, Default)]
pub struct TwigVM {
    // No fields yet.  PR 5+ adds:
    //   - globals table (for top-level value defines)
    //   - profile-feedback cache (LANG20 §"Feedback-slot taxonomy")
    //   - JIT promotion thresholds and cache
    _private: (),
}

impl TwigVM {
    /// Construct a fresh VM.  All runtime state (intern table,
    /// allocator) is process-global at PR 4, so this is cheap —
    /// callers can construct as many VMs as they want without
    /// leaking memory.
    pub fn new() -> Self {
        TwigVM { _private: () }
    }

    /// Compile Twig source into an `IIRModule`.
    ///
    /// Wraps [`twig_ir_compiler::compile_source`] with the LANG20
    /// convention `module_name = "twig"` so the resulting module
    /// is recognisable in profile dumps.  Callers needing custom
    /// module names should call `compile_source` directly.
    pub fn compile(&self, source: &str) -> Result<IIRModule, TwigCompileError> {
        compile_twig(source, "twig")
    }

    /// Compile with an explicit module name.  Useful when
    /// embedding Twig source from multiple files in one process.
    pub fn compile_with_name(&self, source: &str, module_name: &str) -> Result<IIRModule, TwigCompileError> {
        compile_twig(source, module_name)
    }

    /// Resolve a builtin by name through [`LispyBinding`].
    ///
    /// The returned [`BuiltinFn`] is a stable function pointer the
    /// dispatcher uses on every `call_builtin` opcode.  Callers
    /// rarely need this directly — [`TwigVM::run`] handles the
    /// resolution internally — but it's exposed for tooling
    /// (linters, debuggers) that want to verify a name resolves
    /// before execution.
    ///
    /// Returns `None` for names not in the Lispy builtin set.
    pub fn resolve_builtin(name: &str) -> Option<BuiltinFn<LispyBinding>> {
        LispyBinding::resolve_builtin(name)
    }

    /// Compile Twig `source` and execute it end-to-end, returning
    /// the value produced by the synthesised `main` function.
    ///
    /// This is **PR 4 of LANG20** — the first version of the VM
    /// that can actually run a Twig program.
    ///
    /// # Supported subset
    ///
    /// PR 4 covers the IIR opcodes emitted by `twig-ir-compiler`
    /// for programs without closures, top-level value defines, or
    /// quoted symbols.  In Twig source terms: `if`, `let`, `begin`,
    /// `define`-of-functions, recursion, arithmetic, comparisons,
    /// `cons` / `car` / `cdr`, and the predicates.
    ///
    /// Programs using `lambda`, `(define x value)`, or `'symbol`
    /// will compile (the IR compiler emits valid IIR for them) but
    /// the dispatcher returns
    /// [`RunError::UnsupportedOpcode`](dispatch::RunError) — those
    /// land in PR 5+.
    ///
    /// # Errors
    ///
    /// Returns [`TwigCompileError`] if compilation fails, or
    /// [`RunError`](dispatch::RunError) if execution fails.  The
    /// two error types are wrapped in [`TwigRunError`] so callers
    /// don't need to mix two error families.
    ///
    /// # Example
    ///
    /// ```
    /// use twig_vm::TwigVM;
    ///
    /// let vm = TwigVM::new();
    /// let v = vm.run("(+ 1 2)").unwrap();
    /// assert_eq!(v.as_int(), Some(3));
    ///
    /// let v = vm.run("
    ///     (define (fact n)
    ///       (if (= n 0) 1 (* n (fact (- n 1)))))
    ///     (fact 5)
    /// ").unwrap();
    /// assert_eq!(v.as_int(), Some(120));
    /// ```
    pub fn run(&self, source: &str) -> Result<LispyValue, TwigRunError> {
        let module = self.compile(source).map_err(TwigRunError::Compile)?;
        run(&module).map_err(TwigRunError::Run)
    }
}

// ---------------------------------------------------------------------------
// Combined error type for `TwigVM::run`
// ---------------------------------------------------------------------------

/// Combined error type returned by [`TwigVM::run`].
///
/// Either compilation failed (the source was malformed) or
/// execution failed (the IR was structurally fine but tripped a
/// runtime trap).  Keeping the two variants distinct lets callers
/// distinguish "user typo" from "user logic bug".
#[derive(Debug)]
pub enum TwigRunError {
    /// Compilation failed — the source was malformed.
    Compile(TwigCompileError),
    /// Execution failed — see [`RunError`] for the variant table.
    Run(RunError),
}

impl std::fmt::Display for TwigRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TwigRunError::Compile(e) => write!(f, "compile error: {e}"),
            TwigRunError::Run(e) => write!(f, "run error: {e}"),
        }
    }
}

impl std::error::Error for TwigRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TwigRunError::Compile(e) => Some(e),
            TwigRunError::Run(e) => Some(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Crate-level integration tests
// ---------------------------------------------------------------------------
//
// These tests exercise the public surface of `TwigVM` end to end —
// `dispatch::tests` covers the dispatcher in isolation; here we
// only test that the facade composes correctly.

#[cfg(test)]
mod tests {
    use super::*;

    // ── Compile-only tests (preserved from PR 3) ────────────────────

    #[test]
    fn compile_returns_iir_module_with_main() {
        let vm = TwigVM::new();
        let m = vm.compile("(+ 1 2)").unwrap();
        assert!(m.functions.iter().any(|f| f.name == "main"));
        assert_eq!(m.entry_point.as_deref(), Some("main"));
        assert_eq!(m.language, "twig");
    }

    #[test]
    fn compile_with_name_uses_supplied_name() {
        let vm = TwigVM::new();
        let m = vm.compile_with_name("42", "my_module").unwrap();
        assert_eq!(m.name, "my_module");
    }

    #[test]
    fn compile_propagates_parser_errors() {
        let vm = TwigVM::new();
        assert!(vm.compile("(+ 1 2").is_err()); // unmatched paren
    }

    // ── Builtin resolution ──────────────────────────────────────────

    #[test]
    fn resolve_builtin_finds_arithmetic() {
        for name in ["+", "-", "*", "/", "=", "<", ">"] {
            assert!(TwigVM::resolve_builtin(name).is_some(), "{name} should resolve");
        }
    }

    #[test]
    fn resolve_builtin_finds_cons_family() {
        for name in ["cons", "car", "cdr", "null?", "pair?"] {
            assert!(TwigVM::resolve_builtin(name).is_some(), "{name} should resolve");
        }
    }

    #[test]
    fn resolve_builtin_returns_none_for_unknown() {
        assert!(TwigVM::resolve_builtin("does_not_exist").is_none());
    }

    // ── End-to-end execution via TwigVM::run ────────────────────────

    #[test]
    fn run_arithmetic_returns_value() {
        let vm = TwigVM::new();
        assert_eq!(vm.run("(+ 2 3)").unwrap().as_int(), Some(5));
    }

    #[test]
    fn run_comparison_returns_bool() {
        let vm = TwigVM::new();
        assert_eq!(vm.run("(< 1 2)").unwrap(), LispyValue::TRUE);
    }

    #[test]
    fn run_cons_returns_heap_value() {
        let vm = TwigVM::new();
        assert!(vm.run("(cons 1 2)").unwrap().is_heap());
    }

    #[test]
    fn run_if_takes_correct_branch() {
        let vm = TwigVM::new();
        assert_eq!(vm.run("(if (< 1 2) 100 200)").unwrap().as_int(), Some(100));
    }

    #[test]
    fn run_let_binds_locals() {
        let vm = TwigVM::new();
        assert_eq!(vm.run("(let ((x 5)) (* x x))").unwrap().as_int(), Some(25));
    }

    #[test]
    fn run_user_defined_function() {
        let vm = TwigVM::new();
        let src = "(define (square x) (* x x)) (square 7)";
        assert_eq!(vm.run(src).unwrap().as_int(), Some(49));
    }

    #[test]
    fn run_factorial_via_recursion() {
        let vm = TwigVM::new();
        let src = "
            (define (fact n)
              (if (= n 0) 1 (* n (fact (- n 1)))))
            (fact 5)
        ";
        assert_eq!(vm.run(src).unwrap().as_int(), Some(120));
    }

    // ── Error paths through the facade ──────────────────────────────

    #[test]
    fn run_propagates_compile_error() {
        let vm = TwigVM::new();
        let err = vm.run("(+ 1 2").unwrap_err();
        assert!(matches!(err, TwigRunError::Compile(_)));
    }

    #[test]
    fn run_propagates_run_error() {
        let vm = TwigVM::new();
        // Division by zero — compiles fine, traps at runtime.
        let err = vm.run("(/ 7 0)").unwrap_err();
        assert!(matches!(err, TwigRunError::Run(RunError::Runtime(_))));
    }

    #[test]
    fn twig_run_error_displays_compile_variant() {
        let e = TwigRunError::Run(RunError::DepthExceeded);
        let s = format!("{e}");
        assert!(s.contains("run error"));
    }

    #[test]
    fn twig_run_error_source_unwraps() {
        use std::error::Error;
        let e = TwigRunError::Run(RunError::DepthExceeded);
        assert!(e.source().is_some(), "should expose underlying error");
    }
}
