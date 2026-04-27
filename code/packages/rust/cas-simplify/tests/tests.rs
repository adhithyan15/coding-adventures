// Integration tests for cas-simplify.
//
// The test suite mirrors the Python reference tests in
// code/packages/python/cas-simplify/tests/ to ensure the Rust port is
// behaviourally equivalent.

use cas_simplify::{canonical, numeric_fold, simplify};
use symbolic_ir::{
    apply, flt, int, rat, sym, IRNode, ADD, COS, DIV, EXP, LOG, MUL, POW, SIN, SUB,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn x() -> IRNode {
    sym("x")
}
fn y() -> IRNode {
    sym("y")
}
fn z() -> IRNode {
    sym("z")
}
fn a() -> IRNode {
    sym("a")
}
fn b() -> IRNode {
    sym("b")
}
fn c() -> IRNode {
    sym("c")
}
fn zero() -> IRNode {
    int(0)
}
fn one() -> IRNode {
    int(1)
}

// ---------------------------------------------------------------------------
// canonical — structural normalization
// ---------------------------------------------------------------------------

#[test]
fn canonical_flatten_add() {
    // Add(a, Add(b, c)) → Add(a, b, c)  (after sort: Add(a, b, c))
    let inner = apply(sym(ADD), vec![b(), c()]);
    let expr = apply(sym(ADD), vec![a(), inner]);
    let out = canonical(expr);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.head, sym(ADD));
            assert_eq!(ap.args.len(), 3);
            // Sorted alphabetically: a < b < c
            assert_eq!(ap.args, vec![a(), b(), c()]);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn canonical_flatten_mul() {
    // Mul(Mul(a, b), c) → Mul(a, b, c)
    let inner = apply(sym(MUL), vec![a(), b()]);
    let expr = apply(sym(MUL), vec![inner, c()]);
    let out = canonical(expr);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.head, sym(MUL));
            assert_eq!(ap.args.len(), 3);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn canonical_sort_args_alphabetical() {
    // Add(c, a, b) → Add(a, b, c)
    let expr = apply(sym(ADD), vec![c(), a(), b()]);
    let out = canonical(expr);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.args, vec![a(), b(), c()]);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn canonical_integer_sorts_before_symbol() {
    // Add(x, 2) → Add(2, x)  (Integer rank < Symbol rank)
    let expr = apply(sym(ADD), vec![x(), int(2)]);
    let out = canonical(expr);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.args[0], int(2));
            assert_eq!(ap.args[1], x());
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn canonical_singleton_add_drops() {
    // Add(x) → x
    assert_eq!(canonical(apply(sym(ADD), vec![x()])), x());
}

#[test]
fn canonical_singleton_mul_drops() {
    // Mul(x) → x
    assert_eq!(canonical(apply(sym(MUL), vec![x()])), x());
}

#[test]
fn canonical_empty_add_is_zero() {
    assert_eq!(canonical(apply(sym(ADD), vec![])), zero());
}

#[test]
fn canonical_empty_mul_is_one() {
    assert_eq!(canonical(apply(sym(MUL), vec![])), one());
}

#[test]
fn canonical_is_idempotent() {
    let expr = apply(sym(ADD), vec![c(), a(), b()]);
    let once = canonical(expr);
    let twice = canonical(once.clone());
    assert_eq!(once, twice);
}

#[test]
fn canonical_non_commutative_head_unchanged() {
    // Sub(b, a) — Sub is not commutative, args must NOT be reordered.
    let expr = apply(sym(SUB), vec![b(), a()]);
    assert_eq!(canonical(expr.clone()), expr);
}

#[test]
fn canonical_deep_nested_flatten() {
    // Add(a, Add(b, Add(c, x))) → Add(a, b, c, x) (all flattened at once)
    let inner = apply(sym(ADD), vec![c(), x()]);
    let middle = apply(sym(ADD), vec![b(), inner]);
    let outer = apply(sym(ADD), vec![a(), middle]);
    let out = canonical(outer);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.args.len(), 4);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// numeric_fold — constant folding
// ---------------------------------------------------------------------------

#[test]
fn fold_add_two_integers() {
    // Add(2, 3) → 5  (two literals fold to one)
    let expr = apply(sym(ADD), vec![int(2), int(3)]);
    assert_eq!(numeric_fold(expr), int(5));
}

#[test]
fn fold_mul_two_integers() {
    // Mul(3, 4) → 12
    let expr = apply(sym(MUL), vec![int(3), int(4)]);
    assert_eq!(numeric_fold(expr), int(12));
}

#[test]
fn fold_add_leaves_non_numerics() {
    // Add(2, 3, x) → Add(5, x)
    let expr = apply(sym(ADD), vec![int(2), int(3), x()]);
    let out = numeric_fold(expr);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.head, sym(ADD));
            assert_eq!(ap.args, vec![int(5), x()]);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn fold_mul_leaves_non_numerics() {
    // Mul(2, 3, x) → Mul(6, x)
    let expr = apply(sym(MUL), vec![int(2), int(3), x()]);
    let out = numeric_fold(expr);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.args, vec![int(6), x()]);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn fold_add_identity_dropped() {
    // Add(0, x) → x  (0 is identity for Add; dropped when other args remain)
    let expr = apply(sym(ADD), vec![zero(), x()]);
    assert_eq!(numeric_fold(expr), x());
}

#[test]
fn fold_mul_identity_dropped() {
    // Mul(1, x) → x
    let expr = apply(sym(MUL), vec![one(), x()]);
    assert_eq!(numeric_fold(expr), x());
}

#[test]
fn fold_float_contaminates() {
    // Add(1, 2.5) → 3.5  (float contamination)
    let expr = apply(sym(ADD), vec![int(1), flt(2.5)]);
    assert_eq!(numeric_fold(expr), flt(3.5));
}

#[test]
fn fold_rational_arithmetic() {
    // Add(1/2, 1/3) → 5/6
    let expr = apply(sym(ADD), vec![rat(1, 2), rat(1, 3)]);
    assert_eq!(numeric_fold(expr), rat(5, 6));
}

#[test]
fn fold_rational_mul() {
    // Mul(2/3, 3/4) → 1/2
    let expr = apply(sym(MUL), vec![rat(2, 3), rat(3, 4)]);
    assert_eq!(numeric_fold(expr), rat(1, 2));
}

#[test]
fn fold_no_literals_unchanged() {
    // Mul(x, y) — no numeric literals, return unchanged
    let expr = apply(sym(MUL), vec![x(), y()]);
    let out = numeric_fold(expr.clone());
    // After fold, should be same structure (head + args)
    match (&expr, &out) {
        (IRNode::Apply(e), IRNode::Apply(o)) => {
            assert_eq!(e.args, o.args);
        }
        _ => panic!("expected Apply"),
    }
}

#[test]
fn fold_recursive_inner_mul() {
    // Add(2, Mul(3, 4)) — outer Add sees Add(2, 12) after inner Mul folds
    let inner = apply(sym(MUL), vec![int(3), int(4)]);
    let expr = apply(sym(ADD), vec![int(2), inner]);
    let out = numeric_fold(expr);
    // numeric_fold is bottom-up: inner Mul(3,4) → 12, then Add(2,12) → 14
    assert_eq!(out, int(14));
}

// ---------------------------------------------------------------------------
// simplify — full pipeline
// ---------------------------------------------------------------------------

#[test]
fn simplify_add_zero() {
    // Add(x, 0) → x
    assert_eq!(simplify(apply(sym(ADD), vec![x(), zero()]), 50), x());
}

#[test]
fn simplify_mul_one() {
    // Mul(x, 1) → x
    assert_eq!(simplify(apply(sym(MUL), vec![x(), one()]), 50), x());
}

#[test]
fn simplify_mul_zero() {
    // Mul(x, 0) → 0
    assert_eq!(simplify(apply(sym(MUL), vec![x(), zero()]), 50), zero());
}

#[test]
fn simplify_pow_zero() {
    // Pow(x, 0) → 1
    assert_eq!(simplify(apply(sym(POW), vec![x(), zero()]), 50), one());
}

#[test]
fn simplify_pow_one() {
    // Pow(x, 1) → x
    assert_eq!(simplify(apply(sym(POW), vec![x(), one()]), 50), x());
}

#[test]
fn simplify_one_to_anything() {
    // Pow(1, x) → 1
    assert_eq!(simplify(apply(sym(POW), vec![one(), x()]), 50), one());
}

#[test]
fn simplify_sub_self() {
    // Sub(x, x) → 0
    assert_eq!(simplify(apply(sym(SUB), vec![x(), x()]), 50), zero());
}

#[test]
fn simplify_div_self() {
    // Div(x, x) → 1
    assert_eq!(simplify(apply(sym(DIV), vec![x(), x()]), 50), one());
}

#[test]
fn simplify_log_exp() {
    // Log(Exp(x)) → x
    let inner = apply(sym(EXP), vec![x()]);
    assert_eq!(
        simplify(apply(sym(LOG), vec![inner]), 50),
        x()
    );
}

#[test]
fn simplify_exp_log() {
    // Exp(Log(x)) → x
    let inner = apply(sym(LOG), vec![x()]);
    assert_eq!(
        simplify(apply(sym(EXP), vec![inner]), 50),
        x()
    );
}

#[test]
fn simplify_sin_zero() {
    // Sin(0) → 0
    assert_eq!(simplify(apply(sym(SIN), vec![zero()]), 50), zero());
}

#[test]
fn simplify_cos_zero() {
    // Cos(0) → 1
    assert_eq!(simplify(apply(sym(COS), vec![zero()]), 50), one());
}

#[test]
fn simplify_canonical_then_identity() {
    // Mul(Add(x, 0), 1) → x  (requires canonical to sort, then identity rules)
    let inner = apply(sym(ADD), vec![x(), zero()]);
    let expr = apply(sym(MUL), vec![inner, one()]);
    assert_eq!(simplify(expr, 50), x());
}

#[test]
fn simplify_numeric_fold_collapses_literals() {
    // Add(1, 2, 3, x) → Add(6, x)
    let expr = apply(sym(ADD), vec![int(1), int(2), int(3), x()]);
    let out = simplify(expr, 50);
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.head, sym(ADD));
            assert_eq!(ap.args, vec![int(6), x()]);
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn simplify_double_zero_add_collapses() {
    // ((z + 0) + 0) → z
    let inner = apply(sym(ADD), vec![z(), zero()]);
    let expr = apply(sym(ADD), vec![inner, zero()]);
    assert_eq!(simplify(expr, 50), z());
}

#[test]
fn simplify_already_simple_unchanged() {
    // Add(x, y) — no simplification possible; result is structurally equivalent
    let expr = apply(sym(ADD), vec![x(), y()]);
    let out = simplify(expr, 50);
    // Canonical may reorder args, so accept either ordering.
    match out {
        IRNode::Apply(ref ap) => {
            assert_eq!(ap.head, sym(ADD));
            assert!(ap.args.len() == 2);
            let args_set: std::collections::HashSet<_> = ap.args.iter().collect();
            assert!(args_set.contains(&x()));
            assert!(args_set.contains(&y()));
        }
        other => panic!("expected Apply, got {other:?}"),
    }
}

#[test]
fn simplify_is_idempotent() {
    // simplify(simplify(expr)) == simplify(expr)
    let expr = apply(sym(MUL), vec![apply(sym(ADD), vec![x(), zero()]), one()]);
    let once = simplify(expr, 50);
    let twice = simplify(once.clone(), 50);
    assert_eq!(once, twice);
}

#[test]
fn simplify_descends_into_subexpressions() {
    // Mul(2, Add(z, 0)) → Mul(2, z)
    let inner = apply(sym(ADD), vec![z(), zero()]);
    let expr = apply(sym(MUL), vec![int(2), inner]);
    let out = simplify(expr, 50);
    // Canonical sorts Mul args: Integer rank < Symbol rank → Mul(2, z)
    assert_eq!(out, apply(sym(MUL), vec![int(2), z()]));
}

#[test]
fn simplify_numeric_fold_add_all_literals() {
    // Add(1, 2, 3) → 6  (all literals fold, singleton drop fires)
    let expr = apply(sym(ADD), vec![int(1), int(2), int(3)]);
    assert_eq!(simplify(expr, 50), int(6));
}

#[test]
fn simplify_numeric_fold_mul_all_literals() {
    // Mul(2, 3) → 6
    let expr = apply(sym(MUL), vec![int(2), int(3)]);
    assert_eq!(simplify(expr, 50), int(6));
}

#[test]
fn simplify_atom_unchanged() {
    // Symbols and literals pass through as-is.
    assert_eq!(simplify(x(), 50), x());
    assert_eq!(simplify(int(42), 50), int(42));
    assert_eq!(simplify(flt(3.14), 50), flt(3.14));
}
