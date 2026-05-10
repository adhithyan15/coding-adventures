//! # `iir-type-checker` — optional-typing spectrum checker for InterpreterIR
//!
//! The LANG pipeline's `type_hint` field on each `IIRInstr` is *optional*.
//! It can be:
//!
//! - `"any"` — the language frontend chose not to annotate this instruction;
//!   the VM will profile it and the JIT will speculate.
//! - A concrete type string (e.g. `"u8"`, `"i64"`, `"bool"`, `"f64"`,
//!   `"ref<MyObj>"`) — the frontend knows the type statically.
//!
//! This crate provides:
//!
//! 1. **[`tier::TypingTier`]** — the three-way classification:
//!    `Untyped` / `Partial(f32)` / `FullyTyped`.
//! 2. **[`check::check_module`]** — validate the type annotations that are
//!    already present (no mutation).
//! 3. **[`infer::infer_types_mut`]** — infer `type_hint` for `"any"`
//!    instructions using SSA-propagation rules.
//! 4. **[`infer_and_check`]** — convenience: run inference first, then check.
//!
//! ## Integration with JIT and AOT
//!
//! Both `jit-core` and `aot-core` read `type_hint` via
//! [`IIRInstr::is_typed`](interpreter_ir::instr::IIRInstr::is_typed) and
//! [`IIRInstr::effective_type`](interpreter_ir::instr::IIRInstr::effective_type).
//! Running `infer_and_check` before passing a module to either compiler
//! automatically enriches these fields, enabling specialised code generation
//! for instructions that were typed only through inference.
//!
//! ```text
//! IIRModule (partially typed)
//!     │
//!     ├─ iir_type_checker::infer_and_check(&mut module)
//!     │   ├─ infer_types_mut(&mut module)   ← fills in "any" hints
//!     │   └─ check_module(&module)          ← validates the enriched module
//!     │
//!     ├─ jit_core::execute_with_jit(&module, …)   ← sees more typed hints → better CIR
//!     └─ aot_core::AOTCore::compile(&module)       ← compiles more functions natively
//! ```
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::module::IIRModule;
//! use interpreter_ir::function::IIRFunction;
//! use interpreter_ir::instr::{IIRInstr, Operand};
//! use iir_type_checker::{infer_and_check, tier::TypingTier};
//!
//! // Build a module with an untyped integer constant.
//! let fn_ = IIRFunction::new(
//!     "main", vec![], "void",
//!     vec![
//!         IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any"),
//!         IIRInstr::new("const", Some("y".into()), vec![Operand::Int(1)],  "any"),
//!         IIRInstr::new("add",   Some("z".into()),
//!             vec![Operand::Var("x".into()), Operand::Var("y".into())], "any"),
//!         IIRInstr::new("ret_void", None, vec![], "void"),
//!     ],
//! );
//! let mut module = IIRModule::new("demo", "tetrad");
//! module.functions.push(fn_);
//!
//! let report = infer_and_check(&mut module);
//!
//! // Inference filled in the types.
//! assert_eq!(report.tier, TypingTier::FullyTyped);
//! assert!(report.ok());
//! assert_eq!(module.functions[0].instructions[0].type_hint, "i64"); // x
//! assert_eq!(module.functions[0].instructions[2].type_hint, "i64"); // z = x + y
//! ```

pub mod check;
pub mod errors;
pub mod infer;
pub mod report;
pub mod tier;

pub use check::check_module;
pub use errors::{ErrorKind, TypeCheckError, TypeCheckWarning};
pub use infer::infer_types_mut;
pub use report::TypeCheckReport;
pub use tier::TypingTier;

use interpreter_ir::module::IIRModule;

