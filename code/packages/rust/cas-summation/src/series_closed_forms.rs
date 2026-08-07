// `needless_range_loop`: the index loop indexes a parallel coefficient array and
// mirrors the recogniser's math. `type_complexity`: the returned tuple is an
// internal recogniser signature; a type alias would not improve clarity.
#![allow(clippy::needless_range_loop, clippy::type_complexity)]
//! Canonical infinite-series closed-form recogniser — Track I2.
//!
//! Rust port of
//! `code/packages/python/cas-summation/src/cas_summation/series_closed_forms.py`
//! (Track I1, PR #5382).  See the Python source for the full mathematical
//! background — this module mirrors it 1:1.
//!
//! # Recognised series
//!
//! - `∑_{k=1}^∞ 1/k^(2m)`           (m = 1..6) → `(2π)^(2m) · |B_{2m}| / (2·(2m)!)`
//! - `∑_{k=1}^∞ (-1)^(k-1)/k`                  → `log(2)`
//! - `∑_{k=1}^∞ (-1)^(k-1)/k^(2m)`  (m = 1..3) → `(1 − 2^(1-2m)) · ζ(2m)`
//! - `∑_{k=0}^∞ 1/k!`                          → `%e`
//! - `∑_{k=0}^∞ x^k/k!`                        → `exp(x)`
//! - `∑_{k=0}^∞ (-1)^k · x^(2k)/(2k)!`         → `cos(x)`
//! - `∑_{k=0}^∞ (-1)^k · x^(2k+1)/(2k+1)!`     → `sin(x)`
//! - `∑_{k=0}^∞ x^(2k)/(2k)!`                  → `cosh(x)`
//! - `∑_{k=0}^∞ x^(2k+1)/(2k+1)!`              → `sinh(x)`
//!
//! # Design constraints (per Python reference)
//!
//! - One generic Bernoulli helper, computed via the textbook recurrence
//!   `B_0 = 1; Σ_{j=0}^{n} C(n+1, j) · B_j = 0`.  Bounded depth: caller
//!   never asks for `n > 12`.
//! - Exact rational arithmetic; intermediate products use `i128` to stay
//!   below i64 overflow even for the largest case (η(6) ≈ 31/30240).
//! - Only fires when `hi = %inf`.

use std::sync::OnceLock;

use symbolic_ir::{
    apply, int, rat, sym, IRNode, ADD, COS, COSH, DIV, EXP, LOG, MUL, NEG, POW, SIN, SINH, SUB,
};

use crate::GAMMA_FUNC;

/// Maximum even-zeta exponent — spec covers k = 2..12 (m = 1..6).
const MAX_ZETA_M: usize = 6;
/// Maximum even-eta exponent — spec covers m = 1..3.
const MAX_ETA_M: usize = 3;

// ---------------------------------------------------------------------------
// Exact rational arithmetic on i128.  Mirrors Python's `Fraction` and the
// outer `Rational` type in lib.rs.  Kept module-local so we don't widen the
// public API; we use i128 internally to handle the binomial recurrence
// intermediates without overflow even for η(6).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frac {
    /// Numerator (carries sign).
    pub n: i128,
    /// Denominator (always positive after normalisation).
    pub d: i128,
}

fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    if a == 0 {
        1
    } else {
        a
    }
}

fn mk_f(n: i128, d: i128) -> Frac {
    assert!(d != 0, "Frac denominator cannot be zero");
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let g = gcd_i128(n, d);
    Frac { n: n / g, d: d / g }
}

const F0: Frac = Frac { n: 0, d: 1 };
const F1: Frac = Frac { n: 1, d: 1 };

fn f_add(a: Frac, b: Frac) -> Frac {
    mk_f(a.n * b.d + b.n * a.d, a.d * b.d)
}

fn f_sub(a: Frac, b: Frac) -> Frac {
    mk_f(a.n * b.d - b.n * a.d, a.d * b.d)
}

fn f_mul(a: Frac, b: Frac) -> Frac {
    mk_f(a.n * b.n, a.d * b.d)
}

