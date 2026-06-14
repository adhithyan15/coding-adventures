//! # Twig `RefinementBridge` implementation (LANG54)
//!
//! `TwigRefinementBridge` connects the generic
//! [`lang_refinement_protocol::RefinementBridge`] trait to Twig's concrete
//! AST and kind types.  It is the *only* place where Twig-specific knowledge
//! lives — the generic [`check_call_site_refinements`] and
//! [`compute_if_narrowing`] functions do the rest.
//!
//! ## Implementing the three methods
//!
//! ### `evidence_for` — mapping Twig expressions to proof-obligation evidence
//!
//! | Twig expression | Evidence |
//! |---|---|
//! | `Expr::IntLit(n)` | `Concrete(n)` — exact solver check |
//! | `Expr::VarRef` with `TwigKind::RefinedInt(p)` in scope | `Predicated([p])` |
//! | anything else | `Unconstrained` — solver cannot determine outcome |
//!
//! ### `narrowing_facts` — delegating to `narrowing::extract_narrowing_facts`
//!
//! Re-uses the existing `src/narrowing.rs` guard-analysis logic (unchanged
//! from LANG53).  The bridge is the thin adapter that plugs it in.
//!
//! ### `narrow_kind` — delegating to `narrowing::merge_kind_with_predicate`
//!
//! Re-uses the existing `src/narrowing.rs` kind-merging logic (unchanged
//! from LANG53).
//!
//! ## Usage in `check.rs`
//!
//! ```rust,ignore
//! // In infer_apply:
//! let diags = check_call_site_refinements(
//!     &TwigRefinementBridge,
//!     callee_name, app.line, app.column,
//!     &app.args, &arg_kinds, param_refinements, mode.into(),
//! );
//!
//! // In infer_if:
//! let narrowed = compute_if_narrowing(
//!     &TwigRefinementBridge,
//!     &if_expr.cond,
//!     |var| scope.lookup(var).cloned().or_else(|| env.lookup_global(var).cloned()),
//! );
//! ```

use lang_refinement_protocol::{Evidence, Predicate, RefinementBridge};
use twig_parser::Expr;

use crate::kinds::TwigKind;
use crate::narrowing::{extract_narrowing_facts, merge_kind_with_predicate};

/// The `RefinementBridge` implementation for the Twig type system.
///
/// Bridges [`lang_refinement_protocol`]'s generic checker functions to Twig's
/// [`Expr`] AST and [`TwigKind`] type system.
///
/// This struct is zero-sized (no fields) — construct with `TwigRefinementBridge`.
///
/// # Example
///
/// ```rust,ignore
/// use twig_type_checker::bridge::TwigRefinementBridge;
/// use lang_refinement_protocol::{check_call_site_refinements, RefinementMode};
///
/// let diags = check_call_site_refinements(
///     &TwigRefinementBridge,
///     "ascii-info",   // callee name
///     10, 5,          // line, column
///     &arg_exprs,
///     &arg_kinds,
///     &param_refinements,
///     RefinementMode::Strict,
/// );
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TwigRefinementBridge;

impl RefinementBridge for TwigRefinementBridge {
    /// Twig's AST expression node — parsed from source by `twig-parser`.
    type Expr = Expr;

    /// Twig's kind system — the static type inferred for each expression.
    ///
    /// `RefinedInt(p)` is the key variant: it carries the narrowing predicate
    /// gathered from guard analysis or parameter annotations.
    type Kind = TwigKind;

