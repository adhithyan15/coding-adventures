//! `iir-refinement-pass` — LANG42 pre-codegen refinement obligation checker.
//!
//! # What this crate does
//!
//! LANG23 built the entire refinement-type infrastructure:
//!
//! - **`lang-refined-types`** — `RefinedType`, `Predicate`, `Kind` data types.
//! - **`constraint-vm` + `constraint-engine`** — DPLL SAT solver + Cooper's LIA.
//! - **`lang-refinement-checker`** — per-binding `Checker`, plus function-,
//!   module-, and program-scope checkers.
//! - **`twig-ir-compiler`** — populates `IIRFunction::param_refinements` and
//!   `return_refinement` from parsed type annotations.
//!
//! However, the IIR never reached the checker: the `twig-aot` pipeline compiled
//! and emitted machine code without asking whether any refinement obligation
//! was violated.
//!
//! **LANG42** wires the checker into the pipeline.  After `twig-ir-compiler`
//! emits an `IIRModule`, a new pre-codegen pass (this crate) scans call sites
//! and return sites, resolves argument evidence via lightweight constant
//! propagation, and discharges proof obligations through the existing
//! `lang-refinement-checker` API.
//!
//! The result: a literal argument that provably violates a parameter
//! annotation becomes a **compile error with a counter-example** rather than
//! silently producing broken machine code.
//!
//! # Algorithm summary
//!
//! For each function in the module:
//!
//! 1. **Constant-propagation map** — scan `const` instructions for
//!    `Operand::Int` assignments; build `ConstMap: HashMap<String, i128>`.
//! 2. **Call-site checking** — for each `call` instruction, look up the
//!    callee's `param_refinements`, resolve each argument's evidence (Concrete
//!    if a literal or ConstMap hit; Unconstrained otherwise), and call
//!    [`lang_refinement_checker::Checker::check`].
//! 3. **Return-site checking** — for each `ret` instruction in a function
//!    with `return_refinement = Some(ann)`, check the returned value's
//!    evidence against `ann`.
//!
//! # Modes
//!
//! | Mode | `ProvenUnsafe` | `Unknown` |
//! |---|---|---|
//! | [`RefinementMode::Lenient`] | compile error | silent |
//! | [`RefinementMode::Strict`]  | compile error | compile error |
//!
//! # Public API
//!
//! ```
//! use iir_refinement_pass::{check_module, RefinementMode};
//! use interpreter_ir::module::IIRModule;
//!
//! let module = IIRModule::new("test.twig", "twig");
//! let errors = check_module(&module, RefinementMode::Lenient);
//! assert!(errors.is_empty());
//! ```

pub mod call_checker;
pub mod const_prop;
pub mod ret_checker;

use interpreter_ir::module::IIRModule;

// Re-export the checker infrastructure we rely on so downstream callers that
// want to inspect outcomes can import them from one place.
pub use lang_refinement_checker::{Evidence, CheckOutcome};

// ---------------------------------------------------------------------------
// RefinementMode
// ---------------------------------------------------------------------------

/// How to handle `Unknown` outcomes from the constraint solver.
///
/// `ProvenUnsafe` always becomes a compile error regardless of mode.
///
/// # Background
///
/// The per-binding solver uses Cooper's LIA for integer ranges and DPLL for
/// membership sets.  When the evidence is `Unconstrained` (the value comes
/// from user input, a function parameter, or a heap load), the solver cannot
/// determine whether the annotation is satisfied — it returns `Unknown`.
///
/// - **Lenient** (default): silently accept `Unknown`.  This matches the
///   behaviour of a type system that only *rejects* programs it can *prove*
///   wrong.  A future pass (LANG46) will insert runtime checks for these
///   sites.
/// - **Strict**: treat `Unknown` as an error.  Used for `(typed strict)`
///   modules (TW05-A) and for pipelines where soundness is mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefinementMode {
    /// `ProvenUnsafe` → error.  `Unknown` → silent.
    #[default]
    Lenient,
    /// `ProvenUnsafe` → error.  `Unknown` → error.
    Strict,
}

// ---------------------------------------------------------------------------
// RefinementError
// ---------------------------------------------------------------------------

/// A refinement violation found during the pass.
///
/// One `RefinementError` is produced for each proof obligation that the
/// solver determines is `ProvenUnsafe`, or for `Unknown` outcomes when running
/// in [`RefinementMode::Strict`].
///
/// # Fields
///
/// - **`function`** — the function that contains the violation (the *caller*
///   for call-site errors, the *callee* for return-site errors).
/// - **`site`** — human-readable label identifying which parameter or return
///   site triggered the error.
/// - **`counter_example`** — the concrete integer value that witnesses the
///   violation (0 for `Unknown`-promoted errors in strict mode).
/// - **`description`** — a full sentence from the solver explaining the
///   violation or the reason the outcome is unknown.
#[derive(Debug, Clone)]
pub struct RefinementError {
    /// The function containing the violation.
    pub function: String,
    /// Human-readable description: which parameter or return, what annotation.
    pub site: String,
    /// Counter-example value that proves the violation (0 for `Unknown`).
    pub counter_example: i128,
    /// Full description from the checker or solver.
    pub description: String,
}

