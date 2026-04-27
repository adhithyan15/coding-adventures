// Integration tests for cas-substitution.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-substitution/tests/.

use cas_pattern_matching::{blank, named, rule};
use cas_substitution::{replace_all, replace_all_many, subst, subst_many};
use symbolic_ir::{apply, int, sym, ADD, MUL, POW, SIN};

// ---------------------------------------------------------------------------
// subst — structural substitution
// ---------------------------------------------------------------------------

#[test]
fn subst_at_root() {
    // subst(2, x, x) → 2
    assert_eq!(subst(int(2), &sym("x"), sym("x")), int(2));
}

#[test]
fn subst_no_occurrence() {
    // subst(2, x, y) → y
    assert_eq!(subst(int(2), &sym("x"), sym("y")), sym("y"));
}

#[test]
fn subst_in_compound() {
    // subst(2, x, x^2) → Pow(2, 2)
    let expr = apply(sym(POW), vec![sym("x"), int(2)]);
    let expected = apply(sym(POW), vec![int(2), int(2)]);
    assert_eq!(subst(int(2), &sym("x"), expr), expected);
}

#[test]
fn subst_multiple_occurrences() {
    // subst(2, x, x*x) → Mul(2, 2)
    let expr = apply(sym(MUL), vec![sym("x"), sym("x")]);
    let expected = apply(sym(MUL), vec![int(2), int(2)]);
    assert_eq!(subst(int(2), &sym("x"), expr), expected);
}

#[test]
fn subst_with_compound_value() {
    // subst(a+b, x, x*x) → Mul(Add(a,b), Add(a,b))
    let value = apply(sym(ADD), vec![sym("a"), sym("b")]);
    let expr = apply(sym(MUL), vec![sym("x"), sym("x")]);
    let expected = apply(sym(MUL), vec![value.clone(), value.clone()]);
    assert_eq!(subst(value, &sym("x"), expr), expected);
}

#[test]
fn subst_nested_expression() {
    // subst(2, x, sin(x^2 + 1)) → sin(Pow(2,2) + 1)
    let inner = apply(sym(ADD), vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]);
    let expr = apply(sym(SIN), vec![inner]);
    let expected_inner = apply(
        sym(ADD),
        vec![apply(sym(POW), vec![int(2), int(2)]), int(1)],
    );
    let expected = apply(sym(SIN), vec![expected_inner]);
    assert_eq!(subst(int(2), &sym("x"), expr), expected);
}

#[test]
fn subst_searches_for_compound_target() {
    // subst(z, x+1, (x+1)*(x+1)) → z*z
    let target = apply(sym(ADD), vec![sym("x"), int(1)]);
    let expr = apply(sym(MUL), vec![target.clone(), target.clone()]);
    let expected = apply(sym(MUL), vec![sym("z"), sym("z")]);
    assert_eq!(subst(sym("z"), &target, expr), expected);
}

#[test]
fn subst_integer_target() {
    // subst(99, 0, Add(0, x)) → Add(99, x)
    let expr = apply(sym(ADD), vec![int(0), sym("x")]);
    let expected = apply(sym(ADD), vec![int(99), sym("x")]);
    assert_eq!(subst(int(99), &int(0), expr), expected);
}

#[test]
fn subst_leaf_unchanged_when_no_match() {
    // Symbol leaves not equal to var pass through.
    assert_eq!(subst(int(2), &sym("x"), int(42)), int(42));
}

#[test]
fn subst_many_sequential() {
    // subst_many([(x, 2), (y, 3)], x + y) → Add(2, 3)
    let expr = apply(sym(ADD), vec![sym("x"), sym("y")]);
    let rules = vec![(sym("x"), int(2)), (sym("y"), int(3))];
    let expected = apply(sym(ADD), vec![int(2), int(3)]);
    assert_eq!(subst_many(&rules, expr), expected);
}

#[test]
fn subst_many_order_matters() {
    // x → y, then y → z  ⟹  x → z
    let rules = vec![(sym("x"), sym("y")), (sym("y"), sym("z"))];
    assert_eq!(subst_many(&rules, sym("x")), sym("z"));
}

#[test]
fn subst_many_empty_rules() {
    // Empty rule list → unchanged
    let expr = apply(sym(ADD), vec![sym("a"), sym("b")]);
    assert_eq!(subst_many(&[], expr.clone()), expr);
}

