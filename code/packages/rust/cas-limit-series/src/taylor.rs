//! Polynomial Taylor expansion.
//!
//! ## Algorithm
//!
//! For a polynomial `p(var)`, the Taylor series around `point` to order `n` is:
//!
//! ```text
//! p(var) = Σ_{k=0..n}  (1/k!) · p^(k)(point) · (var − point)^k
//! ```
//!
//! This is computed in three steps:
//!
//! 1. **IR → coefficient list**: Convert the input IR expression to a
//!    `Vec<Frac>` where index `i` holds the coefficient of `var^i`.
//!
//! 2. **Polynomial shift**: Rewrite the coefficient list in terms of
//!    `(var − point)` rather than `var`.  Each shifted coefficient is
//!    `(1/k!) · p^(k)(point)`, computed via the falling-factorial formula:
//!    ```text
//!    out[k] = (1/k!) · Σ_{i≥k}  i^{(k)} · a_i · point^{i-k}
//!    ```
//!    where `i^{(k)} = i·(i−1)·…·(i−k+1)` is the falling factorial.
//!
//! 3. **Truncation and back-conversion**: Truncate the shifted coefficients to
//!    `[0..order]` and rebuild the expression as
//!    `Add(c₀, c₁·(var-point), c₂·(var-point)², …)`.
//!
//! ## Inputs accepted
//!
//! Only polynomial expressions are supported:
//! - `Add`, `Sub`, `Neg`, `Mul` of polynomial subexpressions.
//! - `Pow(base, k)` where `k` is a non-negative `Integer` literal.
//! - `Div(num, den)` where `den` is a numeric literal.
//! - Integer, Rational, Float literals.
//! - The single variable symbol (`var`).
//!
//! Anything else (transcendental functions, other symbols) raises
//! [`PolynomialError`].
//!
//! ## Fraction arithmetic
//!
//! Coefficients are tracked as `Frac { numer: i128, denom: i128 }` — exact
//! rational arithmetic with 128-bit integers and GCD reduction.  This is an
//! internal type; all public outputs are `IRNode`.

use std::fmt;

use symbolic_ir::{apply, int, sym, IRNode, ADD, MUL, POW, SUB};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Raised when an IR expression cannot be interpreted as a polynomial in the
/// given variable.
///
/// Examples: transcendental functions (`Sin(x)`, `Exp(x)`), variables other
/// than the expansion variable, or structural anomalies (`Pow` with a
/// non-integer exponent).
#[derive(Debug, Clone, PartialEq)]
pub struct PolynomialError(pub String);

impl fmt::Display for PolynomialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PolynomialError: {}", self.0)
    }
}

impl std::error::Error for PolynomialError {}

// ---------------------------------------------------------------------------
// Internal exact-rational type
// ---------------------------------------------------------------------------

/// Exact rational number in reduced form.
///
/// `denom > 0` always; sign lives in `numer`.
/// Uses `i128` for overflow safety during coefficient arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Frac {
    numer: i128,
    denom: i128,
}

impl Frac {
    fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    fn one() -> Self {
        Self { numer: 1, denom: 1 }
    }

    fn new(n: i128, d: i128) -> Self {
        assert!(d != 0, "Frac: denominator must not be zero");
        if n == 0 {
            return Self::zero();
        }
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Self { numer: n / g, denom: d / g }
    }

    fn from_i64(v: i64) -> Self {
        Self { numer: v as i128, denom: 1 }
    }

    fn is_zero(&self) -> bool {
        self.numer == 0
    }

    fn is_one(&self) -> bool {
        self.numer == 1 && self.denom == 1
    }

    /// Convert to an `IRNode` — `Integer` when denom==1, else `Rational`.
    fn to_irnode(self) -> IRNode {
        if self.denom == 1 {
            int(self.numer as i64)
        } else {
            symbolic_ir::rat(self.numer as i64, self.denom as i64)
        }
    }

    /// Raise this fraction to an unsigned integer power.
    fn powi(self, exp: u32) -> Self {
        let mut result = Self::one();
        for _ in 0..exp {
            result = result * self;
        }
        result
    }
}

impl std::ops::Add for Frac {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Frac::new(self.numer * rhs.denom + rhs.numer * self.denom, self.denom * rhs.denom)
    }
}

