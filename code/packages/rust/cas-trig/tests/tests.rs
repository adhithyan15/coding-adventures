// Integration tests for cas-trig.

use cas_trig::{
    asin_eval, atan_eval, cos_eval, expand_trig, power_reduce, sin_eval, tan_eval,
    trig_simplify, PI,
};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, COS, MUL, NEG, POW, SIN, SQRT, SUB, TAN};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build `(n/d) * Pi`.
fn pi_mult(n: i64, d: i64) -> IRNode {
    apply(sym(MUL), vec![rat(n, d), sym(PI)])
}

/// Build `Sqrt(n)`.
fn sqrt_of(n: i64) -> IRNode {
    apply(sym(SQRT), vec![int(n)])
}

// ---------------------------------------------------------------------------
// sin_eval — special values
// ---------------------------------------------------------------------------

#[test]
fn sin_of_integer_zero_is_zero() {
    assert_eq!(sin_eval(&int(0)), int(0));
}

#[test]
fn sin_of_pi_is_zero() {
    assert_eq!(sin_eval(&sym(PI)), int(0));
}

#[test]
fn sin_of_pi_over_2_is_one() {
    assert_eq!(sin_eval(&pi_mult(1, 2)), int(1));
}

#[test]
fn sin_of_3pi_over_2_is_neg_one() {
    // sin(270°) = -1
    assert_eq!(sin_eval(&pi_mult(3, 2)), int(-1));
}

#[test]
fn sin_of_pi_over_6_is_half() {
    // sin(30°) = 1/2
    assert_eq!(sin_eval(&pi_mult(1, 6)), rat(1, 2));
}

#[test]
fn sin_of_5pi_over_6_is_half() {
    // sin(150°) = 1/2
    assert_eq!(sin_eval(&pi_mult(5, 6)), rat(1, 2));
}

#[test]
fn sin_of_7pi_over_6_is_neg_half() {
    // sin(210°) = -1/2
    assert_eq!(sin_eval(&pi_mult(7, 6)), rat(-1, 2));
}

#[test]
fn sin_of_11pi_over_6_is_neg_half() {
    // sin(330°) = -1/2
    assert_eq!(sin_eval(&pi_mult(11, 6)), rat(-1, 2));
}

#[test]
fn sin_of_pi_over_4_is_sqrt2_over_2() {
    // sin(45°) = √2/2 = Mul(1/2, Sqrt(2))
    let expected = apply(sym(MUL), vec![rat(1, 2), sqrt_of(2)]);
    assert_eq!(sin_eval(&pi_mult(1, 4)), expected);
}

#[test]
fn sin_of_3pi_over_4_is_sqrt2_over_2() {
    // sin(135°) = √2/2
    let expected = apply(sym(MUL), vec![rat(1, 2), sqrt_of(2)]);
    assert_eq!(sin_eval(&pi_mult(3, 4)), expected);
}

#[test]
fn sin_of_5pi_over_4_is_neg_sqrt2_over_2() {
    // sin(225°) = -√2/2 = Neg(Mul(1/2, Sqrt(2)))
    let inner = apply(sym(MUL), vec![rat(1, 2), sqrt_of(2)]);
    let expected = apply(sym(NEG), vec![inner]);
    assert_eq!(sin_eval(&pi_mult(5, 4)), expected);
}

#[test]
fn sin_of_pi_over_3_is_sqrt3_over_2() {
    // sin(60°) = √3/2
    let expected = apply(sym(MUL), vec![rat(1, 2), sqrt_of(3)]);
    assert_eq!(sin_eval(&pi_mult(1, 3)), expected);
}

#[test]
fn sin_of_2pi_over_3_is_sqrt3_over_2() {
    // sin(120°) = √3/2
    let expected = apply(sym(MUL), vec![rat(1, 2), sqrt_of(3)]);
    assert_eq!(sin_eval(&pi_mult(2, 3)), expected);
}

// ---------------------------------------------------------------------------
// sin_eval — periodicity
// ---------------------------------------------------------------------------

#[test]
fn sin_of_2pi_is_zero() {
    // sin(2π) = sin(0) = 0
    let arg = apply(sym(MUL), vec![int(2), sym(PI)]);
    assert_eq!(sin_eval(&arg), int(0));
}

#[test]
fn sin_of_neg_pi_over_2() {
    // sin(-π/2) = -1
    let arg = apply(sym(NEG), vec![pi_mult(1, 2)]);
    assert_eq!(sin_eval(&arg), int(-1));
}

