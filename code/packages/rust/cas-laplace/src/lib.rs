//! Laplace and inverse Laplace transforms over symbolic IR.
//!
//! This crate provides:
//!
//! - [`laplace_transform`] — forward Laplace transform via table lookup and
//!   linearity rules.
//! - [`inverse_laplace`] — inverse Laplace transform via a two-stage pipeline:
//!   direct table matching followed by a full partial-fraction decomposition
//!   engine.
//!
//! ## Partial-fraction engine
//!
//! The engine handles three classes that the direct table cannot:
//!
//! 1. **Improper fractions** — polynomial long division extracts the quotient
//!    P(s); a constant quotient contributes a DiracDelta(t) term.
//!
//! 2. **Repeated rational poles** — formal power-series expansion around the
//!    pole (no symbolic differentiation needed).  All arithmetic is exact,
//!    using a `Frac` struct backed by `i64`.
//!
//! 3. **Irreducible quadratic factors** — complex-conjugate poles produce
//!    `exp(−αt)·cos(βt)` / `exp(−αt)·sin(βt)` pairs by completing the
//!    square.  When β is irrational a `Sqrt(β²)` IR node keeps the result
//!    exact.

use std::collections::BTreeMap;

use symbolic_ir::{
    apply, int, rat, sym, IRNode, ADD, COS, COSH, DIV, EXP, MUL, NEG, POW, SIN, SINH, SQRT, SUB,
};

pub const LAPLACE: &str = "Laplace";
pub const ILT: &str = "ILT";
pub const DIRAC_DELTA: &str = "DiracDelta";
pub const UNIT_STEP: &str = "UnitStep";

pub type EvalFn = dyn Fn(IRNode) -> IRNode;
pub type Handler = fn(&IRNode, &EvalFn) -> IRNode;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

pub fn laplace_transform(f: IRNode, t: IRNode, s: IRNode) -> IRNode {
    // Linearity: L{f + g} = L{f} + L{g}
    if let Some((a, b)) = binary_args(&f, ADD) {
        return binary(
            ADD,
            laplace_transform(a.clone(), t.clone(), s.clone()),
            laplace_transform(b.clone(), t.clone(), s),
        );
    }

    // Linearity: L{c·f} = c·L{f}
    if let Some((coeff, body)) = extract_coeff_and_fn(&f, &t) {
        if !is_int(&coeff, 1) {
            return binary(MUL, coeff, laplace_transform(body, t, s));
        }
    }

    table_lookup(&f, &t, &s).unwrap_or_else(|| apply(sym(LAPLACE), vec![f, t, s]))
}

pub fn inverse_laplace(f: IRNode, s: IRNode, t: IRNode) -> IRNode {
    inverse_lookup(&f, &s, &t).unwrap_or_else(|| apply(sym(ILT), vec![f, s, t]))
}

pub fn laplace_handler(expr: &IRNode, eval: &EvalFn) -> IRNode {
    let Some(args) = apply_args(expr, LAPLACE) else {
        return expr.clone();
    };
    if args.len() != 3
        || !matches!(args[1], IRNode::Symbol(_))
        || !matches!(args[2], IRNode::Symbol(_))
    {
        return expr.clone();
    }
    eval(laplace_transform(
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
    ))
}

pub fn ilt_handler(expr: &IRNode, eval: &EvalFn) -> IRNode {
    let Some(args) = apply_args(expr, ILT) else {
        return expr.clone();
    };
    if args.len() != 3
        || !matches!(args[1], IRNode::Symbol(_))
        || !matches!(args[2], IRNode::Symbol(_))
    {
        return expr.clone();
    }
    eval(inverse_laplace(
        args[0].clone(),
        args[1].clone(),
        args[2].clone(),
    ))
}

pub fn dirac_delta_handler(expr: &IRNode) -> IRNode {
    let Some(args) = apply_args(expr, DIRAC_DELTA) else {
        return expr.clone();
    };
    if args.len() == 1 && is_int(&args[0], 0) {
        int(1)
    } else {
        expr.clone()
    }
}

pub fn unit_step_handler(expr: &IRNode) -> IRNode {
    let Some(args) = apply_args(expr, UNIT_STEP) else {
        return expr.clone();
    };
    if args.len() != 1 {
        return expr.clone();
    }
    match args[0] {
        IRNode::Integer(n) if n < 0 => int(0),
        IRNode::Integer(0) => rat(1, 2),
        IRNode::Integer(_) => int(1),
        _ => expr.clone(),
    }
}

pub fn build_laplace_handler_table() -> BTreeMap<&'static str, Handler> {
    BTreeMap::from([
        (LAPLACE, laplace_handler as Handler),
        (ILT, ilt_handler as Handler),
        (DIRAC_DELTA, dirac_delta_eval_handler as Handler),
        (UNIT_STEP, unit_step_eval_handler as Handler),
    ])
}

fn dirac_delta_eval_handler(expr: &IRNode, _eval: &EvalFn) -> IRNode {
    dirac_delta_handler(expr)
}

fn unit_step_eval_handler(expr: &IRNode, _eval: &EvalFn) -> IRNode {
    unit_step_handler(expr)
}

// ─────────────────────────────────────────────────────────────────────────────
// Forward transform table
// ─────────────────────────────────────────────────────────────────────────────
//
// Entries are checked in order; the first match wins.  The t^n·trig entries
// for n ≥ 2 must appear before the n=1 entry.

