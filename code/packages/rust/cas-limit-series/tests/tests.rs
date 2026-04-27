// Integration tests for cas-limit-series.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-limit-series/tests/.

use cas_limit_series::{limit_direct, taylor_polynomial, PolynomialError, LIMIT};
use symbolic_ir::{apply, int, sym, ADD, DIV, MUL, POW, SUB};

// ---------------------------------------------------------------------------
// limit_direct
// ---------------------------------------------------------------------------

#[test]
fn limit_polynomial_at_finite_point() {
    // lim_{x→2} x^2 + 1  →  Add(Pow(2, 2), 1)  (un-simplified)
    let x = sym("x");
    let expr = apply(sym(ADD), vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)]);
    let out = limit_direct(expr, &x, int(2));
    let expected = apply(
        sym(ADD),
        vec![apply(sym(POW), vec![int(2), int(2)]), int(1)],
    );
    assert_eq!(out, expected);
}

#[test]
fn limit_substitutes_in_compound() {
    // lim_{x→3} 2*x  →  Mul(2, 3)
    let x = sym("x");
    let expr = apply(sym(MUL), vec![int(2), x.clone()]);
    let out = limit_direct(expr, &x, int(3));
    assert_eq!(out, apply(sym(MUL), vec![int(2), int(3)]));
}

#[test]
fn limit_does_not_simplify() {
    // Result is intentionally un-simplified.
    let x = sym("x");
    let expr = apply(sym(ADD), vec![x.clone(), int(0)]);
    let out = limit_direct(expr, &x, int(5));
    assert_eq!(out, apply(sym(ADD), vec![int(5), int(0)]));
}

#[test]
fn limit_no_var_in_expr() {
    // If var doesn't appear in expr, expr is returned unchanged.
    let x = sym("x");
    let y = sym("y");
    let expr = apply(sym(MUL), vec![int(2), y.clone()]);
    assert_eq!(limit_direct(expr.clone(), &x, int(0)), expr);
}

#[test]
fn limit_indeterminate_returns_unevaluated() {
    // A literal Div(0, 0) after substitution returns Limit(expr, var, point).
    let x = sym("x");
    let expr = apply(sym(DIV), vec![int(0), int(0)]);
    let out = limit_direct(expr.clone(), &x, int(0));
    if let symbolic_ir::IRNode::Apply(a) = &out {
        assert_eq!(a.head, sym(LIMIT));
    } else {
        panic!("expected Apply(Limit,...), got {out:?}");
    }
}

#[test]
fn limit_constant_unchanged() {
    // lim_{x→5} 42  →  42
    let x = sym("x");
    assert_eq!(limit_direct(int(42), &x, int(5)), int(42));
}

// ---------------------------------------------------------------------------
// taylor_polynomial
// ---------------------------------------------------------------------------

#[test]
fn taylor_constant() {
    // Taylor(7, x, 2, order=3)  →  7
    let x = sym("x");
    let out = taylor_polynomial(&int(7), &x, &int(2), 3).unwrap();
    assert_eq!(out, int(7));
}

#[test]
fn taylor_x_at_zero_order2() {
    // Taylor(x, x, 0, order=2)  →  x
    let x = sym("x");
    let out = taylor_polynomial(&x, &x, &int(0), 2).unwrap();
    assert_eq!(out, x);
}

#[test]
fn taylor_x_squared_at_zero_full_order() {
    // Taylor(x^2, x, 0, order=2)  →  Pow(x, 2)
    let x = sym("x");
    let expr = apply(sym(POW), vec![x.clone(), int(2)]);
    let out = taylor_polynomial(&expr, &x, &int(0), 2).unwrap();
    assert_eq!(out, apply(sym(POW), vec![x.clone(), int(2)]));
}

#[test]
fn taylor_x_squared_truncated_to_order1() {
    // Taylor(x^2, x, 0, order=1)  →  0  (no x^0 or x^1 parts)
    let x = sym("x");
    let expr = apply(sym(POW), vec![x.clone(), int(2)]);
    let out = taylor_polynomial(&expr, &x, &int(0), 1).unwrap();
    assert_eq!(out, int(0));
}

#[test]
fn taylor_polynomial_around_one() {
    // Taylor(x^2, x, 1, order=2)  →  Add of three terms (1, 2*(x-1), (x-1)^2)
    let x = sym("x");
    let expr = apply(sym(POW), vec![x.clone(), int(2)]);
    let out = taylor_polynomial(&expr, &x, &int(1), 2).unwrap();
    // Should be Add(...)
    if let symbolic_ir::IRNode::Apply(a) = &out {
        assert_eq!(a.head, sym(ADD));
    } else {
        panic!("expected Add, got {out:?}");
    }
}