fn f_div(a: Frac, b: Frac) -> Frac {
    assert!(b.n != 0, "Frac division by zero");
    mk_f(a.n * b.d, a.d * b.n)
}

fn f_neg(a: Frac) -> Frac {
    Frac { n: -a.n, d: a.d }
}

fn f_abs(a: Frac) -> Frac {
    Frac { n: a.n.abs(), d: a.d }
}

/// Convert a Frac to the smallest IR literal.  Down-casts to i64 because
/// every value we emit (Bernoulli, factorial denominators, etc.) is well
/// within i64 range — `638512875` for ζ(12), much smaller for η(6).
fn frac_to_ir(c: Frac) -> IRNode {
    let n = i128_to_i64(c.n);
    let d = i128_to_i64(c.d);
    if d == 1 {
        int(n)
    } else {
        rat(n, d)
    }
}

fn i128_to_i64(v: i128) -> i64 {
    assert!(v >= i64::MIN as i128 && v <= i64::MAX as i128, "value overflows i64");
    v as i64
}

// ---------------------------------------------------------------------------
// IR construction helpers
// ---------------------------------------------------------------------------

fn binary(head: &str, a: IRNode, b: IRNode) -> IRNode {
    apply(sym(head), vec![a, b])
}

fn unary(head: &str, a: IRNode) -> IRNode {
    apply(sym(head), vec![a])
}

fn pow_ir(base: IRNode, exp: IRNode) -> IRNode {
    binary(POW, base, exp)
}

fn mul_ir(a: IRNode, b: IRNode) -> IRNode {
    binary(MUL, a, b)
}

fn div_ir(a: IRNode, b: IRNode) -> IRNode {
    binary(DIV, a, b)
}

fn pi() -> IRNode {
    sym("%pi")
}

fn e_sym() -> IRNode {
    sym("%e")
}

fn head_is(node: &IRNode, name: &str) -> bool {
    matches!(node, IRNode::Symbol(actual) if actual == name)
}

fn is_int_val(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(actual) if *actual == value)
}

/// True for `-1` whether stored as `Integer(-1)` or `Neg(1)`.
fn is_neg_one_base(node: &IRNode) -> bool {
    if is_int_val(node, -1) {
        return true;
    }
    matches!(node, IRNode::Apply(a)
        if head_is(&a.head, NEG)
            && a.args.len() == 1
            && is_int_val(&a.args[0], 1))
}

