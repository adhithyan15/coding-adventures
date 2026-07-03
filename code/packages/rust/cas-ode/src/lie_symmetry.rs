//! Lie point-symmetry handler for first-order ODEs — Track L2.
//!
//! Rust port of `cas_ode.lie_symmetry` (Python Track L1, commit
//! `d138e00f6`).  See the Python module for the full literate
//! explanation; this file is the structural twin.
//!
//! Three textbook point-symmetry groups are recognised numerically and
//! reduced to a quadrature:
//!
//! 1. **Translation in y** `(x, y) → (x, y + c)` — `y' = f(x)` →
//!    direct integration `y = ∫ f dx + C`.
//! 2. **Translation in x** `(x, y) → (x + c, y)` — autonomous
//!    `y' = g(y)` → inverse quadrature `x = ∫ 1/g(y) dy + C`.
//! 3. **Scaling** `(x, y) → (λx, λ^k y)` for integer `k ∈ [-3, 3] \ {0}`
//!    — similarity reduction `v = y / x^k`, giving a separable ODE in
//!    `(v, x)`.
//!
//! Detection is *numerical*: we substitute the candidate transformation
//! into the IR-level `f(x, y)` and compare the result to the predicted
//! transform at fixed sample points.  All iteration is bounded — the
//! `k` search visits seven candidates, three scale factors per
//! candidate, three sample points per scale factor = at most 63
//! numerical evaluations per ODE.  No symbolic linearised determining
//! equation is computed.
//!
//! Per Rust-cas-ode convention, integration produces a structural
//! `Integrate(expr, var)` IR node — the package's downstream consumer
//! evaluates these via the symbolic-vm.  The Lie reduction therefore
//! never "fails on unevaluated Integrate" the way the Python module
//! does; the analogous bail is when a structural prerequisite (e.g.
//! the scaling certificate) does not hold.

use symbolic_ir::{
    apply, int, sym, IRNode, ADD, COS, DIV, EQUAL, EXP, LOG, MUL, NEG, POW, SIN, SUB,
};

use crate::{
    binary_args, c, flatten_add, integrate, is_const_wrt, sub as ir_sub, unary_arg, unwrap_neg,
    y_prime,
};

// -----------------------------------------------------------------------------
// Section 1 — Bounded test points (mirror Python constants exactly).
// -----------------------------------------------------------------------------

const AUTONOMY_TEST_PTS: &[(f64, f64, f64)] = &[
    (0.7, 1.1, 2.3),
    (1.3, 0.4, 1.9),
    (2.1, 0.9, 3.0),
];

const AUTONOMY_TOL: f64 = 1e-9;

const SCALING_LAMBDAS: &[f64] = &[2.0, 3.0, 0.5];
const SCALING_POINTS: &[(f64, f64)] = &[(1.0, 1.0), (2.0, 3.0), (1.0, 2.0)];
/// Ordered with positive exponents first so we prefer simple `v = y/x` shapes.
const SCALING_K_RANGE: &[i64] = &[1, 2, 3, -1, -2, -3];
const SCALING_TOL: f64 = 1e-7;

const CERT_SAMPLE_X: &[f64] = &[1.5, 2.5, 0.4];
const CERT_SAMPLE_V: &[f64] = &[0.7, 1.3, 2.1];
const CERT_TOL: f64 = 1e-6;

// -----------------------------------------------------------------------------
// Section 2 — Numerical evaluator (over the operators that appear in
// textbook first-order ODEs).  Any unsupported head causes a `None`
// return which we treat as "give up".
// -----------------------------------------------------------------------------

