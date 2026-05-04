//! # `lang-refinement-checker` — LANG23 refinement proof-obligation checker.
//!
//! **LANG23 PRs 23-C and 23-D.**  The compiler pass that takes `RefinedType`
//! annotations from the IIR, lowers their predicates to
//! `ConstraintInstructions`, runs them through `constraint-vm`, and classifies
//! the solver's answer into one of the three LANG23 outcomes:
//!
//! ```text
//! PROVEN_SAFE    → strip the runtime check; narrow the downstream type
//! PROVEN_UNSAFE  → compile error with a concrete counter-example value
//! UNKNOWN        → emit a runtime check; warn; proceed in lenient mode
//! ```
//!
//! ## Modules
//!
//! | Module | PR | Description |
//! |--------|----|-------------|
//! | (top-level) | 23-C | Per-binding `Checker`: checks one proof obligation at a time given concrete/predicated/unconstrained evidence. |
//! | [`function_checker`] | 23-D | Function-scope `FunctionChecker`: walks a CFG, accumulates guard predicates path-by-path, checks each return site. |
//!
//! ## Architecture
//!
//! ```text
//! lang-refined-types          (RefinedType, Predicate, Kind)
//!         │
//! lang-refinement-checker     (this crate)
//!         │  PR 23-C: Checker — per-binding proof obligations
//!         │  PR 23-D: FunctionChecker — CFG-based, path-sensitive
//!         │  both lower via ProgramBuilder → constraint-vm
//!         │
//! constraint-vm ──► constraint-engine ──► SAT / LIA tactics
//! ```
//!
//! ## Proof obligation: what are we proving?
//!
//! For each annotated binding we check:
//!
//! > "Given everything we know about the value (evidence), can the annotation
//! > predicate ever be violated?"
//!
//! Formally, the obligation is **refutation**: check satisfiability of
//! `E(x) ∧ ¬P(x)` where `E` is the evidence and `P` is the annotation.
//!
//! | Solver answers  | Interpretation | LANG23 outcome |
//! |-----------------|----------------|----------------|
//! | UNSAT            | No `x` satisfies evidence AND violates annotation → safe for all `x` | `PROVEN_SAFE` |
//! | SAT(model)       | Found `x` consistent with evidence that violates annotation → definite bug | `PROVEN_UNSAFE` |
//! | UNKNOWN          | Solver gave up (incomplete tactic, opaque predicate, …) | `UNKNOWN` |
//!
//! ## Evidence
//!
//! The three evidence shapes cover the common call-site patterns:
//!
//! | Evidence | Typical source | Example |
//! |----------|---------------|---------|
//! | `Concrete(v)` | Literal at call site | `(define x : (Int 1 256) 25)` |
//! | `Predicated(preds)` | Guard/annotation at call site | `(if (< x 128) (ascii-info x) …)` |
//! | `Unconstrained` | Unknown at compile time | `(define x : (Int 1 256) (read-int))` |
//!
//! ## Variable naming convention
//!
//! The checker uses the sentinel variable name `"__v"` for the value being
//! checked in all generated constraint programs.  Callers do not need to
//! know this — it's an implementation detail of the per-binding API.
//! The [`function_checker`] module uses actual parameter names in its CFG
//! model and remaps them to `"__v"` internally via [`function_checker::substitute_var`].
//!
//! ## Lenient vs strict mode
//!
//! This crate is **mode-agnostic**: it always returns one of the three
//! outcomes.  The caller (e.g., the compiler frontend or IIR lowering pass)
//! decides what to do with `UNKNOWN` based on the configured
//! `--refinement-mode`.  In lenient mode: emit runtime check.  In strict
//! mode: treat `UNKNOWN` like a compile error.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod function_checker;

use constraint_core::{Logic, Predicate as CorePredicate, Sort};
use constraint_engine::{Model, SolverResult, Value};
use constraint_vm::ProgramBuilder;
use lang_refined_types::{Kind, Predicate, RefinedType};

// ---------------------------------------------------------------------------
// Evidence — what we know about the value being checked
// ---------------------------------------------------------------------------

