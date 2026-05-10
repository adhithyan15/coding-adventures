//! # Program-scope refinement checker — LANG23 PR 23-G.
//!
//! Implements rung 4 of the LANG23 refinement-type hierarchy: **closed-world
//! program-scope enforcement**.  In strict mode, every public binding in every
//! module the program transitively consumes must carry a refinement annotation.
//! Any unrefined (`: any`) binding in a module's public surface is a link-time
//! error.
//!
//! ## What "program-scope" means
//!
//! The LANG23 spec (§"Rung 4 — program-scope"):
//!
//! > The compiler now refuses to link any module whose public surface includes
//! > type-only `: any` bindings.  Every cross-module call is proven
//! > refinement-safe at build time.
//!
//! ## The two-axis check
//!
//! The program checker inspects each function in each registered module along
//! two axes:
//!
//! | Axis | Always checked? | Unrefined means… |
//! |------|-----------------|------------------|
//! | Parameters | Yes | Callers cannot prove call-site safety without a runtime check |
//! | Return type | Optional (`with_return_type_checking()`) | Callers cannot inherit narrowed types from the call's return |
//!
//! Parameter checking is always on — a function with an unrefined parameter is
//! a concrete gap in the public contract.  Return-type checking is opt-in
//! because many functions have unrefined returns by design (they produce values
//! that callers don't need to reason about).
//!
//! ## Relationship to rungs 1–3
//!
//! ```text
//! Rung 1 (23-C): per-binding Checker
//! Rung 2 (23-D): function-scope FunctionChecker (CFG walk + solver)
//! Rung 3 (23-F): module-scope ModuleChecker (cross-function call-site reasoning)
//! Rung 4 (23-G): program-scope ProgramChecker  ← this module
//!                 no solver; no CFG; pure structural annotation audit
//! ```
//!
//! The program checker deliberately does **not** call the solver — it is a
//! "gate-check" that runs before the module checker.  A program that passes
//! the program checker can then run the module checker on each function body
//! and expect every call-site check to be at least `Unknown`-free (modulo
//! body logic), because every public binding has an annotation.
//!
//! ## Usage
//!
//! ```rust
//! use lang_refined_types::{Kind, Predicate, RefinedType};
//! use lang_refinement_checker::function_checker::FunctionSignature;
//! use lang_refinement_checker::module_checker::ModuleScope;
//! use lang_refinement_checker::program_checker::{ProgramChecker, ProgramModule};
//!
//! // Fully annotated module — passes strict check.
//! let mut clean_scope = ModuleScope::new();
//! clean_scope.register("decode", FunctionSignature {
//!     params: vec![(
//!         "codepoint".into(),
//!         RefinedType::refined(Kind::Int, Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false }),
//!     )],
//!     return_type: RefinedType::unrefined(Kind::Int),
//! });
//!
//! let modules = vec![ProgramModule { name: "text/ascii".into(), scope: clean_scope }];
//! let checker = ProgramChecker::new();
//! let result = checker.check_program(&modules);
//!
//! assert!(result.is_clean(), "fully annotated module passes strict check");
//! ```

#![allow(clippy::module_name_repetitions)]

use crate::{function_checker::FunctionSignature, module_checker::ModuleScope};

// ---------------------------------------------------------------------------
// ViolationKind — what kind of annotation is missing
// ---------------------------------------------------------------------------

/// The kind of missing annotation that caused a violation.
///
/// | Kind | Meaning |
/// |------|---------|
/// | `UnrefinedParam` | A function parameter has no refinement (is `: any`). |
/// | `UnrefinedReturn` | The function's return type has no refinement. |
///
/// `UnrefinedReturn` is only produced when the checker was constructed with
/// [`ProgramChecker::with_return_type_checking`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationKind {
    /// A function parameter carries no refinement annotation.
    ///
    /// Callers of this function cannot statically prove that the argument
    /// satisfies the parameter's contract — they must emit a runtime check.
    UnrefinedParam {
        /// Zero-based index of the unrefined parameter.
        param_index: usize,
        /// Name of the unrefined parameter (from the `FunctionSignature`).
        param_name: String,
    },
    /// The function's return type carries no refinement annotation.
    ///
    /// Callers cannot inherit a narrowed type from the call's return value
    /// and must treat the result as fully unconstrained.
    UnrefinedReturn,
}