// ---------------------------------------------------------------------------
// replace_all — pattern-aware substitution
// ---------------------------------------------------------------------------

#[test]
fn replace_all_simple_rule() {
    // Pow(a_, 2) → Mul(a_, a_)
    let r = rule(
        apply(sym(POW), vec![named("a", blank()), int(2)]),
        apply(sym(MUL), vec![named("a", blank()), named("a", blank())]),
    );
    let expr = apply(sym(POW), vec![sym("y"), int(2)]);
    let expected = apply(sym(MUL), vec![sym("y"), sym("y")]);
    assert_eq!(replace_all(expr, &r), expected);
}

#[test]
fn replace_all_fires_in_subtree() {
    // Rule fires deep inside an expression.
    // Mul(2, Add(z, 0)) with rule Add(x_, 0) → x_  ⟹  Mul(2, z)
    let r = rule(
        apply(sym(ADD), vec![named("x", blank()), int(0)]),
        named("x", blank()),
    );
    let inner = apply(sym(ADD), vec![sym("z"), int(0)]);
    let expr = apply(sym(MUL), vec![int(2), inner]);
    let expected = apply(sym(MUL), vec![int(2), sym("z")]);
    assert_eq!(replace_all(expr, &r), expected);
}

#[test]
fn replace_all_no_match_unchanged() {
    // If the rule never fires, expr is returned unchanged.
    let r = rule(
        apply(sym(POW), vec![named("a", blank()), int(0)]),
        int(1),
    );
    let expr = apply(sym(ADD), vec![sym("x"), sym("y")]);
    assert_eq!(replace_all(expr.clone(), &r), expr);
}

#[test]
fn replace_all_does_not_recurse_into_replacement() {
    // Single-pass: once the rule fires, the replacement is not re-searched.
    // Rule: x_ → Add(x_, 0).  If we recursed we'd loop; instead one rewrite.
    let r = rule(
        named("x", blank()),
        apply(sym(ADD), vec![named("x", blank()), int(0)]),
    );
    let out = replace_all(sym("z"), &r);
    assert_eq!(out, apply(sym(ADD), vec![sym("z"), int(0)]));
}

#[test]
fn replace_all_many_two_rules() {
    // Rule 1: Add(x_, 0) → x_
    // Rule 2: Mul(x_, 1) → x_
    // Mul(Add(z, 0), 1) → z
    let r1 = rule(
        apply(sym(ADD), vec![named("x", blank()), int(0)]),
        named("x", blank()),
    );
    let r2 = rule(
        apply(sym(MUL), vec![named("x", blank()), int(1)]),
        named("x", blank()),
    );
    let inner = apply(sym(ADD), vec![sym("z"), int(0)]);
    let expr = apply(sym(MUL), vec![inner, int(1)]);
    assert_eq!(replace_all_many(expr, &[r1, r2]), sym("z"));
}

#[test]
fn replace_all_many_empty_rules() {
    let expr = apply(sym(ADD), vec![sym("a"), sym("b")]);
    assert_eq!(replace_all_many(expr.clone(), &[]), expr);
}

#[test]
fn replace_all_at_root() {
    // Rule fires at root — entire expression is replaced.
    let r = rule(named("x", blank()), int(42));
    assert_eq!(replace_all(sym("anything"), &r), int(42));
}

#[test]
fn replace_all_multiple_matches_at_same_level() {
    // Add(Pow(a,2), Pow(b,2)) → Add(Mul(a,a), Mul(b,b))
    let r = rule(
        apply(sym(POW), vec![named("a", blank()), int(2)]),
        apply(sym(MUL), vec![named("a", blank()), named("a", blank())]),
    );
    let expr = apply(
        sym(ADD),
        vec![
            apply(sym(POW), vec![sym("a"), int(2)]),
            apply(sym(POW), vec![sym("b"), int(2)]),
        ],
    );
    let expected = apply(
        sym(ADD),
        vec![
            apply(sym(MUL), vec![sym("a"), sym("a")]),
            apply(sym(MUL), vec![sym("b"), sym("b")]),
        ],
    );
    assert_eq!(replace_all(expr, &r), expected);
}