fn table_lookup(f: &IRNode, t: &IRNode, s: &IRNode) -> Option<IRNode> {
    // L{1} = 1/s
    if is_one(f) {
        return Some(binary(DIV, int(1), s.clone()));
    }

    // L{t^n} = n! / s^{n+1}
    if let Some(n) = match_power_of_t(f, t) {
        return Some(binary(
            DIV,
            int(factorial(n)),
            binary(POW, s.clone(), int(n + 1)),
        ));
    }

    // L{exp(at)} = 1/(s-a)
    if let Some(a) = match_unary_linear(f, EXP, t) {
        return Some(binary(DIV, int(1), binary(SUB, s.clone(), a)));
    }

    // L{sin(ωt)} = ω/(s²+ω²)
    if let Some(w) = match_unary_linear(f, SIN, t) {
        return Some(binary(DIV, w.clone(), sum_s_sq_param_sq(s, &w)));
    }

    // L{cos(ωt)} = s/(s²+ω²)
    if let Some(w) = match_unary_linear(f, COS, t) {
        return Some(binary(DIV, s.clone(), sum_s_sq_param_sq(s, &w)));
    }

    // L{sinh(at)} = a/(s²-a²)
    if let Some(a) = match_unary_linear(f, SINH, t) {
        return Some(binary(DIV, a.clone(), sub_s_sq_param_sq(s, &a)));
    }

    // L{cosh(at)} = s/(s²-a²)
    if let Some(a) = match_unary_linear(f, COSH, t) {
        return Some(binary(DIV, s.clone(), sub_s_sq_param_sq(s, &a)));
    }

    // L{DiracDelta(t)} = 1,  L{UnitStep(t)} = 1/s
    if is_apply_of_var(f, DIRAC_DELTA, t) {
        return Some(int(1));
    }
    if is_apply_of_var(f, UNIT_STEP, t) {
        return Some(binary(DIV, int(1), s.clone()));
    }

    // L{exp(at)·trig(ωt)}: shifted oscillator pair
    if let Some((a, trig_head, w)) = match_exp_times_trig(f, t) {
        let shifted = binary(SUB, s.clone(), a);
        let denom = binary(
            ADD,
            binary(POW, shifted.clone(), int(2)),
            binary(POW, w.clone(), int(2)),
        );
        return Some(if trig_head == SIN {
            binary(DIV, w, denom)
        } else {
            binary(DIV, shifted, denom)
        });
    }

    // L{t^n·exp(at)}: n! / (s-a)^{n+1}
    if let Some((n, a)) = match_t_power_times_exp(f, t) {
        let shifted = binary(SUB, s.clone(), a);
        return Some(binary(
            DIV,
            int(factorial(n)),
            binary(POW, shifted, int(n + 1)),
        ));
    }

    // L{t^n·sin(ωt)} / L{t^n·cos(ωt)} for n ≥ 2
    //
    // Must appear BEFORE the n=1 trig case below.  Formulas derived by
    // repeated differentiation of L{sin/cos}:
    //
    //   L{t²·sin(ωt)} = 2ω(3s²−ω²) / (s²+ω²)³
    //   L{t²·cos(ωt)} = 2s(s²−3ω²) / (s²+ω²)³
    //   L{t³·sin(ωt)} = 24ωs(s²−ω²) / (s²+ω²)⁴
    //   L{t³·cos(ωt)} = 6(s⁴−6s²ω²+ω⁴) / (s²+ω²)⁴
    if let Some((n, trig_head, w)) = match_tn_times_trig(f, t) {
        let s2 = binary(POW, s.clone(), int(2));
        let w2 = binary(POW, w.clone(), int(2));
        let s2pw2 = binary(ADD, s2.clone(), w2.clone());

        if trig_head == SIN {
            match n {
                2 => {
                    // Numerator: 2ω · (3s² − ω²)
                    let num = binary(
                        MUL,
                        binary(MUL, int(2), w.clone()),
                        binary(SUB, binary(MUL, int(3), s2), w2),
                    );
                    return Some(binary(DIV, num, binary(POW, s2pw2, int(3))));
                }
                3 => {
                    // Numerator: 24ω · s · (s² − ω²)
                    let num = binary(
                        MUL,
                        binary(MUL, int(24), w),
                        binary(MUL, s.clone(), binary(SUB, s2, w2)),
                    );
                    return Some(binary(DIV, num, binary(POW, s2pw2, int(4))));
                }
                _ => return None, // n ≥ 4: fall through to unevaluated
            }
        } else {
            // COS
            match n {
                2 => {
                    // Numerator: 2s · (s² − 3ω²)
                    let num = binary(
                        MUL,
                        binary(MUL, int(2), s.clone()),
                        binary(SUB, s2, binary(MUL, int(3), w2)),
                    );
                    return Some(binary(DIV, num, binary(POW, s2pw2, int(3))));
                }
                3 => {
                    // Numerator: 6 · (s⁴ − 6s²ω² + ω⁴)
                    let s4 = binary(POW, s.clone(), int(4));
                    let w4 = binary(POW, w, int(4));
                    let inner = binary(
                        ADD,
                        binary(SUB, s4, binary(MUL, int(6), binary(MUL, s2, w2))),
                        w4,
                    );
                    return Some(binary(DIV, binary(MUL, int(6), inner), binary(POW, s2pw2, int(4))));
                }
                _ => return None, // n ≥ 4: fall through to unevaluated
            }
        }
    }

    // L{t·sin(ωt)} = 2ωs / (s²+ω²)²,  L{t·cos(ωt)} = (s²−ω²) / (s²+ω²)²
    if let Some((trig_head, w)) = match_t_times_trig(f, t) {
        let denom_base = sum_s_sq_param_sq(s, &w);
        let denom = binary(POW, denom_base, int(2));
        return Some(if trig_head == SIN {
            binary(DIV, binary(MUL, int(2), binary(MUL, w, s.clone())), denom)
        } else {
            binary(
                DIV,
                binary(SUB, binary(POW, s.clone(), int(2)), binary(POW, w, int(2))),
                denom,
            )
        });
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Inverse transform: direct table then partial-fraction engine
// ─────────────────────────────────────────────────────────────────────────────

fn inverse_lookup(f: &IRNode, s: &IRNode, t: &IRNode) -> Option<IRNode> {
    // ── Direct pattern matching ──────────────────────────────────────────────
    let (num, den) = binary_args(f, DIV)?;

    // 1/s → UnitStep(t)
    if is_int(num, 1) && same(den, s) {
        return Some(unary(UNIT_STEP, t.clone()));
    }

    if is_int(num, 1) {
        // 1/(s-a) → exp(at)
        if let Some(a) = match_s_minus_a(den, s) {
            return Some(unary(EXP, binary(MUL, a, t.clone())));
        }
        // 1/s^n  (n ≥ 2) → t^{n-1} / (n-1)!
        if let Some(n) = match_pow_of(den, s) {
            if n >= 2 {
                let power = if n == 2 {
                    t.clone()
                } else {
                    binary(POW, t.clone(), int(n - 1))
                };
                let body = if n == 2 {
                    power
                } else {
                    binary(DIV, power, int(factorial(n - 1)))
                };
                return Some(body);
            }
        }
    }

    // ω/(s²+ω²) → sin(ωt),  s/(s²+ω²) → cos(ωt)
    if let Some(w) = match_s_sq_plus_param_sq(den, s) {
        if same(num, &w) {
            return Some(unary(SIN, binary(MUL, w, t.clone())));
        }
        if same(num, s) {
            return Some(unary(COS, binary(MUL, w, t.clone())));
        }
    }

    // a/(s²-a²) → sinh(at),  s/(s²-a²) → cosh(at)
    if let Some(a) = match_s_sq_minus_param_sq(den, s) {
        if same(num, &a) {
            return Some(unary(SINH, binary(MUL, a, t.clone())));
        }
        if same(num, s) {
            return Some(unary(COSH, binary(MUL, a, t.clone())));
        }
    }

    // ── Partial-fraction decomposition engine ────────────────────────────────
    inverse_pf(f, s, t)
}

// ─────────────────────────────────────────────────────────────────────────────
// Fraction arithmetic over i64
// ─────────────────────────────────────────────────────────────────────────────
//
// `Frac` is always stored with d > 0 and gcd(|n|, d) = 1.
// The `Frac::new` constructor enforces this invariant.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frac {
    n: i64,
    d: i64,
}

impl Frac {
    /// Construct a reduced Frac.  Panics if d = 0.
    fn new(n: i64, d: i64) -> Self {
        assert!(d != 0, "Frac::new: division by zero");
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd_u64(n.unsigned_abs(), d.unsigned_abs()) as i64;
        if g == 0 {
            return Self { n: 0, d: 1 };
        }
        Self { n: n / g, d: d / g }
    }

    fn zero() -> Self {
        Self { n: 0, d: 1 }
    }
    fn one() -> Self {
        Self { n: 1, d: 1 }
    }
    fn is_zero(self) -> bool {
        self.n == 0
    }
    fn is_neg(self) -> bool {
        self.n < 0
    }
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd_u64(b, a % b) }
}

