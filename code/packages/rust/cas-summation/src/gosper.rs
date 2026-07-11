//! Gosper's algorithm for indefinite hypergeometric summation.
//!
//! Track H2 — Rust port of `code/packages/python/cas-summation/src/cas_summation/gosper.py`
//! (Track H1, PR #5366).  See the Python source for the full mathematical
//! background.  The pipeline mirrors Python and the TypeScript port 1:1:
//!
//!   1. Structurally decompose the summand into a hypergeometric product
//!      `poly(k) · ∏ base^exp(k) · ∏ Γ(k+s) / ∏ Γ(k+t)`.
//!   2. Compute the shift ratio `a(k+1)/a(k)` as two polynomials.
//!   3. Petkovšek-normalise: `r(k) = A(k)·C(k+1) / (B(k)·C(k))` with
//!      `gcd(A(k), B(k+h)) = 1` for every integer `h ≥ 0`.
//!   4. Bound the degree of `x(k)` in the Gosper key equation
//!      `A(k)·x(k+1) − B(k−1)·x(k) = C(k)` and solve the linear system
//!      via Gaussian elimination over exact rationals.
//!   5. Reconstruct `T(k) = B(k−1)·x(k)·a(k) / C(k)` and return
//!      `T(hi+1) − T(lo)` as the closed-form IR.
//!
//! Coefficients use exact `i128` rationals — no floats — chosen so the
//! intermediate Petkovšek shift-binomial products for the polynomial
//! degrees Gosper actually sees (typically ≤ 5) stay well inside the
//! 128-bit range.  This avoids a runtime dependency on `num-bigint`
//! while preserving exact arithmetic for the test cases the Python
//! reference covers.  The defensive `MAX_POLY_DEGREE = 64` cap below
//! refuses adversarial polynomial exponents before they could grow the
//! representation.

use symbolic_ir::{apply, int, rat, sym, IRNode, ADD, DIV, MUL, NEG, POW, SUB};

use crate::GAMMA_FUNC;

/// Defensive cap on polynomial degree.  Without this, an adversarial
/// summand like `Pow(k, 10^9)` would balloon `ir_to_poly` (which does
/// repeated multiplication) into a memory-bomb.  Gosper-summable
/// expressions in practice have very small polynomial degree (typically
/// ≤ 5) — anything above this cap is almost certainly not Gosper-
/// accessible and the dispatcher's other paths handle it equally well.
pub const MAX_POLY_DEGREE: i64 = 64;

// ---------------------------------------------------------------------------
// Exact i128 rational arithmetic (mirrors Python's `fractions.Fraction`).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frac {
    pub n: i128,
    pub d: i128,
}

const F0: Frac = Frac { n: 0, d: 1 };
const F1: Frac = Frac { n: 1, d: 1 };

fn igcd(a: i128, b: i128) -> i128 {
    let mut x = a.unsigned_abs();
    let mut y = b.unsigned_abs();
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    if x == 0 {
        1
    } else {
        x as i128
    }
}

fn mkf(n: i128, d: i128) -> Frac {
    if d == 0 {
        panic!("Frac denominator cannot be zero");
    }
    let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
    let g = igcd(n, d);
    Frac { n: n / g, d: d / g }
}