/// True iff `node` is structurally constant in `k`.
fn is_constant_in_k(node: &IRNode, k: &IRNode) -> bool {
    if node == k {
        return false;
    }
    match node {
        IRNode::Apply(a) => a.args.iter().all(|arg| is_constant_in_k(arg, k)),
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// Bernoulli numbers (one generic helper)
// ---------------------------------------------------------------------------

/// Lazily-initialised cache of `B_0..B_12`.  Filled on first call;
/// thread-safe via `OnceLock`.  Bounded depth: caller never asks for
/// indices beyond `2 · MAX_ZETA_M = 12`.
fn bernoulli_cache() -> &'static Vec<Frac> {
    static CACHE: OnceLock<Vec<Frac>> = OnceLock::new();
    CACHE.get_or_init(|| {
        // Compute B_0..B_12 once via the textbook recurrence:
        //   B_0 = 1
        //   B_m = − (1 / (m+1)) · Σ_{j=0}^{m-1} C(m+1, j) · B_j   (m ≥ 1)
        // Iterative `for j in 0..m` loop — depth `m`, bounded.
        const N: usize = 2 * MAX_ZETA_M; // = 12
        let mut bs: Vec<Frac> = vec![F0; N + 1];
        bs[0] = F1;
        for m in 1..=N {
            let mut total = F0;
            // C(m+1, 0) = 1, then update C(m+1, j) → C(m+1, j+1) iteratively.
            let mut binom: i128 = 1;
            let m_big = m as i128;
            for j in 0..m {
                total = f_add(total, f_mul(Frac { n: binom, d: 1 }, bs[j]));
                let j_big = j as i128;
                binom = (binom * (m_big + 1 - j_big)) / (j_big + 1);
            }
            bs[m] = f_div(f_neg(total), Frac { n: m_big + 1, d: 1 });
        }
        bs
    })
}

/// Return `B_n` (the n-th Bernoulli number) as an exact `Frac`.
///
/// Cached: first call computes B_0..B_12; subsequent calls are O(1).
/// Caller must pass `n ≤ 12` (the cache size); larger `n` panics.
pub fn bernoulli_rational(n: usize) -> Frac {
    let cache = bernoulli_cache();
    assert!(n < cache.len(), "Bernoulli index {} exceeds cache size", n);
    cache[n]
}

/// Return the rational coefficient `c` such that `ζ(2m) = c · π^(2m)`.
///
///   c = 2^(2m) · |B_{2m}| / (2 · (2m)!)
fn zeta_even_coeff(m: usize) -> Frac {
    assert!(m >= 1, "zeta-even index must be ≥ 1");
    let b = bernoulli_rational(2 * m);
    let mut factorial_2m: i128 = 1;
    for i in 1..=(2 * m) as i128 {
        factorial_2m *= i;
    }
    let two_to_two_m: i128 = 1_i128 << (2 * m);
    f_div(
        f_mul(Frac { n: two_to_two_m, d: 1 }, f_abs(b)),
        Frac { n: 2 * factorial_2m, d: 1 },
    )
}

/// Return the rational coefficient `c` such that `η(2m) = c · π^(2m)`.
///
///   η(2m) = (1 − 2^(1−2m)) · ζ(2m)
fn eta_even_coeff(m: usize) -> Frac {
    assert!(m >= 1, "eta-even index must be ≥ 1");
    // 1 − 2^(1−2m) = 1 − 1/2^(2m-1).
    let two_exp: i128 = 1_i128 << (2 * m - 1);
    let one_minus = f_sub(F1, Frac { n: 1, d: two_exp });
    f_mul(one_minus, zeta_even_coeff(m))
}

/// Build IR for `coeff · π^power`.  Emits the canonical form
/// `π^power / denom` when `coeff = 1/denom`; otherwise the general
/// `coeff · π^power` shape.
fn pi_power_with_coeff(coeff: Frac, power: i64) -> IRNode {
    if coeff.n == 1 && coeff.d > 1 {
        return div_ir(pow_ir(pi(), int(power)), int(i128_to_i64(coeff.d)));
    }
    mul_ir(frac_to_ir(coeff), pow_ir(pi(), int(power)))
}

// ---------------------------------------------------------------------------
// Pattern recognisers
// ---------------------------------------------------------------------------

/// Match `1/k^m` (or `1/k` ≡ m=1) and return `m`; else `None`.
fn extract_inv_k_pow(f: &IRNode, k: &IRNode) -> Option<i64> {
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    if !is_int_val(&node.args[0], 1) {
        return None;
    }
    let denom = &node.args[1];
    if denom == k {
        return Some(1);
    }
    let IRNode::Apply(d) = denom else {
        return None;
    };
    if !head_is(&d.head, POW) || d.args.len() != 2 {
        return None;
    }
    if &d.args[0] != k {
        return None;
    }
    if let IRNode::Integer(m) = d.args[1] {
        if m >= 1 {
            return Some(m);
        }
    }
    None
}

/// Match `(-1)^(k-1) / k^m` and return `m`; else `None`.
fn extract_alt_inv_k_pow(f: &IRNode, k: &IRNode) -> Option<i64> {
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    let numer = &node.args[0];
    let denom = &node.args[1];
    // Numerator: (-1)^(k-1).
    let IRNode::Apply(num) = numer else {
        return None;
    };
    if !head_is(&num.head, POW) || num.args.len() != 2 {
        return None;
    }
    if !is_neg_one_base(&num.args[0]) {
        return None;
    }
    let exp = &num.args[1];
    let IRNode::Apply(e) = exp else {
        return None;
    };
    if !head_is(&e.head, SUB) || e.args.len() != 2 {
        return None;
    }
    if &e.args[0] != k || !is_int_val(&e.args[1], 1) {
        return None;
    }
    // Denominator: k or k^m.
    if denom == k {
        return Some(1);
    }
    let IRNode::Apply(d) = denom else {
        return None;
    };
    if !head_is(&d.head, POW) || d.args.len() != 2 {
        return None;
    }
    if &d.args[0] != k {
        return None;
    }
    if let IRNode::Integer(m) = d.args[1] {
        if m >= 1 {
            return Some(m);
        }
    }
    None
}

/// `Σ_{k=1}^∞ 1/k^(2m) → ζ(2m) · π^(2m)` for `m = 1..6`.
fn try_zeta_2m(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if !is_int_val(lo, 1) {
        return None;
    }
    let m_exp = extract_inv_k_pow(f, k)?;
    if m_exp % 2 != 0 {
        return None;
    }
    let m = (m_exp / 2) as usize;
    if !(1..=MAX_ZETA_M).contains(&m) {
        return None;
    }
    Some(pi_power_with_coeff(zeta_even_coeff(m), 2 * m as i64))
}

/// `Σ_{k=1}^∞ (-1)^(k-1)/k^(2m) → η(2m) · π^(2m)` for `m = 1..3`.
fn try_eta_2m(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if !is_int_val(lo, 1) {
        return None;
    }
    let m_exp = extract_alt_inv_k_pow(f, k)?;
    if m_exp % 2 != 0 {
        return None;
    }
    let m = (m_exp / 2) as usize;
    if !(1..=MAX_ETA_M).contains(&m) {
        return None;
    }
    Some(pi_power_with_coeff(eta_even_coeff(m), 2 * m as i64))
}

/// `Σ_{k=1}^∞ (-1)^(k-1)/k → log(2)`.
fn try_eta_1(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if !is_int_val(lo, 1) {
        return None;
    }
    if extract_alt_inv_k_pow(f, k)? != 1 {
        return None;
    }
    Some(unary(LOG, int(2)))
}

/// True iff `node = GammaFunc(k + 1)` (= `k!`).
fn match_gamma_kp1(node: &IRNode, k: &IRNode) -> bool {
    let IRNode::Apply(n) = node else {
        return false;
    };
    if !head_is(&n.head, GAMMA_FUNC) || n.args.len() != 1 {
        return false;
    }
    let IRNode::Apply(arg) = &n.args[0] else {
        return false;
    };
    head_is(&arg.head, ADD)
        && arg.args.len() == 2
        && &arg.args[0] == k
        && is_int_val(&arg.args[1], 1)
}

/// True iff `node = GammaFunc(slope·k + intercept + 1)`.
fn match_gamma_of_linear_in_k_plus_1(
    node: &IRNode,
    k: &IRNode,
    slope: i64,
    intercept: i64,
) -> bool {
    let IRNode::Apply(n) = node else {
        return false;
    };
    if !head_is(&n.head, GAMMA_FUNC) || n.args.len() != 1 {
        return false;
    }
    let IRNode::Apply(arg) = &n.args[0] else {
        return false;
    };
    if !head_is(&arg.head, ADD) || arg.args.len() != 2 {
        return false;
    }
    let left = &arg.args[0];
    let right = &arg.args[1];
    let IRNode::Apply(l) = left else {
        return false;
    };
    if !head_is(&l.head, MUL) || l.args.len() != 2 {
        return false;
    }
    if !is_int_val(&l.args[0], slope) || &l.args[1] != k {
        return false;
    }
    is_int_val(right, intercept + 1)
}

/// `Σ_{k=0}^∞ 1/k! → %e`.
fn try_e_series(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if !is_int_val(lo, 0) {
        return None;
    }
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    if !is_int_val(&node.args[0], 1) {
        return None;
    }
    if !match_gamma_kp1(&node.args[1], k) {
        return None;
    }
    Some(e_sym())
}

/// If `node = Pow(x, k)` with `x` constant in `k` and `x ≠ k`, return `x`.
fn extract_pow_of_x_in_k(node: &IRNode, k: &IRNode) -> Option<IRNode> {
    let IRNode::Apply(n) = node else {
        return None;
    };
    if !head_is(&n.head, POW) || n.args.len() != 2 {
        return None;
    }
    if &n.args[1] != k {
        return None;
    }
    if &n.args[0] == k {
        return None;
    }
    if !is_constant_in_k(&n.args[0], k) {
        return None;
    }
    Some(n.args[0].clone())
}

/// `Σ_{k=0}^∞ x^k/k! → exp(x)` (symbolic `x ≠ k`).
fn try_exp_series(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    if !is_int_val(lo, 0) {
        return None;
    }
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    let x = extract_pow_of_x_in_k(&node.args[0], k)?;
    if !match_gamma_kp1(&node.args[1], k) {
        return None;
    }
    Some(unary(EXP, x))
}

/// If `node = Pow(x, slope·k + intercept)` (or `Pow(x, slope·k)` when
/// `intercept == 0`) with `x` constant in `k`, return `x`.
fn extract_pow_of_x_in_linear_k(
    node: &IRNode,
    k: &IRNode,
    slope: i64,
    intercept: i64,
) -> Option<IRNode> {
    let IRNode::Apply(n) = node else {
        return None;
    };
    if !head_is(&n.head, POW) || n.args.len() != 2 {
        return None;
    }
    let base = &n.args[0];
    let exp = &n.args[1];
    if base == k || !is_constant_in_k(base, k) {
        return None;
    }
    // Bare slope·k form.
    if intercept == 0 {
        let IRNode::Apply(e) = exp else {
            return None;
        };
        if !head_is(&e.head, MUL) || e.args.len() != 2 {
            return None;
        }
        if !is_int_val(&e.args[0], slope) || &e.args[1] != k {
            return None;
        }
        return Some(base.clone());
    }
    // slope·k + intercept form.
    let IRNode::Apply(e) = exp else {
        return None;
    };
    if !head_is(&e.head, ADD) || e.args.len() != 2 {
        return None;
    }
    let left = &e.args[0];
    let right = &e.args[1];
    let IRNode::Apply(l) = left else {
        return None;
    };
    if !head_is(&l.head, MUL) || l.args.len() != 2 {
        return None;
    }
    if !is_int_val(&l.args[0], slope) || &l.args[1] != k {
        return None;
    }
    if !is_int_val(right, intercept) {
        return None;
    }
    Some(base.clone())
}

/// Generic alternating Taylor series.
///
///   Σ_{k=0}^∞ (-1)^k · x^(slope·k + intercept) / (slope·k + intercept)!
///
/// Used by try_cos_series (2, 0, COS) and try_sin_series (2, 1, SIN).
/// Tries both operand orientations of the outer Mul.
fn try_alt_taylor_series(
    f: &IRNode,
    k: &IRNode,
    lo: &IRNode,
    slope: i64,
    intercept: i64,
    head: &str,
) -> Option<IRNode> {
    if !is_int_val(lo, 0) {
        return None;
    }
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, MUL) || node.args.len() != 2 {
        return None;
    }
    let a = &node.args[0];
    let b = &node.args[1];

    fn try_orient(
        sign_term: &IRNode,
        body: &IRNode,
        k: &IRNode,
        slope: i64,
        intercept: i64,
        head: &str,
    ) -> Option<IRNode> {
        // sign_term must be (-1)^k.
        let IRNode::Apply(s) = sign_term else {
            return None;
        };
        if !head_is(&s.head, POW) || s.args.len() != 2 {
            return None;
        }
        if !is_neg_one_base(&s.args[0]) || &s.args[1] != k {
            return None;
        }
        // body must be Div(Pow(x, slope·k + intercept), GammaFunc(... + 1)).
        let IRNode::Apply(bo) = body else {
            return None;
        };
        if !head_is(&bo.head, DIV) || bo.args.len() != 2 {
            return None;
        }
        let x = extract_pow_of_x_in_linear_k(&bo.args[0], k, slope, intercept)?;
        if !match_gamma_of_linear_in_k_plus_1(&bo.args[1], k, slope, intercept) {
            return None;
        }
        Some(unary(head, x))
    }

    if let Some(r) = try_orient(a, b, k, slope, intercept, head) {
        return Some(r);
    }
    try_orient(b, a, k, slope, intercept, head)
}

