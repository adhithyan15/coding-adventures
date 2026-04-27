//! Integration tests for cas-pattern-matching.

use cas_pattern_matching::{
    apply_rule, blank, blank_typed, match_pattern, named, rewrite, rule, rule_delayed, Bindings,
    RewriteCycleError,
};
use symbolic_ir::{apply, flt, int, rat, sym, ADD, MUL, POW};

// ---------------------------------------------------------------------------
// Blank — unconstrained wildcard
// ---------------------------------------------------------------------------

#[test]
fn blank_matches_integer() {
    assert!(match_pattern(&blank(), &int(42), Bindings::empty()).is_some());
}

#[test]
fn blank_matches_symbol() {
    assert!(match_pattern(&blank(), &sym("x"), Bindings::empty()).is_some());
}

#[test]
fn blank_matches_apply() {
    let expr = apply(sym(ADD), vec![int(1), int(2)]);
    assert!(match_pattern(&blank(), &expr, Bindings::empty()).is_some());
}

#[test]
fn blank_matches_rational() {
    assert!(match_pattern(&blank(), &rat(1, 2), Bindings::empty()).is_some());
}

#[test]
fn blank_matches_float() {
    assert!(match_pattern(&blank(), &flt(3.14), Bindings::empty()).is_some());
}

// ---------------------------------------------------------------------------
// Blank("T") — type-constrained wildcard
// ---------------------------------------------------------------------------

#[test]
fn blank_typed_integer_matches_integer() {
    assert!(match_pattern(&blank_typed("Integer"), &int(7), Bindings::empty()).is_some());
}

#[test]
fn blank_typed_integer_rejects_symbol() {
    assert!(match_pattern(&blank_typed("Integer"), &sym("x"), Bindings::empty()).is_none());
}

#[test]
fn blank_typed_symbol_matches_symbol() {
    assert!(match_pattern(&blank_typed("Symbol"), &sym("Pi"), Bindings::empty()).is_some());
}

#[test]
fn blank_typed_symbol_rejects_integer() {
    assert!(match_pattern(&blank_typed("Symbol"), &int(1), Bindings::empty()).is_none());
}

#[test]
fn blank_typed_rational_matches_rational() {
    assert!(match_pattern(&blank_typed("Rational"), &rat(1, 3), Bindings::empty()).is_some());
}

#[test]
fn blank_typed_float_matches_float() {
    assert!(match_pattern(&blank_typed("Float"), &flt(2.5), Bindings::empty()).is_some());
}

#[test]
fn blank_typed_head_name_matches_apply() {
    let expr = apply(sym(ADD), vec![int(1), int(2)]);
    assert!(match_pattern(&blank_typed("Add"), &expr, Bindings::empty()).is_some());
}

#[test]
fn blank_typed_head_name_rejects_different_head() {
    let expr = apply(sym(MUL), vec![int(2), int(3)]);
    assert!(match_pattern(&blank_typed("Add"), &expr, Bindings::empty()).is_none());
}

// ---------------------------------------------------------------------------
// Pattern(name, inner) — named capture
// ---------------------------------------------------------------------------

#[test]
fn named_captures_value() {
    let p = named("x", blank());
    let b = match_pattern(&p, &int(5), Bindings::empty()).unwrap();
    assert_eq!(b.get("x"), Some(&int(5)));
}

#[test]
fn named_multiple_captures() {
    let pat = apply(sym(ADD), vec![named("a", blank()), named("b", blank())]);
    let target = apply(sym(ADD), vec![int(2), int(3)]);
    let b = match_pattern(&pat, &target, Bindings::empty()).unwrap();
    assert_eq!(b.get("a"), Some(&int(2)));
    assert_eq!(b.get("b"), Some(&int(3)));
}

#[test]
fn named_consistency_same_value() {
    // x_ == x_  (same name, same value) — should succeed
    let x = named("x", blank());
    let pat = apply(sym(ADD), vec![x.clone(), x.clone()]);
    let target = apply(sym(ADD), vec![sym("a"), sym("a")]);
    assert!(match_pattern(&pat, &target, Bindings::empty()).is_some());
}

