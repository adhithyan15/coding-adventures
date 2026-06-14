//! Flow-sensitive type narrowing for `if`-guards (TW05-C).
//!
//! ## What is narrowing?
//!
//! When the Twig checker encounters `(if guard then else)`, the guard tells us
//! something about the *values* of variables at each branch.  If the guard is
//! `(< x 128)`, then:
//!
//! - In the **true** branch we know `x < 128`.
//! - In the **false** branch we know `x >= 128`.
//!
//! "Narrowing" means we update `x`'s kind in the scope to reflect this
//! additional knowledge.  If `x` is already `Int` or `RefinedInt(p)`, we can
//! narrow it to `RefinedInt(new_pred)` inside the branch, allowing the
//! refinement checker to prove call-site obligations it couldn't prove without
//! the guard.
//!
//! ## Example
//!
//! ```scheme
//! (define (ascii-info x : (Int 0 128)) ...)    ;; x ∈ [0, 128)
//! (define (process n)
//!   (if (< n 128)
//!     (ascii-info n)   ;; ← we know n < 128 here; narrowed to RefinedInt(Range{lo:None, hi:Some(128)})
//!     0))              ;; ← we know n >= 128 here
//! ```
//!
//! Without narrowing, `n` is `Any` (no annotation on the param), and the
//! call `(ascii-info n)` is `Unknown` → strict-mode error.  With narrowing,
//! the checker sees `n: RefinedInt(n < 128)` in the true branch, intersects
//! it with the annotation `[0, 128)`, and proves the call safe.
//!
//! ## Scope of TW05-C
//!
//! This module handles **AST-level** narrowing:
//! - Simple comparison guards: `(<`, `<=`, `>`, `>=`, `=`) where one side is a
//!   `VarRef` and the other is an `IntLit`.
//! - Logical combinations: `(and c1 c2 …)` merges facts; `(not c)` negates.
//! - Everything else: **conservative — no narrowing**.
//!
//! CFG-based loop invariants, inter-procedural narrowing (`(byte? x)` user
//! predicates), and `let`/`let*` binding refinements are deferred to TW05-D.

use lang_refined_types::Predicate;
use twig_parser::{Apply, Expr};

use crate::kinds::TwigKind;

// ---------------------------------------------------------------------------
// extract_narrowing_facts
// ---------------------------------------------------------------------------

/// Analyse a guard expression and return `(variable_name, narrowing_predicate)`
/// pairs for variables that can be narrowed in the *true* branch.
///
/// ## Handled forms
///
/// | Guard expression | Narrowing fact emitted |
/// |-----------------|------------------------|
/// | `(< x k)`      | `x: Range { lo: None, hi: Some(k), inclusive_hi: false }` |
/// | `(<= x k)`     | `x: Range { lo: None, hi: Some(k), inclusive_hi: true }` |
/// | `(> x k)`      | `x: Range { lo: Some(k+1), hi: None, inclusive_hi: false }` |
/// | `(>= x k)`     | `x: Range { lo: Some(k), hi: None, inclusive_hi: false }` |
/// | `(= x k)`      | `x: Range { lo: Some(k), hi: Some(k), inclusive_hi: true }` |
/// | `(and c1 c2 …)` | Merge all child facts; same var → `Predicate::and` |
/// | `(not c)`      | Negate each fact from `c` with `Predicate::not` |
/// | anything else  | Empty `Vec` (conservative) |
///
/// ## Symmetry
///
/// We only handle `(op VarRef IntLit)`.  `(op IntLit VarRef)` is left to the
/// TW05-D follow-up.  All comparisons are strict-left: `(< x k)` narrows `x`.
pub fn extract_narrowing_facts(guard: &Expr) -> Vec<(String, Predicate)> {
    match guard {
        // `(op arg1 arg2)` — look at the head name and operands.
        Expr::Apply(app) => extract_from_apply(app),

        // Any other expression form: no narrowing.
        _ => vec![],
    }
}