/// What the checker knows about the value of the binding being annotated.
///
/// The three variants cover the major call-site patterns the LANG23 compiler
/// encounters:
///
/// - `Concrete(v)`: The value is a compile-time constant literal.  The checker
///   evaluates the annotation predicate directly against `v`.
///
/// - `Predicated(preds)`: The value is constrained by a set of predicates
///   gathered from CFG guards, parameter annotations, and call-return types.
///   The checker builds a refutation query `evidence ∧ ¬annotation`.
///
/// - `Unconstrained`: The value is produced by a source the solver can not
///   characterise at compile time (e.g., user input, FFI, file I/O).  The
///   checker always returns `UNKNOWN` so the caller emits a runtime check.
#[derive(Debug, Clone)]
pub enum Evidence {
    /// The value is a compile-time integer constant (literal, enum member, …).
    Concrete(i128),
    /// A set of predicates holds over the value at this program point.
    ///
    /// Each predicate in the `Vec` is expressed over the variable `"__v"` —
    /// the same sentinel the checker uses internally.  Callers building
    /// evidence from CFG guards should substitute the guarded variable's name
    /// for `"__v"` before calling [`Checker::check`].
    Predicated(Vec<Predicate>),
    /// Nothing is known about the value at compile time.
    Unconstrained,
}

// ---------------------------------------------------------------------------
// CheckOutcome — the three LANG23 outcomes
// ---------------------------------------------------------------------------

/// A counter-example produced when the solver finds a concrete violation.
///
/// Provides enough detail for the compiler to emit a helpful error message:
/// *"The annotation requires `x ∈ [1, 256)`; the value `500` violates this."*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterExample {
    /// The integer value that violates the annotation predicate.
    pub value: i128,
    /// Human-readable description of the violation.
    ///
    /// Example: `"value 500 violates (Int 1 256)"`
    pub description: String,
}

/// The outcome of a single refinement proof obligation.
///
/// Maps directly to the three-way split in the LANG23 spec:
///
/// ```text
///      ┌─────────────────────────────┐
///      │  proof obligation           │
///      └──────────────┬──────────────┘
///                     │
///          ┌──────────┼──────────┐
///          ▼          ▼          ▼
///      ProvenSafe  ProvenUnsafe  Unknown
///          │          │          │
///          ▼          ▼          ▼
///      strip RT    compile    emit RT
///      check       error      check
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// The annotation predicate holds for every value consistent with the
    /// evidence.  The caller may strip the runtime check and narrow the
    /// downstream type accordingly.
    ProvenSafe,
    /// The solver found a concrete value consistent with the evidence that
    /// violates the annotation predicate.  This is a definite compile-time
    /// bug.  The caller should emit an error with the counter-example.
    ProvenUnsafe(CounterExample),
    /// The solver could not determine the outcome (incomplete tactic, opaque
    /// predicate, unsupported sort, …).  The caller should emit a runtime
    /// check and (in lenient mode) proceed.
    Unknown(String),
}

impl CheckOutcome {
    /// Return `true` if this outcome is `ProvenSafe`.
    pub fn is_safe(&self) -> bool {
        matches!(self, CheckOutcome::ProvenSafe)
    }

    /// Return `true` if this outcome is `ProvenUnsafe`.
    pub fn is_unsafe(&self) -> bool {
        matches!(self, CheckOutcome::ProvenUnsafe(_))
    }

    /// Return `true` if this outcome is `Unknown`.
    pub fn is_unknown(&self) -> bool {
        matches!(self, CheckOutcome::Unknown(_))
    }