#[test]
fn sin_of_neg_pi_is_zero() {
    // sin(-π) = 0
    let arg = apply(sym(NEG), vec![sym(PI)]);
    assert_eq!(sin_eval(&arg), int(0));
}

// ---------------------------------------------------------------------------
// cos_eval — special values
// ---------------------------------------------------------------------------

#[test]
fn cos_of_integer_zero_is_one() {
    assert_eq!(cos_eval(&int(0)), int(1));
}

#[test]
fn cos_of_pi_is_neg_one() {
    assert_eq!(cos_eval(&sym(PI)), int(-1));
}

#[test]
fn cos_of_pi_over_2_is_zero() {
    assert_eq!(cos_eval(&pi_mult(1, 2)), int(0));
}

#[test]
fn cos_of_3pi_over_2_is_zero() {
    assert_eq!(cos_eval(&pi_mult(3, 2)), int(0));
}

#[test]
fn cos_of_pi_over_3_is_half() {
    // cos(60°) = 1/2
    assert_eq!(cos_eval(&pi_mult(1, 3)), rat(1, 2));
}

#[test]
fn cos_of_2pi_over_3_is_neg_half() {
    // cos(120°) = -1/2
    assert_eq!(cos_eval(&pi_mult(2, 3)), rat(-1, 2));
}

#[test]
fn cos_of_pi_over_4_is_sqrt2_over_2() {
    // cos(45°) = √2/2
    let expected = apply(sym(MUL), vec![rat(1, 2), sqrt_of(2)]);
    assert_eq!(cos_eval(&pi_mult(1, 4)), expected);
}

#[test]
fn cos_of_3pi_over_4_is_neg_sqrt2_over_2() {
    // cos(135°) = -√2/2
    let inner = apply(sym(MUL), vec![rat(1, 2), sqrt_of(2)]);
    let expected = apply(sym(NEG), vec![inner]);
    assert_eq!(cos_eval(&pi_mult(3, 4)), expected);
}

#[test]
fn cos_of_pi_over_6_is_sqrt3_over_2() {
    // cos(30°) = √3/2
    let expected = apply(sym(MUL), vec![rat(1, 2), sqrt_of(3)]);
    assert_eq!(cos_eval(&pi_mult(1, 6)), expected);
}

#[test]
fn cos_of_5pi_over_6_is_neg_sqrt3_over_2() {
    // cos(150°) = -√3/2
    let inner = apply(sym(MUL), vec![rat(1, 2), sqrt_of(3)]);
    let expected = apply(sym(NEG), vec![inner]);
    assert_eq!(cos_eval(&pi_mult(5, 6)), expected);
}

// ---------------------------------------------------------------------------
// cos_eval — periodicity
// ---------------------------------------------------------------------------

#[test]
fn cos_of_2pi_is_one() {
    let arg = apply(sym(MUL), vec![int(2), sym(PI)]);
    assert_eq!(cos_eval(&arg), int(1));
}

#[test]
fn cos_of_neg_pi_is_neg_one() {
    let arg = apply(sym(NEG), vec![sym(PI)]);
    assert_eq!(cos_eval(&arg), int(-1));
}

#[test]
fn cos_of_neg_pi_over_2_is_zero() {
    let arg = apply(sym(NEG), vec![pi_mult(1, 2)]);
    assert_eq!(cos_eval(&arg), int(0));
}

// ---------------------------------------------------------------------------
// tan_eval — special values
// ---------------------------------------------------------------------------

#[test]
fn tan_of_zero_is_zero() {
    assert_eq!(tan_eval(&int(0)), int(0));
}

#[test]
fn tan_of_pi_is_zero() {
    assert_eq!(tan_eval(&sym(PI)), int(0));
}

#[test]
fn tan_of_pi_over_4_is_one() {
    assert_eq!(tan_eval(&pi_mult(1, 4)), int(1));
}

#[test]
fn tan_of_3pi_over_4_is_neg_one() {
    // tan(135°) = -1
    assert_eq!(tan_eval(&pi_mult(3, 4)), int(-1));
}

#[test]
fn tan_of_pi_over_3_is_sqrt3() {
    // tan(60°) = √3
    assert_eq!(tan_eval(&pi_mult(1, 3)), sqrt_of(3));
}

