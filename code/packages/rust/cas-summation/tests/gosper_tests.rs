//! Tests for Gosper's algorithm — Track H2 (Rust port of Python H1).
//!
//! Mirrors `code/packages/python/cas-summation/tests/test_gosper.py` —
//! 15 tests: 4 polynomial helpers, 4 acceptance, 2 fall-through, 2
//! regression, 2 structural pieces, 1 DoS cap.

use cas_summation::gosper::{
    decompose, hyp_ratio, ir_to_poly, try_gosper_sum, Frac, MAX_POLY_DEGREE,
};
use cas_summation::{evaluate_sum, rational_value, Rational, GAMMA_FUNC, SUM};
use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, MUL, NEG, POW, SUB};

// ---------------------------------------------------------------------------
// Stub evaluator that reduces arithmetic and folds GammaFunc(integer)
// down to the corresponding factorial so we can verify closed-form
// numerics that contain Gamma terms.
// ---------------------------------------------------------------------------

fn eval(node: IRNode) -> IRNode {
    match node {
        IRNode::Apply(a) => {
            let head = a.head.clone();
            let args: Vec<IRNode> = a.args.into_iter().map(eval).collect();
            let name = match &head {
                IRNode::Symbol(s) => s.as_str(),
                _ => return apply(head, args),
            };
            // GammaFunc(Integer n ≥ 1) → (n − 1)!
            if name == GAMMA_FUNC && args.len() == 1 {
                if let IRNode::Integer(v) = args[0] {
                    if v >= 1 {
                        let mut f: i64 = 1;
                        for i in 1..v {
                            f *= i;
                        }
                        return int(f);
                    }
                }
                return apply(head, args);
            }
            let out: Option<IRNode> = match name {
                ADD => fold_numeric(&args, Rational::new(0, 1), |a, b| a + b),
                MUL => fold_numeric(&args, Rational::new(1, 1), |a, b| a * b),
                SUB if args.len() == 2 => num_binary(&args[0], &args[1], |a, b| a - b),
                DIV if args.len() == 2 => num_binary(&args[0], &args[1], |a, b| a / b),
                POW if args.len() == 2 => num_pow(&args[0], &args[1]),
                NEG if args.len() == 1 => rational_value(&args[0]).map(|v| (-v).to_ir()),
                _ => None,
            };
            out.unwrap_or_else(|| apply(head, args))
        }
        other => other,
    }
}

fn fold_numeric(
    args: &[IRNode],
    init: Rational,
    op: impl Fn(Rational, Rational) -> Rational,
) -> Option<IRNode> {
    let mut acc = init;
    for arg in args {
        acc = op(acc, rational_value(arg)?);
    }
    Some(acc.to_ir())
}

fn num_binary(
    a: &IRNode,
    b: &IRNode,
    op: impl FnOnce(Rational, Rational) -> Rational,
) -> Option<IRNode> {
    Some(op(rational_value(a)?, rational_value(b)?).to_ir())
}

fn num_pow(base: &IRNode, exp: &IRNode) -> Option<IRNode> {
    let base = rational_value(base)?;
    let IRNode::Integer(e) = exp else {
        return None;
    };
    let mut r = Rational::new(1, 1);
    if *e >= 0 {
        for _ in 0..*e {
            r = r * base;
        }
        Some(r.to_ir())
    } else {
        for _ in 0..-*e {
            r = r * base;
        }
        Some((Rational::new(1, 1) / r).to_ir())
    }
}

fn substitute(node: &IRNode, from: &IRNode, to: &IRNode) -> IRNode {
    if node == from {
        return to.clone();
    }
    match node {
        IRNode::Apply(a) => apply(
            a.head.clone(),
            a.args.iter().map(|x| substitute(x, from, to)).collect(),
        ),
        _ => node.clone(),
    }
}

fn eval_at(node: &IRNode, sym_node: &IRNode, value: i64) -> Option<Rational> {
    rational_value(&eval(substitute(node, sym_node, &int(value))))
}

fn k_sym() -> IRNode {
    sym("k")
}
fn n_sym() -> IRNode {
    sym("N")
}