impl std::fmt::Display for RefinementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error[E0042]: refinement violation\n  → function `{}`, {}\n  counter-example: {}\n  detail: {}",
            self.function,
            self.site,
            self.counter_example,
            self.description,
        )
    }
}

// ---------------------------------------------------------------------------
// check_module — the main entry point
// ---------------------------------------------------------------------------

/// Run the refinement obligation pass over all functions in `module`.
///
/// Returns a (possibly empty) list of violations.  The caller decides whether
/// to abort compilation (all errors in `twig-aot`) or emit warnings.
///
/// # Order of operations
///
/// For each function in `module.functions`:
///
/// 1. Build a [`const_prop::ConstMap`] by scanning `const` instructions for
///    integer literals.
/// 2. Call [`call_checker::check_calls`] to scan `call` instructions.
/// 3. Call [`ret_checker::check_returns`] to scan `ret` instructions.
///
/// The pass runs on the **original IIR before any lowering** (before
/// `prepare_module_for_aot`), so variable names still correspond to the
/// annotations the compiler attached.  Running it after lowering would break
/// the name correspondence.
///
/// # Example
///
/// ```
/// use iir_refinement_pass::{check_module, RefinementMode};
/// use interpreter_ir::module::IIRModule;
///
/// let module = IIRModule::new("empty.twig", "twig");
/// let errors = check_module(&module, RefinementMode::Lenient);
/// assert!(errors.is_empty(), "empty module has no violations");
/// ```
pub fn check_module(module: &IIRModule, mode: RefinementMode) -> Vec<RefinementError> {
    let mut errors = Vec::new();

    for func in &module.functions {
        // Step 1: build the constant-propagation map for this function.
        let const_map = const_prop::build_const_map(func);

        // Step 2: check each call site against callee param_refinements.
        call_checker::check_calls(func, module, &const_map, mode, &mut errors);

        // Step 3: check each return site against this function's return_refinement.
        ret_checker::check_returns(func, &const_map, mode, &mut errors);
    }

    errors
}