// ---------------------------------------------------------------------------
// AnnotationViolation — one missing annotation
// ---------------------------------------------------------------------------

/// A single annotation violation in the program's public surface.
///
/// The `description` field carries a human-readable error message suitable
/// for surfacing to the programmer:
///
/// ```text
/// [text/ascii] decode: parameter 0 'codepoint' has no refinement annotation (': any')
/// ```
#[derive(Debug, Clone)]
pub struct AnnotationViolation {
    /// Name of the module containing the violating function.
    pub module_name: String,
    /// Name of the violating function.
    pub function_name: String,
    /// The kind of missing annotation.
    pub kind: ViolationKind,
    /// Human-readable description suitable for a compiler error message.
    pub description: String,
}

// ---------------------------------------------------------------------------
// ProgramCheckResult — aggregate outcome
// ---------------------------------------------------------------------------

/// The aggregate outcome of checking an entire program's public surface.
///
/// A `ProgramCheckResult` collects every annotation violation found across
/// all modules.  The caller interprets it according to the configured mode:
///
/// - `is_clean()` → no violations; the program is fully refinement-typed.
/// - `has_violations()` → at least one violation; in strict mode this is a
///   link-time error.
#[derive(Debug, Clone)]
pub struct ProgramCheckResult {
    /// All annotation violations found, in module-then-function order.
    pub violations: Vec<AnnotationViolation>,
}

impl ProgramCheckResult {
    /// Returns `true` if no violations were found.
    ///
    /// When this holds, the program's public surface is fully refinement-typed.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }

    /// Returns `true` if at least one violation was found.
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// The total number of annotation violations.
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    /// A multi-line human-readable error message listing all violations.
    ///
    /// Returns an empty string if `is_clean()`.
    ///
    /// # Example output
    ///
    /// ```text
    /// 2 refinement annotation violation(s) found:
    ///   [text/ascii] helper: parameter 0 'x' has no refinement annotation (': any')
    ///   [text/ascii] helper: parameter 1 'y' has no refinement annotation (': any')
    /// ```
    pub fn error_message(&self) -> String {
        if self.violations.is_empty() {
            return String::new();
        }
        let mut msg = format!(
            "{} refinement annotation violation(s) found:\n",
            self.violations.len()
        );
        for v in &self.violations {
            msg.push_str("  ");
            msg.push_str(&v.description);
            msg.push('\n');
        }
        msg
    }

    /// The names of modules that contain at least one violation.
    ///
    /// Useful for reporting which deps need to be updated.
    pub fn violating_modules(&self) -> Vec<&str> {
        let mut modules: Vec<&str> = self
            .violations
            .iter()
            .map(|v| v.module_name.as_str())
            .collect();
        modules.dedup();
        modules
    }
}

// ---------------------------------------------------------------------------
// ProgramModule — a named module with its public scope
// ---------------------------------------------------------------------------

/// A named module with its public function-signature registry.
///
/// The program checker iterates over all modules and inspects every function
/// registered in the module's [`ModuleScope`].
///
/// ## Per-symbol opt-out
///
/// A function that should be excluded from the strict check (e.g., a
/// third-party dependency or an internal helper not part of the public
/// surface) should simply **not** be registered in the `scope`.  The program
/// checker only audits functions that are present in the scope.
///
/// This is consistent with the module-scope checker's per-symbol opt-out.
#[derive(Debug)]
pub struct ProgramModule {
    /// Human-readable module name, used in violation descriptions.
    ///
    /// Examples: `"text/ascii"`, `"auth"`, `"my-library/v1"`.
    pub name: String,
    /// The module's public function-signature registry.
    pub scope: ModuleScope,
}

// ---------------------------------------------------------------------------
// ProgramChecker — the entry point
// ---------------------------------------------------------------------------