impl std::ops::Sub for Frac {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + Frac { numer: -rhs.numer, denom: rhs.denom }
    }
}

impl std::ops::Mul for Frac {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Frac::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }
}

impl std::ops::Div for Frac {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        assert!(rhs.numer != 0, "Frac: division by zero");
        Frac::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Compute the factorial `n!` as `u128`.
///
/// Panics if `n > 20` (20! ≈ 2.4 × 10¹⁸ fits in u64; 21! overflows u64 but
/// fits in u128).  For realistic polynomial orders (≤ 20) this is fine.
fn factorial(n: usize) -> u128 {
    (1..=n as u128).product()
}

/// Compute the falling factorial `n^(k) = n·(n-1)·…·(n-k+1)` as `u128`.
///
/// Always a non-negative integer: all the consecutive integers from n down to
/// (n-k+1) are positive for `k ≤ n`.
fn falling_factorial(n: usize, k: usize) -> u128 {
    if k == 0 {
        return 1;
    }
    (n - k + 1..=n).map(|v| v as u128).product()
}

// ---------------------------------------------------------------------------
// IR → coefficient list
// ---------------------------------------------------------------------------

/// Convert a polynomial IR expression to coefficient list `[a₀, a₁, …, aₙ]`
/// where `a_i` is the coefficient of `var^i`.
///
/// Raises [`PolynomialError`] for any non-polynomial node.
fn to_coefficients(expr: &IRNode, var: &IRNode) -> Result<Vec<Frac>, PolynomialError> {
    match expr {
        IRNode::Integer(v) => Ok(vec![Frac::from_i64(*v)]),
        IRNode::Rational(n, d) => Ok(vec![Frac::new(*n as i128, *d as i128)]),
        IRNode::Float(v) => {
            // Convert float to rational approximation (lossy but stable).
            // Limit denominator to avoid huge numerators.
            let (n, d) = float_to_rational(*v, 1_000_000);
            Ok(vec![Frac::new(n, d)])
        }
        // Symbols
        _ if expr == var => {
            // The variable itself: coefficient list [0, 1] (i.e., 0·var^0 + 1·var^1)
            Ok(vec![Frac::zero(), Frac::one()])
        }
        IRNode::Symbol(s) => Err(PolynomialError(format!(
            "taylor: expression contains symbol {s:?} other than the expansion variable"
        ))),
        // Compound applications
        IRNode::Apply(a) => {
            let head = match &a.head {
                IRNode::Symbol(s) => s.as_str(),
                _ => {
                    return Err(PolynomialError(format!(
                        "taylor: unsupported compound head {:?}",
                        a.head
                    )))
                }
            };
            match head {
                "Add" => {
                    let mut result = vec![Frac::zero()];
                    for arg in &a.args {
                        let term = to_coefficients(arg, var)?;
                        result = coeffs_add(&result, &term);
                    }
                    Ok(result)
                }
                "Sub" => {
                    if a.args.len() != 2 {
                        return Err(PolynomialError("Sub must have exactly 2 args".into()));
                    }
                    let lhs = to_coefficients(&a.args[0], var)?;
                    let rhs = to_coefficients(&a.args[1], var)?;
                    Ok(coeffs_sub(&lhs, &rhs))
                }
                "Neg" => {
                    if a.args.len() != 1 {
                        return Err(PolynomialError("Neg must have exactly 1 arg".into()));
                    }
                    let inner = to_coefficients(&a.args[0], var)?;
                    Ok(inner.into_iter().map(|c| Frac::zero() - c).collect())
                }
                "Mul" => {
                    let mut result = vec![Frac::one()];
                    for arg in &a.args {
                        let term = to_coefficients(arg, var)?;
                        result = coeffs_mul(&result, &term);
                    }
                    Ok(result)
                }
                "Pow" => {
                    if a.args.len() != 2 {
                        return Err(PolynomialError("Pow must have exactly 2 args".into()));
                    }
                    let exp = match &a.args[1] {
                        IRNode::Integer(e) if *e >= 0 => *e as usize,
                        other => {
                            return Err(PolynomialError(format!(
                                "Pow exponent must be a non-negative integer literal, got {other:?}"
                            )))
                        }
                    };
                    let base_coeffs = to_coefficients(&a.args[0], var)?;
                    let mut result = vec![Frac::one()];
                    for _ in 0..exp {
                        result = coeffs_mul(&result, &base_coeffs);
                    }
                    Ok(result)
                }
                "Div" => {
                    if a.args.len() != 2 {
                        return Err(PolynomialError("Div must have exactly 2 args".into()));
                    }
                    let den_val = match &a.args[1] {
                        IRNode::Integer(v) => Frac::from_i64(*v),
                        IRNode::Rational(n, d) => Frac::new(*n as i128, *d as i128),
                        other => {
                            return Err(PolynomialError(format!(
                                "Div: denominator must be a numeric literal for polynomial Taylor, got {other:?}"
                            )))
                        }
                    };
                    let num_coeffs = to_coefficients(&a.args[0], var)?;
                    Ok(num_coeffs.into_iter().map(|c| c / den_val).collect())
                }
                other => Err(PolynomialError(format!(
                    "taylor: unsupported operation {other:?} for polynomial input"
                ))),
            }
        }
        other => Err(PolynomialError(format!(
            "taylor: unsupported expression {other:?}"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Polynomial arithmetic on coefficient lists
// ---------------------------------------------------------------------------

/// Add two coefficient lists (zero-padded).
fn coeffs_add(a: &[Frac], b: &[Frac]) -> Vec<Frac> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| {
            let ai = a.get(i).copied().unwrap_or(Frac::zero());
            let bi = b.get(i).copied().unwrap_or(Frac::zero());
            ai + bi
        })
        .collect()
}

/// Subtract two coefficient lists.
fn coeffs_sub(a: &[Frac], b: &[Frac]) -> Vec<Frac> {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| {
            let ai = a.get(i).copied().unwrap_or(Frac::zero());
            let bi = b.get(i).copied().unwrap_or(Frac::zero());
            ai - bi
        })
        .collect()
}

/// Multiply two coefficient lists (convolution).
fn coeffs_mul(a: &[Frac], b: &[Frac]) -> Vec<Frac> {
    if a.is_empty() || b.is_empty() {
        return vec![];
    }
    let mut out = vec![Frac::zero(); a.len() + b.len() - 1];
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            out[i + j] = out[i + j] + (*ai * *bj);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Polynomial shift
// ---------------------------------------------------------------------------

/// Rewrite `p(var)` as `q(var - shift)` by computing Taylor coefficients at `shift`.
///
/// The k-th output coefficient is `(1/k!) · p^(k)(shift)`, computed via:
/// ```text
/// out[k] = (1/k!) · Σ_{i≥k}  falling_factorial(i, k) · a_i · shift^{i-k}
/// ```
fn shift_polynomial(coeffs: &[Frac], shift: Frac) -> Vec<Frac> {
    let n = coeffs.len();
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let mut sub_total = Frac::zero();
        for i in k..n {
            let ff = falling_factorial(i, k) as i128;
            let power = shift.powi((i - k) as u32);
            sub_total = sub_total + Frac::new(ff, 1) * coeffs[i] * power;
        }
        let k_fact = factorial(k) as i128;
        out.push(sub_total / Frac::new(k_fact, 1));
    }
    out
}

// ---------------------------------------------------------------------------
// Coefficient list → IR expression
// ---------------------------------------------------------------------------

/// Rebuild `Σ c_k · (var - point)^k` as an IR tree.
///
/// - `k=0`: constant term → just `c_0`.
/// - `k=1`: linear term → `c·delta` (or `delta` if `c=1`).
/// - `k≥2`: higher term → `c·Pow(delta, k)` (or `Pow(delta, k)` if `c=1`).
///
/// where `delta = (var - point)` if `point ≠ 0`, else `var`.
///
/// Zero coefficients are skipped.  If no non-zero terms remain, returns `0`.
fn from_coefficients(coeffs: &[Frac], var: &IRNode, point: &IRNode) -> IRNode {
    let mut terms: Vec<IRNode> = Vec::new();

    for (k, &c) in coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let coef_node = c.to_irnode();
        if k == 0 {
            terms.push(coef_node);
            continue;
        }
        // (var - point)^k
        //
        // When the expansion point is 0, `var - 0 = var`, so we simplify
        // the delta to just `var` directly.
        let delta = if matches!(point, IRNode::Integer(0)) {
            var.clone()
        } else {
            apply(sym(SUB), vec![var.clone(), point.clone()])
        };
        let base = if k == 1 {
            delta
        } else {
            apply(sym(POW), vec![delta, int(k as i64)])
        };
        let term = if c.is_one() {
            base
        } else {
            apply(sym(MUL), vec![coef_node, base])
        };
        terms.push(term);
    }

    match terms.len() {
        0 => int(0),
        1 => terms.into_iter().next().unwrap(),
        _ => apply(sym(ADD), terms),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract a numeric literal as a `Frac`.  Raises for non-literals.
fn to_fraction(node: &IRNode) -> Result<Frac, PolynomialError> {
    match node {
        IRNode::Integer(v) => Ok(Frac::from_i64(*v)),
        IRNode::Rational(n, d) => Ok(Frac::new(*n as i128, *d as i128)),
        IRNode::Float(v) => {
            let (n, d) = float_to_rational(*v, 1_000_000);
            Ok(Frac::new(n, d))
        }
        other => Err(PolynomialError(format!(
            "taylor: expansion point must be a literal number, got {other:?}"
        ))),
    }
}

/// Truncated Taylor expansion of a polynomial `expr` in `var` around `point`.
///
/// The expansion is:
/// ```text
/// Σ_{k=0..order}  (1/k!) · p^(k)(point) · (var − point)^k
/// ```
///
/// The result is un-simplified IR.  Pass through `cas_simplify::simplify`
/// to reduce numeric entries.
///
/// Returns [`PolynomialError`] if `expr` contains non-polynomial subexpressions.
///
/// # Panics
///
/// Panics if `order > 20` (factorial overflow guard).
///
/// ```rust
/// use cas_limit_series::taylor_polynomial;
/// use symbolic_ir::{apply, int, sym, ADD, POW};
///
/// let x = sym("x");
///
/// // Taylor(x^2, x, 0, 2) = x^2
/// let expr = apply(sym(POW), vec![x.clone(), int(2)]);
/// let out = taylor_polynomial(&expr, &x, &int(0), 2).unwrap();
/// assert_eq!(out, apply(sym(POW), vec![x.clone(), int(2)]));
///
/// // Taylor(7, x, 2, 3) = 7
/// let out2 = taylor_polynomial(&int(7), &x, &int(2), 3).unwrap();
/// assert_eq!(out2, int(7));
/// ```
pub fn taylor_polynomial(
    expr: &IRNode,
    var: &IRNode,
    point: &IRNode,
    order: usize,
) -> Result<IRNode, PolynomialError> {
    // Convert expression to coefficient list.
    let coeffs_in_var = to_coefficients(expr, var)?;

    // Compute the expansion point as a fraction.
    let shift = to_fraction(point)?;

    // Shift the polynomial: rewrite coefficients in terms of (var - point).
    let coeffs_in_delta = shift_polynomial(&coeffs_in_var, shift);

    // Truncate to the requested order (inclusive).
    let truncated: Vec<Frac> = coeffs_in_delta.into_iter().take(order + 1).collect();

    // Convert back to IR.
    Ok(from_coefficients(&truncated, var, point))
}

// ---------------------------------------------------------------------------
// Float helper
// ---------------------------------------------------------------------------

/// Convert an `f64` to a rational approximation `(numer, denom)` by
/// continued-fraction approximation, limiting the denominator.
fn float_to_rational(v: f64, max_denom: i128) -> (i128, i128) {
    if v == 0.0 {
        return (0, 1);
    }
    let sign = if v < 0.0 { -1i128 } else { 1i128 };
    let v = v.abs();
    let mut best_n = v.round() as i128;
    let mut best_d = 1i128;
    let mut best_err = (v - best_n as f64 / best_d as f64).abs();
    for denominator in 1i128..=max_denom {
        let numerator = (v * denominator as f64).round() as i128;
        let err = (v - numerator as f64 / denominator as f64).abs();
        if err < best_err {
            best_err = err;
            best_n = numerator;
            best_d = denominator;
        }
        if best_err == 0.0 {
            break;
        }
    }
    (sign * best_n, best_d)
}