fn make_k_times_2_to_k() -> IRNode {
    apply(sym(MUL), vec![k_sym(), apply(sym(POW), vec![int(2), k_sym()])])
}
fn make_k_times_k_fact() -> IRNode {
    let g = apply(
        sym(GAMMA_FUNC),
        vec![apply(sym(ADD), vec![k_sym(), int(1)])],
    );
    apply(sym(MUL), vec![k_sym(), g])
}

fn fact_i64(n: i64) -> i64 {
    let mut r: i64 = 1;
    for i in 1..=n {
        r *= i;
    }
    r
}

// ---------------------------------------------------------------------------
// Internal helper tests — polynomial primitives via the public surface
// (using ir_to_poly to exercise them indirectly).
// ---------------------------------------------------------------------------

#[test]
fn poly_helpers_smoke() {
    let k = k_sym();
    // 1 + 2k as a polynomial: ir_to_poly should produce [1, 2].
    let f = apply(sym(ADD), vec![int(1), apply(sym(MUL), vec![int(2), k.clone()])]);
    let p = ir_to_poly(&f, &k).expect("polynomial");
    assert_eq!(p.len(), 2);
    assert_eq!(p[0], Frac { n: 1, d: 1 });
    assert_eq!(p[1], Frac { n: 2, d: 1 });
}

#[test]
fn poly_mul_via_pow() {
    // (1 + k)^2 = 1 + 2k + k^2 via the POW handler.
    let k = k_sym();
    let f = apply(
        sym(POW),
        vec![apply(sym(ADD), vec![int(1), k.clone()]), int(2)],
    );
    let p = ir_to_poly(&f, &k).expect("polynomial");
    assert_eq!(p.len(), 3);
    assert_eq!(p[0], Frac { n: 1, d: 1 });
    assert_eq!(p[1], Frac { n: 2, d: 1 });
    assert_eq!(p[2], Frac { n: 1, d: 1 });
}

#[test]
fn poly_shift_via_substitute() {
    // (k+1)^2 = 1 + 2k + k^2: build via ADD(k, 1)^2 then ir_to_poly.
    let k = k_sym();
    let kp1 = apply(sym(ADD), vec![k.clone(), int(1)]);
    let f = apply(sym(POW), vec![kp1, int(2)]);
    let p = ir_to_poly(&f, &k).expect("polynomial");
    assert_eq!(p.len(), 3);
    assert_eq!(p[0], Frac { n: 1, d: 1 });
    assert_eq!(p[1], Frac { n: 2, d: 1 });
    assert_eq!(p[2], Frac { n: 1, d: 1 });
}

// ---------------------------------------------------------------------------
// Acceptance cases.
// ---------------------------------------------------------------------------

#[test]
fn k_times_2_to_k_concrete_dispatcher() {
    // ∑_{k=1}^{5} k·2^k = 258.
    let f = make_k_times_2_to_k();
    let result = evaluate_sum(f, k_sym(), int(1), int(5), eval);
    assert_eq!(rational_value(&result), Some(Rational::new(258, 1)));
}

#[test]
fn k_times_2_to_k_symbolic_closed_form() {
    // With hi = N symbolic, Gosper must produce the closed form.
    let f = make_k_times_2_to_k();
    let result = try_gosper_sum(&f, &k_sym(), &int(1), &n_sym()).expect("Gosper should accept k·2^k");
    // Not unevaluated SUM.
    if let IRNode::Apply(a) = &result {
        assert_ne!(a.head, sym(SUM));
    }
    for n in [1i64, 2, 3, 5, 7] {
        let mut expected: i64 = 0;
        for j in 1..=n {
            expected += j * (1i64 << j);
        }
        let val = eval_at(&result, &n_sym(), n);
        assert_eq!(val, Some(Rational::new(expected, 1)), "k·2^k mismatch at N={n}");
    }
}

#[test]
fn k_times_k_factorial_symbolic_closed_form() {
    // ∑_{k=0}^{N} k·k! = (N+1)! − 1
    let f = make_k_times_k_fact();
    let result = try_gosper_sum(&f, &k_sym(), &int(0), &n_sym()).expect("Gosper should accept k·k!");
    if let IRNode::Apply(a) = &result {
        assert_ne!(a.head, sym(SUM));
    }
    for n in [0i64, 1, 2, 3, 4, 5] {
        let mut expected: i64 = 0;
        for j in 0..=n {
            expected += j * fact_i64(j);
        }
        let val = eval_at(&result, &n_sym(), n);
        assert_eq!(val, Some(Rational::new(expected, 1)), "k·k! mismatch at N={n}");
    }
}