    /// Classify a call-site argument as refinement `Evidence`.
    ///
    /// ## Classification rules (in order)
    ///
    /// 1. **`IntLit(n)`** → `Concrete(n as i128)`.
    ///    The literal value is known exactly at compile time — the solver can
    ///    evaluate the annotation predicate without any constraint programs.
    ///
    /// 2. **`VarRef` with `RefinedInt(p)` kind** → `Predicated([p])`.
    ///    The variable has been narrowed by a guard (or has a refined annotation
    ///    from its binding site).  The solver checks whether every value
    ///    satisfying `p` also satisfies the callee's annotation.
    ///
    /// 3. **`VarRef` with `Int` / `Any` kind** → `Unconstrained`.
    ///    The type checker knows the variable is an integer but has no further
    ///    bounds.  The solver cannot determine the outcome — emit a runtime
    ///    check (lenient) or an error (strict).
    ///
    /// 4. **Anything else** → `Unconstrained`.
    ///    Complex expressions (lambdas, applies, lets, …) are not analysed;
    ///    conservatively Unconstrained.
    fn evidence_for(&self, expr: &Expr, inferred_kind: Option<&TwigKind>) -> Evidence {
        match expr {
            // ── Integer literal: the value is known exactly ───────────────────
            Expr::IntLit(lit) => Evidence::Concrete(lit.value as i128),

            // ── Variable reference: check if the inferred kind carries a pred ─
            Expr::VarRef(_) => match inferred_kind {
                Some(TwigKind::RefinedInt(pred)) => {
                    // The variable's kind has been narrowed to a specific
                    // integer predicate — pass that as Predicated evidence.
                    Evidence::Predicated(vec![pred.clone()])
                }
                // Int (unrefined) or Any: the solver knows the value is an
                // integer but has no further constraint on it.
                _ => Evidence::Unconstrained,
            },

            // ── All other expressions: no static evidence ─────────────────────
            //
            // Lambda, Apply, Let, LetStar, Begin, Match, StrLit, BoolLit,
            // NilLit, SymLit — none of these have a compile-time integer value
            // the solver can use.  Fall through to Unconstrained.
            _ => Evidence::Unconstrained,
        }
    }

    /// Extract narrowing facts from a Twig guard expression.
    ///
    /// Delegates entirely to [`extract_narrowing_facts`] from `narrowing.rs`
    /// (LANG53).  The bridge is the thin adapter connecting the generic
    /// protocol to the existing guard-analysis implementation.
    ///
    /// Handles:
    /// - `(< x k)`, `(<= x k)`, `(> x k)`, `(>= x k)`, `(= x k)` with
    ///   `VarRef op IntLit` form.
    /// - `(and c1 c2 …)` — conjunction, merging facts.
    /// - `(not c)` — negation.
    /// - Everything else → empty (conservative; no narrowing applied).
    fn narrowing_facts(&self, guard: &Expr) -> Vec<(String, Predicate)> {
        extract_narrowing_facts(guard)
    }

    /// Narrow a `TwigKind` by intersecting it with a guard predicate.
    ///
    /// Delegates to [`merge_kind_with_predicate`] from `narrowing.rs` (LANG53).
    ///
    /// | Base kind | Result |
    /// |---|---|
    /// | `Int` | `RefinedInt(pred)` — add first refinement |
    /// | `RefinedInt(existing)` | `RefinedInt(and([existing, pred]))` — intersect |
    /// | Any other kind | unchanged — non-integer kinds are not narrowed |
    fn narrow_kind(&self, base: &TwigKind, pred: Predicate) -> TwigKind {
        merge_kind_with_predicate(base, pred)
    }
}