fn try_cos_series(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    try_alt_taylor_series(f, k, lo, 2, 0, COS)
}

fn try_sin_series(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    try_alt_taylor_series(f, k, lo, 2, 1, SIN)
}

/// Generic hyperbolic Taylor series.
///
///   Σ_{k=0}^∞ x^(slope·k + intercept) / (slope·k + intercept)!
///
/// No alternating sign; the body is just Div(Pow(x, …), GammaFunc(… + 1)).
fn try_hyperbolic_taylor_series(
    f: &IRNode,
    k: &IRNode,
    lo: &IRNode,
    slope: i64,
    intercept: i64,
    head: &str,
) -> Option<IRNode> {
    if !is_int_val(lo, 0) {
        return None;
    }
    let IRNode::Apply(node) = f else {
        return None;
    };
    if !head_is(&node.head, DIV) || node.args.len() != 2 {
        return None;
    }
    let x = extract_pow_of_x_in_linear_k(&node.args[0], k, slope, intercept)?;
    if !match_gamma_of_linear_in_k_plus_1(&node.args[1], k, slope, intercept) {
        return None;
    }
    Some(unary(head, x))
}

fn try_cosh_series(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    try_hyperbolic_taylor_series(f, k, lo, 2, 0, COSH)
}

fn try_sinh_series(f: &IRNode, k: &IRNode, lo: &IRNode) -> Option<IRNode> {
    try_hyperbolic_taylor_series(f, k, lo, 2, 1, SINH)
}