    /// Extract the counter-example if `ProvenUnsafe`, otherwise `None`.
    pub fn counter_example(&self) -> Option<&CounterExample> {
        match self {
            CheckOutcome::ProvenUnsafe(cx) => Some(cx),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Checker — the main entry point
// ---------------------------------------------------------------------------

/// The refinement obligation checker.
///
/// Construct once and call [`Checker::check`] for each proof obligation.
///
/// The checker is stateless between calls — each invocation builds and runs
/// an independent constraint program.  Consumers that need incremental
/// solving or scope-sharing should use `constraint-vm` directly.
///
/// # Example — literal in range
///
/// ```rust
/// use lang_refined_types::{RefinedType, Kind, Predicate};
/// use lang_refinement_checker::{Checker, Evidence, CheckOutcome};
///
/// let annotation = RefinedType::refined(
///     Kind::Int,
///     Predicate::Range { lo: Some(1), hi: Some(256), inclusive_hi: false },
/// );
/// let mut checker = Checker::new();
///
/// // Value 25 is in [1, 256) → proven safe.
/// assert!(checker.check(&annotation, &Evidence::Concrete(25)).is_safe());
///
/// // Value 500 is NOT in [1, 256) → proven unsafe.
/// assert!(checker.check(&annotation, &Evidence::Concrete(500)).is_unsafe());
///
/// // Unknown source → unknown outcome; caller emits runtime check.
/// assert!(checker.check(&annotation, &Evidence::Unconstrained).is_unknown());
/// ```
#[derive(Debug, Default)]
pub struct Checker;

impl Checker {
    /// Construct a new checker.
    pub fn new() -> Self {
        Checker
    }

    /// Check a single refinement proof obligation.
    ///
    /// # Arguments
    ///
    /// * `annotation` — the `RefinedType` declared by the source-code
    ///   annotation.  If the annotation has no predicate (`is_unrefined()`),
    ///   this function immediately returns `ProvenSafe`.
    ///
    /// * `evidence` — what the compiler knows about the value being checked
    ///   at the point of use.
    ///
    /// # Outcomes
    ///
    /// See [`CheckOutcome`] for the three possible outcomes.  The caller
    /// interprets them according to the configured `--refinement-mode`.
    pub fn check(&mut self, annotation: &RefinedType, evidence: &Evidence) -> CheckOutcome {
        // ── (1) Unrefined annotation → nothing to prove ──────────────────────
        let predicate = match &annotation.predicate {
            None => return CheckOutcome::ProvenSafe,
            Some(p) => p,
        };

        // ── (2) Kind must be solver-supported in v1 ───────────────────────────
        //
        // Float, Str, Any, Nil, and ClassId all degrade to Opaque in v1.
        // We still handle Opaque predicates specially below — but if the
        // *kind itself* is not LIA/Bool territory, the solver has nothing
        // to say even if the predicate looks simple.
        if !annotation.kind.is_solver_supported() {
            return CheckOutcome::Unknown(format!(
                "kind `{}` is not supported by the solver in v1; \
                 predicate degrades to a runtime check",
                annotation.kind
            ));
        }

        // ── (3) Dispatch on evidence type ─────────────────────────────────────
        match evidence {
            Evidence::Concrete(v) => self.check_concrete(*v, annotation, predicate),
            Evidence::Predicated(preds) => {
                self.check_predicated(preds, annotation, predicate)
            }
            Evidence::Unconstrained => CheckOutcome::Unknown(
                "value is unconstrained at compile time; emitting runtime check".into(),
            ),
        }
    }

    // -------------------------------------------------------------------------
    // Concrete evidence: value is a compile-time constant
    // -------------------------------------------------------------------------

    fn check_concrete(
        &self,
        value: i128,
        annotation: &RefinedType,
        predicate: &Predicate,
    ) -> CheckOutcome {
        // Fast path: evaluate the predicate directly without the solver.
        // This avoids IPC / constraint-vm overhead for the common case.
        match eval_predicate_concrete(predicate, value) {
            Some(true) => CheckOutcome::ProvenSafe,
            Some(false) => CheckOutcome::ProvenUnsafe(CounterExample {
                value,
                description: format!(
                    "value {value} violates annotation `{}`",
                    annotation.kind
                ),
            }),
            None => {
                // Predicate couldn't be evaluated directly (e.g., LinearCmp
                // with multiple variables).  Fall back to the solver.
                self.check_concrete_via_solver(value, annotation, predicate)
            }
        }
    }

    fn check_concrete_via_solver(
        &self,
        value: i128,
        annotation: &RefinedType,
        predicate: &Predicate,
    ) -> CheckOutcome {
        // Strategy: assert x = value AND ¬P(x), then check-sat.
        // UNSAT → value satisfies P → ProvenSafe.
        // SAT → value violates P → ProvenUnsafe.
        let neg_p = match predicate.to_constraint_predicate("__v") {
            Some(p) => negate(p),
            None => {
                return CheckOutcome::Unknown(
                    "predicate contains Opaque; cannot evaluate with solver".into(),
                )
            }
        };

        let prog = start_program_for_kind(&annotation.kind)
            .assert_eq_int("__v", value)
            .assert_pred(neg_p)
            .check_sat()
            .build();

        match constraint_vm::check_sat(&prog) {
            Ok(SolverResult::Unsat) => CheckOutcome::ProvenSafe,
            Ok(SolverResult::Sat(_)) => CheckOutcome::ProvenUnsafe(CounterExample {
                value,
                description: format!(
                    "value {value} violates annotation `{}`",
                    annotation.kind
                ),
            }),
            Ok(SolverResult::Unknown(r)) => CheckOutcome::Unknown(r),
            Err(e) => CheckOutcome::Unknown(format!("VM error: {e}")),
        }
    }

    // -------------------------------------------------------------------------
    // Predicated evidence: value satisfies a set of predicates
    // -------------------------------------------------------------------------

    fn check_predicated(
        &self,
        evidence_preds: &[Predicate],
        annotation: &RefinedType,
        annotation_pred: &Predicate,
    ) -> CheckOutcome {
        // Strategy: assert ∧E ∧ ¬P and check satisfiability (refutation).
        //
        //   UNSAT   → no value consistent with evidence can violate annotation → ProvenSafe
        //   SAT(m)  → m["__v"] witnesses the violation → ProvenUnsafe
        //   UNKNOWN → solver gave up → Unknown

        // Lower annotation predicate to its negation in core form.
        let neg_p = match annotation_pred.to_constraint_predicate("__v") {
            Some(p) => negate(p),
            None => {
                return CheckOutcome::Unknown(
                    "annotation predicate is Opaque; cannot reason with solver".into(),
                )
            }
        };

        // Lower evidence predicates.
        let mut evidence_core: Vec<CorePredicate> = Vec::new();
        for ep in evidence_preds {
            match ep.to_constraint_predicate("__v") {
                Some(p) => evidence_core.push(p),
                None => {
                    return CheckOutcome::Unknown(
                        "evidence contains Opaque predicate; cannot reason fully".into(),
                    );
                }
            }
        }

        // ── Pass 1: check-sat only (no get-model) ─────────────────────────────
        //
        // We deliberately separate the SAT check from model extraction so that
        // an UNSAT result (which forbids get-model) doesn't produce a VmError.
        let check_prog = {
            let mut b = start_program_for_kind(&annotation.kind);
            if let Some((lo, hi)) = annotation.kind.integer_bounds() {
                b = b.assert_ge_int("__v", lo).assert_le_int("__v", hi);
            }
            for ep in evidence_core.iter() {
                b = b.assert_pred(ep.clone());
            }
            b.assert_pred(neg_p.clone()).check_sat().build()
        };

        let sat_result = match constraint_vm::check_sat(&check_prog) {
            Ok(r) => r,
            Err(e) => return CheckOutcome::Unknown(format!("VM error: {e}")),
        };

        match sat_result {
            // ── UNSAT: evidence entails annotation → safe ──────────────────
            SolverResult::Unsat => CheckOutcome::ProvenSafe,

            // ── UNKNOWN: solver gave up → defer to runtime ─────────────────
            SolverResult::Unknown(r) => CheckOutcome::Unknown(r),

            // ── SAT: there exists a witness violating the annotation ────────
            // Pass 2: re-run with get-model to extract the counter-example.
            SolverResult::Sat(_) => {
                let model_prog = {
                    let mut b = start_program_for_kind(&annotation.kind);
                    if let Some((lo, hi)) = annotation.kind.integer_bounds() {
                        b = b.assert_ge_int("__v", lo).assert_le_int("__v", hi);
                    }
                    for ep in evidence_core.iter() {
                        b = b.assert_pred(ep.clone());
                    }
                    b.assert_pred(neg_p).check_sat().get_model().build()
                };

                match constraint_vm::get_model(&model_prog) {
                    Ok(Some(model)) => {
                        let cx_value = extract_int_value(&model, "__v");
                        CheckOutcome::ProvenUnsafe(CounterExample {
                            value: cx_value,
                            description: format!(
                                "counter-example {cx_value} is consistent with evidence \
                                 but violates annotation `{}`",
                                annotation.kind
                            ),
                        })
                    }
                    Ok(None) | Err(_) => {
                        // SAT but model not extractable — still unsafe, no specific value.
                        CheckOutcome::ProvenUnsafe(CounterExample {
                            value: 0,
                            description: format!(
                                "annotation `{}` is violated (no specific counter-example available)",
                                annotation.kind
                            ),
                        })
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Direct predicate evaluation (no solver) for concrete values
// ---------------------------------------------------------------------------

/// Try to evaluate `predicate` at a concrete integer `value`.
///
/// Returns `Some(true)` if the predicate holds, `Some(false)` if it doesn't,
/// and `None` if the predicate cannot be evaluated without a solver (e.g.,
/// `LinearCmp` with multiple variables, `Opaque`).
fn eval_predicate_concrete(predicate: &Predicate, value: i128) -> Option<bool> {
    match predicate {
        Predicate::Range { lo, hi, inclusive_hi } => {
            if let Some(lo_val) = lo {
                if value < *lo_val {
                    return Some(false);
                }
            }
            if let Some(hi_val) = hi {
                let ok = if *inclusive_hi {
                    value <= *hi_val
                } else {
                    value < *hi_val
                };
                if !ok {
                    return Some(false);
                }
            }
            Some(true)
        }

        Predicate::Membership { values } => {
            Some(values.contains(&value))
        }

        Predicate::And(preds) => {
            for p in preds {
                match eval_predicate_concrete(p, value) {
                    Some(false) => return Some(false),
                    None => return None,
                    Some(true) => {}
                }
            }
            Some(true)
        }

        Predicate::Or(preds) => {
            for p in preds {
                match eval_predicate_concrete(p, value) {
                    Some(true) => return Some(true),
                    None => return None,
                    Some(false) => {}
                }
            }
            Some(false)
        }

        Predicate::Not(inner) => {
            eval_predicate_concrete(inner, value).map(|b| !b)
        }

        // LinearCmp might have a single variable `__v` → can evaluate.
        // If it has multiple coefficients over other variables, we can't.
        Predicate::LinearCmp { coefs, op, rhs } => {
            // Evaluable iff all coefficients are over the same "__v" variable
            // (single-variable linear inequality).
            if coefs.iter().all(|(vid, _)| vid.0 == "__v") {
                // Use checked arithmetic to avoid integer overflow when
                // coefficient or value is near i128::MIN/MAX.  On overflow
                // we return None so the solver handles it instead.
                let lhs_opt: Option<i128> = coefs.iter().try_fold(0i128, |acc, (_, c)| {
                    let term = c.checked_mul(value)?;
                    acc.checked_add(term)
                });
                match lhs_opt {
                    None => None, // arithmetic overflow → fall through to solver
                    Some(lhs) => Some(match op {
                        lang_refined_types::CmpOp::Lt => lhs < *rhs,
                        lang_refined_types::CmpOp::Le => lhs <= *rhs,
                        lang_refined_types::CmpOp::Eq => lhs == *rhs,
                        lang_refined_types::CmpOp::Ge => lhs >= *rhs,
                        lang_refined_types::CmpOp::Gt => lhs > *rhs,
                    }),
                }
            } else {
                None // multi-variable: defer to solver
            }
        }

        Predicate::Opaque { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// Helpers — bridging lang-refined-types to constraint-core/vm
// ---------------------------------------------------------------------------

/// Return the `constraint_core::Sort` for the given `Kind`.
fn kind_to_sort(kind: &Kind) -> Sort {
    if kind.is_integer() {
        Sort::Int
    } else if matches!(kind, Kind::Bool) {
        Sort::Bool
    } else {
        // Fallback (caller should have checked is_solver_supported before here).
        Sort::Uninterpreted(kind.to_string())
    }
}

/// Return the tightest supported `Logic` for a given sort.
fn sort_to_logic(sort: &Sort) -> Logic {
    match sort {
        Sort::Bool => Logic::QF_Bool,
        Sort::Int => Logic::QF_LIA,
        _ => Logic::QF_LIA, // best-effort
    }
}

/// Negate a `constraint_core::Predicate`.
///
/// We wrap in `Not` and leave simplification to the engine/normaliser rather
/// than attempting to push negation through here.
fn negate(p: CorePredicate) -> CorePredicate {
    CorePredicate::Not(Box::new(p))
}

/// Create a `ProgramBuilder` pre-configured for the given `Kind`.
///
/// Declares the sentinel variable `"__v"` with the appropriate sort and
/// sets the matching SMT logic so the engine picks the right tactic.
fn start_program_for_kind(kind: &Kind) -> ProgramBuilder {
    let sort = kind_to_sort(kind);
    let logic = sort_to_logic(&sort);
    let builder = ProgramBuilder::new().set_logic(logic);
    match sort {
        Sort::Bool => builder.declare_bool("__v"),
        _ => builder.declare_int("__v"),
    }
}

/// Extract an integer value from a model.
///
/// Returns 0 if the variable is missing or not an integer (conservative
/// fallback — callers report the model value in error messages, not in
/// safety decisions).
fn extract_int_value(model: &Model, var: &str) -> i128 {
    match model.get(var) {
        Some(Value::Int(n)) => *n,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Obligation — higher-level API for the compiler pass
// ---------------------------------------------------------------------------

/// A single refinement proof obligation: one annotated binding at one use site.
///
/// Packages all the context needed to call [`Checker::check`] in a
/// serialisable struct — useful for batch processing and for recording
/// obligations in the IIR for deferred checking.
#[derive(Debug, Clone)]
pub struct Obligation {
    /// A human-readable label (function name, variable name, source location)
    /// for use in error messages.
    pub label: String,
    /// The declared annotation type.
    pub annotation: RefinedType,
    /// What is known about the value at the point of use.
    pub evidence: Evidence,
}

impl Obligation {
    /// Construct a new obligation.
    pub fn new(
        label: impl Into<String>,
        annotation: RefinedType,
        evidence: Evidence,
    ) -> Self {
        Obligation { label: label.into(), annotation, evidence }
    }

    /// Run this obligation through `checker` and return the outcome.
    pub fn check(&self, checker: &mut Checker) -> CheckOutcome {
        checker.check(&self.annotation, &self.evidence)
    }
}

/// Run a batch of obligations and collect their outcomes.
///
/// Returns `(label, outcome)` pairs in the same order as `obligations`.
///
/// This is the primary entry point for a compiler pass that has already
/// gathered all obligations from the IIR CFG and wants to discharge them
/// in one sweep.
pub fn check_all(obligations: &[Obligation]) -> Vec<(&str, CheckOutcome)> {
    let mut checker = Checker::new();
    obligations
        .iter()
        .map(|ob| (ob.label.as_str(), ob.check(&mut checker)))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use lang_refined_types::{CmpOp, Kind, Predicate, RefinedType, VarId};

    use super::*;

    // ─── helpers ─────────────────────────────────────────────────────────────

    fn range(lo: i128, hi: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: false }
    }

    fn range_inclusive(lo: i128, hi: i128) -> Predicate {
        Predicate::Range { lo: Some(lo), hi: Some(hi), inclusive_hi: true }
    }

    fn membership(values: &[i128]) -> Predicate {
        Predicate::Membership { values: values.to_vec() }
    }

    fn int_annotation(pred: Predicate) -> RefinedType {
        RefinedType::refined(Kind::Int, pred)
    }

    fn u8_annotation(pred: Predicate) -> RefinedType {
        RefinedType::refined(Kind::U8, pred)
    }

    fn unrefined_int() -> RefinedType {
        RefinedType::unrefined(Kind::Int)
    }

    // ─── Unrefined annotation ─────────────────────────────────────────────────

    #[test]
    fn unrefined_annotation_is_always_safe() {
        let mut ck = Checker::new();
        let ann = unrefined_int();

        assert!(ck.check(&ann, &Evidence::Concrete(42)).is_safe());
        assert!(ck.check(&ann, &Evidence::Unconstrained).is_safe());
        assert!(ck.check(&ann, &Evidence::Predicated(vec![])).is_safe());
    }

    // ─── Concrete evidence — Range ────────────────────────────────────────────

    #[test]
    fn concrete_in_range_is_safe() {
        let mut ck = Checker::new();
        let ann = int_annotation(range(1, 256));

        assert!(ck.check(&ann, &Evidence::Concrete(1)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(128)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(255)).is_safe());
    }

    #[test]
    fn concrete_below_lo_is_unsafe() {
        let mut ck = Checker::new();
        let ann = int_annotation(range(1, 256));

        let out = ck.check(&ann, &Evidence::Concrete(0));
        assert!(out.is_unsafe());
        assert_eq!(out.counter_example().map(|cx| cx.value), Some(0));
    }

    #[test]
    fn concrete_at_exclusive_hi_is_unsafe() {
        let mut ck = Checker::new();
        let ann = int_annotation(range(0, 128)); // [0, 128) exclusive

        let out = ck.check(&ann, &Evidence::Concrete(128));
        assert!(out.is_unsafe());
    }

    #[test]
    fn concrete_at_inclusive_hi_is_safe() {
        let mut ck = Checker::new();
        let ann = int_annotation(range_inclusive(0, 128)); // [0, 128] inclusive

        assert!(ck.check(&ann, &Evidence::Concrete(128)).is_safe());
    }

    #[test]
    fn concrete_above_hi_is_unsafe() {
        let mut ck = Checker::new();
        let ann = int_annotation(range(1, 256));

        let out = ck.check(&ann, &Evidence::Concrete(500));
        assert!(out.is_unsafe(), "500 should be outside [1, 256)");
        assert_eq!(out.counter_example().unwrap().value, 500);
    }

    // ─── Concrete evidence — Membership ──────────────────────────────────────

    #[test]
    fn concrete_member_of_set_is_safe() {
        let mut ck = Checker::new();
        let ann = int_annotation(membership(&[1, 2, 3, 4]));

        for v in [1i128, 2, 3, 4] {
            assert!(ck.check(&ann, &Evidence::Concrete(v)).is_safe(), "v={v}");
        }
    }

    #[test]
    fn concrete_not_member_is_unsafe() {
        let mut ck = Checker::new();
        let ann = int_annotation(membership(&[1, 2, 3]));

        let out = ck.check(&ann, &Evidence::Concrete(5));
        assert!(out.is_unsafe());
        assert_eq!(out.counter_example().unwrap().value, 5);
    }

    // ─── Concrete evidence — And/Or/Not ──────────────────────────────────────

    #[test]
    fn concrete_and_predicate() {
        let mut ck = Checker::new();
        let pred = Predicate::and(vec![range(0, 100), range(50, 200)]);
        let ann = int_annotation(pred);

        // Intersection is [50, 100).
        assert!(ck.check(&ann, &Evidence::Concrete(50)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(99)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(49)).is_unsafe());
        assert!(ck.check(&ann, &Evidence::Concrete(100)).is_unsafe());
    }

    #[test]
    fn concrete_or_predicate() {
        let mut ck = Checker::new();
        let pred = Predicate::or(vec![range(0, 10), range(90, 100)]);
        let ann = int_annotation(pred);

        assert!(ck.check(&ann, &Evidence::Concrete(5)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(95)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(50)).is_unsafe());
    }

    #[test]
    fn concrete_not_predicate() {
        let mut ck = Checker::new();
        let pred = Predicate::not(membership(&[0, 1, 2]));
        let ann = int_annotation(pred);

        // Not in {0,1,2} → values 3+ are safe; 0,1,2 are unsafe.
        assert!(ck.check(&ann, &Evidence::Concrete(3)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(0)).is_unsafe());
        assert!(ck.check(&ann, &Evidence::Concrete(1)).is_unsafe());
    }

    // ─── Unconstrained evidence ───────────────────────────────────────────────

    #[test]
    fn unconstrained_is_always_unknown() {
        let mut ck = Checker::new();
        let ann = int_annotation(range(0, 128));
        assert!(ck.check(&ann, &Evidence::Unconstrained).is_unknown());
    }

    // ─── Predicated evidence ─────────────────────────────────────────────────

    #[test]
    fn predicated_evidence_implies_annotation_is_safe() {
        let mut ck = Checker::new();

        // Evidence: x ∈ [0, 50).  Annotation: x ∈ [0, 100).
        // [0, 50) ⊆ [0, 100) → proven safe (no value in [0,50) violates [0,100)).
        let evidence = vec![range(0, 50)];
        let ann = int_annotation(range(0, 100));

        let out = ck.check(&ann, &Evidence::Predicated(evidence));
        assert!(out.is_safe(), "expected ProvenSafe, got {out:?}");
    }

    #[test]
    fn predicated_evidence_with_violation_is_unsafe() {
        let mut ck = Checker::new();

        // Evidence: x ∈ [0, 200).  Annotation: x ∈ [0, 100).
        // Values in [100, 200) are in evidence but violate annotation.
        let evidence = vec![range(0, 200)];
        let ann = int_annotation(range(0, 100));

        let out = ck.check(&ann, &Evidence::Predicated(evidence));
        assert!(out.is_unsafe(), "expected ProvenUnsafe, got {out:?}");
        // Counter-example must be outside annotation range.
        let cx = out.counter_example().unwrap();
        assert!(cx.value >= 100 && cx.value < 200,
            "counter-example {} should be in [100, 200)", cx.value);
    }

    #[test]
    fn predicated_evidence_guard_narrows_safely() {
        let mut ck = Checker::new();

        // Simulate: `if (< n 128) (ascii-info n) ...`
        // Evidence (inside then-branch): n ∈ [0, ∞) AND n < 128
        // Annotation: n ∈ [0, 128)
        let evidence = vec![
            Predicate::Range { lo: Some(0), hi: None, inclusive_hi: false }, // n ≥ 0
            Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false }, // n < 128
        ];
        let ann = int_annotation(range(0, 128));

        let out = ck.check(&ann, &Evidence::Predicated(evidence));
        assert!(out.is_safe(), "guard narrowing should yield ProvenSafe; got {out:?}");
    }

    // ─── Unsupported kind → Unknown ───────────────────────────────────────────

    #[test]
    fn float_kind_is_unknown() {
        let mut ck = Checker::new();
        let ann = RefinedType::refined(
            Kind::Float,
            Predicate::Range { lo: Some(0), hi: Some(1), inclusive_hi: true },
        );
        assert!(ck.check(&ann, &Evidence::Concrete(0)).is_unknown());
    }

    #[test]
    fn str_kind_is_unknown() {
        let mut ck = Checker::new();
        let ann = RefinedType::refined(Kind::Str, membership(&[0, 1]));
        assert!(ck.check(&ann, &Evidence::Concrete(0)).is_unknown());
    }

    // ─── U8-bounded kind ─────────────────────────────────────────────────────

    #[test]
    fn u8_annotation_with_range() {
        let mut ck = Checker::new();
        // U8 ∈ [0, 255]; annotation further restricts to [0, 128)
        let ann = u8_annotation(range(0, 128));

        assert!(ck.check(&ann, &Evidence::Concrete(0)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(127)).is_safe());
        assert!(ck.check(&ann, &Evidence::Concrete(128)).is_unsafe());
    }

    // ─── LinearCmp evaluation ────────────────────────────────────────────────

    #[test]
    fn linear_cmp_concrete_single_var() {
        let mut ck = Checker::new();
        // 2 * x ≤ 10  →  x ≤ 5 (annotation in LinearCmp form)
        let pred = Predicate::LinearCmp {
            coefs: vec![(VarId("__v".into()), 2)],
            op: CmpOp::Le,
            rhs: 10,
        };
        let ann = int_annotation(pred);

        assert!(ck.check(&ann, &Evidence::Concrete(5)).is_safe()); // 2*5=10 ≤ 10 ✓
        assert!(ck.check(&ann, &Evidence::Concrete(6)).is_unsafe()); // 2*6=12 > 10 ✗
    }

    // ─── Obligation / check_all ───────────────────────────────────────────────

    #[test]
    fn obligation_check_all_batch() {
        let obligations = vec![
            Obligation::new(
                "ascii-index",
                int_annotation(range(0, 128)),
                Evidence::Concrete(64),
            ),
            Obligation::new(
                "out-of-range",
                int_annotation(range(0, 128)),
                Evidence::Concrete(200),
            ),
            Obligation::new(
                "read-int",
                int_annotation(range(0, 128)),
                Evidence::Unconstrained,
            ),
        ];

        let results = check_all(&obligations);
        assert_eq!(results.len(), 3);
        assert!(results[0].1.is_safe(),   "ascii-index(64) should be safe");
        assert!(results[1].1.is_unsafe(), "ascii-index(200) should be unsafe");
        assert!(results[2].1.is_unknown(), "unconstrained should be unknown");
    }

    // ─── Opaque predicate ────────────────────────────────────────────────────

    #[test]
    fn opaque_predicate_is_unknown() {
        let mut ck = Checker::new();
        let pred = Predicate::Opaque { display: "custom-invariant".into() };
        let ann = int_annotation(pred);

        // Opaque predicate → solver can't help → Unknown regardless of evidence.
        assert!(ck.check(&ann, &Evidence::Concrete(42)).is_unknown());
        assert!(ck.check(&ann, &Evidence::Unconstrained).is_unknown());
    }

    // ─── CheckOutcome accessors ───────────────────────────────────────────────

    #[test]
    fn check_outcome_accessors() {
        let safe = CheckOutcome::ProvenSafe;
        assert!(safe.is_safe());
        assert!(!safe.is_unsafe());
        assert!(!safe.is_unknown());
        assert!(safe.counter_example().is_none());

        let cx = CounterExample { value: 500, description: "test".into() };
        let unsafe_out = CheckOutcome::ProvenUnsafe(cx.clone());
        assert!(!unsafe_out.is_safe());
        assert!(unsafe_out.is_unsafe());
        assert!(!unsafe_out.is_unknown());
        assert_eq!(unsafe_out.counter_example(), Some(&cx));

        let unk = CheckOutcome::Unknown("reason".into());
        assert!(!unk.is_safe());
        assert!(!unk.is_unsafe());
        assert!(unk.is_unknown());
        assert!(unk.counter_example().is_none());
    }
}