/// Helper — dispatch on the application head to extract facts.
fn extract_from_apply(app: &Apply) -> Vec<(String, Predicate)> {
    // The function position must be a VarRef to a builtin comparison operator.
    let op_name = match app.fn_expr.as_ref() {
        Expr::VarRef(v) => v.name.as_str(),
        _ => return vec![],
    };

    match op_name {
        // ── Binary comparison operators ─────────────────────────────────────
        "<" | "<=" | ">" | ">=" | "=" => {
            if app.args.len() != 2 {
                return vec![];
            }
            extract_comparison(op_name, &app.args[0], &app.args[1])
        }

        // ── Logical conjunction: (and c1 c2 …) ─────────────────────────────
        "and" => {
            // Merge all children's facts.  When the same variable appears in
            // more than one child, combine their predicates with `Predicate::and`.
            let mut facts: Vec<(String, Predicate)> = vec![];
            for child in &app.args {
                let child_facts = extract_narrowing_facts(child);
                for (var, pred) in child_facts {
                    merge_fact_into(&mut facts, var, pred);
                }
            }
            facts
        }

        // ── Logical negation: (not c) ───────────────────────────────────────
        "not" => {
            if app.args.len() != 1 {
                return vec![];
            }
            // Negate each fact from the child.
            extract_narrowing_facts(&app.args[0])
                .into_iter()
                .map(|(var, pred)| (var, Predicate::not(pred)))
                .collect()
        }

        // ── Anything else: conservative ─────────────────────────────────────
        _ => vec![],
    }
}

/// Extract a narrowing fact from a binary comparison `(op lhs rhs)`.
///
/// Handles `(op VarRef IntLit)` only.  The predicate is expressed as a
/// `Range` over the variable — the variable's name is the key.
///
/// ## Derivation table
///
/// | op  | condition | Range produced |
/// |-----|-----------|----------------|
/// | `<`  | `x < k`   | `Range { lo: None, hi: Some(k), inclusive_hi: false }` |
/// | `<=` | `x <= k`  | `Range { lo: None, hi: Some(k), inclusive_hi: true }` |
/// | `>`  | `x > k`   | `Range { lo: Some(k+1), hi: None, … }` (equivalent to `x >= k+1`) |
/// | `>=` | `x >= k`  | `Range { lo: Some(k), hi: None, … }` |
/// | `=`  | `x = k`   | `Range { lo: Some(k), hi: Some(k), inclusive_hi: true }` (singleton) |
fn extract_comparison(op: &str, lhs: &Expr, rhs: &Expr) -> Vec<(String, Predicate)> {
    // Only handle (op VarRef IntLit).
    let (var_name, k) = match (lhs, rhs) {
        (Expr::VarRef(v), Expr::IntLit(i)) => (v.name.clone(), i.value as i128),
        _ => return vec![],
    };

    let pred = match op {
        "<" => Predicate::Range {
            lo: None,
            hi: Some(k),
            inclusive_hi: false, // x < k  ⟺  x ∈ (-∞, k)
        },
        "<=" => Predicate::Range {
            lo: None,
            hi: Some(k),
            inclusive_hi: true, // x <= k  ⟺  x ∈ (-∞, k]
        },
        ">" => Predicate::Range {
            // x > k  ⟺  x >= k+1  ⟺  x ∈ [k+1, +∞)
            lo: Some(k + 1),
            hi: None,
            inclusive_hi: false,
        },
        ">=" => Predicate::Range {
            lo: Some(k),
            hi: None,
            inclusive_hi: false, // x >= k  ⟺  x ∈ [k, +∞)
        },
        "=" => Predicate::Range {
            lo: Some(k),
            hi: Some(k),
            inclusive_hi: true, // x = k  ⟺  x ∈ {k}
        },
        _ => return vec![],
    };

    vec![(var_name, pred)]
}

/// Merge a `(var, pred)` fact into the accumulator.
///
/// If `var` already has a predicate in `acc`, combine the two with
/// `Predicate::and`.  Otherwise push a new entry.
fn merge_fact_into(acc: &mut Vec<(String, Predicate)>, var: String, pred: Predicate) {
    if let Some(existing) = acc.iter_mut().find(|(v, _)| *v == var) {
        // Intersect: both predicates must hold simultaneously.
        let combined = Predicate::and(vec![existing.1.clone(), pred]);
        existing.1 = combined;
    } else {
        acc.push((var, pred));
    }
}

// ---------------------------------------------------------------------------
// merge_kind_with_predicate
// ---------------------------------------------------------------------------