#[test]
fn named_consistency_different_value_fails() {
    // x_ + x_  where lhs != rhs  — must fail
    let x = named("x", blank());
    let pat = apply(sym(ADD), vec![x.clone(), x.clone()]);
    let target = apply(sym(ADD), vec![sym("a"), sym("b")]);
    assert!(match_pattern(&pat, &target, Bindings::empty()).is_none());
}

#[test]
fn named_with_type_constraint() {
    let p = named("n", blank_typed("Integer"));
    assert!(match_pattern(&p, &int(10), Bindings::empty()).is_some());
    assert!(match_pattern(&p, &sym("x"), Bindings::empty()).is_none());
}

// ---------------------------------------------------------------------------
// Structural (literal) matching
// ---------------------------------------------------------------------------

#[test]
fn integer_matches_same_integer() {
    assert!(match_pattern(&int(7), &int(7), Bindings::empty()).is_some());
}

#[test]
fn integer_rejects_different_integer() {
    assert!(match_pattern(&int(7), &int(8), Bindings::empty()).is_none());
}

#[test]
fn symbol_matches_same_symbol() {
    assert!(match_pattern(&sym("Pi"), &sym("Pi"), Bindings::empty()).is_some());
}

#[test]
fn apply_matches_structurally() {
    let pat = apply(sym(POW), vec![sym("x"), int(2)]);
    let tgt = apply(sym(POW), vec![sym("x"), int(2)]);
    assert!(match_pattern(&pat, &tgt, Bindings::empty()).is_some());
}

#[test]
fn apply_rejects_different_head() {
    let pat = apply(sym(ADD), vec![int(1), int(2)]);
    let tgt = apply(sym(MUL), vec![int(1), int(2)]);
    assert!(match_pattern(&pat, &tgt, Bindings::empty()).is_none());
}

#[test]
fn apply_rejects_different_arg_count() {
    let pat = apply(sym(ADD), vec![int(1), int(2)]);
    let tgt = apply(sym(ADD), vec![int(1), int(2), int(3)]);
    assert!(match_pattern(&pat, &tgt, Bindings::empty()).is_none());
}

// ---------------------------------------------------------------------------
// apply_rule — single-rule application
// ---------------------------------------------------------------------------

#[test]
fn rule_fires_and_substitutes() {
    // Pow(x_, 0) → 1
    let r = rule(
        apply(sym(POW), vec![named("x", blank()), int(0)]),
        int(1),
    );
    let target = apply(sym(POW), vec![sym("z"), int(0)]);
    assert_eq!(apply_rule(&r, &target), Some(int(1)));
}

#[test]
fn rule_returns_none_on_no_match() {
    let r = rule(
        apply(sym(POW), vec![named("x", blank()), int(0)]),
        int(1),
    );
    assert_eq!(apply_rule(&r, &sym("y")), None);
}

#[test]
fn rule_substitutes_captured_pattern_in_rhs() {
    // Add(x_, x_) → Mul(2, x_)
    let x = named("x", blank());
    let r = rule(
        apply(sym(ADD), vec![x.clone(), x.clone()]),
        apply(sym(MUL), vec![int(2), x.clone()]),
    );
    let target = apply(sym(ADD), vec![sym("a"), sym("a")]);
    let expected = apply(sym(MUL), vec![int(2), sym("a")]);
    assert_eq!(apply_rule(&r, &target), Some(expected));
}

#[test]
fn rule_delayed_behaves_same_as_rule() {
    let x = named("x", blank());
    let r = rule_delayed(
        apply(sym(ADD), vec![x.clone(), int(0)]),
        x.clone(),
    );
    let target = apply(sym(ADD), vec![int(7), int(0)]);
    assert_eq!(apply_rule(&r, &target), Some(int(7)));
}

// ---------------------------------------------------------------------------
// rewrite — bottom-up fixed-point
// ---------------------------------------------------------------------------

#[test]
fn rewrite_single_rule_once() {
    let x = named("x", blank());
    let r = rule(apply(sym(ADD), vec![x.clone(), int(0)]), x.clone());
    let expr = apply(sym(ADD), vec![sym("z"), int(0)]);
    assert_eq!(rewrite(expr, &[r], 100).unwrap(), sym("z"));
}