#[test]
fn geometric_2_to_k_no_regression() {
    // Still routed via the geometric handler.
    let f = apply(sym(POW), vec![int(2), k_sym()]);
    let result = evaluate_sum(f, k_sym(), int(0), int(5), eval);
    assert_eq!(rational_value(&result), Some(Rational::new(63, 1)));
}

// ---------------------------------------------------------------------------
// Fall-through safety.
// ---------------------------------------------------------------------------

#[test]
fn sin_summand_falls_through() {
    let f = apply(sym("Sin"), vec![k_sym()]);
    let result = evaluate_sum(f, k_sym(), int(1), n_sym(), eval);
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym(SUM)));
}

#[test]
fn log_summand_falls_through() {
    let f = apply(sym("Log"), vec![k_sym()]);
    let result = evaluate_sum(f, k_sym(), int(1), n_sym(), eval);
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym(SUM)));
}

// ---------------------------------------------------------------------------
// Regression: existing handlers still take priority.
// ---------------------------------------------------------------------------

#[test]
fn faulhaber_still_works() {
    let k = k_sym();
    let result = evaluate_sum(k.clone(), k, int(1), int(4), eval);
    assert_eq!(rational_value(&result), Some(Rational::new(10, 1)));
}

#[test]
fn constant_summand_unchanged() {
    let result = evaluate_sum(int(5), k_sym(), int(1), int(10), eval);
    assert_eq!(rational_value(&result), Some(Rational::new(50, 1)));
}

// ---------------------------------------------------------------------------
// Lower-level structural pieces.
// ---------------------------------------------------------------------------

#[test]
fn decompose_k_times_2_to_k() {
    let f = make_k_times_2_to_k();
    let h = decompose(&f, &k_sym()).expect("decomposable");
    // poly is [0, 1] (= k).
    assert_eq!(h.poly.len(), 2);
    assert_eq!(h.poly[0], Frac { n: 0, d: 1 });
    assert_eq!(h.poly[1], Frac { n: 1, d: 1 });
    assert_eq!(h.exp_factors.len(), 1);
    let (base, exp_poly) = &h.exp_factors[0];
    assert_eq!(*base, Frac { n: 2, d: 1 });
    assert_eq!(exp_poly.len(), 2);
    assert_eq!(exp_poly[0], Frac { n: 0, d: 1 });
    assert_eq!(exp_poly[1], Frac { n: 1, d: 1 });
}

#[test]
fn ratio_k_times_2_to_k() {
    let f = make_k_times_2_to_k();
    let h = decompose(&f, &k_sym()).expect("decomposable");
    let (numer, denom) = hyp_ratio(&h).expect("ratio");
    // numer = 2 + 2k
    assert_eq!(numer.len(), 2);
    assert_eq!(numer[0], Frac { n: 2, d: 1 });
    assert_eq!(numer[1], Frac { n: 2, d: 1 });
    // denom = k = [0, 1]
    assert_eq!(denom.len(), 2);
    assert_eq!(denom[0], Frac { n: 0, d: 1 });
    assert_eq!(denom[1], Frac { n: 1, d: 1 });
}

// ---------------------------------------------------------------------------
// DoS cap.
// ---------------------------------------------------------------------------

#[test]
fn max_poly_degree_cap_refuses_giant_exponent() {
    assert_eq!(MAX_POLY_DEGREE, 64);
    // Pow(k, large): the IR can't even hold 10^9 in i64 — use i64::MAX
    // which is still > 64 and gets rejected by the cap.
    let f = apply(sym(POW), vec![k_sym(), int(i64::MAX)]);
    // Symbolic N — must fall through to unevaluated SUM, no memory
    // blowup, prompt return.
    let result = evaluate_sum(f, k_sym(), int(1), n_sym(), eval);
    assert!(matches!(&result, IRNode::Apply(a) if a.head == sym(SUM)));
}

// Avoid unused-import warning when Frac changes shape.
const _RAT_UNUSED: fn() -> IRNode = || rat(1, 2);