/// Narrow a variable's existing `TwigKind` by adding a refinement predicate
/// from a guard analysis.
///
/// ## Rules
///
/// | Base kind | Predicate | Result |
/// |-----------|-----------|--------|
/// | `Int` | `p` | `RefinedInt(p)` — guard constrains an unrefined int |
/// | `RefinedInt(existing)` | `p` | `RefinedInt(and([existing, p]))` — intersect |
/// | Any other kind | any | `base` unchanged (can't narrow `Bool`, `Str`, etc.) |
///
/// ## Rationale
///
/// When we enter the true branch of `(if (< x 128) …)`, `x`'s current kind
/// (which might be `Int` from an annotation, or `RefinedInt(p)` from an outer
/// guard) gets intersected with the new guard predicate.  If the base kind
/// isn't numeric, the guard is meaningless for type-narrowing (a `Bool` tested
/// by a numeric comparison is odd but not our concern here).
pub fn merge_kind_with_predicate(base: &TwigKind, pred: Predicate) -> TwigKind {
    match base {
        // Unrefined integer: the guard adds the first predicate.
        TwigKind::Int => TwigKind::RefinedInt(pred),

        // Already refined: intersect with the new guard predicate.
        TwigKind::RefinedInt(existing) => {
            TwigKind::RefinedInt(Predicate::and(vec![existing.clone(), pred]))
        }

        // Non-numeric kinds: don't touch.
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use twig_parser::{IntLit, VarRef};

    // Helper: build a simple `(op varname literal)` Apply node for testing.
    // Note: Expr::Apply(Apply{…}) — the variant takes Apply directly (not boxed);
    // Apply.fn_expr is Box<Expr> though, so Box::new is needed there.
    fn make_cmp(op: &str, var: &str, k: i64) -> Expr {
        Expr::Apply(Apply {
            fn_expr: Box::new(Expr::VarRef(VarRef {
                name: op.to_owned(),
                line: 1,
                column: 1,
            })),
            args: vec![
                Expr::VarRef(VarRef {
                    name: var.to_owned(),
                    line: 1,
                    column: 1,
                }),
                Expr::IntLit(IntLit {
                    value: k,
                    line: 1,
                    column: 1,
                }),
            ],
            line: 1,
            column: 1,
        })
    }

    // Helper: wrap an Expr in `(not ...)`.
    fn make_not(inner: Expr) -> Expr {
        Expr::Apply(Apply {
            fn_expr: Box::new(Expr::VarRef(VarRef {
                name: "not".to_owned(),
                line: 1,
                column: 1,
            })),
            args: vec![inner],
            line: 1,
            column: 1,
        })
    }

    // Helper: wrap two Exprs in `(and a b)`.
    fn make_and(a: Expr, b: Expr) -> Expr {
        Expr::Apply(Apply {
            fn_expr: Box::new(Expr::VarRef(VarRef {
                name: "and".to_owned(),
                line: 1,
                column: 1,
            })),
            args: vec![a, b],
            line: 1,
            column: 1,
        })
    }

    #[test]
    fn lt_produces_upper_bound() {
        // (< x 128) → x: Range { lo: None, hi: Some(128), inclusive_hi: false }
        let guard = make_cmp("<", "x", 128);
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1);
        let (var, pred) = &facts[0];
        assert_eq!(var, "x");
        assert_eq!(
            *pred,
            Predicate::Range {
                lo: None,
                hi: Some(128),
                inclusive_hi: false,
            }
        );
    }

    #[test]
    fn le_produces_inclusive_upper_bound() {
        // (<= x 127) → x: Range { lo: None, hi: Some(127), inclusive_hi: true }
        let guard = make_cmp("<=", "x", 127);
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].1,
            Predicate::Range {
                lo: None,
                hi: Some(127),
                inclusive_hi: true,
            }
        );
    }

    #[test]
    fn gt_produces_lower_bound() {
        // (> x 0) → x >= 1 → Range { lo: Some(1), hi: None, … }
        let guard = make_cmp(">", "x", 0);
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].1,
            Predicate::Range {
                lo: Some(1),
                hi: None,
                inclusive_hi: false,
            }
        );
    }

    #[test]
    fn ge_produces_exact_lower_bound() {
        // (>= x 5) → Range { lo: Some(5), hi: None, … }
        let guard = make_cmp(">=", "x", 5);
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].1,
            Predicate::Range {
                lo: Some(5),
                hi: None,
                inclusive_hi: false,
            }
        );
    }

    #[test]
    fn eq_produces_singleton_range() {
        // (= x 42) → Range { lo: Some(42), hi: Some(42), inclusive_hi: true }
        let guard = make_cmp("=", "x", 42);
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].1,
            Predicate::Range {
                lo: Some(42),
                hi: Some(42),
                inclusive_hi: true,
            }
        );
    }

    #[test]
    fn and_combines_two_facts() {
        // (and (>= x 0) (< x 128)) → x: And([lo>=0, hi<128])
        let guard = make_and(make_cmp(">=", "x", 0), make_cmp("<", "x", 128));
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1, "same var should be merged");
        let (var, pred) = &facts[0];
        assert_eq!(var, "x");
        // The predicate should be an And of the two Range predicates.
        assert!(
            matches!(pred, Predicate::And(_)),
            "expected And predicate, got {pred:?}"
        );
    }

    #[test]
    fn not_negates_comparison() {
        // (not (< x 128)) → x: Not(Range { hi: 128 })
        let guard = make_not(make_cmp("<", "x", 128));
        let facts = extract_narrowing_facts(&guard);
        assert_eq!(facts.len(), 1);
        assert!(
            matches!(&facts[0].1, Predicate::Not(_)),
            "expected Not predicate, got {:?}",
            facts[0].1
        );
    }

    #[test]
    fn bool_literal_guard_produces_no_facts() {
        // #t is not a comparison → empty.
        let guard = Expr::BoolLit(twig_parser::BoolLit {
            value: true,
            line: 1,
            column: 1,
        });
        let facts = extract_narrowing_facts(&guard);
        assert!(facts.is_empty(), "bool literal guard should produce no facts");
    }

    #[test]
    fn int_lit_on_both_sides_produces_no_facts() {
        // (< 1 128) — no VarRef on LHS → no narrowing.
        let guard = Expr::Apply(Apply {
            fn_expr: Box::new(Expr::VarRef(VarRef {
                name: "<".to_owned(),
                line: 1,
                column: 1,
            })),
            args: vec![
                Expr::IntLit(IntLit { value: 1, line: 1, column: 1 }),
                Expr::IntLit(IntLit { value: 128, line: 1, column: 1 }),
            ],
            line: 1,
            column: 1,
        });
        let facts = extract_narrowing_facts(&guard);
        assert!(facts.is_empty());
    }

    // ── merge_kind_with_predicate ─────────────────────────────────────────────

    #[test]
    fn merge_adds_predicate_to_int() {
        let pred = Predicate::Range {
            lo: None,
            hi: Some(128),
            inclusive_hi: false,
        };
        let result = merge_kind_with_predicate(&TwigKind::Int, pred.clone());
        assert_eq!(result, TwigKind::RefinedInt(pred));
    }

    #[test]
    fn merge_intersects_existing_refined_int() {
        let existing = Predicate::Range {
            lo: Some(0),
            hi: None,
            inclusive_hi: false,
        };
        let new_pred = Predicate::Range {
            lo: None,
            hi: Some(128),
            inclusive_hi: false,
        };
        let base = TwigKind::RefinedInt(existing.clone());
        let result = merge_kind_with_predicate(&base, new_pred.clone());
        assert!(
            matches!(result, TwigKind::RefinedInt(Predicate::And(_))),
            "expected RefinedInt(And(…)), got {result:?}"
        );
    }

    #[test]
    fn merge_leaves_bool_unchanged() {
        let pred = Predicate::Range {
            lo: None,
            hi: Some(1),
            inclusive_hi: false,
        };
        let result = merge_kind_with_predicate(&TwigKind::Bool, pred);
        assert_eq!(result, TwigKind::Bool, "Bool kind should not be narrowed");
    }

    #[test]
    fn merge_leaves_any_unchanged() {
        let pred = Predicate::Range {
            lo: None,
            hi: Some(10),
            inclusive_hi: false,
        };
        let result = merge_kind_with_predicate(&TwigKind::Any, pred);
        assert_eq!(result, TwigKind::Any, "Any kind should not be narrowed");
    }
}