/// Program-scope (rung 4) refinement annotation checker.
///
/// Audits the public surface of every module in the program for missing
/// refinement annotations.  In strict mode any unrefined parameter is a
/// link-time error.
///
/// The checker is **purely structural** — it inspects `RefinedType`s for the
/// presence of a predicate (`is_unrefined()`) without invoking the solver.
/// This makes it fast enough to run at link time on large programs.
///
/// # Example — primary acceptance criterion
///
/// ```rust
/// use lang_refined_types::{Kind, Predicate, RefinedType};
/// use lang_refinement_checker::function_checker::FunctionSignature;
/// use lang_refinement_checker::module_checker::ModuleScope;
/// use lang_refinement_checker::program_checker::{ProgramChecker, ProgramModule, ViolationKind};
///
/// // A refinement-incomplete module: `helper` has an unrefined param.
/// let mut scope = ModuleScope::new();
/// scope.register("decode", FunctionSignature {
///     params: vec![(
///         "codepoint".into(),
///         RefinedType::refined(
///             Kind::Int,
///             Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
///         ),
///     )],
///     return_type: RefinedType::unrefined(Kind::Int),
/// });
/// scope.register("helper", FunctionSignature {
///     params: vec![("x".into(), RefinedType::unrefined(Kind::Int))], // ← `: any`
///     return_type: RefinedType::unrefined(Kind::Int),
/// });
///
/// let modules = vec![ProgramModule { name: "text/ascii".into(), scope }];
///
/// let checker = ProgramChecker::new();
/// let result = checker.check_program(&modules);
///
/// // Strict check catches the unrefined parameter in `helper`.
/// assert!(result.has_violations());
/// assert_eq!(result.violation_count(), 1);
/// assert_eq!(result.violations[0].function_name, "helper");
/// assert!(matches!(
///     result.violations[0].kind,
///     ViolationKind::UnrefinedParam { param_index: 0, .. }
/// ));
/// ```
#[derive(Debug)]
pub struct ProgramChecker {
    /// Whether to also flag unrefined return types as violations.
    ///
    /// Default: `false` — only parameter annotations are checked.  Enable
    /// with [`ProgramChecker::with_return_type_checking`].
    check_return_types: bool,
}

impl Default for ProgramChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramChecker {
    /// Construct a program checker.
    ///
    /// By default only unrefined **parameters** are flagged (return types
    /// are not checked).  Use [`with_return_type_checking`] to opt into
    /// stricter return-type enforcement.
    ///
    /// [`with_return_type_checking`]: Self::with_return_type_checking
    pub fn new() -> Self {
        ProgramChecker { check_return_types: false }
    }

    /// Enable return-type annotation checking.
    ///
    /// When enabled, functions whose return type is unrefined also produce a
    /// [`ViolationKind::UnrefinedReturn`] violation.
    ///
    /// ```rust
    /// use lang_refined_types::{Kind, RefinedType, Predicate};
    /// use lang_refinement_checker::function_checker::FunctionSignature;
    /// use lang_refinement_checker::module_checker::ModuleScope;
    /// use lang_refinement_checker::program_checker::{ProgramChecker, ProgramModule};
    ///
    /// let mut scope = ModuleScope::new();
    /// scope.register("f", FunctionSignature {
    ///     params: vec![("x".into(), RefinedType::refined(Kind::Int, Predicate::Range { lo: Some(0), hi: Some(10), inclusive_hi: false }))],
    ///     return_type: RefinedType::unrefined(Kind::Int), // unrefined return
    /// });
    ///
    /// let modules = vec![ProgramModule { name: "m".into(), scope }];
    ///
    /// // Without return-type checking: clean.
    /// let checker = ProgramChecker::new();
    /// assert!(checker.check_program(&modules).is_clean());
    ///
    /// // With return-type checking: violation.
    /// let strict_checker = ProgramChecker::new().with_return_type_checking();
    /// assert!(strict_checker.check_program(&modules).has_violations());
    /// ```
    pub fn with_return_type_checking(mut self) -> Self {
        self.check_return_types = true;
        self
    }

