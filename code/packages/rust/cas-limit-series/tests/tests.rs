// Integration tests for cas-limit-series.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-limit-series/tests/.

use cas_limit_series::{
    limit_advanced, limit_direct, taylor_polynomial, LimitAdvancedOptions, LimitDirection,
    PolynomialError, LIMIT,
};
use symbolic_ir::{apply, int, sym, IRNode, ADD, DIV, EXP, LOG, MUL, POW, SUB};

// ---------------------------------------------------------------------------
// limit_direct
// ---------------------------------------------------------------------------

#[test]
fn limit_polynomial_at_finite_point() {
    // lim_{x→2} x^2 + 1  →  Add(Pow(2, 2), 1)  (un-simplified)
    let x = sym("x");
    let expr = apply(
        sym(ADD),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
    );
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
// limit_advanced
// ---------------------------------------------------------------------------

fn test_diff(expr: &IRNode, var: &IRNode) -> IRNode {
    if expr == var {
        return int(1);
    }
    match expr {
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_) | IRNode::Symbol(_) => {
            int(0)
        }
        IRNode::Str(_) => int(0),
        IRNode::Apply(a) if a.head == sym(ADD) => apply(
            sym(ADD),
            a.args.iter().map(|arg| test_diff(arg, var)).collect(),
        ),
        IRNode::Apply(a) if a.head == sym(SUB) && a.args.len() == 2 => apply(
            sym(SUB),
            vec![test_diff(&a.args[0], var), test_diff(&a.args[1], var)],
        ),
        IRNode::Apply(a) if a.head == sym(MUL) && a.args.len() == 2 => {
            let f = &a.args[0];
            let g = &a.args[1];
            apply(
                sym(ADD),
                vec![
                    apply(sym(MUL), vec![test_diff(f, var), g.clone()]),
                    apply(sym(MUL), vec![f.clone(), test_diff(g, var)]),
                ],
            )
        }
        IRNode::Apply(a) if a.head == sym(POW) && a.args.len() == 2 => {
            if let IRNode::Integer(n) = a.args[1] {
                if n == 0 {
                    int(0)
                } else {
                    apply(
                        sym(MUL),
                        vec![
                            apply(
                                sym(MUL),
                                vec![int(n), apply(sym(POW), vec![a.args[0].clone(), int(n - 1)])],
                            ),
                            test_diff(&a.args[0], var),
                        ],
                    )
                }
            } else {
                int(0)
            }
        }
        _ => int(0),
    }
}

fn test_eval(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(a) => {
            let args: Vec<_> = a.args.into_iter().map(test_eval).collect();
            if a.head == sym(ADD) {
                if args.iter().all(|arg| matches!(arg, IRNode::Integer(_))) {
                    return int(args.iter().map(as_int).sum());
                }
            }
            if a.head == sym(SUB) && args.len() == 2 {
                if let (IRNode::Integer(lhs), IRNode::Integer(rhs)) = (&args[0], &args[1]) {
                    return int(lhs - rhs);
                }
            }
            if a.head == sym(MUL) {
                if args.iter().any(|arg| *arg == int(0)) {
                    return int(0);
                }
                let non_one: Vec<_> = args.into_iter().filter(|arg| *arg != int(1)).collect();
                if non_one.is_empty() {
                    return int(1);
                }
                if non_one.len() == 1 {
                    return non_one[0].clone();
                }
                if non_one.iter().all(|arg| matches!(arg, IRNode::Integer(_))) {
                    return int(non_one.iter().map(as_int).product());
                }
                return apply(sym(MUL), non_one);
            }
            if a.head == sym(DIV) && args.len() == 2 {
                if args[1] == int(1) {
                    return args[0].clone();
                }
                if let (IRNode::Integer(lhs), IRNode::Integer(rhs)) = (&args[0], &args[1]) {
                    if *rhs != 0 && lhs % rhs == 0 {
                        return int(lhs / rhs);
                    }
                }
            }
            if a.head == sym(POW) && args.len() == 2 {
                if args[1] == int(0) {
                    return int(1);
                }
                if args[1] == int(1) {
                    return args[0].clone();
                }
                if let (IRNode::Integer(base), IRNode::Integer(exp)) = (&args[0], &args[1]) {
                    if let Ok(exp) = u32::try_from(*exp) {
                        return int(base.pow(exp));
                    }
                }
            }
            apply(a.head, args)
        }
        other => other,
    }
}

fn as_int(node: &IRNode) -> i64 {
    match node {
        IRNode::Integer(v) => *v,
        _ => panic!("expected integer, got {node:?}"),
    }
}

#[test]
fn limit_advanced_direct_finite_result() {
    let x = sym("x");
    let expr = apply(
        sym(ADD),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
    );
    let out = limit_advanced(
        expr,
        &x,
        int(2),
        LimitAdvancedOptions {
            eval_fn: Some(&test_eval),
            ..Default::default()
        },
    );
    assert_eq!(out, int(5));
}

#[test]
fn limit_advanced_indeterminate_without_callbacks_is_unevaluated() {
    let x = sym("x");
    let expr = apply(sym(DIV), vec![x.clone(), x.clone()]);
    let out = limit_advanced(expr.clone(), &x, int(0), LimitAdvancedOptions::default());
    assert_eq!(out, apply(sym(LIMIT), vec![expr, x, int(0)]));
}

#[test]
fn limit_advanced_callback_lhopital_simple_rational() {
    let x = sym("x");
    let numer = apply(
        sym(SUB),
        vec![apply(sym(POW), vec![x.clone(), int(2)]), int(1)],
    );
    let denom = apply(sym(SUB), vec![x.clone(), int(1)]);
    let expr = apply(sym(DIV), vec![numer, denom]);
    let out = limit_advanced(
        expr,
        &x,
        int(1),
        LimitAdvancedOptions {
            diff_fn: Some(&test_diff),
            eval_fn: Some(&test_eval),
            ..Default::default()
        },
    );
    assert_eq!(out, int(2));
}

#[test]
fn limit_advanced_one_sided_infinity() {
    let x = sym("x");
    let expr = apply(sym(DIV), vec![int(1), x.clone()]);
    let out = limit_advanced(
        expr,
        &x,
        int(0),
        LimitAdvancedOptions {
            direction: Some(LimitDirection::Minus),
            ..Default::default()
        },
    );
    assert_eq!(out, sym("minf"));
}

#[test]
fn limit_advanced_power_rewrite_falls_back_inside_exp() {
    let x = sym("x");
    let expr = apply(sym(POW), vec![x.clone(), x.clone()]);
    let out = limit_advanced(
        expr,
        &x,
        int(0),
        LimitAdvancedOptions {
            direction: Some(LimitDirection::Plus),
            ..Default::default()
        },
    );
    let expected_inner = apply(
        sym(LIMIT),
        vec![
            apply(
                sym(DIV),
                vec![
                    apply(sym(LOG), vec![x.clone()]),
                    apply(sym(DIV), vec![int(1), x.clone()]),
                ],
            ),
            x,
            int(0),
        ],
    );
    assert_eq!(out, apply(sym(EXP), vec![expected_inner]));
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
    let expr = apply(
        sym(ADD),
        vec![apply(sym(MUL), vec![int(3), x.clone()]), int(2)],
    );
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
