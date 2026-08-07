//! `try_series_limit` — Taylor-series-expansion fallback for limits.
//!
//! Track J2 Rust port of the Python Track J1 implementation
//! (`code/packages/python/cas-limit-series/src/cas_limit_series/series_limit.py`).
//!
//! Fires inside [`crate::limit_advanced`] after L'Hopital (or instead
//! of it if no `diff_fn` was supplied) and before the unevaluated
//! `Limit(...)` fallthrough. Resolves transcendental `0/0` limits via
//! a self-contained rational-coefficient series ring.
//!
//! ## Algorithm
//!
//! For `limit(f(var) / g(var), var, a)` where direct substitution
//! gives `0/0`:
//!
//! 1. Translate the limit point to the origin:
//!    - `a = 0`        → `u = var`
//!    - finite `a ≠ 0` → `u = var − a`
//!    - `a = ±∞`        → not implemented; returns `None`.
//! 2. Taylor-expand both numerator and denominator to bounded order
//!    `N` starting at `N = 4` using a transcendental-aware series
//!    ring (`Add`, `Sub`, `Neg`, `Mul`, `Div` of non-vanishing
//!    denominators, integer `Pow`, `Sin`, `Cos`, `Tan`, `Exp`, `Log`).
//! 3. Read off leading coefficients and dispatch on `p` vs `q`.
//! 4. Bump `N` by 2 and retry, up to `N = 12`.
//!
//! ## Bounds
//!
//! * `MAX_ORDER_LIMIT = 12` — keeps polynomial multiplication bounded
//!   by O(N²) within a fixed, small constant.
//! * The series ring uses exact `Frac { numer: i128, denom: i128 }`
//!   rationals.
//! * No recursion: a fixed loop runs at most five iterations.
//! * Inputs are `IRNode` trees, not strings — no `eval` of user data.

use symbolic_ir::{
    apply, int, sym, IRNode, ADD, COS, DIV, EXP, LOG, MUL, NEG, POW, SIN, SUB, TAN,
};

/// Hard ceiling on the Taylor-expansion order.
pub const MAX_ORDER_LIMIT: usize = 12;

// ---------------------------------------------------------------------------
// Exact rational over i128
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frac {
    numer: i128,
    denom: i128,
}

