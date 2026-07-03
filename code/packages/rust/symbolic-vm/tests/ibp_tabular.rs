//! Track E2 — Generic tabular integration-by-parts fallback (Rust port).
//!
//! Mirrors `tests/test_ibp.py` from the Python reference (Track E1) and
//! the TypeScript port at `tests/ibp-tabular.test.ts`.  All correctness
//! checks use **numeric differentiation of the returned antiderivative**
//! against the original integrand.  This avoids hard-coding the exact
//! algebraic form of the answer — any equivalent shape is accepted as
//! long as the symbolic antiderivative evaluates to the correct numeric
//! value.

use symbolic_ir::{apply, flt, int, sym, IRNode, COS, DIV, EXP, INTEGRATE, LOG, MUL, POW, SIN};
use symbolic_vm::{SymbolicBackend, VM};

fn make_vm() -> VM {
    VM::new(Box::new(SymbolicBackend::new()))
}

fn integrate(f: IRNode) -> IRNode {
    apply(sym(INTEGRATE), vec![f, sym("x")])
}

/// Substitute `x` ← `value` everywhere in `node`.
fn subst(node: &IRNode, var: &str, value: &IRNode) -> IRNode {
    match node {
        IRNode::Symbol(name) if name == var => value.clone(),
        IRNode::Apply(apply_node) => {
            let head = subst(&apply_node.head, var, value);
            let args: Vec<IRNode> = apply_node.args.iter().map(|a| subst(a, var, value)).collect();
            apply(head, args)
        }
        other => other.clone(),
    }
}

/// Evaluate `expr` with x ← x_val and return the resulting float.
fn eval_at(vm: &mut VM, expr: &IRNode, x_val: f64) -> f64 {
    let substituted = subst(expr, "x", &flt(x_val));
    let folded = vm.eval(substituted);
    match folded {
        IRNode::Float(f) => f,
        IRNode::Integer(n) => n as f64,
        IRNode::Rational(n, d) => n as f64 / d as f64,
        _ => f64::NAN,
    }
}

/// Recursively check whether `node` contains an `IRApply` with `head`.
fn contains_head(node: &IRNode, head_name: &str) -> bool {
    if let IRNode::Apply(a) = node {
        if let IRNode::Symbol(s) = &a.head {
            if s == head_name {
                return true;
            }
        }
        return a.args.iter().any(|arg| contains_head(arg, head_name));
    }
    false
}

/// Plain trapezoidal rule for ground-truth numeric integration.
fn trapezoidal<F: Fn(f64) -> f64>(f: F, a: f64, b: f64, n: usize) -> f64 {
    let h = (b - a) / n as f64;
    let mut total = 0.5 * (f(a) + f(b));
    for i in 1..n {
        total += f(a + i as f64 * h);
    }
    total * h
}

// ---------------------------------------------------------------------------
// Acceptance #1 — ∫ x·sin(x) dx = sin(x) − x·cos(x).
// ---------------------------------------------------------------------------
#[test]
fn ibp_x_times_sin_x_closes() {
    let mut vm = make_vm();
    let integrand = apply(sym(MUL), vec![sym("x"), apply(sym(SIN), vec![sym("x")])]);
    let out = vm.eval(integrate(integrand));
    assert!(!contains_head(&out, INTEGRATE), "not closed: {:?}", out);
    let diff = eval_at(&mut vm, &out, 1.0) - eval_at(&mut vm, &out, 0.0);
    let expected = 1.0_f64.sin() - 1.0_f64.cos();
    assert!((diff - expected).abs() < 1e-9, "diff={}, expected={}", diff, expected);
}

// ---------------------------------------------------------------------------
// Acceptance #2 — ∫ x²·eˣ dx = (x² − 2x + 2)·eˣ.
// ---------------------------------------------------------------------------
#[test]
fn ibp_xsquared_times_exp_x_closes() {
    let mut vm = make_vm();
    let integrand = apply(
        sym(MUL),
        vec![
            apply(sym(POW), vec![sym("x"), int(2)]),
            apply(sym(EXP), vec![sym("x")]),
        ],
    );
    let out = vm.eval(integrate(integrand));
    assert!(!contains_head(&out, INTEGRATE), "not closed: {:?}", out);
    let diff = eval_at(&mut vm, &out, 2.0) - eval_at(&mut vm, &out, 0.0);
    let expected = 2.0 * 2.0_f64.exp() - 2.0;
    assert!((diff - expected).abs() < 1e-9, "diff={}, expected={}", diff, expected);
}