fn f_add(a: Frac, b: Frac) -> Frac {
    Frac::new(a.n * b.d + b.n * a.d, a.d * b.d)
}
fn f_sub(a: Frac, b: Frac) -> Frac {
    Frac::new(a.n * b.d - b.n * a.d, a.d * b.d)
}
fn f_mul(a: Frac, b: Frac) -> Frac {
    Frac::new(a.n * b.n, a.d * b.d)
}
fn f_div(a: Frac, b: Frac) -> Frac {
    Frac::new(a.n * b.d, a.d * b.n)
}
fn f_neg(a: Frac) -> Frac {
    Frac { n: -a.n, d: a.d }
}

/// Convert a Frac to an IRNode (Integer when d=1, Rational otherwise).
fn frac_to_ir(f: Frac) -> IRNode {
    if f.d == 1 {
        int(f.n)
    } else {
        rat(f.n, f.d)
    }
}

/// Return √f as a Frac if f is a perfect rational square, else None.
fn frac_rational_sqrt(f: Frac) -> Option<Frac> {
    let sn = isqrt_exact(f.n)?;
    let sd = isqrt_exact(f.d)?;
    Some(Frac::new(sn, sd))
}

fn isqrt_exact(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    if n == 0 {
        return Some(0);
    }
    let r = (n as f64).sqrt() as i64;
    for candidate in [r - 1, r, r + 1] {
        if candidate >= 0 && candidate * candidate == n {
            return Some(candidate);
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Polynomial arithmetic over Frac
// ─────────────────────────────────────────────────────────────────────────────
//
// `Poly = Vec<Frac>` in ascending degree order: poly[i] is the coefficient
// of s^i.

type Poly = Vec<Frac>;

fn poly_normalize(p: Poly) -> Poly {
    let mut r = p;
    while r.len() > 1 && r.last().map_or(false, |c| c.is_zero()) {
        r.pop();
    }
    if r.is_empty() {
        r.push(Frac::zero());
    }
    r
}

fn poly_degree(p: &[Frac]) -> usize {
    let n = poly_normalize(p.to_vec());
    n.len() - 1
}

fn poly_is_zero(p: &[Frac]) -> bool {
    let n = poly_normalize(p.to_vec());
    n.len() == 1 && n[0].is_zero()
}

fn poly_add(a: &[Frac], b: &[Frac]) -> Poly {
    let len = a.len().max(b.len());
    let result: Poly = (0..len)
        .map(|i| {
            let ca = a.get(i).copied().unwrap_or(Frac::zero());
            let cb = b.get(i).copied().unwrap_or(Frac::zero());
            f_add(ca, cb)
        })
        .collect();
    poly_normalize(result)
}

fn poly_neg(p: &[Frac]) -> Poly {
    p.iter().map(|&c| f_neg(c)).collect()
}

fn poly_mul(a: &[Frac], b: &[Frac]) -> Poly {
    let mut result = vec![Frac::zero(); a.len() + b.len() - 1];
    for (i, &ca) in a.iter().enumerate() {
        for (j, &cb) in b.iter().enumerate() {
            result[i + j] = f_add(result[i + j], f_mul(ca, cb));
        }
    }
    poly_normalize(result)
}

fn poly_scale(p: &[Frac], c: Frac) -> Poly {
    p.iter().map(|&x| f_mul(x, c)).collect()
}

/// Raise polynomial p to the non-negative integer power n.
fn poly_pow(p: &[Frac], n: i64) -> Poly {
    if n == 0 {
        return vec![Frac::one()];
    }
    if n == 1 {
        return p.to_vec();
    }
    let mut result = vec![Frac::one()];
    let mut base = p.to_vec();
    let mut k = n;
    while k > 0 {
        if k & 1 == 1 {
            result = poly_mul(&result, &base);
        }
        base = poly_mul(&base, &base);
        k >>= 1;
    }
    result
}

/// Evaluate polynomial at x using Horner's method.
fn poly_eval(p: &[Frac], x: Frac) -> Frac {
    let mut result = Frac::zero();
    for &coeff in p.iter().rev() {
        result = f_add(f_mul(result, x), coeff);
    }
    result
}

/// Polynomial long division: returns (quotient, remainder).
fn poly_divmod(num: &[Frac], den: &[Frac]) -> (Poly, Poly) {
    let n = poly_normalize(num.to_vec());
    let d = poly_normalize(den.to_vec());
    let deg_n = poly_degree(&n);
    let deg_d = poly_degree(&d);

    if deg_n < deg_d {
        return (vec![Frac::zero()], n);
    }

    let mut q = vec![Frac::zero(); deg_n - deg_d + 1];
    let mut rem = n.clone();

    for i in (0..=(deg_n - deg_d)).rev() {
        if i + deg_d < rem.len() && !d[deg_d].is_zero() {
            let coeff = f_div(rem[i + deg_d], d[deg_d]);
            q[i] = coeff;
            for j in 0..=deg_d {
                if i + j < rem.len() {
                    rem[i + j] = f_sub(rem[i + j], f_mul(coeff, d[j]));
                }
            }
        }
    }
    (poly_normalize(q), poly_normalize(rem))
}

/// Compute p(s + r) as a polynomial in s via binomial expansion.
fn poly_shift(p: &[Frac], r: Frac) -> Poly {
    let mut result = vec![Frac::zero()];
    for (i, &coeff) in p.iter().enumerate() {
        if coeff.is_zero() {
            continue;
        }
        let term: Poly = if i == 0 {
            vec![coeff]
        } else {
            let s_plus_r = vec![r, Frac::one()];
            poly_scale(&poly_pow(&s_plus_r, i as i64), coeff)
        };
        result = poly_add(&result, &term);
    }
    poly_normalize(result)
}

/// First `terms` Taylor coefficients of the formal power series N(t)/D(t).
/// Requires D[0] ≠ 0.
fn power_series_coeffs(num: &[Frac], den: &[Frac], terms: usize) -> Vec<Frac> {
    let q0 = den[0];
    let mut g: Vec<Frac> = Vec::with_capacity(terms);
    for k in 0..terms {
        let nk = num.get(k).copied().unwrap_or(Frac::zero());
        let mut subtract = Frac::zero();
        for j in 0..k {
            let dkj = den.get(k - j).copied().unwrap_or(Frac::zero());
            subtract = f_add(subtract, f_mul(dkj, g[j]));
        }
        g.push(f_div(f_sub(nk, subtract), q0));
    }
    g
}

/// Compute Laurent coefficients [A_m, …, A_1] for a pole of order m at r.
fn compute_repeated_residues(num: &[Frac], den: &[Frac], r: Frac, m: usize) -> Option<Vec<Frac>> {
    let nt = poly_shift(num, r);
    let dt = poly_shift(den, r);

    // Verify first m coefficients of dt are zero
    for i in 0..m {
        let val = dt.get(i).copied().unwrap_or(Frac::zero());
        if !val.is_zero() {
            return None; // wrong multiplicity
        }
    }
    if dt.len() <= m {
        return None; // degenerate
    }
    let q_other = dt[m..].to_vec();
    if q_other[0].is_zero() {
        return None; // higher multiplicity
    }

    Some(power_series_coeffs(&nt, &q_other, m))
}

// ─────────────────────────────────────────────────────────────────────────────
// Rational root finding
// ─────────────────────────────────────────────────────────────────────────────

fn rational_roots(p: &[Frac]) -> Vec<Frac> {
    let p = poly_normalize(p.to_vec());
    if p.len() <= 1 {
        return vec![];
    }

    // s = 0 is a root iff constant term is zero
    if p[0].is_zero() {
        return vec![Frac::zero()];
    }

    // Clear denominators to get integer coefficients
    let mut lcm: i64 = 1;
    for c in &p {
        let g = gcd_u64(lcm.unsigned_abs(), c.d.unsigned_abs()) as i64;
        lcm = (lcm / g) * c.d;
    }
    let int_coeffs: Vec<i64> = p.iter().map(|c| c.n * (lcm / c.d)).collect();

    let constant_term = int_coeffs[0].unsigned_abs();
    let lead_coeff = int_coeffs.last().unwrap().unsigned_abs();

    let p_divs = divisors(constant_term);
    let q_divs = divisors(lead_coeff);

    let mut roots = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for &pv in &p_divs {
        for &qv in &q_divs {
            for &sign in &[1i64, -1i64] {
                let candidate = Frac::new(sign * pv as i64, qv as i64);
                let key = (candidate.n, candidate.d);
                if seen.contains(&key) {
                    continue;
                }
                seen.insert(key);
                if poly_eval(&p, candidate).is_zero() {
                    roots.push(candidate);
                }
            }
        }
    }
    roots
}

fn divisors(n: u64) -> Vec<u64> {
    if n == 0 {
        return vec![0];
    }
    let mut divs = Vec::new();
    let mut i = 1u64;
    while i * i <= n {
        if n % i == 0 {
            divs.push(i);
            if i != n / i {
                divs.push(n / i);
            }
        }
        i += 1;
    }
    divs
}

/// Extract all rational roots with multiplicity (each root repeated).
fn extract_all_rational_roots(p: &[Frac]) -> Vec<Frac> {
    let mut p = poly_normalize(p.to_vec());
    let mut roots = Vec::new();

    while poly_degree(&p) >= 1 {
        let found = rational_roots(&p);
        if found.is_empty() {
            break;
        }
        let root = found[0];
        let mut progress = false;
        while poly_degree(&p) >= 1 && poly_eval(&p, root).is_zero() {
            roots.push(root);
            let linear = vec![f_neg(root), Frac::one()];
            let (q, _) = poly_divmod(&p, &linear);
            p = poly_normalize(q);
            progress = true;
        }
        if !progress {
            break;
        }
    }
    roots
}

// ─────────────────────────────────────────────────────────────────────────────
// IR ↔ rational function conversion
// ─────────────────────────────────────────────────────────────────────────────

/// Convert an IR node to a rational function (num/den pair of Poly).
/// Returns None if the node is not a polynomial in s or ratio of two.
fn ir_to_rational(node: &IRNode, s: &IRNode) -> Option<(Poly, Poly)> {
    match node {
        IRNode::Integer(n) => Some((vec![Frac::new(*n, 1)], vec![Frac::one()])),
        IRNode::Rational(n, d) => Some((vec![Frac::new(*n, *d)], vec![Frac::one()])),
        IRNode::Symbol(_) => {
            if same(node, s) {
                // s = 0 + 1·s
                Some((vec![Frac::zero(), Frac::one()], vec![Frac::one()]))
            } else {
                None // unknown symbol
            }
        }
        IRNode::Apply(app_node) => {
            let h = &app_node.head;
            let args = &app_node.args;

            if h == &sym(ADD) && args.len() == 2 {
                let (n1, d1) = ir_to_rational(&args[0], s)?;
                let (n2, d2) = ir_to_rational(&args[1], s)?;
                // (n1/d1) + (n2/d2) = (n1·d2 + n2·d1) / (d1·d2)
                let num = poly_normalize(poly_add(&poly_mul(&n1, &d2), &poly_mul(&n2, &d1)));
                let den = poly_normalize(poly_mul(&d1, &d2));
                return Some((num, den));
            }

            if h == &sym(SUB) && args.len() == 2 {
                let (n1, d1) = ir_to_rational(&args[0], s)?;
                let (n2, d2) = ir_to_rational(&args[1], s)?;
                let num = poly_normalize(poly_add(
                    &poly_mul(&n1, &d2),
                    &poly_mul(&poly_neg(&n2), &d1),
                ));
                let den = poly_normalize(poly_mul(&d1, &d2));
                return Some((num, den));
            }

            if h == &sym(MUL) && args.len() == 2 {
                let (n1, d1) = ir_to_rational(&args[0], s)?;
                let (n2, d2) = ir_to_rational(&args[1], s)?;
                return Some((
                    poly_normalize(poly_mul(&n1, &n2)),
                    poly_normalize(poly_mul(&d1, &d2)),
                ));
            }

            if h == &sym(DIV) && args.len() == 2 {
                let (n1, d1) = ir_to_rational(&args[0], s)?;
                let (n2, d2) = ir_to_rational(&args[1], s)?;
                // (n1/d1) / (n2/d2) = (n1·d2) / (d1·n2)
                return Some((
                    poly_normalize(poly_mul(&n1, &d2)),
                    poly_normalize(poly_mul(&d1, &n2)),
                ));
            }

            if h == &sym(NEG) && args.len() == 1 {
                let (n1, d1) = ir_to_rational(&args[0], s)?;
                return Some((poly_normalize(poly_neg(&n1)), d1));
            }

            if h == &sym(POW) && args.len() == 2 {
                let exp_node = &args[1];
                let n_exp = match exp_node {
                    IRNode::Integer(n) => *n,
                    _ => return None,
                };
                let (n1, d1) = ir_to_rational(&args[0], s)?;
                if n_exp < 0 {
                    let pos_n = (-n_exp) as i64;
                    return Some((
                        poly_normalize(poly_pow(&d1, pos_n)),
                        poly_normalize(poly_pow(&n1, pos_n)),
                    ));
                }
                return Some((
                    poly_normalize(poly_pow(&n1, n_exp)),
                    poly_normalize(poly_pow(&d1, n_exp)),
                ));
            }

            None
        }
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Inverse transform builders (pole → time-domain term)
// ─────────────────────────────────────────────────────────────────────────────

/// L⁻¹{A/(s−a)}: returns A·exp(at).  If a=0, returns A·UnitStep(t).
fn ilt_simple_pole(big_a: Frac, a: Frac, t: &IRNode) -> IRNode {
    if a.is_zero() {
        let step = unary(UNIT_STEP, t.clone());
        return if big_a == Frac::one() {
            step
        } else {
            binary(MUL, frac_to_ir(big_a), step)
        };
    }
    let exp_term = if a == Frac::one() {
        unary(EXP, t.clone())
    } else {
        unary(EXP, binary(MUL, frac_to_ir(a), t.clone()))
    };
    if big_a == Frac::one() {
        exp_term
    } else if big_a == Frac::new(-1, 1) {
        unary(NEG, exp_term)
    } else {
        binary(MUL, frac_to_ir(big_a), exp_term)
    }
}

/// L⁻¹{A/(s−a)^n}  (n ≥ 2):  A·t^{n-1}·exp(at) / (n-1)!
fn ilt_repeated_pole(big_a: Frac, a: Frac, n: usize, t: &IRNode) -> IRNode {
    let fact_nm1 = factorial((n - 1) as i64);
    let coeff = f_div(big_a, Frac::new(fact_nm1, 1));
    let coeff_node = frac_to_ir(coeff);

    // t^{n-1}  (n-1=1 → just t)
    let t_pow: IRNode = if n - 1 == 1 {
        t.clone()
    } else {
        binary(POW, t.clone(), int((n - 1) as i64))
    };

    if a.is_zero() {
        return if coeff == Frac::one() {
            t_pow
        } else {
            binary(MUL, coeff_node, t_pow)
        };
    }

    let exp_term = if a == Frac::one() {
        unary(EXP, t.clone())
    } else {
        unary(EXP, binary(MUL, frac_to_ir(a), t.clone()))
    };
    let inner = binary(MUL, t_pow, exp_term);
    if coeff == Frac::one() {
        inner
    } else {
        binary(MUL, coeff_node, inner)
    }
}

/// Invert (A·s + B) / (s² + b·s + c) with discriminant b²−4c < 0.
fn ilt_irreducible_quad(lin_num: &[Frac], quad_den: &[Frac], t: &IRNode) -> Option<Vec<IRNode>> {
    if poly_degree(quad_den) != 2 {
        return None;
    }

    let leading = quad_den.get(2).copied().unwrap_or(Frac::zero());
    if leading.is_zero() {
        return None;
    }

    // Make monic
    let inv_leading = f_div(Frac::one(), leading);
    let c = f_mul(quad_den.first().copied().unwrap_or(Frac::zero()), inv_leading);
    let b = f_mul(
        quad_den.get(1).copied().unwrap_or(Frac::zero()),
        inv_leading,
    );
    let big_b = f_mul(
        lin_num.first().copied().unwrap_or(Frac::zero()),
        inv_leading,
    );
    let big_a = f_mul(
        lin_num.get(1).copied().unwrap_or(Frac::zero()),
        inv_leading,
    );

    // Discriminant check: b²−4c < 0
    let disc = f_sub(f_mul(b, b), f_mul(Frac::new(4, 1), c));
    if !disc.is_neg() {
        return None; // real roots
    }

    // Complete the square: α = b/2, β² = c − α²
    let alpha = f_div(b, Frac::new(2, 1));
    let beta_sq = f_sub(c, f_mul(alpha, alpha));
    if beta_sq.is_zero() || beta_sq.is_neg() {
        return None;
    }

    let beta_rat = frac_rational_sqrt(beta_sq);
    let beta_ir: IRNode = match beta_rat {
        Some(br) => frac_to_ir(br),
        None => unary(SQRT, frac_to_ir(beta_sq)),
    };
    let neg_alpha = f_neg(alpha);

    let beta_is_one = beta_rat.map_or(false, |b| b == Frac::one());
    let alpha_is_zero = alpha.is_zero();

    // Build coeff · exp(−α·t) · trig(β·t)
    let make_exp_trig = |coeff_ir: IRNode, is_cos: bool| -> IRNode {
        let trig_arg: IRNode = if beta_is_one {
            t.clone()
        } else {
            binary(MUL, beta_ir.clone(), t.clone())
        };
        let trig_fn = if is_cos { COS } else { SIN };
        let trig_term = unary(trig_fn, trig_arg);

        let oscillator: IRNode = if alpha_is_zero {
            trig_term
        } else {
            let exp_arg: IRNode = if neg_alpha == Frac::one() {
                t.clone()
            } else if neg_alpha == Frac::new(-1, 1) {
                unary(NEG, t.clone())
            } else {
                binary(MUL, frac_to_ir(neg_alpha), t.clone())
            };
            binary(MUL, unary(EXP, exp_arg), trig_term)
        };

        if is_one(&coeff_ir) {
            oscillator
        } else {
            binary(MUL, coeff_ir, oscillator)
        }
    };

    let mut terms = Vec::new();

    // Term 1: A · exp(−αt) · cos(βt)
    if !big_a.is_zero() {
        terms.push(make_exp_trig(frac_to_ir(big_a), true));
    }

    // Term 2: (B − A·α) / β · exp(−αt) · sin(βt)
    let baa = f_sub(big_b, f_mul(big_a, alpha));
    if !baa.is_zero() {
        let coeff2: IRNode = match beta_rat {
            Some(br) => frac_to_ir(f_div(baa, br)),
            None => binary(DIV, frac_to_ir(baa), beta_ir.clone()),
        };
        terms.push(make_exp_trig(coeff2, false));
    }

    Some(terms)
}

// ─────────────────────────────────────────────────────────────────────────────
// Full partial-fraction decomposition engine
// ─────────────────────────────────────────────────────────────────────────────

fn inverse_pf(f_ir: &IRNode, s: &IRNode, t: &IRNode) -> Option<IRNode> {
    let (num_raw, den_raw) = ir_to_rational(f_ir, s)?;
    let mut num = poly_normalize(num_raw);
    let den = poly_normalize(den_raw);

    // ── Step 1: Improper fractions ─────────────────────────────────────────
    let mut poly_part = vec![Frac::zero()];
    if poly_degree(&num) >= poly_degree(&den) {
        let (q, r) = poly_divmod(&num, &den);
        poly_part = poly_normalize(q);
        num = poly_normalize(r);
    }

    // ── Step 2: Extract rational roots ────────────────────────────────────
    let roots = extract_all_rational_roots(&den);

    let mut q_rat: Poly = vec![Frac::one()];
    for &r in &roots {
        q_rat = poly_mul(&q_rat, &[f_neg(r), Frac::one()]);
    }
    q_rat = poly_normalize(q_rat);

    let (q_irred, rem_check) = poly_divmod(&den, &q_rat);
    if !poly_is_zero(&rem_check) {
        return None;
    }

    let irred_deg = poly_degree(&q_irred);
    if irred_deg > 2 {
        return None;
    }

    // ── Step 3: Polynomial-part terms ─────────────────────────────────────
    let mut ir_terms: Vec<IRNode> = Vec::new();

    for (deg, &coeff) in poly_part.iter().enumerate() {
        if coeff.is_zero() {
            continue;
        }
        if deg == 0 {
            let delta = unary(DIRAC_DELTA, t.clone());
            ir_terms.push(if coeff == Frac::one() {
                delta
            } else {
                binary(MUL, frac_to_ir(coeff), delta)
            });
        } else {
            return None; // DiracDelta derivatives not supported
        }
    }

    // ── Step 4: Rational pole terms ────────────────────────────────────────
    // Group roots by value and count multiplicity
    let mut distinct_roots: std::collections::BTreeMap<(i64, i64), (Frac, usize)> =
        std::collections::BTreeMap::new();
    for &r in &roots {
        let key = (r.n, r.d);
        distinct_roots.entry(key).and_modify(|(_, cnt)| *cnt += 1).or_insert((r, 1));
    }

    let mut residues_map: std::collections::BTreeMap<(i64, i64), Vec<Frac>> =
        std::collections::BTreeMap::new();

    for (&key, &(r, m)) in &distinct_roots {
        let linear_pow_m = poly_pow(&[f_neg(r), Frac::one()], m as i64);
        let (q_rat_no_rm, _) = poly_divmod(&q_rat, &linear_pow_m);
        let q_other = poly_normalize(poly_mul(&q_rat_no_rm, &q_irred));

        if m == 1 {
            let q_other_r = poly_eval(&q_other, r);
            if q_other_r.is_zero() {
                return None;
            }
            let res_a = f_div(poly_eval(&num, r), q_other_r);
            residues_map.insert(key, vec![res_a]);
            ir_terms.push(ilt_simple_pole(res_a, r, t));
        } else {
            let residues = compute_repeated_residues(&num, &den, r, m)?;
            residues_map.insert(key, residues.clone());
            for (k, &res_a) in residues.iter().enumerate() {
                if res_a.is_zero() {
                    continue;
                }
                let pole_order = m - k;
                ir_terms.push(if pole_order == 1 {
                    ilt_simple_pole(res_a, r, t)
                } else {
                    ilt_repeated_pole(res_a, r, pole_order, t)
                });
            }
        }
    }

    // ── Step 5: Irreducible quadratic term ─────────────────────────────────
    if irred_deg == 2 {
        let mut rat_poly: Poly = vec![Frac::zero()];
        for (&key, &(r, m)) in &distinct_roots {
            let residues = residues_map.get(&key).unwrap();
            for (k_idx, &res_a) in residues.iter().enumerate() {
                if res_a.is_zero() {
                    continue;
                }
                let pole_order = m - k_idx;
                let linear_pow_k = poly_pow(&[f_neg(r), Frac::one()], pole_order as i64);
                let (q_rat_div_k, _) = poly_divmod(&q_rat, &linear_pow_k);
                let contrib = poly_scale(&poly_mul(&q_irred, &q_rat_div_k), res_a);
                rat_poly = poly_add(&rat_poly, &contrib);
            }
        }

        let irred_times_qrat = poly_normalize(poly_add(&num, &poly_neg(&rat_poly)));
        let (lin_num, rem2) = poly_divmod(&irred_times_qrat, &q_rat);
        if !poly_is_zero(&rem2) {
            return None;
        }

        let irred_terms = ilt_irreducible_quad(&poly_normalize(lin_num), &poly_normalize(q_irred), t)?;
        ir_terms.extend(irred_terms);
    }

    // ── Step 6: Assemble ───────────────────────────────────────────────────
    if ir_terms.is_empty() {
        return Some(int(0));
    }
    if ir_terms.len() == 1 {
        return Some(ir_terms.remove(0));
    }
    let mut result = ir_terms.remove(0);
    for term in ir_terms {
        result = binary(ADD, result, term);
    }
    Some(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern matchers
// ─────────────────────────────────────────────────────────────────────────────

/// Match t^n · trig(ω·t) for n ≥ 2.
fn match_tn_times_trig(f: &IRNode, t: &IRNode) -> Option<(i64, &'static str, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (power_node, trig_node) in [(a, b), (b, a)] {
        let n = match_power_of_t(power_node, t)?;
        if n < 2 {
            continue;
        }
        if let Some(w) = match_unary_linear(trig_node, SIN, t) {
            return Some((n, SIN, w));
        }
        if let Some(w) = match_unary_linear(trig_node, COS, t) {
            return Some((n, COS, w));
        }
    }
    None
}

fn match_exp_times_trig(f: &IRNode, t: &IRNode) -> Option<(IRNode, &'static str, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (exp_node, trig_node) in [(a, b), (b, a)] {
        let shift = match_unary_linear(exp_node, EXP, t)?;
        if let Some(w) = match_unary_linear(trig_node, SIN, t) {
            return Some((shift, SIN, w));
        }
        if let Some(w) = match_unary_linear(trig_node, COS, t) {
            return Some((shift, COS, w));
        }
    }
    None
}

fn match_t_power_times_exp(f: &IRNode, t: &IRNode) -> Option<(i64, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (power_node, exp_node) in [(a, b), (b, a)] {
        let n = match_power_of_t(power_node, t)?;
        let shift = match_unary_linear(exp_node, EXP, t)?;
        return Some((n, shift));
    }
    None
}

fn match_t_times_trig(f: &IRNode, t: &IRNode) -> Option<(&'static str, IRNode)> {
    let (a, b) = binary_args(f, MUL)?;
    for (left, right) in [(a, b), (b, a)] {
        if !same(left, t) {
            continue;
        }
        if let Some(w) = match_unary_linear(right, SIN, t) {
            return Some((SIN, w));
        }
        if let Some(w) = match_unary_linear(right, COS, t) {
            return Some((COS, w));
        }
    }
    None
}

fn match_power_of_t(f: &IRNode, t: &IRNode) -> Option<i64> {
    if same(f, t) {
        return Some(1);
    }
    let (base, exp) = binary_args(f, POW)?;
    if same(base, t) {
        if let IRNode::Integer(n) = exp {
            if *n >= 1 {
                return Some(*n);
            }
        }
    }
    None
}

fn match_unary_linear(f: &IRNode, head: &str, t: &IRNode) -> Option<IRNode> {
    let args = apply_args(f, head)?;
    if args.len() == 1 {
        extract_linear_arg(&args[0], t)
    } else {
        None
    }
}

fn extract_linear_arg(arg: &IRNode, t: &IRNode) -> Option<IRNode> {
    if same(arg, t) {
        return Some(int(1));
    }
    if let Some((a, b)) = binary_args(arg, MUL) {
        if same(a, t) && is_constant(b, t) {
            return Some(b.clone());
        }
        if same(b, t) && is_constant(a, t) {
            return Some(a.clone());
        }
    }
    if let Some(args) = apply_args(arg, NEG) {
        if args.len() == 1 {
            return extract_linear_arg(&args[0], t).map(negate);
        }
    }
    None
}

fn extract_coeff_and_fn(node: &IRNode, t: &IRNode) -> Option<(IRNode, IRNode)> {
    let (a, b) = binary_args(node, MUL)?;
    if is_constant(a, t) {
        return Some((a.clone(), b.clone()));
    }
    if is_constant(b, t) {
        return Some((b.clone(), a.clone()));
    }
    None
}

fn is_constant(node: &IRNode, var: &IRNode) -> bool {
    if same(node, var) {
        return false;
    }
    match node {
        IRNode::Apply(apply_node) => apply_node.args.iter().all(|arg| is_constant(arg, var)),
        _ => true,
    }
}

fn match_s_minus_a(node: &IRNode, s: &IRNode) -> Option<IRNode> {
    let (left, right) = binary_args(node, SUB)?;
    if same(left, s) {
        Some(right.clone())
    } else {
        None
    }
}

fn match_pow_of(node: &IRNode, base: &IRNode) -> Option<i64> {
    let (pow_base, exp) = binary_args(node, POW)?;
    if same(pow_base, base) {
        if let IRNode::Integer(n) = exp {
            return Some(*n);
        }
    }
    None
}

fn match_s_sq_plus_param_sq(node: &IRNode, s: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(node, ADD)?;
    match_s_sq_param_sq(a, b, s).or_else(|| match_s_sq_param_sq(b, a, s))
}

fn match_s_sq_minus_param_sq(node: &IRNode, s: &IRNode) -> Option<IRNode> {
    let (a, b) = binary_args(node, SUB)?;
    match_s_sq_param_sq(a, b, s)
}

fn match_s_sq_param_sq(s_sq: &IRNode, param_sq: &IRNode, s: &IRNode) -> Option<IRNode> {
    if !matches!(match_pow_of(s_sq, s), Some(2)) {
        return None;
    }
    sqrt_param(param_sq)
}

fn sqrt_param(node: &IRNode) -> Option<IRNode> {
    if let Some((base, exp)) = binary_args(node, POW) {
        if is_int(exp, 2) {
            return Some(base.clone());
        }
    }
    if let IRNode::Integer(n) = node {
        if *n >= 0 {
            let root = (*n as f64).sqrt() as i64;
            if root * root == *n {
                return Some(int(root));
            }
        }
    }
    None
}

fn is_apply_of_var(node: &IRNode, head: &str, var: &IRNode) -> bool {
    matches!(apply_args(node, head), Some(args) if args.len() == 1 && same(&args[0], var))
}

fn sum_s_sq_param_sq(s: &IRNode, param: &IRNode) -> IRNode {
    binary(
        ADD,
        binary(POW, s.clone(), int(2)),
        binary(POW, param.clone(), int(2)),
    )
}

fn sub_s_sq_param_sq(s: &IRNode, param: &IRNode) -> IRNode {
    binary(
        SUB,
        binary(POW, s.clone(), int(2)),
        binary(POW, param.clone(), int(2)),
    )
}

fn apply_args<'a>(node: &'a IRNode, head: &str) -> Option<&'a [IRNode]> {
    match node {
        IRNode::Apply(apply_node) if apply_node.head == sym(head) => Some(&apply_node.args),
        _ => None,
    }
}

fn binary_args<'a>(node: &'a IRNode, head: &str) -> Option<(&'a IRNode, &'a IRNode)> {
    let args = apply_args(node, head)?;
    if args.len() == 2 {
        Some((&args[0], &args[1]))
    } else {
        None
    }
}

fn binary(head: &str, a: IRNode, b: IRNode) -> IRNode {
    apply(sym(head), vec![a, b])
}

fn unary(head: &str, arg: IRNode) -> IRNode {
    apply(sym(head), vec![arg])
}

fn negate(node: IRNode) -> IRNode {
    match node {
        IRNode::Integer(n) => int(-n),
        _ => unary(NEG, node),
    }
}

fn same(a: &IRNode, b: &IRNode) -> bool {
    a == b
}

fn is_int(node: &IRNode, value: i64) -> bool {
    matches!(node, IRNode::Integer(n) if *n == value)
}

fn is_one(node: &IRNode) -> bool {
    is_int(node, 1) || matches!(node, IRNode::Rational(n, d) if *n == *d)
}

fn factorial(n: i64) -> i64 {
    (1..=n).product()
}