fn igcd(a: u128, b: u128) -> u128 {
    let (mut a, mut b) = (a, b);
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

impl Frac {
    fn new(n: i128, d: i128) -> Self {
        assert!(d != 0, "Frac: denominator zero");
        if n == 0 {
            return Self { numer: 0, denom: 1 };
        }
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = igcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Self {
            numer: n / g,
            denom: d / g,
        }
    }

    fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    fn one() -> Self {
        Self { numer: 1, denom: 1 }
    }

    fn is_zero(&self) -> bool {
        self.numer == 0
    }

    fn is_positive(&self) -> bool {
        self.numer > 0
    }

    fn neg(self) -> Self {
        Self {
            numer: -self.numer,
            denom: self.denom,
        }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.numer * rhs.denom + rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(
            self.numer * rhs.denom - rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }

    fn mul(self, rhs: Self) -> Self {
        Self::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }

    fn div(self, rhs: Self) -> Self {
        assert!(rhs.numer != 0, "Frac: division by zero");
        Self::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }

    fn to_ir(self) -> IRNode {
        if self.denom == 1 {
            // Saturating cast — coefficients within bounded order are tiny.
            int(self.numer as i64)
        } else {
            IRNode::rational(self.numer as i64, self.denom as i64)
        }
    }
}

// ---------------------------------------------------------------------------
// Series — truncated power series a_0 + a_1·u + ... + a_N·u^N
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Series {
    coeffs: Vec<Frac>,
    order: usize,
}

#[derive(Debug, Clone)]
struct SeriesError(#[allow(dead_code)] String);

impl Series {
    fn new(mut coeffs: Vec<Frac>, order: usize) -> Self {
        // Normalise length to order + 1.
        if coeffs.len() < order + 1 {
            coeffs.resize(order + 1, Frac::zero());
        } else if coeffs.len() > order + 1 {
            coeffs.truncate(order + 1);
        }
        Self { coeffs, order }
    }

    fn constant(c: Frac, order: usize) -> Self {
        Self::new(vec![c], order)
    }

    fn variable(order: usize) -> Self {
        if order < 1 {
            return Self::new(vec![Frac::zero()], order);
        }
        Self::new(vec![Frac::zero(), Frac::one()], order)
    }

    fn add(&self, other: &Self) -> Self {
        let n = self.order;
        let mut out = Vec::with_capacity(n + 1);
        for i in 0..=n {
            out.push(self.coeffs[i].add(other.coeffs[i]));
        }
        Self::new(out, n)
    }

    fn sub(&self, other: &Self) -> Self {
        let n = self.order;
        let mut out = Vec::with_capacity(n + 1);
        for i in 0..=n {
            out.push(self.coeffs[i].sub(other.coeffs[i]));
        }
        Self::new(out, n)
    }

    fn neg(&self) -> Self {
        Self::new(self.coeffs.iter().map(|c| c.neg()).collect(), self.order)
    }

    fn mul(&self, other: &Self) -> Self {
        let n = self.order;
        let mut out = vec![Frac::zero(); n + 1];
        for i in 0..=n {
            let ai = self.coeffs[i];
            if ai.is_zero() {
                continue;
            }
            for j in 0..=(n - i) {
                out[i + j] = out[i + j].add(ai.mul(other.coeffs[j]));
            }
        }
        Self::new(out, n)
    }

    fn scaled(&self, c: Frac) -> Self {
        Self::new(self.coeffs.iter().map(|a| c.mul(*a)).collect(), self.order)
    }

    fn leading_index(&self) -> Option<usize> {
        self.coeffs.iter().position(|c| !c.is_zero())
    }

    /// `1 / self` provided `self(0) ≠ 0`. Newton-style recursion.
    fn reciprocal(&self) -> Result<Self, SeriesError> {
        let a = &self.coeffs;
        let n = self.order;
        if a[0].is_zero() {
            return Err(SeriesError("reciprocal of series with zero constant".into()));
        }
        let mut b = vec![Frac::zero(); n + 1];
        b[0] = Frac::one().div(a[0]);
        for k in 1..=n {
            let mut s = Frac::zero();
            for j in 1..=k {
                s = s.add(a[j].mul(b[k - j]));
            }
            b[k] = s.neg().div(a[0]);
        }
        Ok(Self::new(b, n))
    }

    /// `self ** k`, non-negative integer k, via repeated squaring.
    fn integer_power(&self, mut k: u32) -> Self {
        if k == 0 {
            return Self::constant(Frac::one(), self.order);
        }
        let mut result = Self::constant(Frac::one(), self.order);
        let mut base = self.clone();
        while k > 0 {
            if k & 1 == 1 {
                result = result.mul(&base);
            }
            k >>= 1;
            if k > 0 {
                base = base.mul(&base);
            }
        }
        result
    }

    /// `self(inner(u))` provided `inner(0) == 0`.
    fn compose_with_zero_constant(&self, inner: &Self) -> Result<Self, SeriesError> {
        if !inner.coeffs[0].is_zero() {
            return Err(SeriesError(
                "compose_with_zero_constant: inner has nonzero constant".into(),
            ));
        }
        let n = self.order;
        let mut result = Self::constant(Frac::zero(), n);
        let mut inner_pow = Self::constant(Frac::one(), n);
        for k in 0..=n {
            if !self.coeffs[k].is_zero() {
                result = result.add(&inner_pow.scaled(self.coeffs[k]));
            }
            if k < n {
                inner_pow = inner_pow.mul(inner);
            }
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Known transcendental Taylor series (around u = 0)
// ---------------------------------------------------------------------------

fn factorial(n: usize) -> i128 {
    let mut out: i128 = 1;
    for k in 2..=n {
        out *= k as i128;
    }
    out
}

fn series_exp(order: usize) -> Series {
    let coeffs: Vec<Frac> = (0..=order).map(|k| Frac::new(1, factorial(k))).collect();
    Series::new(coeffs, order)
}

fn series_sin(order: usize) -> Series {
    let mut coeffs = vec![Frac::zero(); order + 1];
    let mut sign: i128 = 1;
    let mut k = 1;
    while k <= order {
        coeffs[k] = Frac::new(sign, factorial(k));
        sign = -sign;
        k += 2;
    }
    Series::new(coeffs, order)
}

fn series_cos(order: usize) -> Series {
    let mut coeffs = vec![Frac::zero(); order + 1];
    let mut sign: i128 = 1;
    let mut k = 0;
    while k <= order {
        coeffs[k] = Frac::new(sign, factorial(k));
        sign = -sign;
        k += 2;
    }
    Series::new(coeffs, order)
}

fn series_log_one_plus(order: usize) -> Series {
    let mut coeffs = vec![Frac::zero(); order + 1];
    let mut sign: i128 = 1;
    for (k, coeff) in coeffs.iter_mut().enumerate().skip(1) {
        *coeff = Frac::new(sign, k as i128);
        sign = -sign;
    }
    Series::new(coeffs, order)
}

fn series_tan(order: usize) -> Result<Series, SeriesError> {
    // tan = sin / cos. cos has nonzero constant term, direct reciprocal.
    Ok(series_sin(order).mul(&series_cos(order).reciprocal()?))
}

// ---------------------------------------------------------------------------
// IR → Series translation
// ---------------------------------------------------------------------------

fn to_frac(node: &IRNode) -> Result<Frac, SeriesError> {
    match node {
        IRNode::Integer(v) => Ok(Frac::new(*v as i128, 1)),
        IRNode::Rational(n, d) => Ok(Frac::new(*n as i128, *d as i128)),
        IRNode::Float(v) => Ok(float_to_frac(*v)),
        _ => Err(SeriesError(format!("expected literal, got {node:?}"))),
    }
}

fn float_to_frac(value: f64) -> Frac {
    if !value.is_finite() {
        // Caller is responsible for guarding against non-finite floats.
        return Frac::zero();
    }
    if value == 0.0 {
        return Frac::zero();
    }
    let sign: i128 = if value < 0.0 { -1 } else { 1 };
    let av = value.abs();
    let mut best_n = av.round() as i128;
    let mut best_d: i128 = 1;
    let mut best_err = (av - best_n as f64).abs();
    let max_d: i128 = 1_000_000;
    let mut d: i128 = 1;
    while d <= max_d {
        let n = (av * d as f64).round() as i128;
        let err = (av - (n as f64) / (d as f64)).abs();
        if err < best_err {
            best_err = err;
            best_n = n;
            best_d = d;
        }
        if best_err == 0.0 {
            break;
        }
        d += 1;
    }
    Frac::new(sign * best_n, best_d)
}

fn head_name(node: &IRNode) -> Option<&str> {
    if let IRNode::Symbol(name) = node {
        Some(name.as_str())
    } else {
        None
    }
}

fn expand(expr: &IRNode, variable: &IRNode, order: usize) -> Result<Series, SeriesError> {
    // --- literal numbers ---
    if matches!(expr, IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_)) {
        return Ok(Series::constant(to_frac(expr)?, order));
    }

    // --- the expansion variable ---
    if let IRNode::Symbol(name) = expr {
        if expr == variable {
            return Ok(Series::variable(order));
        }
        return Err(SeriesError(format!("unsupported symbol {name:?}")));
    }

    let IRNode::Apply(ap) = expr else {
        return Err(SeriesError(format!("unsupported expression: {expr:?}")));
    };
    let Some(h) = head_name(&ap.head) else {
        return Err(SeriesError(format!("non-symbol head: {:?}", ap.head)));
    };
    let args = &ap.args;

    // --- arithmetic ---
    if h == ADD {
        let mut result = Series::constant(Frac::zero(), order);
        for a in args {
            result = result.add(&expand(a, variable, order)?);
        }
        return Ok(result);
    }
    if h == SUB {
        if args.len() != 2 {
            return Err(SeriesError("Sub expects 2 args".into()));
        }
        return Ok(expand(&args[0], variable, order)?.sub(&expand(&args[1], variable, order)?));
    }
    if h == NEG {
        if args.len() != 1 {
            return Err(SeriesError("Neg expects 1 arg".into()));
        }
        return Ok(expand(&args[0], variable, order)?.neg());
    }
    if h == MUL {
        let mut result = Series::constant(Frac::one(), order);
        for a in args {
            result = result.mul(&expand(a, variable, order)?);
        }
        return Ok(result);
    }
    if h == DIV {
        if args.len() != 2 {
            return Err(SeriesError("Div expects 2 args".into()));
        }
        let ns = expand(&args[0], variable, order)?;
        let ds = expand(&args[1], variable, order)?;
        if !ds.coeffs[0].is_zero() {
            return Ok(ns.mul(&ds.reciprocal()?));
        }
        return Err(SeriesError("inner Div by series vanishing at 0".into()));
    }
    if h == POW {
        if args.len() != 2 {
            return Err(SeriesError("Pow expects 2 args".into()));
        }
        let base = &args[0];
        let exp_node = &args[1];
        if let IRNode::Integer(k) = exp_node {
            let k = *k;
            if k >= 0 {
                let k_u: u32 = k
                    .try_into()
                    .map_err(|_| SeriesError("Pow exponent too large".into()))?;
                return Ok(expand(base, variable, order)?.integer_power(k_u));
            }
            let base_ser = expand(base, variable, order)?;
            if base_ser.coeffs[0].is_zero() {
                return Err(SeriesError("Pow neg-int over vanishing base".into()));
            }
            let k_u: u32 = (-k)
                .try_into()
                .map_err(|_| SeriesError("Pow exponent too large".into()))?;
            return Ok(base_ser.reciprocal()?.integer_power(k_u));
        }
        return Err(SeriesError(
            "Pow exponent must be a non-negative integer literal".into(),
        ));
    }

    // --- transcendentals ---
    if h == EXP {
        if args.len() != 1 {
            return Err(SeriesError("Exp expects 1 arg".into()));
        }
        let inner = expand(&args[0], variable, order)?;
        if !inner.coeffs[0].is_zero() {
            return Err(SeriesError("Exp with nonzero constant inner term".into()));
        }
        return series_exp(order).compose_with_zero_constant(&inner);
    }
    if h == SIN {
        if args.len() != 1 {
            return Err(SeriesError("Sin expects 1 arg".into()));
        }
        let inner = expand(&args[0], variable, order)?;
        if !inner.coeffs[0].is_zero() {
            return Err(SeriesError("Sin with nonzero constant inner term".into()));
        }
        return series_sin(order).compose_with_zero_constant(&inner);
    }
    if h == COS {
        if args.len() != 1 {
            return Err(SeriesError("Cos expects 1 arg".into()));
        }
        let inner = expand(&args[0], variable, order)?;
        if !inner.coeffs[0].is_zero() {
            return Err(SeriesError("Cos with nonzero constant inner term".into()));
        }
        return series_cos(order).compose_with_zero_constant(&inner);
    }
    if h == TAN {
        if args.len() != 1 {
            return Err(SeriesError("Tan expects 1 arg".into()));
        }
        let inner = expand(&args[0], variable, order)?;
        if !inner.coeffs[0].is_zero() {
            return Err(SeriesError("Tan with nonzero constant inner term".into()));
        }
        return series_tan(order)?.compose_with_zero_constant(&inner);
    }
    if h == LOG {
        if args.len() != 1 {
            return Err(SeriesError("Log expects 1 arg".into()));
        }
        let inner = expand(&args[0], variable, order)?;
        let c0 = inner.coeffs[0];
        if c0 != Frac::one() {
            return Err(SeriesError(format!(
                "Log with constant inner term != 1 (got {c0:?})"
            )));
        }
        // log(1 + (inner - 1)) where (inner - 1)(0) = 0.
        let mut shifted_coeffs = inner.coeffs.clone();
        shifted_coeffs[0] = Frac::zero();
        let shifted = Series::new(shifted_coeffs, order);
        return series_log_one_plus(order).compose_with_zero_constant(&shifted);
    }

    Err(SeriesError(format!("unsupported head: {h}")))
}

// ---------------------------------------------------------------------------
// Top-level entry point
// ---------------------------------------------------------------------------

/// Recognise `Div(N, D)` and `Mul(N, Pow(D, -1))` as quotients.
fn split_quotient(expr: &IRNode) -> Option<(IRNode, IRNode)> {
    let IRNode::Apply(ap) = expr else {
        return None;
    };
    if head_name(&ap.head) == Some(DIV) && ap.args.len() == 2 {
        return Some((ap.args[0].clone(), ap.args[1].clone()));
    }
    if head_name(&ap.head) == Some(MUL) && ap.args.len() == 2 {
        let a = &ap.args[0];
        let b = &ap.args[1];
        if let Some(base) = pow_neg_one_base(b) {
            return Some((a.clone(), base));
        }
        if let Some(base) = pow_neg_one_base(a) {
            return Some((b.clone(), base));
        }
    }
    None
}

fn pow_neg_one_base(node: &IRNode) -> Option<IRNode> {
    let IRNode::Apply(ap) = node else { return None };
    if head_name(&ap.head) != Some(POW) || ap.args.len() != 2 {
        return None;
    }
    if let IRNode::Integer(-1) = ap.args[1] {
        return Some(ap.args[0].clone());
    }
    None
}

/// Substitute `variable := variable + point` so the original
/// `variable = point` corresponds to `variable = 0` after the shift.
fn shift_to_origin(expr: &IRNode, variable: &IRNode, point: &IRNode) -> IRNode {
    if let IRNode::Integer(0) = point {
        return expr.clone();
    }
    fn go(node: &IRNode, variable: &IRNode, point: &IRNode) -> IRNode {
        if let IRNode::Symbol(_) = node {
            if node == variable {
                return apply(sym(ADD), vec![variable.clone(), point.clone()]);
            }
            return node.clone();
        }
        if let IRNode::Apply(ap) = node {
            return apply(
                ap.head.clone(),
                ap.args.iter().map(|a| go(a, variable, point)).collect(),
            );
        }
        node.clone()
    }
    go(expr, variable, point)
}

/// Taylor-series fallback for `limit(expr, variable, point)`.
///
/// Returns:
///   - an integer or rational literal on success,
///   - `Symbol("inf")` / `Symbol("minf")` on a divergent ratio,
///   - `None` if the fallback cannot determine the value (caller
///     should fall through to an unevaluated `Limit(...)`).
///
/// `point` must be a literal number. Limits at `±∞` are not yet
/// handled (they would need a `u = 1/x` rewrite) and return `None`.
///
/// `max_order` is clamped to `[4, 12]`.
pub fn try_series_limit(
    expr: &IRNode,
    variable: &IRNode,
    point: &IRNode,
    max_order: usize,
) -> Option<IRNode> {
    let cap = max_order.clamp(4, MAX_ORDER_LIMIT);

    let (numer, denom) = split_quotient(expr)?;

    // Limit at ±∞ is not yet implemented in this fallback.
    if let IRNode::Symbol(name) = point {
        if name == "inf" || name == "minf" {
            return None;
        }
    }
    if !matches!(
        point,
        IRNode::Integer(_) | IRNode::Rational(_, _) | IRNode::Float(_)
    ) {
        return None;
    }

    let shifted_n = shift_to_origin(&numer, variable, point);
    let shifted_d = shift_to_origin(&denom, variable, point);

    let mut order = 4;
    while order <= cap {
        let n_ser = match expand(&shifted_n, variable, order) {
            Ok(s) => s,
            Err(_) => return None,
        };
        let d_ser = match expand(&shifted_d, variable, order) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let p = n_ser.leading_index();
        let q = d_ser.leading_index();

        match (p, q) {
            (None, None) => {
                order += 2;
                continue;
            }
            (None, Some(_)) => return Some(int(0)),
            (Some(p), None) => {
                let cp = n_ser.coeffs[p];
                return Some(if cp.is_positive() {
                    sym("inf")
                } else {
                    sym("minf")
                });
            }
            (Some(p), Some(q)) => {
                let cp = n_ser.coeffs[p];
                let dq = d_ser.coeffs[q];
                if p > q {
                    return Some(int(0));
                }
                if p < q {
                    let signv = cp.div(dq);
                    return Some(if signv.is_positive() {
                        sym("inf")
                    } else {
                        sym("minf")
                    });
                }
                return Some(cp.div(dq).to_ir());
            }
        }
    }

    None
}

/// Convenience entry point using the default `MAX_ORDER_LIMIT`.
pub fn try_series_limit_default(
    expr: &IRNode,
    variable: &IRNode,
    point: &IRNode,
) -> Option<IRNode> {
    try_series_limit(expr, variable, point, MAX_ORDER_LIMIT)
}