#[test]
fn tan_of_2pi_over_3_is_neg_sqrt3() {
    // tan(120°) = -√3
    let expected = apply(sym(NEG), vec![sqrt_of(3)]);
    assert_eq!(tan_eval(&pi_mult(2, 3)), expected);
}

#[test]
fn tan_of_pi_over_6_is_inv_sqrt3() {
    // tan(30°) = 1/√3 = √3/3 = Mul(1/3, Sqrt(3))
    let expected = apply(sym(MUL), vec![rat(1, 3), sqrt_of(3)]);
    assert_eq!(tan_eval(&pi_mult(1, 6)), expected);
}

#[test]
fn tan_of_5pi_over_6_is_neg_inv_sqrt3() {
    // tan(150°) = -1/√3
    let inner = apply(sym(MUL), vec![rat(1, 3), sqrt_of(3)]);
    let expected = apply(sym(NEG), vec![inner]);
    assert_eq!(tan_eval(&pi_mult(5, 6)), expected);
}

#[test]
fn tan_of_pi_over_2_is_unevaluated() {
    // tan(π/2) = ∞ — must return unevaluated Tan(…)
    let arg = pi_mult(1, 2);
    let result = tan_eval(&arg);
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym(TAN)));
}

#[test]
fn tan_of_3pi_over_2_is_unevaluated() {
    // tan(3π/2) = ∞ — must return unevaluated Tan(…)
    let arg = pi_mult(3, 2);
    let result = tan_eval(&arg);
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym(TAN)));
}

// ---------------------------------------------------------------------------
// Numeric evaluation (float arguments)
// ---------------------------------------------------------------------------

#[test]
fn sin_numeric_of_float_zero() {
    // sin(0.0) snaps to Integer(0)
    assert_eq!(sin_eval(&IRNode::Float(0.0)), int(0));
}

#[test]
fn cos_numeric_of_float_zero() {
    // cos(0.0) snaps to Integer(1)
    assert_eq!(cos_eval(&IRNode::Float(0.0)), int(1));
}

#[test]
fn sin_of_pi_float_snaps_to_zero() {
    // sin(π as f64) ≈ 1.2e-16 → snapped to Integer(0)
    assert_eq!(sin_eval(&IRNode::Float(std::f64::consts::PI)), int(0));
}

#[test]
fn cos_of_pi_float_snaps_to_neg_one() {
    // cos(π as f64) ≈ -1.0 → snapped to Integer(-1)
    assert_eq!(cos_eval(&IRNode::Float(std::f64::consts::PI)), int(-1));
}