#[test]
fn rewrite_applies_nested() {
    // Add(Add(z, 0), 0)  →  z
    let x = named("x", blank());
    let r = rule(apply(sym(ADD), vec![x.clone(), int(0)]), x.clone());
    let inner = apply(sym(ADD), vec![sym("z"), int(0)]);
    let outer = apply(sym(ADD), vec![inner, int(0)]);
    assert_eq!(rewrite(outer, &[r], 100).unwrap(), sym("z"));
}

#[test]
fn rewrite_no_rules_leaves_unchanged() {
    let expr = apply(sym(ADD), vec![int(1), int(2)]);
    let result = rewrite(expr.clone(), &[], 100).unwrap();
    assert_eq!(result, expr);
}

#[test]
fn rewrite_multiple_rules_first_match_wins() {
    // Rule 1: x_ + 0 → x_
    // Rule 2: x_ + x_ → Mul(2, x_)  (not relevant here)
    let x = named("x", blank());
    let r1 = rule(apply(sym(ADD), vec![x.clone(), int(0)]), x.clone());
    let r2 = rule(
        apply(sym(ADD), vec![x.clone(), x.clone()]),
        apply(sym(MUL), vec![int(2), x.clone()]),
    );
    let expr = apply(sym(ADD), vec![sym("y"), int(0)]);
    assert_eq!(rewrite(expr, &[r1, r2], 100).unwrap(), sym("y"));
}

#[test]
fn rewrite_cycle_detection() {
    // A rule that keeps rewriting forever:
    // f(x_) → f(x_)  (fires but result equals input so no cycle in our code)
    // To actually trigger a cycle we need a rule that keeps growing.
    // f(x_) → Add(f(x_), 0) with r2: Add(y_, 0) → y_
    // But that converges. Let's just check the counter is bounded:
    let x = named("x", blank());
    // f(x) → f(x) — rewrite sees result == input → terminates
    let r = rule(
        apply(sym("f"), vec![x.clone()]),
        apply(sym("f"), vec![x.clone()]),
    );
    let expr = apply(sym("f"), vec![int(1)]);
    // Should terminate without error (result == input immediately)
    let _ = rewrite(expr, &[r], 100).unwrap();
}

#[test]
fn rewrite_returns_error_on_non_converging() {
    // A rule where applying it always changes the expression:
    // f(x_) → g(x_), g(x_) → f(x_)  — alternates forever
    let x = named("x", blank());
    let r1 = rule(
        apply(sym("f"), vec![x.clone()]),
        apply(sym("g"), vec![x.clone()]),
    );
    let r2 = rule(
        apply(sym("g"), vec![x.clone()]),
        apply(sym("f"), vec![x.clone()]),
    );
    let expr = apply(sym("f"), vec![int(1)]);
    let result = rewrite(expr, &[r1, r2], 10);
    assert!(matches!(result, Err(RewriteCycleError { max_iterations: 10 })));
}

// ---------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------

#[test]
fn bindings_empty() {
    let b = Bindings::empty();
    assert!(b.is_empty());
    assert_eq!(b.len(), 0);
}

#[test]
fn bindings_bind_and_get() {
    let b = Bindings::empty().bind("x", int(5));
    assert_eq!(b.get("x"), Some(&int(5)));
    assert_eq!(b.len(), 1);
}

#[test]
fn bindings_bind_idempotent() {
    let b1 = Bindings::empty().bind("x", int(5));
    let b2 = b1.bind("x", int(5)); // same key, same value
    assert_eq!(b1, b2);
}

#[test]
fn bindings_iter() {
    let b = Bindings::empty().bind("a", int(1)).bind("b", int(2));
    let mut pairs: Vec<(&str, &symbolic_ir::IRNode)> = b.iter().collect();
    pairs.sort_by_key(|(k, _)| *k);
    assert_eq!(pairs[0], ("a", &int(1)));
    assert_eq!(pairs[1], ("b", &int(2)));
}