// ---------------------------------------------------------------------------
// Tests — TwigRefinementBridge specifically
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use lang_refined_types::{Predicate, RefinedType, Kind as RefKind};
    use lang_refinement_protocol::{
        check_call_site_refinements, compute_if_narrowing, RefinementMode,
    };
    use twig_parser::{IntLit, VarRef};

    use super::*;

    // ─── evidence_for ─────────────────────────────────────────────────────────

    #[test]
    fn evidence_int_lit_is_concrete() {
        let bridge = TwigRefinementBridge;
        let lit = Expr::IntLit(IntLit { value: 42, line: 1, column: 1 });
        assert!(matches!(bridge.evidence_for(&lit, None), Evidence::Concrete(42)));
    }

    #[test]
    fn evidence_var_ref_with_refined_kind_is_predicated() {
        let bridge = TwigRefinementBridge;
        let pred = Predicate::Range { lo: Some(0), hi: Some(50), inclusive_hi: false };
        let var = Expr::VarRef(VarRef { name: "x".into(), line: 1, column: 1 });
        let kind = TwigKind::RefinedInt(pred.clone());
        let ev = bridge.evidence_for(&var, Some(&kind));
        assert!(matches!(ev, Evidence::Predicated(_)));
    }

    #[test]
    fn evidence_var_ref_with_plain_int_is_unconstrained() {
        let bridge = TwigRefinementBridge;
        let var = Expr::VarRef(VarRef { name: "n".into(), line: 1, column: 1 });
        let ev = bridge.evidence_for(&var, Some(&TwigKind::Int));
        assert!(matches!(ev, Evidence::Unconstrained));
    }

    #[test]
    fn evidence_bool_lit_is_unconstrained() {
        use twig_parser::BoolLit;
        let bridge = TwigRefinementBridge;
        let b = Expr::BoolLit(BoolLit { value: true, line: 1, column: 1 });
        assert!(matches!(bridge.evidence_for(&b, None), Evidence::Unconstrained));
    }

    // ─── narrow_kind ──────────────────────────────────────────────────────────

    #[test]
    fn narrow_int_produces_refined_int() {
        let bridge = TwigRefinementBridge;
        let pred = Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false };
        let narrowed = bridge.narrow_kind(&TwigKind::Int, pred);
        assert!(matches!(narrowed, TwigKind::RefinedInt(_)));
    }

    #[test]
    fn narrow_refined_int_intersects_predicates() {
        let bridge = TwigRefinementBridge;
        let existing = Predicate::Range { lo: Some(0), hi: None, inclusive_hi: false };
        let new_pred = Predicate::Range { lo: None, hi: Some(128), inclusive_hi: false };
        let narrowed = bridge.narrow_kind(&TwigKind::RefinedInt(existing), new_pred);
        // Should be RefinedInt(And([…]))
        assert!(matches!(narrowed, TwigKind::RefinedInt(Predicate::And(_))));
    }

    #[test]
    fn narrow_bool_is_unchanged() {
        let bridge = TwigRefinementBridge;
        let pred = Predicate::Range { lo: Some(0), hi: Some(1), inclusive_hi: true };
        let narrowed = bridge.narrow_kind(&TwigKind::Bool, pred);
        assert_eq!(narrowed, TwigKind::Bool);
    }

    // ─── Integration via generic functions ───────────────────────────────────

    #[test]
    fn check_call_site_int_lit_in_range_no_diagnostic() {
        let ann = RefinedType::refined(
            RefKind::Int,
            Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
        );
        let lit = Expr::IntLit(IntLit { value: 42, line: 1, column: 1 });
        let diags = check_call_site_refinements(
            &TwigRefinementBridge,
            "ascii-info",
            1, 1,
            &[lit],
            &[TwigKind::Int],
            &[Some(ann)],
            RefinementMode::Strict,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn check_call_site_int_lit_out_of_range_is_diagnostic() {
        let ann = RefinedType::refined(
            RefKind::Int,
            Predicate::Range { lo: Some(0), hi: Some(128), inclusive_hi: false },
        );
        let lit = Expr::IntLit(IntLit { value: 200, line: 3, column: 5 });
        let diags = check_call_site_refinements(
            &TwigRefinementBridge,
            "ascii-info",
            3, 5,
            &[lit],
            &[TwigKind::Int],
            &[Some(ann)],
            RefinementMode::Strict,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("ascii-info"));
        assert_eq!(diags[0].line, 3);
    }

    #[test]
    fn compute_if_narrowing_narrows_variable() {
        use twig_parser::{Apply, VarRef as VR, IntLit as IL};
        // Build a guard: (< x 128) — Apply with fn_expr=VarRef("<"), args=[VarRef("x"), IntLit(128)]
        let guard = Expr::Apply(Apply {
            fn_expr: Box::new(Expr::VarRef(VR { name: "<".into(), line: 1, column: 1 })),
            args: vec![
                Expr::VarRef(VR { name: "x".into(), line: 1, column: 3 }),
                Expr::IntLit(IL { value: 128, line: 1, column: 5 }),
            ],
            line: 1,
            column: 1,
        });
        let nb = compute_if_narrowing(
            &TwigRefinementBridge,
            &guard,
            |var| if var == "x" { Some(TwigKind::Int) } else { None },
        );
        // True branch: x narrowed to RefinedInt.
        assert_eq!(nb.true_branch.len(), 1);
        assert_eq!(nb.true_branch[0].0, "x");
        assert!(matches!(nb.true_branch[0].1, TwigKind::RefinedInt(_)));
        // False branch: x narrowed to RefinedInt(negated).
        assert_eq!(nb.false_branch.len(), 1);
        assert!(matches!(nb.false_branch[0].1, TwigKind::RefinedInt(_)));
    }
}