fn eval_at_xy(node: &IRNode, x_sym: &IRNode, y_sym: &IRNode, xv: f64, yv: f64) -> Option<f64> {
    if node == x_sym {
        return Some(xv);
    }
    if node == y_sym {
        return Some(yv);
    }
    match node {
        IRNode::Integer(n) => Some(*n as f64),
        IRNode::Rational(n, d) => Some(*n as f64 / *d as f64),
        IRNode::Float(v) => Some(*v),
        IRNode::Symbol(_) => None,
        IRNode::Str(_) => None,
        IRNode::Apply(app) => {
            let head = app.head.clone();
            if head == sym(ADD) {
                let mut acc = 0.0;
                for a in &app.args {
                    let v = eval_at_xy(a, x_sym, y_sym, xv, yv)?;
                    if !v.is_finite() {
                        return None;
                    }
                    acc += v;
                }
                Some(acc)
            } else if head == sym(MUL) {
                let mut acc = 1.0;
                for a in &app.args {
                    let v = eval_at_xy(a, x_sym, y_sym, xv, yv)?;
                    if !v.is_finite() {
                        return None;
                    }
                    acc *= v;
                }
                Some(acc)
            } else if let Some((a, b)) = binary_args(node, SUB) {
                Some(eval_at_xy(a, x_sym, y_sym, xv, yv)? - eval_at_xy(b, x_sym, y_sym, xv, yv)?)
            } else if let Some((a, b)) = binary_args(node, DIV) {
                let bv = eval_at_xy(b, x_sym, y_sym, xv, yv)?;
                if bv == 0.0 {
                    return None;
                }
                Some(eval_at_xy(a, x_sym, y_sym, xv, yv)? / bv)
            } else if let Some(inner) = unary_arg(node, NEG) {
                Some(-eval_at_xy(inner, x_sym, y_sym, xv, yv)?)
            } else if let Some((a, b)) = binary_args(node, POW) {
                let av = eval_at_xy(a, x_sym, y_sym, xv, yv)?;
                let bv = eval_at_xy(b, x_sym, y_sym, xv, yv)?;
                let r = av.powf(bv);
                if r.is_finite() {
                    Some(r)
                } else {
                    None
                }
            } else if let Some(a) = unary_arg(node, EXP) {
                let v = eval_at_xy(a, x_sym, y_sym, xv, yv)?;
                let r = v.exp();
                if r.is_finite() {
                    Some(r)
                } else {
                    None
                }
            } else if let Some(a) = unary_arg(node, LOG) {
                let v = eval_at_xy(a, x_sym, y_sym, xv, yv)?;
                if v == 0.0 {
                    return None;
                }
                Some(v.abs().ln())
            } else if let Some(a) = unary_arg(node, SIN) {
                Some(eval_at_xy(a, x_sym, y_sym, xv, yv)?.sin())
            } else if let Some(a) = unary_arg(node, COS) {
                Some(eval_at_xy(a, x_sym, y_sym, xv, yv)?.cos())
            } else {
                None
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Section 3 — Normalise the zero-form ODE to `y' = f(x, y)`.
//
// We pull the bare `D(y, x)` summand out, treat the rest (negated) as f.
// A coefficient on `y'` other than +1 is rejected — the linear family
// already handled those.
// -----------------------------------------------------------------------------

fn extract_f(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let yp = y_prime(y, x);
    let mut found = false;
    let mut rest: Vec<IRNode> = vec![];
    for term in flatten_add(expr) {
        let (is_neg, core) = unwrap_neg(&term);
        if core == yp {
            if is_neg {
                return None; // `-y'` summand — not in our normal form
            }
            found = true;
            continue;
        }
        rest.push(term);
    }
    if !found {
        return None;
    }
    if rest.is_empty() {
        return Some(int(0));
    }
    let mut acc = rest[0].clone();
    for t in rest.into_iter().skip(1) {
        acc = apply(sym(ADD), vec![acc, t]);
    }
    Some(apply(sym(NEG), vec![acc]))
}

// -----------------------------------------------------------------------------
// Section 4 — Autonomy and scaling detection.
// -----------------------------------------------------------------------------

fn is_x_autonomous(f: &IRNode, x: &IRNode, y: &IRNode) -> bool {
    for &(yv, x1, x2) in AUTONOMY_TEST_PTS {
        let v1 = eval_at_xy(f, x, y, x1, yv);
        let v2 = eval_at_xy(f, x, y, x2, yv);
        match (v1, v2) {
            (Some(a), Some(b)) if (a - b).abs() <= AUTONOMY_TOL => {}
            _ => return false,
        }
    }
    true
}

fn is_y_autonomous(f: &IRNode, x: &IRNode, y: &IRNode) -> bool {
    for &(xv, y1, y2) in AUTONOMY_TEST_PTS {
        let v1 = eval_at_xy(f, x, y, xv, y1);
        let v2 = eval_at_xy(f, x, y, xv, y2);
        match (v1, v2) {
            (Some(a), Some(b)) if (a - b).abs() <= AUTONOMY_TOL => {}
            _ => return false,
        }
    }
    true
}

fn detect_scaling_k(f: &IRNode, x: &IRNode, y: &IRNode) -> Option<i64> {
    'outer: for &k in SCALING_K_RANGE {
        for &lam in SCALING_LAMBDAS {
            for &(xv, yv) in SCALING_POINTS {
                let y_scaled = lam.powi(k as i32) * yv;
                let lhs = eval_at_xy(f, x, y, lam * xv, y_scaled);
                let base = eval_at_xy(f, x, y, xv, yv);
                let (Some(lhs), Some(base)) = (lhs, base) else {
                    continue 'outer;
                };
                let expected = lam.powi((k - 1) as i32) * base;
                let scale = expected.abs().max(1.0);
                if (lhs - expected).abs() > SCALING_TOL * scale {
                    continue 'outer;
                }
            }
        }
        return Some(k);
    }
    None
}

// -----------------------------------------------------------------------------
// Section 5 — Reductions.
//
// The Rust solver keeps `Integrate(...)` as structural IR nodes (it has
// no symbolic integrator).  Translation-in-y and translation-in-x
// therefore always succeed in producing *some* implicit form, and the
// distinguishing feature for "we recognised this symmetry" is that we
// reach this code at all — by the time the dispatcher gets here, every
// existing handler (linear, Bernoulli, separable, exact,
// homogeneous-type) has already declined.
// -----------------------------------------------------------------------------

fn subst_ir_local(node: &IRNode, var: &IRNode, replacement: &IRNode) -> IRNode {
    if node == var {
        return replacement.clone();
    }
    match node {
        IRNode::Apply(app) => apply(
            app.head.clone(),
            app.args
                .iter()
                .map(|arg| subst_ir_local(arg, var, replacement))
                .collect(),
        ),
        _ => node.clone(),
    }
}

fn reduce_translation_y(f: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    let int_f = integrate(f.clone(), x.clone());
    Some(apply(
        sym(EQUAL),
        vec![y.clone(), apply(sym(ADD), vec![int_f, c()])],
    ))
}

fn reduce_translation_x(f: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    // Guard the trivial f = 0 case (caught by separable upstream anyway).
    if matches!(f, IRNode::Integer(0)) {
        return None;
    }
    let inv = apply(sym(DIV), vec![int(1), f.clone()]);
    let int_inv = integrate(inv, y.clone());
    Some(apply(
        sym(EQUAL),
        vec![x.clone(), apply(sym(ADD), vec![int_inv, c()])],
    ))
}

fn verify_scaling_certificate(
    f_subst: &IRNode,
    g_raw: &IRNode,
    k: i64,
    x: &IRNode,
    v: &IRNode,
) -> bool {
    for &xv in CERT_SAMPLE_X {
        for &vv in CERT_SAMPLE_V {
            let lhs = eval_at_xy(f_subst, x, v, xv, vv);
            let g = eval_at_xy(g_raw, x, v, xv, vv); // G is x-free; xv ignored
            let (Some(lhs), Some(g)) = (lhs, g) else {
                return false;
            };
            let expected = xv.powi((k - 1) as i32) * g;
            let scale = expected.abs().max(1.0);
            if (lhs - expected).abs() > CERT_TOL * scale {
                return false;
            }
        }
    }
    true
}

fn x_to_k(x: &IRNode, k: i64) -> IRNode {
    if k == 1 {
        x.clone()
    } else if k > 1 {
        apply(sym(POW), vec![x.clone(), int(k)])
    } else {
        // k < 0: 1 / x^|k|
        apply(
            sym(DIV),
            vec![int(1), apply(sym(POW), vec![x.clone(), int(-k)])],
        )
    }
}

fn reduce_scaling(f: &IRNode, k: i64, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    if k == 0 {
        return None;
    }
    let v = sym("_lie_v");
    let xtok = x_to_k(x, k);

    // Step 1: f_subst = f(x, v · x^k).
    let v_times_xtok = apply(sym(MUL), vec![v.clone(), xtok.clone()]);
    let f_subst = subst_ir_local(f, y, &v_times_xtok);

    // Step 2: G(v) = f_subst|_{x=1}.  At the certificate point x = 1,
    // x^(k-1) = 1, so f_subst evaluates to G(v) directly.  We verify
    // numerically that G is x-independent and that
    // f_subst(x, v) = x^(k-1) · G(v) at sample (x, v).
    let g_raw = subst_ir_local(&f_subst, x, &int(1));
    if !is_const_wrt(&g_raw, x) {
        return None;
    }
    if !verify_scaling_certificate(&f_subst, &g_raw, k, x, &v) {
        return None;
    }

    // Step 3: separable denominator G(v) − k·v.  Degenerate case G = k·v
    // ⇒ v = const ⇒ y = C·x^k.
    let denom = ir_sub(g_raw.clone(), apply(sym(MUL), vec![int(k), v.clone()]));

    // Heuristic degeneracy: when the denominator is *literally* the
    // integer 0 after the cheap structural builders run.  More
    // sophisticated zero-detection would require a real simplifier.
    if matches!(denom, IRNode::Integer(0)) {
        return Some(apply(
            sym(EQUAL),
            vec![y.clone(), apply(sym(MUL), vec![c(), xtok.clone()])],
        ));
    }

    // Step 4: integrate 1 / (G(v) − k·v) and back-substitute v → y/x^k.
    let integrand = apply(sym(DIV), vec![int(1), denom]);
    let h_v = integrate(integrand, v.clone());
    let y_over_xtok = apply(sym(DIV), vec![y.clone(), xtok.clone()]);
    let h_yxk = subst_ir_local(&h_v, &v, &y_over_xtok);

    let log_x = apply(sym(LOG), vec![x.clone()]);
    let rhs = apply(sym(ADD), vec![log_x, c()]);
    Some(apply(sym(EQUAL), vec![h_yxk, rhs]))
}

// -----------------------------------------------------------------------------
// Section 6 — Public entry point.
//
// Dispatch order matches the Python original:
//   1. Translation in y   (cheapest, explicit `y = ∫ f dx + C`)
//   2. Translation in x   (autonomous, implicit `x = ∫ 1/g dy + C`)
//   3. Scaling            (similarity reduction `v = y/x^k`)
// -----------------------------------------------------------------------------

pub fn try_lie_symmetry(expr: &IRNode, y: &IRNode, x: &IRNode) -> Option<IRNode> {
    if !matches!(y, IRNode::Symbol(_)) || !matches!(x, IRNode::Symbol(_)) {
        return None;
    }
    let f = extract_f(expr, y, x)?;

    if is_y_autonomous(&f, x, y) {
        if let Some(sol) = reduce_translation_y(&f, y, x) {
            return Some(sol);
        }
    }

    if is_x_autonomous(&f, x, y) {
        if let Some(sol) = reduce_translation_x(&f, y, x) {
            return Some(sol);
        }
    }

    if let Some(k) = detect_scaling_k(&f, x, y) {
        if let Some(sol) = reduce_scaling(&f, k, y, x) {
            return Some(sol);
        }
    }

    None
}