fn f_add(a: Frac, b: Frac) -> Frac {
    mkf(a.n * b.d + b.n * a.d, a.d * b.d)
}
fn f_sub(a: Frac, b: Frac) -> Frac {
    mkf(a.n * b.d - b.n * a.d, a.d * b.d)
}
fn f_mul(a: Frac, b: Frac) -> Frac {
    mkf(a.n * b.n, a.d * b.d)
}
fn f_div(a: Frac, b: Frac) -> Frac {
    if b.n == 0 {
        panic!("Frac division by zero");
    }
    mkf(a.n * b.d, a.d * b.n)
}
fn f_neg(a: Frac) -> Frac {
    Frac { n: -a.n, d: a.d }
}
fn f_from_i128(n: i128) -> Frac {
    Frac { n, d: 1 }
}
fn f_is_zero(a: Frac) -> bool {
    a.n == 0
}
fn f_pow(base: Frac, exp: i128) -> Frac {
    if exp == 0 {
        return F1;
    }
    if exp < 0 {
        if base.n == 0 {
            panic!("0 to a negative power");
        }
        let sign: i128 = if base.n < 0 { -1 } else { 1 };
        let inv = Frac {
            n: base.d * sign,
            d: base.n.abs(),
        };
        return f_pow(inv, -exp);
    }
    let mut result = F1;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = f_mul(result, b);
        }
        e >>= 1;
        if e > 0 {
            b = f_mul(b, b);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Univariate polynomial arithmetic over `Frac`.  `Poly` is a `Vec<Frac>`
// with `p[i]` the coefficient of `k^i`; trailing zeros trimmed.
// ---------------------------------------------------------------------------

pub type Poly = Vec<Frac>;

fn poly_trim(p: &[Frac]) -> Poly {
    let mut n = p.len();
    while n > 0 && f_is_zero(p[n - 1]) {
        n -= 1;
    }
    p[..n].to_vec()
}

fn poly_deg(p: &[Frac]) -> i64 {
    let pp = poly_trim(p);
    pp.len() as i64 - 1
}

fn poly_add(a: &[Frac], b: &[Frac]) -> Poly {
    let n = a.len().max(b.len());
    let mut out = vec![F0; n];
    for i in 0..n {
        let ai = if i < a.len() { a[i] } else { F0 };
        let bi = if i < b.len() { b[i] } else { F0 };
        out[i] = f_add(ai, bi);
    }
    poly_trim(&out)
}

fn poly_sub(a: &[Frac], b: &[Frac]) -> Poly {
    let neg_b: Vec<Frac> = b.iter().map(|x| f_neg(*x)).collect();
    poly_add(a, &neg_b)
}

fn poly_mul(a: &[Frac], b: &[Frac]) -> Poly {
    let ta = poly_trim(a);
    let tb = poly_trim(b);
    if ta.is_empty() || tb.is_empty() {
        return vec![];
    }
    let mut out = vec![F0; ta.len() + tb.len() - 1];
    for i in 0..ta.len() {
        if f_is_zero(ta[i]) {
            continue;
        }
        for j in 0..tb.len() {
            if f_is_zero(tb[j]) {
                continue;
            }
            out[i + j] = f_add(out[i + j], f_mul(ta[i], tb[j]));
        }
    }
    poly_trim(&out)
}

fn poly_scalar(p: &[Frac], c: Frac) -> Poly {
    if f_is_zero(c) {
        return vec![];
    }
    p.iter().map(|x| f_mul(*x, c)).collect()
}

/// Return `p(k + h)` via binomial expansion of `(k + h)^i`.
fn poly_shift(p: &[Frac], h: i128) -> Poly {
    let n = p.len();
    let mut out = vec![F0; n];
    for i in 0..n {
        if f_is_zero(p[i]) {
            continue;
        }
        // Pascal's row i: C(i, j) · h^(i - j) for j = 0..=i.
        let mut binom: i128 = 1;
        for j in 0..=i {
            let hpow = h.pow((i - j) as u32);
            let term = f_mul(p[i], f_from_i128(binom * hpow));
            out[j] = f_add(out[j], term);
            // next binom = binom * (i - j) / (j + 1)
            binom = binom * ((i - j) as i128) / ((j + 1) as i128);
        }
    }
    poly_trim(&out)
}

fn poly_divmod(a: &[Frac], b: &[Frac]) -> Option<(Poly, Poly)> {
    let ta = poly_trim(a);
    let tb = poly_trim(b);
    if tb.is_empty() {
        return None;
    }
    if poly_deg(&ta) < poly_deg(&tb) {
        return Some((vec![], ta));
    }
    let q_len = ta.len() - tb.len() + 1;
    let mut q = vec![F0; q_len];
    let mut r = ta;
    while poly_deg(&r) >= poly_deg(&tb) {
        let deg_diff = (poly_deg(&r) - poly_deg(&tb)) as usize;
        let coeff = f_div(r[r.len() - 1], tb[tb.len() - 1]);
        q[deg_diff] = coeff;
        let mut shifted = vec![F0; deg_diff];
        for c in tb.iter() {
            shifted.push(f_mul(*c, coeff));
        }
        r = poly_sub(&r, &shifted);
    }
    Some((poly_trim(&q), poly_trim(&r)))
}

fn poly_gcd(a: &[Frac], b: &[Frac]) -> Poly {
    let mut x = poly_trim(a);
    let mut y = poly_trim(b);
    while !y.is_empty() {
        let (_, r) = poly_divmod(&x, &y).expect("non-zero divisor");
        x = y;
        y = r;
    }
    if x.is_empty() {
        return vec![];
    }
    let lc = x[x.len() - 1];
    x.iter().map(|c| f_div(*c, lc)).collect()
}

fn poly_eq(a: &[Frac], b: &[Frac]) -> bool {
    let ta = poly_trim(a);
    let tb = poly_trim(b);
    ta == tb
}

/// Solve `M · x = rhs` over the rationals via Gaussian elimination.
fn solve_linear_system(matrix: Vec<Vec<Frac>>, rhs: Vec<Frac>) -> Option<Vec<Frac>> {
    if matrix.is_empty() {
        return if rhs.is_empty() { Some(vec![]) } else { None };
    }
    let rows = matrix.len();
    let cols = matrix[0].len();
    let mut m: Vec<Vec<Frac>> = matrix
        .into_iter()
        .enumerate()
        .map(|(i, mut row)| {
            row.push(rhs[i]);
            row
        })
        .collect();
    let mut row = 0;
    for col in 0..cols {
        let mut pivot: Option<usize> = None;
        for r in row..rows {
            if !f_is_zero(m[r][col]) {
                pivot = Some(r);
                break;
            }
        }
        let Some(pivot) = pivot else { continue };
        m.swap(row, pivot);
        let piv = m[row][col];
        for c in 0..=cols {
            m[row][c] = f_div(m[row][c], piv);
        }
        for r in 0..rows {
            if r == row {
                continue;
            }
            let factor = m[r][col];
            if f_is_zero(factor) {
                continue;
            }
            for c in 0..=cols {
                m[r][c] = f_sub(m[r][c], f_mul(factor, m[row][c]));
            }
        }
        row += 1;
    }
    // Inconsistency check.
    for r in 0..rows {
        let all_zero = (0..cols).all(|c| f_is_zero(m[r][c]));
        if all_zero && !f_is_zero(m[r][cols]) {
            return None;
        }
    }
    let mut x = vec![F0; cols];
    for c in 0..cols {
        // Find a row whose pivot column is `c`.
        for r in 0..rows {
            if m[r][c] == F1 && (0..rows).all(|r2| r2 == r || f_is_zero(m[r2][c])) {
                x[c] = m[r][cols];
                break;
            }
        }
    }
    Some(x)
}

// ---------------------------------------------------------------------------
// IR ↔ polynomial bridge.
// ---------------------------------------------------------------------------

fn frac_of(node: &IRNode) -> Option<Frac> {
    match node {
        IRNode::Integer(v) => Some(f_from_i128(*v as i128)),
        IRNode::Rational(n, d) => Some(mkf(*n as i128, *d as i128)),
        _ => None,
    }
}

fn head_is(node: &IRNode, name: &str) -> bool {
    matches!(node, IRNode::Symbol(s) if s == name)
}

/// Convert an IR expression that is a polynomial in `k` to `Poly`.
/// Returns `None` for any non-polynomial structure or when an exponent
/// would exceed `MAX_POLY_DEGREE` (DoS cap).
pub fn ir_to_poly(node: &IRNode, k: &IRNode) -> Option<Poly> {
    if let Some(r) = frac_of(node) {
        return Some(vec![r]);
    }
    if let IRNode::Symbol(_) = node {
        if node == k {
            return Some(vec![F0, F1]);
        }
        return None;
    }
    let IRNode::Apply(a) = node else {
        return None;
    };
    if head_is(&a.head, NEG) && a.args.len() == 1 {
        let inner = ir_to_poly(&a.args[0], k)?;
        return Some(poly_scalar(&inner, f_from_i128(-1)));
    }
    if head_is(&a.head, ADD) {
        let mut out: Poly = vec![];
        for arg in &a.args {
            let sub = ir_to_poly(arg, k)?;
            out = poly_add(&out, &sub);
        }
        return Some(out);
    }
    if head_is(&a.head, SUB) && a.args.len() == 2 {
        let x = ir_to_poly(&a.args[0], k)?;
        let y = ir_to_poly(&a.args[1], k)?;
        return Some(poly_sub(&x, &y));
    }
    if head_is(&a.head, MUL) {
        let mut out: Poly = vec![F1];
        for arg in &a.args {
            let sub = ir_to_poly(arg, k)?;
            out = poly_mul(&out, &sub);
        }
        return Some(out);
    }
    if head_is(&a.head, POW) && a.args.len() == 2 {
        let base_poly = ir_to_poly(&a.args[0], k)?;
        let IRNode::Integer(exp_val) = a.args[1] else {
            return None;
        };
        if exp_val < 0 {
            return None;
        }
        if exp_val > MAX_POLY_DEGREE {
            return None;
        }
        let mut result: Poly = vec![F1];
        for _ in 0..exp_val {
            result = poly_mul(&result, &base_poly);
            if poly_deg(&result) > MAX_POLY_DEGREE {
                return None;
            }
        }
        return Some(result);
    }
    if head_is(&a.head, DIV) && a.args.len() == 2 {
        let np = ir_to_poly(&a.args[0], k)?;
        let dp = ir_to_poly(&a.args[1], k)?;
        if poly_deg(&dp) != 0 {
            return None;
        }
        return Some(poly_scalar(&np, f_div(F1, dp[0])));
    }
    None
}

fn frac_to_ir(f: Frac) -> IRNode {
    if f.d == 1 {
        int(f.n as i64)
    } else {
        rat(f.n as i64, f.d as i64)
    }
}

fn poly_to_ir(p: &[Frac], k: &IRNode) -> IRNode {
    let tp = poly_trim(p);
    if tp.is_empty() {
        return int(0);
    }
    let mut terms: Vec<IRNode> = vec![];
    for (i, c) in tp.iter().enumerate() {
        if f_is_zero(*c) {
            continue;
        }
        let term = if i == 0 {
            frac_to_ir(*c)
        } else if i == 1 {
            if *c == F1 {
                k.clone()
            } else {
                apply(sym(MUL), vec![frac_to_ir(*c), k.clone()])
            }
        } else {
            let power = apply(sym(POW), vec![k.clone(), int(i as i64)]);
            if *c == F1 {
                power
            } else {
                apply(sym(MUL), vec![frac_to_ir(*c), power])
            }
        };
        terms.push(term);
    }
    if terms.is_empty() {
        return int(0);
    }
    if terms.len() == 1 {
        return terms.into_iter().next().unwrap();
    }
    apply(sym(ADD), terms)
}

// ---------------------------------------------------------------------------
// Structural factoring.
// ---------------------------------------------------------------------------

pub struct Hyp {
    pub poly: Poly,
    pub exp_factors: Vec<(Frac, Poly)>,
    pub gamma_shifts: Vec<i128>,
    pub recip_gamma_shifts: Vec<i128>,
}

impl Hyp {
    fn new() -> Self {
        Self {
            poly: vec![F1],
            exp_factors: vec![],
            gamma_shifts: vec![],
            recip_gamma_shifts: vec![],
        }
    }
}

fn try_linear_in_k(node: &IRNode, k: &IRNode) -> Option<(i128, i128)> {
    let p = ir_to_poly(node, k)?;
    if p.is_empty() {
        return Some((0, 0));
    }
    if poly_deg(&p) > 1 {
        return None;
    }
    let a = if p.len() >= 2 { p[1] } else { F0 };
    let b = p[0];
    if a.d != 1 || b.d != 1 {
        return None;
    }
    Some((a.n, b.n))
}

pub fn decompose(node: &IRNode, k: &IRNode) -> Option<Hyp> {
    let mut h = Hyp::new();
    decompose_into(node, k, &mut h)?;
    Some(h)
}

fn decompose_into(node: &IRNode, k: &IRNode, h: &mut Hyp) -> Option<()> {
    if let Some(poly) = ir_to_poly(node, k) {
        h.poly = poly_mul(&h.poly, &poly);
        return Some(());
    }
    let IRNode::Apply(a) = node else {
        return None;
    };
    if head_is(&a.head, MUL) {
        for arg in &a.args {
            decompose_into(arg, k, h)?;
        }
        return Some(());
    }
    if head_is(&a.head, NEG) && a.args.len() == 1 {
        decompose_into(&a.args[0], k, h)?;
        h.poly = poly_scalar(&h.poly, f_from_i128(-1));
        return Some(());
    }
    if head_is(&a.head, DIV) && a.args.len() == 2 {
        let num = &a.args[0];
        let den = &a.args[1];
        decompose_into(num, k, h)?;
        if let Some(den_poly) = ir_to_poly(den, k) {
            if poly_deg(&den_poly) != 0 || den_poly.is_empty() {
                return None;
            }
            h.poly = poly_scalar(&h.poly, f_div(F1, den_poly[0]));
            return Some(());
        }
        if let IRNode::Apply(dn) = den {
            if head_is(&dn.head, GAMMA_FUNC) && dn.args.len() == 1 {
                let (alpha, beta) = try_linear_in_k(&dn.args[0], k)?;
                if alpha != 1 {
                    return None;
                }
                h.recip_gamma_shifts.push(beta);
                return Some(());
            }
        }
        return None;
    }
    if head_is(&a.head, POW) && a.args.len() == 2 {
        let base_poly = ir_to_poly(&a.args[0], k)?;
        if poly_deg(&base_poly) != 0 || base_poly.is_empty() {
            return None;
        }
        let b = base_poly[0];
        if f_is_zero(b) {
            return None;
        }
        let exp_poly = ir_to_poly(&a.args[1], k)?;
        if poly_deg(&exp_poly) > 1 {
            return None;
        }
        h.exp_factors.push((b, exp_poly));
        return Some(());
    }
    if head_is(&a.head, GAMMA_FUNC) && a.args.len() == 1 {
        let (alpha, beta) = try_linear_in_k(&a.args[0], k)?;
        if alpha != 1 {
            return None;
        }
        h.gamma_shifts.push(beta);
        return Some(());
    }
    None
}

// ---------------------------------------------------------------------------
// Ratio computation.
// ---------------------------------------------------------------------------

pub fn hyp_ratio(h: &Hyp) -> Option<(Poly, Poly)> {
    let poly = &h.poly;
    if poly_trim(poly).is_empty() {
        return None;
    }
    let mut numer = poly_shift(poly, 1);
    let mut denom: Poly = poly.clone();
    for (base, exp) in &h.exp_factors {
        if poly_deg(exp) == 0 {
            continue;
        }
        let alpha = exp[1];
        if alpha.d != 1 {
            return None;
        }
        let alpha_int = alpha.n;
        let factor = if alpha_int >= 0 {
            f_pow(*base, alpha_int)
        } else {
            if f_is_zero(*base) {
                return None;
            }
            f_div(F1, f_pow(*base, -alpha_int))
        };
        numer = poly_scalar(&numer, factor);
    }
    for s in &h.gamma_shifts {
        numer = poly_mul(&numer, &[f_from_i128(*s), F1]);
    }
    for t in &h.recip_gamma_shifts {
        denom = poly_mul(&denom, &[f_from_i128(*t), F1]);
    }
    Some((numer, denom))
}

// ---------------------------------------------------------------------------
// Petkovšek normalisation.
// ---------------------------------------------------------------------------

fn petkovsek_normalise(a: &[Frac], b: &[Frac]) -> Option<(Poly, Poly, Poly)> {
    let mut a_poly: Poly = a.to_vec();
    let mut b_poly: Poly = b.to_vec();
    let mut c_poly: Poly = vec![F1];
    let max_h = poly_deg(&a_poly).max(poly_deg(&b_poly)).max(0) as i128 + 2;
    loop {
        let mut peeled = false;
        for h in 0..=max_h {
            let b_shifted = poly_shift(&b_poly, h);
            let g = poly_gcd(&a_poly, &b_shifted);
            if poly_deg(&g) >= 1 {
                let (a_new, rem_a) = poly_divmod(&a_poly, &g)?;
                if !rem_a.is_empty() {
                    return None;
                }
                let g_back = poly_shift(&g, -h);
                let (b_new, rem_b) = poly_divmod(&b_poly, &g_back)?;
                if !rem_b.is_empty() {
                    return None;
                }
                let mut acc: Poly = vec![F1];
                for i in 1..=h {
                    acc = poly_mul(&acc, &poly_shift(&g, -i));
                }
                c_poly = poly_mul(&c_poly, &acc);
                a_poly = a_new;
                b_poly = b_new;
                peeled = true;
                break;
            }
        }
        if !peeled {
            return Some((a_poly, b_poly, c_poly));
        }
    }
}

// ---------------------------------------------------------------------------
// Gosper degree bound + linear solve.
// ---------------------------------------------------------------------------

fn gosper_degree_bound(a: &[Frac], b: &[Frac], c: &[Frac]) -> i64 {
    let b_shifted = poly_shift(b, -1);
    let s = poly_add(a, &b_shifted);
    let d = poly_sub(a, &b_shifted);
    let deg_s = poly_deg(&s);
    let deg_d = poly_deg(&d);
    let deg_c = poly_deg(c);
    let bound: i64;
    if deg_s > deg_d + 1 {
        bound = deg_c - deg_s;
    } else {
        let m = poly_deg(a).max(poly_deg(&b_shifted));
        if m < 0 {
            return 0;
        }
        let s_top = if (m as usize) < s.len() { s[m as usize] } else { F0 };
        if f_is_zero(s_top) {
            bound = deg_c - m;
        } else {
            let d_at_m1 = if m >= 1 && ((m - 1) as usize) < d.len() {
                d[(m - 1) as usize]
            } else {
                F0
            };
            // candidate = -2·d[m-1]/s[m] - 1; ceiling toward +∞.
            let cand = f_sub(f_div(f_mul(f_from_i128(-2), d_at_m1), s_top), F1);
            let cand_int: i64 = if cand.n < 0 {
                0
            } else {
                let q = cand.n / cand.d;
                let r = cand.n % cand.d;
                if r == 0 {
                    q as i64
                } else {
                    q as i64 + 1
                }
            };
            bound = (deg_c - m).max(cand_int);
        }
    }
    if bound < 0 {
        return -1;
    }
    bound + 1
}

fn solve_key_equation(a: &[Frac], b: &[Frac], c: &[Frac], deg_bound: i64) -> Option<Poly> {
    if deg_bound < 0 {
        return None;
    }
    let n_unknowns = (deg_bound + 1) as usize;
    let b_shifted = poly_shift(b, -1);
    let mut basis_polys: Vec<Poly> = vec![];
    let mut max_deg: i64 = 0;
    for i in 0..n_unknowns {
        let mut k_pow_i: Poly = vec![F0; i];
        k_pow_i.push(F1);
        let kp1_pow_i = poly_shift(&k_pow_i, 1);
        let left = poly_mul(a, &kp1_pow_i);
        let right = poly_mul(&b_shifted, &k_pow_i);
        let bp = poly_sub(&left, &right);
        if poly_deg(&bp) > max_deg {
            max_deg = poly_deg(&bp);
        }
        basis_polys.push(bp);
    }
    let c_trim = poly_trim(c);
    let mut rhs_len = ((max_deg + 1) as usize).max(c_trim.len());
    if rhs_len == 0 {
        rhs_len = 1;
    }
    let mut rhs = vec![F0; rhs_len];
    for (j, cv) in c_trim.iter().enumerate() {
        rhs[j] = *cv;
    }
    let mut matrix: Vec<Vec<Frac>> = vec![];
    for j in 0..rhs_len {
        let mut row: Vec<Frac> = vec![];
        for i in 0..n_unknowns {
            let bp = &basis_polys[i];
            row.push(if j < bp.len() { bp[j] } else { F0 });
        }
        matrix.push(row);
    }
    let sol = solve_linear_system(matrix, rhs)?;
    let x_poly = poly_trim(&sol);
    if x_poly.is_empty() {
        if !poly_trim(c).is_empty() {
            return None;
        }
        return Some(vec![F0]);
    }
    let x_shifted = poly_shift(&x_poly, 1);
    let lhs = poly_sub(&poly_mul(a, &x_shifted), &poly_mul(&b_shifted, &x_poly));
    if !poly_eq(&lhs, c) {
        return None;
    }
    Some(x_poly)
}

// ---------------------------------------------------------------------------
// Top-level entry.
// ---------------------------------------------------------------------------

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

/// Attempt Gosper's algorithm on `∑_{k=lo}^{hi} summand`.  Returns the
/// IR closed form `T(hi+1) − T(lo)` on success, or `None` to signal
/// fall-through.
pub fn try_gosper_sum(summand: &IRNode, k: &IRNode, lo: &IRNode, hi: &IRNode) -> Option<IRNode> {
    let hyp = decompose(summand, k)?;
    let (a_top, b_bot) = hyp_ratio(&hyp)?;
    if poly_trim(&hyp.poly).is_empty() {
        return Some(int(0));
    }
    let (a_norm, b_norm, c_poly) = petkovsek_normalise(&a_top, &b_bot)?;
    let deg_bound = gosper_degree_bound(&a_norm, &b_norm, &c_poly);
    let x_poly = solve_key_equation(&a_norm, &b_norm, &c_poly, deg_bound)?;
    if poly_trim(&x_poly).is_empty() {
        return None;
    }

    // Reconstruct T(k) with boundary-singularity cancellation.
    let b_at_k_minus_1 = poly_shift(&b_norm, -1);
    let mut full_numer_poly = poly_mul(&poly_mul(&b_at_k_minus_1, &x_poly), &hyp.poly);
    let mut denom_poly: Poly = c_poly.clone();
    let g = poly_gcd(&full_numer_poly, &denom_poly);
    if poly_deg(&g) >= 1 {
        if let (Some((nq, rem_n)), Some((dq, rem_d))) = (
            poly_divmod(&full_numer_poly, &g),
            poly_divmod(&denom_poly, &g),
        ) {
            if rem_n.is_empty() && rem_d.is_empty() {
                full_numer_poly = nq;
                denom_poly = dq;
            }
        }
    }

    // Build the transcendental IR.
    let transcendental_ir = {
        let mut pieces: Vec<IRNode> = vec![];
        for (base, exp_poly) in &hyp.exp_factors {
            pieces.push(apply(
                sym(POW),
                vec![frac_to_ir(*base), poly_to_ir(exp_poly, k)],
            ));
        }
        for s in &hyp.gamma_shifts {
            let arg = if *s == 0 {
                k.clone()
            } else {
                apply(sym(ADD), vec![k.clone(), int(*s as i64)])
            };
            pieces.push(apply(sym(GAMMA_FUNC), vec![arg]));
        }
        let mut denom_gammas: Vec<IRNode> = vec![];
        for t in &hyp.recip_gamma_shifts {
            let arg = if *t == 0 {
                k.clone()
            } else {
                apply(sym(ADD), vec![k.clone(), int(*t as i64)])
            };
            denom_gammas.push(apply(sym(GAMMA_FUNC), vec![arg]));
        }
        if pieces.is_empty() && denom_gammas.is_empty() {
            int(1)
        } else {
            let numer = if pieces.is_empty() {
                int(1)
            } else if pieces.len() == 1 {
                pieces.into_iter().next().unwrap()
            } else {
                apply(sym(MUL), pieces)
            };
            if denom_gammas.is_empty() {
                numer
            } else {
                let denom = if denom_gammas.len() == 1 {
                    denom_gammas.into_iter().next().unwrap()
                } else {
                    apply(sym(MUL), denom_gammas)
                };
                apply(sym(DIV), vec![numer, denom])
            }
        }
    };

    let t_at = |k_value: &IRNode| -> IRNode {
        let numer_ir = poly_to_ir(&full_numer_poly, k);
        let denom_ir = poly_to_ir(&denom_poly, k);
        let numer_at = substitute(&numer_ir, k, k_value);
        let denom_at = substitute(&denom_ir, k, k_value);
        let trans_at = substitute(&transcendental_ir, k, k_value);
        apply(
            sym(DIV),
            vec![apply(sym(MUL), vec![numer_at, trans_at]), denom_at],
        )
    };

    let hi_plus_one = apply(sym(ADD), vec![hi.clone(), int(1)]);
    let t_hi = t_at(&hi_plus_one);
    let t_lo = t_at(lo);
    Some(apply(sym(SUB), vec![t_hi, t_lo]))
}
