//! # `twig-vm` — runtime wiring between Twig and the LANG-runtime.
//!
//! Implementation of LANG20 PR 3 from the
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
//! state needed to compile and (eventually) execute Twig programs.
//!
//! ## What this PR ships (PR 3)
//!
//! - **Compilation**: [`TwigVM::compile`] takes Twig source and
//!   returns an `IIRModule` ready for execution.
//! - **Builtin resolution**: [`TwigVM::resolve_builtin`] proxies
//!   through `LispyBinding::resolve_builtin` so callers see the
//!   binding's resolved fn pointer.
//! - **Operand → Value conversion**: [`operand_to_value`] maps an
//!   IIR `Operand` (the universal-IR enum) into a `LispyValue`
//!   (the Lispy runtime's tagged-i64 representation).  This is
//!   the seam between the language-agnostic IIR and the
//!   per-language value model — it's small but crucial.
//! - **Integration tests**: a "1-instruction evaluator" that
//!   takes a single `call_builtin` instruction, resolves the
//!   builtin via the binding, and dispatches it.  Proves the
//!   substrate composes end-to-end without yet needing vm-core.
//!
//! ## What this PR does NOT ship (PR 4+)
//!
//! - **Real execution**: PR 4 wires `vm-core` against
//!   `LangBinding`.  Until then, [`TwigVM::run`] is intentionally
//!   absent — there's nothing to run *with* yet.  The
//!   [`TwigVM::evaluate_call_builtin`] helper covers the
//!   single-instruction case for tests.
//! - **Closures / control flow / locals**: those need the full
//!   interpreter dispatch loop (PR 4).  PR 3's evaluator handles
//!   only `call_builtin` with [`Operand`]-form arguments.
//!
//! ## Pipeline
//!
//! ```text
//! Twig source
//!     │
//!     ▼  twig_lexer → twig_parser → twig_ir_compiler
//! IIRModule  ◄────────────────────────────────  THIS CRATE COMPILES TO HERE
//!     │
//!     ▼  vm-core (PR 4)
//! execution
//!     │
//!     ▼  LispyBinding (this crate's runtime tie-in)
//! LispyValue results
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod evaluate;
pub mod operand;

use interpreter_ir::IIRModule;
use lispy_runtime::LispyBinding;
use twig_ir_compiler::compile_source as compile_twig;

pub use evaluate::{evaluate_call_builtin, EvaluateError};
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
/// table is process-global too.  The struct exists so that PR 4+
/// can grow per-VM state (an interpreter dispatch context, a
/// frame stack, a JIT cache) without breaking callers.
///
/// # Lifecycle
///
/// 1. Construct via [`TwigVM::new`].
/// 2. Compile Twig source via [`TwigVM::compile`] — returns an
///    `IIRModule` ready for the future interpreter.
/// 3. (PR 4+) Run the module via `TwigVM::run`.
///
/// # Example
///
/// ```
/// use twig_vm::TwigVM;
///
/// let vm = TwigVM::new();
/// let module = vm.compile("(+ 1 2)").unwrap();
/// assert_eq!(module.functions.len(), 1); // synthesised `main`
/// ```
#[derive(Debug, Default)]
pub struct TwigVM {
    // No fields yet.  PR 4 adds:
    //   - frame stack
    //   - register file scratch space
    //   - profile-feedback cache (LANG20 §"Feedback-slot taxonomy")
    //   - JIT promotion thresholds
    _private: (),
}

impl TwigVM {
    /// Construct a fresh VM.  All runtime state (intern table,
    /// allocator) is process-global at PR 3, so this is cheap —
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
    /// interpreter (PR 4) will dispatch to.  PR 3 callers use
    /// this together with [`evaluate_call_builtin`] to exercise
    /// the integration without a real interpreter.
    ///
    /// Returns `None` for names not in the Lispy builtin set.
    pub fn resolve_builtin(name: &str) -> Option<BuiltinFn<LispyBinding>> {
        LispyBinding::resolve_builtin(name)
    }
}