// ---------------------------------------------------------------------------
// Public dispatcher
// ---------------------------------------------------------------------------

/// Return the closed form for a recognised canonical infinite series, or
/// `None` when no pattern matches.
///
/// Mirrors `try_closed_form_series` in the Python reference.  Only fires
/// when `hi = %inf`; finite `hi` returns `None` so the caller falls
/// through to Faulhaber / geometric / Gosper.
pub fn try_closed_form_series(
    summand: &IRNode,
    k: &IRNode,
    lo: &IRNode,
    hi: &IRNode,
) -> Option<IRNode> {
    // Infinite-bound only.
    match hi {
        IRNode::Symbol(name) if name == "inf" || name == "%inf" => {}
        _ => return None,
    }

    let patterns: &[fn(&IRNode, &IRNode, &IRNode) -> Option<IRNode>] = &[
        try_zeta_2m,
        try_eta_2m,
        try_eta_1,
        try_e_series,
        try_exp_series,
        try_cos_series,
        try_sin_series,
        try_cosh_series,
        try_sinh_series,
    ];
    for pattern in patterns {
        if let Some(result) = pattern(summand, k, lo) {
            return Some(result);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn k_sym() -> IRNode {
        sym("k")
    }

    fn x_sym() -> IRNode {
        sym("x")
    }

    fn inf() -> IRNode {
        sym("%inf")
    }

    // ---- Bernoulli table ----

    #[test]
    fn bernoulli_known_values() {
        assert_eq!(bernoulli_rational(0), Frac { n: 1, d: 1 });
        assert_eq!(bernoulli_rational(1), Frac { n: -1, d: 2 });
        assert_eq!(bernoulli_rational(2), Frac { n: 1, d: 6 });
        assert_eq!(bernoulli_rational(3), Frac { n: 0, d: 1 });
        assert_eq!(bernoulli_rational(4), Frac { n: -1, d: 30 });
        assert_eq!(bernoulli_rational(6), Frac { n: 1, d: 42 });
        assert_eq!(bernoulli_rational(8), Frac { n: -1, d: 30 });
        assert_eq!(bernoulli_rational(10), Frac { n: 5, d: 66 });
        assert_eq!(bernoulli_rational(12), Frac { n: -691, d: 2730 });
    }

    #[test]
    fn bernoulli_odd_indices_zero() {
        for n in [3, 5, 7, 9, 11] {
            assert_eq!(bernoulli_rational(n), Frac { n: 0, d: 1 });
        }
    }

    #[test]
    fn zeta_coefficient_table() {
        assert_eq!(zeta_even_coeff(1), Frac { n: 1, d: 6 });
        assert_eq!(zeta_even_coeff(2), Frac { n: 1, d: 90 });
        assert_eq!(zeta_even_coeff(3), Frac { n: 1, d: 945 });
        assert_eq!(zeta_even_coeff(4), Frac { n: 1, d: 9450 });
        assert_eq!(zeta_even_coeff(5), Frac { n: 1, d: 93555 });
        assert_eq!(zeta_even_coeff(6), Frac { n: 691, d: 638_512_875 });
    }

    #[test]
    fn eta_coefficient_table() {
        assert_eq!(eta_even_coeff(1), Frac { n: 1, d: 12 });
        assert_eq!(eta_even_coeff(2), Frac { n: 7, d: 720 });
        assert_eq!(eta_even_coeff(3), Frac { n: 31, d: 30_240 });
    }

    // ---- IR-shape helpers (mirror Python tests) ----

    fn inv_k_pow(m: i64) -> IRNode {
        if m == 1 {
            div_ir(int(1), k_sym())
        } else {
            div_ir(int(1), pow_ir(k_sym(), int(m)))
        }
    }

    fn alt_inv_k_pow(m: i64) -> IRNode {
        let neg_one_pow = pow_ir(int(-1), binary(SUB, k_sym(), int(1)));
        if m == 1 {
            div_ir(neg_one_pow, k_sym())
        } else {
            div_ir(neg_one_pow, pow_ir(k_sym(), int(m)))
        }
    }

    fn inv_factorial() -> IRNode {
        let gamma = unary(GAMMA_FUNC, binary(ADD, k_sym(), int(1)));
        div_ir(int(1), gamma)
    }

    fn xk_over_factorial() -> IRNode {
        let gamma = unary(GAMMA_FUNC, binary(ADD, k_sym(), int(1)));
        div_ir(pow_ir(x_sym(), k_sym()), gamma)
    }

    fn gamma_lin(slope: i64, intercept: i64) -> IRNode {
        unary(
            GAMMA_FUNC,
            binary(
                ADD,
                binary(MUL, int(slope), k_sym()),
                int(intercept + 1),
            ),
        )
    }

    fn pow_x_lin(slope: i64, intercept: i64) -> IRNode {
        let exp = if intercept == 0 {
            binary(MUL, int(slope), k_sym())
        } else {
            binary(ADD, binary(MUL, int(slope), k_sym()), int(intercept))
        };
        pow_ir(x_sym(), exp)
    }

    fn cos_summand() -> IRNode {
        let sign = pow_ir(int(-1), k_sym());
        let body = div_ir(pow_x_lin(2, 0), gamma_lin(2, 0));
        binary(MUL, sign, body)
    }

    fn sin_summand() -> IRNode {
        let sign = pow_ir(int(-1), k_sym());
        let body = div_ir(pow_x_lin(2, 1), gamma_lin(2, 1));
        binary(MUL, sign, body)
    }

    fn cosh_summand() -> IRNode {
        div_ir(pow_x_lin(2, 0), gamma_lin(2, 0))
    }

    fn sinh_summand() -> IRNode {
        div_ir(pow_x_lin(2, 1), gamma_lin(2, 1))
    }

    // ---- Zeta(2m) family ----

    #[test]
    fn zeta_2m_basel() {
        // ζ(2) = π²/6
        let result = try_closed_form_series(&inv_k_pow(2), &k_sym(), &int(1), &inf()).unwrap();
        // π²/6 emitted as Div(Pow(%pi, 2), 6).
        assert_eq!(
            result,
            div_ir(pow_ir(pi(), int(2)), int(6))
        );
    }

    #[test]
    fn zeta_2m_through_m6() {
        for (two_m, expected_den) in [(2, 6), (4, 90), (6, 945), (8, 9450), (10, 93555)] {
            let result =
                try_closed_form_series(&inv_k_pow(two_m), &k_sym(), &int(1), &inf()).unwrap();
            assert_eq!(result, div_ir(pow_ir(pi(), int(two_m)), int(expected_den)));
        }
    }

    #[test]
    fn zeta_12() {
        // ζ(12) = 691·π¹²/638512875 — non-1 numerator branch.
        let result = try_closed_form_series(&inv_k_pow(12), &k_sym(), &int(1), &inf()).unwrap();
        assert_eq!(
            result,
            mul_ir(rat(691, 638_512_875), pow_ir(pi(), int(12)))
        );
    }

    #[test]
    fn odd_zeta_falls_through() {
        assert!(try_closed_form_series(&inv_k_pow(3), &k_sym(), &int(1), &inf()).is_none());
    }

    #[test]
    fn zeta_past_m6_falls_through() {
        assert!(try_closed_form_series(&inv_k_pow(14), &k_sym(), &int(1), &inf()).is_none());
    }

    #[test]
    fn zeta_wrong_lo_falls_through() {
        assert!(try_closed_form_series(&inv_k_pow(2), &k_sym(), &int(2), &inf()).is_none());
    }

    // ---- Eta family ----

    #[test]
    fn eta_1_mercator() {
        let result = try_closed_form_series(&alt_inv_k_pow(1), &k_sym(), &int(1), &inf()).unwrap();
        assert_eq!(result, unary(LOG, int(2)));
    }

    #[test]
    fn eta_2() {
        // η(2) = π²/12
        let result = try_closed_form_series(&alt_inv_k_pow(2), &k_sym(), &int(1), &inf()).unwrap();
        assert_eq!(result, div_ir(pow_ir(pi(), int(2)), int(12)));
    }

    #[test]
    fn eta_4() {
        // η(4) = 7π⁴/720
        let result = try_closed_form_series(&alt_inv_k_pow(4), &k_sym(), &int(1), &inf()).unwrap();
        assert_eq!(result, mul_ir(rat(7, 720), pow_ir(pi(), int(4))));
    }

    #[test]
    fn eta_6() {
        // η(6) = 31π⁶/30240
        let result = try_closed_form_series(&alt_inv_k_pow(6), &k_sym(), &int(1), &inf()).unwrap();
        assert_eq!(result, mul_ir(rat(31, 30_240), pow_ir(pi(), int(6))));
    }

    // ---- Factorial-based series ----

    #[test]
    fn e_series_inv_factorial() {
        let result =
            try_closed_form_series(&inv_factorial(), &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, e_sym());
    }

    #[test]
    fn exp_series_x_k_over_factorial() {
        let result =
            try_closed_form_series(&xk_over_factorial(), &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, unary(EXP, x_sym()));
    }

    #[test]
    fn cos_taylor_series() {
        let result =
            try_closed_form_series(&cos_summand(), &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, unary(COS, x_sym()));
    }

    #[test]
    fn sin_taylor_series() {
        let result =
            try_closed_form_series(&sin_summand(), &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, unary(SIN, x_sym()));
    }

    #[test]
    fn cosh_taylor_series() {
        let result =
            try_closed_form_series(&cosh_summand(), &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, unary(COSH, x_sym()));
    }

    #[test]
    fn sinh_taylor_series() {
        let result =
            try_closed_form_series(&sinh_summand(), &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, unary(SINH, x_sym()));
    }

    #[test]
    fn wrong_lo_factorial_falls_through() {
        assert!(try_closed_form_series(&inv_factorial(), &k_sym(), &int(1), &inf()).is_none());
    }

    // ---- Fall-through ----

    #[test]
    fn unrecognised_sin_k_falls_through() {
        let f = unary(SIN, k_sym());
        assert!(try_closed_form_series(&f, &k_sym(), &int(1), &inf()).is_none());
    }

    #[test]
    fn finite_hi_falls_through() {
        assert!(
            try_closed_form_series(&inv_k_pow(2), &k_sym(), &int(1), &int(100)).is_none()
        );
    }

    #[test]
    fn negative_x_in_exp_series_still_matches() {
        // Use Neg(y) as the symbolic base — still constant in k.
        let y = sym("y");
        let gamma = unary(GAMMA_FUNC, binary(ADD, k_sym(), int(1)));
        let neg_y = unary(NEG, y.clone());
        let f = div_ir(pow_ir(neg_y.clone(), k_sym()), gamma);
        let result = try_closed_form_series(&f, &k_sym(), &int(0), &inf()).unwrap();
        assert_eq!(result, unary(EXP, neg_y));
    }
}