/// Run type inference followed by type checking.
///
/// This is the primary entry point for callers who want the full optional-typing
/// pipeline:
///
/// 1. **Inference** — [`infer_types_mut`] populates `type_hint` for
///    instructions that are currently `"any"` but whose type can be determined
///    from constants, comparisons, or SSA-propagated arithmetic.
///
/// 2. **Checking** — [`check_module`] validates the enriched module and
///    returns a [`TypeCheckReport`] with the [`TypingTier`], any errors, and
///    the set of newly-inferred types.
///
/// The `inferred_types` field of the returned report maps destination variable
/// names to the types that were **newly inferred** (i.e. instructions that
/// were `"any"` before this call).
///
/// # Example
///
/// ```
/// use interpreter_ir::module::IIRModule;
/// use interpreter_ir::function::IIRFunction;
/// use interpreter_ir::instr::{IIRInstr, Operand};
/// use iir_type_checker::{infer_and_check, tier::TypingTier};
///
/// let fn_ = IIRFunction::new(
///     "main", vec![], "void",
///     vec![
///         IIRInstr::new("const", Some("n".into()), vec![Operand::Int(7)], "any"),
///         IIRInstr::new("ret_void", None, vec![], "void"),
///     ],
/// );
/// let mut module = IIRModule::new("demo", "tetrad");
/// module.functions.push(fn_);
///
/// let report = infer_and_check(&mut module);
/// assert_eq!(report.tier, TypingTier::FullyTyped);
/// assert!(report.ok());
/// assert_eq!(report.inferred_types.get("n").map(String::as_str), Some("i64"));
/// ```
pub fn infer_and_check(module: &mut IIRModule) -> TypeCheckReport {
    // Pass 1: infer types and annotate the module in place.
    let newly_inferred = infer_types_mut(module);

    // Pass 2: validate the now-enriched annotations.
    let mut report = check_module(module);

    // Attach the inferred-types map so callers know what was deduced.
    report.inferred_types = newly_inferred;
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};

    fn make_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        let mut m = IIRModule::new("test", "tetrad");
        m.functions.push(fn_);
        m
    }

    // ── infer_and_check integration tests ────────────────────────────────

    #[test]
    fn infer_and_check_empty_module() {
        let mut m = IIRModule::new("empty", "tetrad");
        let report = infer_and_check(&mut m);
        assert!(report.ok());
        assert_eq!(report.tier, TypingTier::Untyped);
        assert!(report.inferred_types.is_empty());
    }

    #[test]
    fn infer_and_check_populates_inferred_types() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(42)], "any"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = infer_and_check(&mut m);
        assert!(report.ok());
        assert!(report.inferred_types.contains_key("x"));
        assert_eq!(report.inferred_types["x"], "i64");
    }

    #[test]
    fn infer_and_check_mutates_module() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Float(1.0)], "any"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        infer_and_check(&mut m);
        assert_eq!(m.functions[0].instructions[0].type_hint, "f64");
    }

    #[test]
    fn infer_and_check_full_pipeline() {
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "any"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "any"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
            IIRInstr::new("cmp_gt", Some("d".into()),
                vec![Operand::Var("c".into()), Operand::Int(5)], "any"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = infer_and_check(&mut m);
        assert!(report.ok(), "errors: {:?}", report.errors);
        assert_eq!(report.tier, TypingTier::FullyTyped);
        // a, b → i64; c → i64 (arith same); d → bool (cmp)
        assert_eq!(m.functions[0].instructions[2].type_hint, "i64");
        assert_eq!(m.functions[0].instructions[3].type_hint, "bool");
    }

    #[test]
    fn infer_and_check_detects_invalid_type_after_inference() {
        // Inject a bogus type that inference would not produce.
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "notatype"),
        ]);
        let report = infer_and_check(&mut m);
        assert!(!report.ok());
        assert_eq!(report.errors[0].kind, ErrorKind::InvalidType);
    }

    #[test]
    fn infer_and_check_partially_typed_module() {
        // One typed, one unresolvable-any instruction.
        let mut m = make_module(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "u8"),
            // "load_mem" with a symbolic var src — can't infer
            IIRInstr::new("load_mem", Some("b".into()),
                vec![Operand::Var("ptr".into())], "any"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let report = infer_and_check(&mut m);
        assert!(report.ok());
        assert!(matches!(report.tier, TypingTier::Partial(_)));
    }
}