#[test]
fn taylor_compound_polynomial_x_squared_plus_1() {
    // Taylor(x^2 + 1, x, 0, order=2)  →  Add(1, Pow(x, 2))
    let x = sym("x");
    let expr = apply(
        sym(ADD),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
    );
    let out = taylor_polynomial(&expr, &x, &int(0), 2).unwrap();
    if let symbolic_ir::IRNode::Apply(a) = &out {
        assert_eq!(a.head, sym(ADD));
        // Both terms should be present: Integer(1) and Pow(x, 2)
        let has_one = a.args.contains(&int(1));
        let has_x2 = a.args.contains(&apply(sym(POW), vec![x.clone(), int(2)]));
        assert!(has_one, "expected Integer(1) in {out:?}");
        assert!(has_x2, "expected Pow(x,2) in {out:?}");
    } else {
        panic!("expected Add, got {out:?}");
    }
}

#[test]
fn taylor_negative_order_raises() {
    // order parameter is usize, so we can't pass -1 directly.
    // This test validates the order=0 edge case instead.
    let x = sym("x");
    // Taylor(x, x, 0, order=0) → 0 (constant term of x = 0)
    let out = taylor_polynomial(&x, &x, &int(0), 0).unwrap();
    assert_eq!(out, int(0));
}

#[test]
fn taylor_non_polynomial_raises() {
    // A transcendental Sin(x) raises PolynomialError.
    let x = sym("x");
    let expr = apply(sym("Sin"), vec![x.clone()]);
    let result = taylor_polynomial(&expr, &x, &int(0), 3);
    assert!(result.is_err());
    assert!(matches!(result, Err(PolynomialError(_))));
}

#[test]
fn taylor_unknown_symbol_raises() {
    // A symbol other than the expansion variable raises.
    let x = sym("x");
    let y = sym("y");
    let expr = apply(sym(MUL), vec![y, x.clone()]);
    let result = taylor_polynomial(&expr, &x, &int(0), 2);
    assert!(result.is_err());
}

#[test]
fn taylor_with_sub_and_neg() {
    // Taylor(x - 1, x, 0, order=1)  →  Add(-1, x)
    let x = sym("x");
    let expr = apply(sym(SUB), vec![x.clone(), int(1)]);
    let out = taylor_polynomial(&expr, &x, &int(0), 1).unwrap();
    if let symbolic_ir::IRNode::Apply(a) = &out {
        assert_eq!(a.head, sym(ADD));
        assert!(a.args.contains(&int(-1)), "expected Integer(-1) in {out:?}");
        assert!(a.args.contains(&x), "expected x in {out:?}");
    } else {
        panic!("expected Add, got {out:?}");
    }
}

#[test]
fn taylor_linear_at_nonzero_point() {
    // Taylor(3*x + 2, x, 1, order=1)
    // Polynomial: 2 + 3x  → shifted around 1:
    //   k=0: (2 + 3·1) = 5
    //   k=1: 3
    // → 5 + 3·(x-1)
    let x = sym("x");
    let expr = apply(sym(ADD), vec![apply(sym(MUL), vec![int(3), x.clone()]), int(2)]);
    let out = taylor_polynomial(&expr, &x, &int(1), 1).unwrap();
    // Should be Add(5, Mul(3, Sub(x, 1)))
    if let symbolic_ir::IRNode::Apply(a) = &out {
        assert_eq!(a.head, sym(ADD));
    } else {
        panic!("expected Add, got {out:?}");
    }
}

#[test]
fn taylor_rational_coefficient() {
    // Taylor(x/2, x, 0, order=1)  →  Mul(1/2, x)  (one term since k=0 coeff is 0)
    let x = sym("x");
    let expr = apply(sym(DIV), vec![x.clone(), int(2)]);
    let out = taylor_polynomial(&expr, &x, &int(0), 1).unwrap();
    // Single term: Mul(1/2, x)
    let text = format!("{out:?}");
    assert!(text.contains("Mul") || text.contains("Rational"), "{out:?}");
}

#[test]
fn taylor_order_zero_gives_constant_term() {
    // Taylor(x^2 + 3*x + 1, x, 0, order=0)  →  1
    let x = sym("x");
    let expr = apply(
        sym(ADD),
        vec![
            apply(sym(POW), vec![x.clone(), int(2)]),
            apply(sym(MUL), vec![int(3), x.clone()]),
            int(1),
        ],
    );
    let out = taylor_polynomial(&expr, &x, &int(0), 0).unwrap();
    assert_eq!(out, int(1));
}