// ---------------------------------------------------------------------------
// Crate-level integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic compile path works.
    #[test]
    fn compile_returns_iir_module_with_main() {
        let vm = TwigVM::new();
        let m = vm.compile("(+ 1 2)").unwrap();
        // The synthesised `main` is always present.
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

    /// Builtin resolution proxies through LispyBinding.
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

    // ── End-to-end: Twig source → IIR → evaluate one builtin call ───
    //
    // These tests exercise the full PR 1+2+3 stack:
    //   - twig-lexer / twig-parser / twig-ir-compiler produce IIR
    //   - lispy-runtime's LispyBinding resolves the builtin
    //   - twig-vm's evaluate_call_builtin dispatches
    //
    // What we DON'T do (yet) is interpret arbitrary IIR — that's
    // PR 4's vm-core wiring.  We extract the relevant
    // `call_builtin` instruction from the compiled module and
    // evaluate it in isolation.

    /// Find the first `call_builtin <name>` instruction in
    /// `module`'s `main` function.
    fn find_call_builtin<'a>(
        module: &'a interpreter_ir::IIRModule,
        name: &str,
    ) -> Option<&'a interpreter_ir::IIRInstr> {
        let main = module.functions.iter().find(|f| f.name == "main")?;
        main.instructions.iter().find(|i| {
            i.op == "call_builtin"
                && matches!(
                    i.srcs.first(),
                    Some(interpreter_ir::Operand::Var(s)) if s == name,
                )
        })
    }

    #[test]
    fn end_to_end_arithmetic_from_source() {
        // The full pipeline: compile `(+ 2 3)`, find the
        // `call_builtin "+"` instruction in the compiled main,
        // and evaluate it.  Result should be LispyValue::int(5).
        let vm = TwigVM::new();
        let module = vm.compile("(+ 2 3)").unwrap();

        let instr = find_call_builtin(&module, "+")
            .expect("compiled IR should contain a call_builtin \"+\" instruction");

        // The compiled IIR uses register names (e.g. `_n1`, `_n2`)
        // for the integer arguments — they're the dest of `const`
        // instructions emitted earlier.  Walk those to populate
        // the evaluator's frame.
        let frame = build_const_frame(&module);
        let result = evaluate::evaluate_call_builtin(instr, &|n| frame.get(n).copied()).unwrap();
        assert_eq!(result.as_int(), Some(5));
    }

    #[test]
    fn end_to_end_comparison_from_source() {
        let vm = TwigVM::new();
        let module = vm.compile("(< 1 2)").unwrap();
        let instr = find_call_builtin(&module, "<").unwrap();
        let frame = build_const_frame(&module);
        let result = evaluate::evaluate_call_builtin(instr, &|n| frame.get(n).copied()).unwrap();
        assert_eq!(result, LispyValue::TRUE);
    }

    #[test]
    fn end_to_end_cons_from_source() {
        // (cons 1 2) → pair value
        let vm = TwigVM::new();
        let module = vm.compile("(cons 1 2)").unwrap();
        let instr = find_call_builtin(&module, "cons").unwrap();
        let frame = build_const_frame(&module);
        let result = evaluate::evaluate_call_builtin(instr, &|n| frame.get(n).copied()).unwrap();
        assert!(result.is_heap());
    }

    /// Walk every `const` instruction in main and populate a
    /// frame map from `dest -> LispyValue` so subsequent
    /// instructions can resolve their `Operand::Var` arguments.
    ///
    /// This is a tiny model of what vm-core's frame resolution
    /// will do in PR 4.  Keeping it inline in tests rather than
    /// shipping it in the public API because:
    /// (a) PR 4's vm-core implements the real version,
    /// (b) this version handles only `const`, not the full opcode
    ///     set,
    /// (c) shipping it would create a "two interpreters" maintenance
    ///     burden.
    fn build_const_frame(module: &interpreter_ir::IIRModule) -> std::collections::HashMap<String, LispyValue> {
        let mut frame = std::collections::HashMap::new();
        let main = match module.functions.iter().find(|f| f.name == "main") {
            Some(m) => m,
            None => return frame,
        };
        for instr in &main.instructions {
            if instr.op != "const" {
                continue;
            }
            let dest = match &instr.dest {
                Some(d) => d.clone(),
                None => continue,
            };
            let v = match instr.srcs.first() {
                Some(interpreter_ir::Operand::Int(n)) => LispyValue::int(*n),
                Some(interpreter_ir::Operand::Bool(b)) => LispyValue::bool(*b),
                _ => continue,
            };
            frame.insert(dest, v);
        }
        frame
    }
}