// ---------------------------------------------------------------------------
// Integration tests for check_module
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::function::IIRFunction;
    use interpreter_ir::instr::{IIRInstr, Operand};
    use interpreter_ir::module::IIRModule;
    use lang_refined_types::{RefinedType, Kind};
    use lang_refined_types::Predicate;

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Closed range annotation [lo, hi].
    fn range(lo: i128, hi: i128) -> RefinedType {
        RefinedType::refined(
            Kind::Int,
            Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: true },
        )
    }

    /// Membership annotation `{values...}`.
    fn membership(values: Vec<i128>) -> RefinedType {
        RefinedType::refined(
            Kind::Int,
            Predicate::Membership { values },
        )
    }

    /// Build a module with a callee (annotated param) and a caller (single call).
    fn module_with_call(annotation: RefinedType, arg: Operand) -> IIRModule {
        let callee = IIRFunction {
            name: "callee".into(),
            params: vec![("x".into(), "i64".into())],
            param_refinements: vec![Some(annotation)],
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Var("x".into())], "i64"),
            ],
            ..Default::default()
        };
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), arg],
            "i64",
        );
        let caller = IIRFunction {
            name: "main".into(),
            instructions: vec![call_instr],
            ..Default::default()
        };
        let mut m = IIRModule::new("test.twig", "twig");
        m.functions.push(callee);
        m.functions.push(caller);
        m
    }

    // ── Tests ─────────────────────────────────────────────────────────────

    #[test]
    fn concrete_literal_violates_range() {
        // The spec test: literal 200 violates [0, 128) → one error.
        let m = module_with_call(range(0, 127), Operand::Int(200));
        let errs = check_module(&m, RefinementMode::Lenient);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].counter_example, 200);
    }

    #[test]
    fn concrete_literal_in_range() {
        // 42 ∈ [0, 128) → no errors.
        let m = module_with_call(range(0, 127), Operand::Int(42));
        let errs = check_module(&m, RefinementMode::Lenient);
        assert!(errs.is_empty());
    }

    #[test]
    fn const_tracked_variable_violates() {
        // `const arg0 = 500; call callee arg0` — 500 violates [0, 128).
        let ann = range(0, 127);
        let callee = IIRFunction {
            name: "callee".into(),
            params: vec![("x".into(), "i64".into())],
            param_refinements: vec![Some(ann)],
            ..Default::default()
        };
        let caller = IIRFunction {
            name: "main".into(),
            instructions: vec![
                IIRInstr::new("const", Some("arg0".into()), vec![Operand::Int(500)], "i64"),
                IIRInstr::new("call", Some("r".into()),
                    vec![Operand::Var("callee".into()), Operand::Var("arg0".into())], "i64"),
            ],
            ..Default::default()
        };
        let mut m = IIRModule::new("test.twig", "twig");
        m.functions.push(callee);
        m.functions.push(caller);
        let errs = check_module(&m, RefinementMode::Lenient);
        assert_eq!(errs.len(), 1, "500 should be caught via ConstMap");
    }

    #[test]
    fn unconstrained_variable_lenient() {
        // Unknown variable → Unconstrained → UNKNOWN → silent in Lenient.
        let m = module_with_call(range(0, 127), Operand::Var("unknown".into()));
        let errs = check_module(&m, RefinementMode::Lenient);
        assert!(errs.is_empty());
    }

    #[test]
    fn unconstrained_variable_strict() {
        // Unknown variable → UNKNOWN → error in Strict.
        let m = module_with_call(range(0, 127), Operand::Var("unknown".into()));
        let errs = check_module(&m, RefinementMode::Strict);
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn return_literal_violates_return_type() {
        // A function annotated `-> (Int 0 255)` that returns 300 → error.
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: Some(range(0, 255)),
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Int(300)], "i64"),
            ],
            ..Default::default()
        };
        let mut m = IIRModule::new("test.twig", "twig");
        m.functions.push(func);
        let errs = check_module(&m, RefinementMode::Lenient);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].counter_example, 300);
    }

    #[test]
    fn return_literal_satisfies_return_type() {
        // 42 ∈ [0, 255] → no error.
        let func = IIRFunction {
            name: "f".into(),
            return_refinement: Some(range(0, 255)),
            instructions: vec![
                IIRInstr::new("ret", None, vec![Operand::Int(42)], "i64"),
            ],
            ..Default::default()
        };
        let mut m = IIRModule::new("test.twig", "twig");
        m.functions.push(func);
        let errs = check_module(&m, RefinementMode::Lenient);
        assert!(errs.is_empty());
    }

    #[test]
    fn unannotated_function_skipped() {
        // No param_refinements anywhere → always 0 errors even for
        // clearly out-of-range literals.
        let callee = IIRFunction {
            name: "callee".into(),
            params: vec![("x".into(), "i64".into())],
            param_refinements: vec![], // empty
            ..Default::default()
        };
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![Operand::Var("callee".into()), Operand::Int(9999)],
            "i64",
        );
        let caller = IIRFunction {
            name: "main".into(),
            instructions: vec![call_instr],
            ..Default::default()
        };
        let mut m = IIRModule::new("test.twig", "twig");
        m.functions.push(callee);
        m.functions.push(caller);
        let errs = check_module(&m, RefinementMode::Strict);
        assert!(errs.is_empty());
    }

    #[test]
    fn membership_predicate_violation() {
        // `(Int {1, 2, 5})` — only 1, 2, 5 are valid; 3 is a violation.
        let m = module_with_call(membership(vec![1, 2, 5]), Operand::Int(3));
        let errs = check_module(&m, RefinementMode::Lenient);
        assert_eq!(errs.len(), 1, "3 ∉ {{1,2,5}} should be an error");
    }

    #[test]
    fn multiple_violations_all_reported() {
        // Callee with two annotated parameters; both arguments violate.
        let callee = IIRFunction {
            name: "callee".into(),
            params: vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
            param_refinements: vec![Some(range(0, 127)), Some(range(0, 127))],
            ..Default::default()
        };
        let call_instr = IIRInstr::new(
            "call",
            Some("r".into()),
            vec![
                Operand::Var("callee".into()),
                Operand::Int(200), // violates [0,127]
                Operand::Int(300), // violates [0,127]
            ],
            "i64",
        );
        let caller = IIRFunction {
            name: "main".into(),
            instructions: vec![call_instr],
            ..Default::default()
        };
        let mut m = IIRModule::new("test.twig", "twig");
        m.functions.push(callee);
        m.functions.push(caller);
        let errs = check_module(&m, RefinementMode::Lenient);
        assert_eq!(errs.len(), 2, "both violations should be reported");
    }

    #[test]
    fn empty_module_no_errors() {
        let m = IIRModule::new("empty.twig", "twig");
        let errs = check_module(&m, RefinementMode::Strict);
        assert!(errs.is_empty());
    }

    #[test]
    fn error_display_format() {
        // Verify the Display impl produces the expected format string.
        let err = RefinementError {
            function: "main".into(),
            site: "call to `ascii-info`, argument 0".into(),
            counter_example: 200,
            description: "value 200 is not in [0, 128)".into(),
        };
        let s = format!("{err}");
        assert!(s.contains("E0042"));
        assert!(s.contains("main"));
        assert!(s.contains("200"));
    }
}