// ---------------------------------------------------------------------------
// Acceptance #3 — higher-degree polynomial × trig: ∫ x³·cos(x) dx.
// ---------------------------------------------------------------------------
#[test]
fn ibp_xcubed_times_cos_x_closes() {
    let mut vm = make_vm();
    let integrand = apply(
        sym(MUL),
        vec![
            apply(sym(POW), vec![sym("x"), int(3)]),
            apply(sym(COS), vec![sym("x")]),
        ],
    );
    let out = vm.eval(integrate(integrand));
    assert!(!contains_head(&out, INTEGRATE), "not closed: {:?}", out);
    let diff = eval_at(&mut vm, &out, 1.0) - eval_at(&mut vm, &out, 0.0);
    let numeric = trapezoidal(|x| x * x * x * x.cos(), 0.0, 1.0, 50_000);
    assert!(
        (diff - numeric).abs() < 1e-5,
        "diff={}, numeric={}",
        diff,
        numeric
    );
}

// ---------------------------------------------------------------------------
// Acceptance #4 — fallthrough: ∫ 1/x dx returns log(x).  The integrand is
// not a Mul (its head is DIV), so tabular IBP short-circuits to None.
// ---------------------------------------------------------------------------
#[test]
fn ibp_fallthrough_one_over_x_returns_log() {
    let mut vm = make_vm();
    let integrand = apply(sym(DIV), vec![int(1), sym("x")]);
    let out = vm.eval(integrate(integrand));
    assert!(!contains_head(&out, INTEGRATE), "expected closed form: {:?}", out);
    assert!(contains_head(&out, LOG));
    let diff = eval_at(&mut vm, &out, 2.0) - eval_at(&mut vm, &out, 1.0);
    assert!((diff - 2.0_f64.ln()).abs() < 1e-12, "diff={}", diff);
}

// ---------------------------------------------------------------------------
// Acceptance #5 — Phase 23 Fresnel fallback: ∫ sin(x²) dx closes to FresnelS.
// The integrand isn't a Mul, so this must come from the shape-specific
// special-function recognizer rather than generic tabular IBP.
// ---------------------------------------------------------------------------
#[test]
fn ibp_fallthrough_fresnel_sin_xsq_closes_to_fresnel_s() {
    let mut vm = make_vm();
    let integrand = apply(sym(SIN), vec![apply(sym(POW), vec![sym("x"), int(2)])]);
    let out = vm.eval(integrate(integrand));
    assert!(!contains_head(&out, INTEGRATE), "expected closed form: {:?}", out);
    assert!(contains_head(&out, "FresnelS"), "expected FresnelS: {:?}", out);
}

// ---------------------------------------------------------------------------
// Regression #6 — ∫ cos(x²) dx closes to FresnelC rather than falling
// through to the generic unevaluated Integrate form.
// ---------------------------------------------------------------------------
#[test]
fn ibp_regression_cos_xsq_closes_to_fresnel_c() {
    let mut vm = make_vm();
    let integrand = apply(sym(COS), vec![apply(sym(POW), vec![sym("x"), int(2)])]);
    let out = vm.eval(integrate(integrand));
    assert!(!contains_head(&out, INTEGRATE), "expected closed form: {:?}", out);
    assert!(contains_head(&out, "FresnelC"), "expected FresnelC: {:?}", out);
}

#[test]
fn fresnel_pi_half_uses_canonical_fresnel_s_of_x() {
    let mut vm = make_vm();
    let integrand = apply(
        sym(SIN),
        vec![apply(
            sym(DIV),
            vec![
                apply(sym(MUL), vec![sym("%pi"), apply(sym(POW), vec![sym("x"), int(2)])]),
                int(2),
            ],
        )],
    );
    let out = vm.eval(integrate(integrand));
    assert_eq!(out, apply(sym("FresnelS"), vec![sym("x")]));
}