    /// Check the public surfaces of all modules in `modules`.
    ///
    /// Iterates every module's [`ModuleScope`] and every function therein.
    /// For each function, checks:
    /// 1. Every parameter — if `is_unrefined()`, a `UnrefinedParam` violation.
    /// 2. The return type (if `check_return_types`) — if `is_unrefined()`, a
    ///    `UnrefinedReturn` violation.
    ///
    /// Violations are collected in module-then-function order (the order
    /// modules are passed in; within a module the order matches the HashMap's
    /// iteration order, which is non-deterministic but stable per run).
    pub fn check_program(&self, modules: &[ProgramModule]) -> ProgramCheckResult {
        let mut violations: Vec<AnnotationViolation> = Vec::new();

        for module in modules {
            // Iterate all registered functions in this module.
            // ModuleScope's internal HashMap is not exposed directly, so we
            // use get_signature + the fact that we built the scope ourselves.
            // For the checker we need an iterable view — we expose it via the
            // `functions` field (made pub(crate) below) or via a helper.
            for (fn_name, sig) in module.scope.iter() {
                self.check_function(
                    &module.name,
                    fn_name,
                    sig,
                    &mut violations,
                );
            }
        }

        ProgramCheckResult { violations }
    }

    // -------------------------------------------------------------------------
    // Check a single function's annotations
    // -------------------------------------------------------------------------