#[test]
fn sin_of_float_one_returns_float() {
    // sin(1.0) ≈ 0.8414... — not near an integer, returned as Float
    if let IRNode::Float(v) = sin_eval(&IRNode::Float(1.0)) {
        assert!((v - 1.0_f64.sin()).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn cos_of_float_one_returns_float() {
    if let IRNode::Float(v) = cos_eval(&IRNode::Float(1.0)) {
        assert!((v - 1.0_f64.cos()).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn tan_of_float_returns_float() {
    if let IRNode::Float(v) = tan_eval(&IRNode::Float(0.5)) {
        assert!((v - 0.5_f64.tan()).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn sin_numeric_rational_arg() {
    // sin(1/2) — rational arg → numeric
    if let IRNode::Float(v) = sin_eval(&rat(1, 2)) {
        assert!((v - 0.5_f64.sin()).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

// ---------------------------------------------------------------------------
// atan_eval / asin_eval
// ---------------------------------------------------------------------------

#[test]
fn atan_of_zero_is_float_zero() {
    assert_eq!(atan_eval(&int(0)), IRNode::Float(0.0));
}

#[test]
fn atan_of_one_is_pi_over_4_float() {
    if let IRNode::Float(v) = atan_eval(&int(1)) {
        assert!((v - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn atan_symbolic_unevaluated() {
    let result = atan_eval(&sym("x"));
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym("Atan")));
}

#[test]
fn asin_of_zero_is_float_zero() {
    assert_eq!(asin_eval(&int(0)), IRNode::Float(0.0));
}

#[test]
fn asin_of_one_is_pi_over_2_float() {
    if let IRNode::Float(v) = asin_eval(&int(1)) {
        assert!((v - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    } else {
        panic!("expected Float");
    }
}

#[test]
fn asin_out_of_domain_is_unevaluated() {
    // asin(2) is out of domain → return unevaluated
    let result = asin_eval(&int(2));
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym("Asin")));
}

// ---------------------------------------------------------------------------
// trig_simplify — tree walker
// ---------------------------------------------------------------------------

#[test]
fn trig_simplify_atom_unchanged() {
    assert_eq!(trig_simplify(&int(5)), int(5));
    assert_eq!(trig_simplify(&sym("x")), sym("x"));
}

#[test]
fn trig_simplify_sin_of_zero() {
    let expr = apply(sym(SIN), vec![int(0)]);
    assert_eq!(trig_simplify(&expr), int(0));
}

#[test]
fn trig_simplify_cos_of_pi() {
    let expr = apply(sym(COS), vec![sym(PI)]);
    assert_eq!(trig_simplify(&expr), int(-1));
}

#[test]
fn trig_simplify_walks_into_add() {
    // Add(Sin(0), Cos(Pi)) → both trig nodes simplified
    let expr = apply(sym(ADD), vec![
        apply(sym(SIN), vec![int(0)]),
        apply(sym(COS), vec![sym(PI)]),
    ]);
    let result = trig_simplify(&expr);
    if let IRNode::Apply(a) = &result {
        assert_eq!(a.args[0], int(0));
        assert_eq!(a.args[1], int(-1));
    } else {
        panic!("expected Apply(Add, …)");
    }
}

#[test]
fn trig_simplify_symbolic_stays_unevaluated() {
    // sin(x) — symbolic arg, stays as Sin(x)
    let expr = apply(sym(SIN), vec![sym("x")]);
    assert_eq!(trig_simplify(&expr), expr);
}

#[test]
fn trig_simplify_nested_cos_then_sin() {
    // Sin(Mul(1/2, Pi)) → 1, inside a Mul
    let expr = apply(sym(MUL), vec![
        int(3),
        apply(sym(SIN), vec![pi_mult(1, 2)]),
    ]);
    let result = trig_simplify(&expr);
    if let IRNode::Apply(a) = &result {
        assert_eq!(a.head, sym(MUL));
        assert_eq!(a.args[1], int(1));
    } else {
        panic!("expected Apply(Mul, …)");
    }
}

// ---------------------------------------------------------------------------
// expand_trig
// ---------------------------------------------------------------------------

#[test]
fn expand_sin_of_sum() {
    // sin(x + y) → sin(x)·cos(y) + cos(x)·sin(y)
    let expr = apply(sym(SIN), vec![
        apply(sym(ADD), vec![sym("x"), sym("y")])
    ]);
    let expected = apply(sym(ADD), vec![
        apply(sym(MUL), vec![
            apply(sym(SIN), vec![sym("x")]),
            apply(sym(COS), vec![sym("y")]),
        ]),
        apply(sym(MUL), vec![
            apply(sym(COS), vec![sym("x")]),
            apply(sym(SIN), vec![sym("y")]),
        ]),
    ]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_cos_of_sum() {
    // cos(x + y) → cos(x)·cos(y) - sin(x)·sin(y)
    let expr = apply(sym(COS), vec![
        apply(sym(ADD), vec![sym("x"), sym("y")])
    ]);
    let expected = apply(sym(SUB), vec![
        apply(sym(MUL), vec![
            apply(sym(COS), vec![sym("x")]),
            apply(sym(COS), vec![sym("y")]),
        ]),
        apply(sym(MUL), vec![
            apply(sym(SIN), vec![sym("x")]),
            apply(sym(SIN), vec![sym("y")]),
        ]),
    ]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_sin_of_difference() {
    // sin(x - y) → sin(x)·cos(y) - cos(x)·sin(y)
    let expr = apply(sym(SIN), vec![
        apply(sym(SUB), vec![sym("x"), sym("y")])
    ]);
    let expected = apply(sym(SUB), vec![
        apply(sym(MUL), vec![
            apply(sym(SIN), vec![sym("x")]),
            apply(sym(COS), vec![sym("y")]),
        ]),
        apply(sym(MUL), vec![
            apply(sym(COS), vec![sym("x")]),
            apply(sym(SIN), vec![sym("y")]),
        ]),
    ]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_cos_of_difference() {
    // cos(x - y) → cos(x)·cos(y) + sin(x)·sin(y)
    let expr = apply(sym(COS), vec![
        apply(sym(SUB), vec![sym("x"), sym("y")])
    ]);
    let expected = apply(sym(ADD), vec![
        apply(sym(MUL), vec![
            apply(sym(COS), vec![sym("x")]),
            apply(sym(COS), vec![sym("y")]),
        ]),
        apply(sym(MUL), vec![
            apply(sym(SIN), vec![sym("x")]),
            apply(sym(SIN), vec![sym("y")]),
        ]),
    ]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_sin_of_neg() {
    // sin(-x) → -sin(x)
    let expr = apply(sym(SIN), vec![apply(sym(NEG), vec![sym("x")])]);
    let expected = apply(sym(NEG), vec![apply(sym(SIN), vec![sym("x")])]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_cos_of_neg() {
    // cos(-x) → cos(x)
    let expr = apply(sym(COS), vec![apply(sym(NEG), vec![sym("x")])]);
    let expected = apply(sym(COS), vec![sym("x")]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_sin_double_angle() {
    // sin(2*x) → 2*sin(x)*cos(x)
    let expr = apply(sym(SIN), vec![
        apply(sym(MUL), vec![int(2), sym("x")])
    ]);
    let expected = apply(sym(MUL), vec![
        int(2),
        apply(sym(MUL), vec![
            apply(sym(SIN), vec![sym("x")]),
            apply(sym(COS), vec![sym("x")]),
        ]),
    ]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_cos_double_angle() {
    // cos(2*x) → cos(x)^2 - sin(x)^2  [represented as Mul(Cos(x), Cos(x)) - Mul(Sin(x), Sin(x))]
    let expr = apply(sym(COS), vec![
        apply(sym(MUL), vec![int(2), sym("x")])
    ]);
    let sin_x = apply(sym(SIN), vec![sym("x")]);
    let cos_x = apply(sym(COS), vec![sym("x")]);
    let expected = apply(sym(SUB), vec![
        apply(sym(MUL), vec![cos_x.clone(), cos_x]),
        apply(sym(MUL), vec![sin_x.clone(), sin_x]),
    ]);
    assert_eq!(expand_trig(&expr), expected);
}

#[test]
fn expand_non_trig_unchanged() {
    // Non-trig expressions pass through unchanged
    let expr = apply(sym(ADD), vec![int(1), sym("x")]);
    assert_eq!(expand_trig(&expr), expr);
}

#[test]
fn expand_atom_unchanged() {
    assert_eq!(expand_trig(&sym("x")), sym("x"));
    assert_eq!(expand_trig(&int(3)), int(3));
}

// ---------------------------------------------------------------------------
// power_reduce
// ---------------------------------------------------------------------------

#[test]
fn reduce_sin_squared() {
    // sin²(x) → (1 - cos(2x)) / 2  = Mul(1/2, Sub(1, Cos(Mul(2, x))))
    let expr = apply(sym(POW), vec![
        apply(sym(SIN), vec![sym("x")]),
        int(2),
    ]);
    let cos_2x = apply(sym(COS), vec![apply(sym(MUL), vec![int(2), sym("x")])]);
    let expected = apply(sym(MUL), vec![
        rat(1, 2),
        apply(sym(SUB), vec![int(1), cos_2x]),
    ]);
    assert_eq!(power_reduce(&expr), expected);
}

#[test]
fn reduce_cos_squared() {
    // cos²(x) → (1 + cos(2x)) / 2  = Mul(1/2, Add(1, Cos(Mul(2, x))))
    let expr = apply(sym(POW), vec![
        apply(sym(COS), vec![sym("x")]),
        int(2),
    ]);
    let cos_2x = apply(sym(COS), vec![apply(sym(MUL), vec![int(2), sym("x")])]);
    let expected = apply(sym(MUL), vec![
        rat(1, 2),
        apply(sym(ADD), vec![int(1), cos_2x]),
    ]);
    assert_eq!(power_reduce(&expr), expected);
}

#[test]
fn reduce_non_trig_power_unchanged() {
    // x^3 → unchanged
    let expr = apply(sym(POW), vec![sym("x"), int(3)]);
    assert_eq!(power_reduce(&expr), expr);
}

#[test]
fn reduce_sin_power_one_unchanged() {
    // Sin(x)^1 → not a sin², unchanged
    let expr = apply(sym(POW), vec![apply(sym(SIN), vec![sym("x")]), int(1)]);
    assert_eq!(power_reduce(&expr), expr);
}

#[test]
fn reduce_walks_into_add() {
    // Add(Pow(Sin(x), 2), Pow(Cos(x), 2)) — both parts are reduced
    let sin_sq = apply(sym(POW), vec![apply(sym(SIN), vec![sym("x")]), int(2)]);
    let cos_sq = apply(sym(POW), vec![apply(sym(COS), vec![sym("x")]), int(2)]);
    let expr = apply(sym(ADD), vec![sin_sq, cos_sq]);
    let result = power_reduce(&expr);
    // Both should have been reduced to Mul(1/2, …)
    if let IRNode::Apply(a) = &result {
        assert_eq!(a.head, sym(ADD));
        assert_eq!(a.args.len(), 2);
        assert!(matches!(&a.args[0], IRNode::Apply(inner) if inner.head == sym(MUL)));
        assert!(matches!(&a.args[1], IRNode::Apply(inner) if inner.head == sym(MUL)));
    } else {
        panic!("expected Apply(Add, …)");
    }
}

#[test]
fn reduce_atom_unchanged() {
    assert_eq!(power_reduce(&int(7)), int(7));
    assert_eq!(power_reduce(&sym("y")), sym("y"));
}

// ---------------------------------------------------------------------------
// Verify that expanding then simplifying gives correct numeric results
// ---------------------------------------------------------------------------

/// Numerically verify an identity by evaluating at a test point.
/// Checks that sin(expr_evaluated) matches the float constant.
fn check_near(node: &IRNode, expected: f64, tol: f64) {
    // Evaluate all trig nodes in the result numerically.
    fn eval_trig(n: &IRNode) -> f64 {
        match n {
            IRNode::Integer(v) => *v as f64,
            IRNode::Float(v) => *v,
            IRNode::Rational(num, den) => *num as f64 / *den as f64,
            IRNode::Apply(a) => {
                let args: Vec<f64> = a.args.iter().map(eval_trig).collect();
                match &a.head {
                    IRNode::Symbol(s) => match s.as_str() {
                        "Add" => args.iter().sum(),
                        "Sub" if args.len() == 2 => args[0] - args[1],
                        "Mul" => args.iter().product(),
                        "Neg" if args.len() == 1 => -args[0],
                        "Sin" if args.len() == 1 => args[0].sin(),
                        "Cos" if args.len() == 1 => args[0].cos(),
                        "Pow" if args.len() == 2 => args[0].powf(args[1]),
                        "Sqrt" if args.len() == 1 => args[0].sqrt(),
                        _ => f64::NAN,
                    },
                    _ => f64::NAN,
                }
            }
            _ => f64::NAN,
        }
    }
    let got = eval_trig(node);
    assert!(
        (got - expected).abs() < tol,
        "expected {expected}, got {got} (diff {})",
        (got - expected).abs()
    );
}

#[test]
fn pythagorean_identity_numerically() {
    // sin²(x) + cos²(x) = 1 for all x — verify via power_reduce
    let x = IRNode::Float(1.23);
    let sin_sq = apply(sym(POW), vec![apply(sym(SIN), vec![x.clone()]), int(2)]);
    let cos_sq = apply(sym(POW), vec![apply(sym(COS), vec![x.clone()]), int(2)]);
    let sum = apply(sym(ADD), vec![sin_sq, cos_sq]);
    let reduced = power_reduce(&sum);
    // After reduction: Mul(1/2, Sub(1, Cos(2x))) + Mul(1/2, Add(1, Cos(2x)))
    // = 1/2*(1 - cos(2x)) + 1/2*(1 + cos(2x)) = 1/2 + 1/2 = 1
    check_near(&reduced, 1.0, 1e-10);
}

#[test]
fn angle_addition_sin_numerically() {
    // sin(a + b) = sin(a)cos(b) + cos(a)sin(b) — verify at a=1.1, b=0.7
    let a = IRNode::Float(1.1);
    let b = IRNode::Float(0.7);
    let expr = apply(sym(SIN), vec![
        apply(sym(ADD), vec![a.clone(), b.clone()])
    ]);
    let expanded = expand_trig(&expr);
    let direct: f64 = (1.1_f64 + 0.7).sin();
    check_near(&expanded, direct, 1e-10);
}

#[test]
fn double_angle_sin_numerically() {
    // sin(2x) = 2*sin(x)*cos(x) — verify at x = 0.8
    let x = IRNode::Float(0.8);
    let expr = apply(sym(SIN), vec![
        apply(sym(MUL), vec![int(2), x.clone()])
    ]);
    let expanded = expand_trig(&expr);
    let direct: f64 = (2.0 * 0.8_f64).sin();
    check_near(&expanded, direct, 1e-10);
}