    fn check_function(
        &self,
        module_name: &str,
        function_name: &str,
        sig: &FunctionSignature,
        out: &mut Vec<AnnotationViolation>,
    ) {
        // ── Parameter annotations ──────────────────────────────────────────
        //
        // For each parameter: if the RefinedType has no predicate
        // (`is_unrefined()`), the parameter is annotated `: any`.
        //
        // In the Twig frontend, `: any` is both the explicit opt-out syntax
        // and the default for unannotated parameters.  From the program
        // checker's perspective they are equivalent: the public contract has
        // a gap.
        for (i, (param_name, rt)) in sig.params.iter().enumerate() {
            if rt.is_unrefined() {
                out.push(AnnotationViolation {
                    module_name: module_name.to_string(),
                    function_name: function_name.to_string(),
                    kind: ViolationKind::UnrefinedParam {
                        param_index: i,
                        param_name: param_name.clone(),
                    },
                    description: format!(
                        "[{module_name}] {function_name}: \
                         parameter {i} '{param_name}' has no refinement \
                         annotation (': any')"
                    ),
                });
            }
        }

        // ── Return type annotation ─────────────────────────────────────────
        //
        // Only checked when `check_return_types` is enabled.  Unrefined
        // return types are common and often intentional (e.g., functions that
        // produce values the caller doesn't reason about statically), so this
        // check is opt-in.
        if self.check_return_types && sig.return_type.is_unrefined() {
            out.push(AnnotationViolation {
                module_name: module_name.to_string(),
                function_name: function_name.to_string(),
                kind: ViolationKind::UnrefinedReturn,
                description: format!(
                    "[{module_name}] {function_name}: \
                     return type has no refinement annotation (': any')"
                ),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// RefinedType::is_unrefined — needed by program_checker
// ---------------------------------------------------------------------------
//
// `RefinedType` lives in `lang-refined-types`.  It exposes `predicate:
// Option<Predicate>`.  We check `predicate.is_none()` via the public field
// to determine if a type is unrefined.  No new public API is needed here —
// the check is inlined into `check_function`.
//
// `ModuleScope::iter()` is defined as `pub(crate)` in `module_checker.rs`
// (the module that owns the private `functions` field).  We simply call it
// here without re-defining the impl.

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use lang_refined_types::{Kind, Predicate, RefinedType};

    use super::*;
    use crate::{
        function_checker::FunctionSignature,
        module_checker::ModuleScope,
    };

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn range(lo: i128, hi: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: false }
    }

    fn int_annotation(pred: Predicate) -> RefinedType {
        RefinedType::refined(Kind::Int, pred)
    }

    fn unrefined() -> RefinedType {
        RefinedType::unrefined(Kind::Int)
    }

    fn annotated_sig(param_name: &str) -> FunctionSignature {
        FunctionSignature {
            params: vec![(param_name.into(), int_annotation(range(0, 128)))],
            return_type: unrefined(),
        }
    }

    fn unannotated_sig(param_name: &str) -> FunctionSignature {
        FunctionSignature {
            params: vec![(param_name.into(), unrefined())],
            return_type: unrefined(),
        }
    }

    fn make_module(name: &str, fns: &[(&str, FunctionSignature)]) -> ProgramModule {
        let mut scope = ModuleScope::new();
        for (fn_name, sig) in fns {
            scope.register(*fn_name, sig.clone());
        }
        ProgramModule { name: name.into(), scope }
    }

    // ─── Primary acceptance criterion (LANG23 §"Rung 4 — program-scope") ─────

    /// Compiling with strict mode against a refinement-incomplete dep produces
    /// a clear error.
    ///
    /// Spec:
    /// > The compiler now refuses to link any module whose public surface
    /// > includes type-only `: any` bindings.
    #[test]
    fn strict_mode_rejects_refinement_incomplete_module() {
        // `decode` is fully annotated; `helper` has an unrefined param → violation.
        let mut scope = ModuleScope::new();
        scope.register("decode", FunctionSignature {
            params: vec![(
                "codepoint".into(),
                int_annotation(range(0, 128)),
            )],
            return_type: unrefined(),
        });
        scope.register("helper", FunctionSignature {
            params: vec![("x".into(), unrefined())], // ← `: any`
            return_type: unrefined(),
        });

        let modules = vec![ProgramModule { name: "text/ascii".into(), scope }];
        let result = ProgramChecker::new().check_program(&modules);

        assert!(result.has_violations(), "incomplete module should have violations");
        assert_eq!(result.violation_count(), 1, "only helper's param is unrefined");
        let v = &result.violations[0];
        assert_eq!(v.module_name, "text/ascii");
        assert_eq!(v.function_name, "helper");
        assert!(
            matches!(v.kind, ViolationKind::UnrefinedParam { param_index: 0, .. }),
            "violation should be UnrefinedParam at index 0"
        );
    }

    /// A fully annotated program passes the strict check.
    #[test]
    fn fully_annotated_program_is_clean() {
        let modules = vec![make_module("text/ascii", &[
            ("decode", annotated_sig("codepoint")),
            ("encode", annotated_sig("byte")),
        ])];

        let result = ProgramChecker::new().check_program(&modules);
        assert!(result.is_clean(), "fully annotated program should be clean");
        assert_eq!(result.violation_count(), 0);
    }

    /// An empty program has no violations.
    #[test]
    fn empty_program_is_clean() {
        let result = ProgramChecker::new().check_program(&[]);
        assert!(result.is_clean());
        assert_eq!(result.violation_count(), 0);
        assert!(result.error_message().is_empty());
    }

    /// A module with no registered functions has no violations.
    #[test]
    fn empty_module_is_clean() {
        let scope = ModuleScope::new();
        let modules = vec![ProgramModule { name: "empty".into(), scope }];
        let result = ProgramChecker::new().check_program(&modules);
        assert!(result.is_clean());
    }

    // ─── Unrefined parameters ─────────────────────────────────────────────────

    /// An unrefined first parameter produces one violation.
    #[test]
    fn unrefined_first_param_is_violation() {
        let modules = vec![make_module("m", &[("f", unannotated_sig("x"))])];
        let result = ProgramChecker::new().check_program(&modules);

        assert_eq!(result.violation_count(), 1);
        let v = &result.violations[0];
        assert!(matches!(
            v.kind,
            ViolationKind::UnrefinedParam { param_index: 0, ref param_name }
            if param_name == "x"
        ));
    }

    /// A function with two unrefined parameters produces two violations.
    #[test]
    fn two_unrefined_params_produce_two_violations() {
        let sig = FunctionSignature {
            params: vec![
                ("a".into(), unrefined()),
                ("b".into(), unrefined()),
            ],
            return_type: unrefined(),
        };
        let modules = vec![make_module("m", &[("f", sig)])];
        let result = ProgramChecker::new().check_program(&modules);

        assert_eq!(result.violation_count(), 2);
        assert!(matches!(
            result.violations[0].kind,
            ViolationKind::UnrefinedParam { param_index: 0, .. }
        ));
        assert!(matches!(
            result.violations[1].kind,
            ViolationKind::UnrefinedParam { param_index: 1, .. }
        ));
    }

    /// Mixed params: first annotated, second unrefined → one violation at index 1.
    #[test]
    fn mixed_params_one_violation() {
        let sig = FunctionSignature {
            params: vec![
                ("lo".into(), int_annotation(range(0, 100))), // ✓
                ("hi".into(), unrefined()),                   // ✗
            ],
            return_type: unrefined(),
        };
        let modules = vec![make_module("math", &[("clamp", sig)])];
        let result = ProgramChecker::new().check_program(&modules);

        assert_eq!(result.violation_count(), 1);
        assert!(matches!(
            result.violations[0].kind,
            ViolationKind::UnrefinedParam { param_index: 1, ref param_name }
            if param_name == "hi"
        ));
    }

    /// A function with no parameters never produces UnrefinedParam violations.
    #[test]
    fn zero_param_function_is_clean() {
        let sig = FunctionSignature {
            params: vec![],
            return_type: unrefined(),
        };
        let modules = vec![make_module("m", &[("noop", sig)])];
        let result = ProgramChecker::new().check_program(&modules);
        assert!(result.is_clean());
    }

    // ─── Return type checking ─────────────────────────────────────────────────

    /// Unrefined return type is NOT flagged by default.
    #[test]
    fn unrefined_return_not_flagged_by_default() {
        let modules = vec![make_module("m", &[("f", annotated_sig("x"))])];
        // annotated_sig has unrefined return type.
        let result = ProgramChecker::new().check_program(&modules);
        assert!(result.is_clean(), "unrefined return not checked by default");
    }

    /// Unrefined return type IS flagged when `with_return_type_checking()`.
    #[test]
    fn unrefined_return_flagged_when_enabled() {
        let modules = vec![make_module("m", &[("f", annotated_sig("x"))])];
        let result = ProgramChecker::new()
            .with_return_type_checking()
            .check_program(&modules);
        assert_eq!(result.violation_count(), 1);
        assert!(matches!(result.violations[0].kind, ViolationKind::UnrefinedReturn));
        assert_eq!(result.violations[0].function_name, "f");
    }

    /// Annotated return type passes the return-type check.
    #[test]
    fn annotated_return_passes_return_type_check() {
        let sig = FunctionSignature {
            params: vec![("x".into(), int_annotation(range(0, 128)))],
            return_type: int_annotation(range(0, 128)), // annotated return ✓
        };
        let modules = vec![make_module("m", &[("f", sig)])];
        let result = ProgramChecker::new()
            .with_return_type_checking()
            .check_program(&modules);
        assert!(result.is_clean(), "annotated return should pass");
    }

    /// Unrefined param + unrefined return → 2 violations when return checking on.
    #[test]
    fn unrefined_param_and_return_two_violations() {
        let modules = vec![make_module("m", &[("f", unannotated_sig("x"))])];
        let result = ProgramChecker::new()
            .with_return_type_checking()
            .check_program(&modules);
        // One for the param, one for the return.
        assert_eq!(result.violation_count(), 2);
        assert!(result.violations.iter().any(|v| matches!(v.kind, ViolationKind::UnrefinedParam { .. })));
        assert!(result.violations.iter().any(|v| matches!(v.kind, ViolationKind::UnrefinedReturn)));
    }

    // ─── Multiple modules ─────────────────────────────────────────────────────

    /// Violations from multiple modules are all collected.
    #[test]
    fn violations_across_multiple_modules_collected() {
        let modules = vec![
            make_module("auth", &[("login", unannotated_sig("token"))]),
            make_module("storage", &[("read", unannotated_sig("key"))]),
        ];
        let result = ProgramChecker::new().check_program(&modules);

        assert_eq!(result.violation_count(), 2);
        // Each module contributes one violation.
        let module_names: Vec<&str> = result.violations.iter()
            .map(|v| v.module_name.as_str())
            .collect();
        assert!(module_names.contains(&"auth"));
        assert!(module_names.contains(&"storage"));
    }

    /// A clean module and a dirty module: only the dirty module produces violations.
    #[test]
    fn clean_and_dirty_module_mixed() {
        let modules = vec![
            make_module("clean", &[("f", annotated_sig("x"))]),
            make_module("dirty", &[("g", unannotated_sig("y"))]),
        ];
        let result = ProgramChecker::new().check_program(&modules);

        assert_eq!(result.violation_count(), 1);
        assert_eq!(result.violations[0].module_name, "dirty");
        assert_eq!(result.violations[0].function_name, "g");
    }

    // ─── ProgramCheckResult methods ───────────────────────────────────────────

    /// `error_message` returns empty string when no violations.
    #[test]
    fn error_message_empty_when_clean() {
        let result = ProgramCheckResult { violations: vec![] };
        assert!(result.error_message().is_empty());
    }

    /// `error_message` includes violation count and each violation's description.
    #[test]
    fn error_message_lists_violations() {
        let modules = vec![make_module("m", &[("f", unannotated_sig("x"))])];
        let result = ProgramChecker::new().check_program(&modules);

        let msg = result.error_message();
        assert!(msg.contains("1 refinement annotation violation"), "count in message: {msg}");
        assert!(msg.contains("parameter 0 'x'"), "param info in message: {msg}");
        assert!(msg.contains("[m]"), "module name in message: {msg}");
    }

    /// `violating_modules` returns only the names of modules with violations.
    #[test]
    fn violating_modules_returns_correct_names() {
        let modules = vec![
            make_module("clean", &[("f", annotated_sig("x"))]),
            make_module("dirty", &[("g", unannotated_sig("y"))]),
        ];
        let result = ProgramChecker::new().check_program(&modules);
        let violators = result.violating_modules();
        assert_eq!(violators, vec!["dirty"]);
    }

    /// `violating_modules` deduplicates repeated module names.
    #[test]
    fn violating_modules_deduplicates() {
        // Two violations in the same module.
        let sig = FunctionSignature {
            params: vec![("a".into(), unrefined()), ("b".into(), unrefined())],
            return_type: unrefined(),
        };
        let modules = vec![make_module("m", &[("f", sig)])];
        let result = ProgramChecker::new().check_program(&modules);

        assert_eq!(result.violation_count(), 2, "two param violations");
        let violators = result.violating_modules();
        assert_eq!(violators.len(), 1, "deduplicated to one module name");
        assert_eq!(violators[0], "m");
    }

    /// `violation_count` matches the length of `violations`.
    #[test]
    fn violation_count_matches_vec_len() {
        let modules = vec![make_module("m", &[
            ("f", unannotated_sig("x")),
            ("g", unannotated_sig("y")),
        ])];
        let result = ProgramChecker::new().check_program(&modules);
        assert_eq!(result.violation_count(), result.violations.len());
    }

    // ─── Violation description format ─────────────────────────────────────────

    /// Violation description mentions module, function, param index, and name.
    #[test]
    fn violation_description_format_param() {
        let modules = vec![make_module("text/ascii", &[("decode", unannotated_sig("codepoint"))])];
        let result = ProgramChecker::new().check_program(&modules);

        let desc = &result.violations[0].description;
        assert!(desc.contains("[text/ascii]"), "module in desc: {desc}");
        assert!(desc.contains("decode"), "fn name in desc: {desc}");
        assert!(desc.contains("parameter 0"), "param index in desc: {desc}");
        assert!(desc.contains("'codepoint'"), "param name in desc: {desc}");
        assert!(desc.contains("': any'"), "annotation label in desc: {desc}");
    }

    /// Return violation description mentions "return type".
    #[test]
    fn violation_description_format_return() {
        let modules = vec![make_module("m", &[("f", annotated_sig("x"))])];
        let result = ProgramChecker::new()
            .with_return_type_checking()
            .check_program(&modules);

        let desc = &result.violations[0].description;
        assert!(desc.contains("return type"), "return type in desc: {desc}");
    }

    // ─── ProgramChecker default ───────────────────────────────────────────────

    /// `ProgramChecker::default()` is equivalent to `ProgramChecker::new()`.
    #[test]
    fn program_checker_default_same_as_new() {
        let modules = vec![make_module("m", &[("f", annotated_sig("x"))])];
        let r1 = ProgramChecker::new().check_program(&modules);
        let r2 = ProgramChecker::default().check_program(&modules);
        assert_eq!(r1.violation_count(), r2.violation_count());
    }
}
