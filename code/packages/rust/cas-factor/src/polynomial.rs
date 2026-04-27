//! Polynomial helpers for the integer factorizer.
//!
//! We represent polynomials as coefficient lists:
//! `[a_0, a_1, ..., a_n]` represents `a_0 + a_1·x + … + a_n·x^n`.
//!
//! Trailing zeros are stripped by every public function that normalizes.
//! The **zero polynomial** is the empty slice / empty `Vec<i64>`.
//!
//! # Representation
//!
//! ```text
//! x^2 - 1  =  [-1, 0, 1]   (constant term first)
//! 2x + 3   =  [3, 2]
//! 6        =  [6]
//! 0        =  []
//! ```

// ---------------------------------------------------------------------------
// Type alias
// ---------------------------------------------------------------------------

/// A univariate integer polynomial as a coefficient vector.
///
/// Index `i` holds the coefficient of `x^i`. Trailing zeros must be stripped
/// before comparison; use [`normalize`] to ensure this.
pub type Poly = Vec<i64>;

// ---------------------------------------------------------------------------
// Normalization
// ---------------------------------------------------------------------------

/// Strip trailing zeros and return a new `Poly`.
///
/// ```
/// use cas_factor::polynomial::normalize;
/// assert_eq!(normalize(&[1, 2, 0, 0]), vec![1, 2]);
/// assert_eq!(normalize(&[0, 0]),       vec![]);
/// assert_eq!(normalize(&[]),           vec![]);
/// ```
pub fn normalize(p: &[i64]) -> Poly {
    let mut out = p.to_vec();
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// Degree of `p`, or `-1` for the zero polynomial.
///
/// ```
/// use cas_factor::polynomial::degree;
/// assert_eq!(degree(&[1, 2, 3]), 2);
/// assert_eq!(degree(&[5]),       0);
/// assert_eq!(degree(&[]),        -1);
/// ```
pub fn degree(p: &[i64]) -> i32 {
    let p = normalize(p);
    if p.is_empty() {
        -1
    } else {
        (p.len() - 1) as i32
    }
}

// ---------------------------------------------------------------------------
// Content and primitive part
// ---------------------------------------------------------------------------

/// Integer GCD of every coefficient (always non-negative).
///
/// The content of the zero polynomial is 0.  The content of a non-zero
/// polynomial is the GCD of the absolute values of its coefficients:
///
/// ```
/// use cas_factor::polynomial::content;
/// assert_eq!(content(&[2, 4, 6]),   2);
/// assert_eq!(content(&[-6, 4, 2]),  2);
/// assert_eq!(content(&[]),          0);
/// ```
pub fn content(p: &[i64]) -> i64 {
    let p = normalize(p);
    if p.is_empty() {
        return 0;
    }
    let mut g = 0i64;
    for &c in &p {
        g = gcd(g, c.abs());
    }
    g
}

/// Divide `p` by its content.  Returns `[]` for the zero polynomial.
///
/// ```
/// use cas_factor::polynomial::primitive_part;
/// assert_eq!(primitive_part(&[2, 4, 6]), vec![1, 2, 3]);
/// ```
pub fn primitive_part(p: &[i64]) -> Poly {
    let c = content(p);
    if c <= 1 {
        return p.to_vec();
    }
    p.iter().map(|&coef| coef / c).collect()
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluate `p(x)` at an integer `x` using Horner's method.
///
/// ```
/// use cas_factor::polynomial::evaluate;
/// // p(x) = 1 + 2x + 3x^2,  p(2) = 1 + 4 + 12 = 17
/// assert_eq!(evaluate(&[1, 2, 3], 2), 17);
/// assert_eq!(evaluate(&[5, 7, 9], 0), 5);
/// ```
pub fn evaluate(p: &[i64], x: i64) -> i64 {
    let p = normalize(p);
    let mut out = 0i64;
    for &c in p.iter().rev() {
        out = out.wrapping_mul(x).wrapping_add(c);
    }
    out
}

// ---------------------------------------------------------------------------
// Synthetic division
// ---------------------------------------------------------------------------

/// Divide `p(x)` by the linear factor `(x − root)` using synthetic division.
///
/// The caller is responsible for ensuring `root` is an actual root (i.e.,
/// `evaluate(p, root) == 0`).  If it is not, the result is the quotient with
/// a non-zero remainder that is silently discarded.
///
/// ```
/// use cas_factor::polynomial::divide_linear;
/// // (x^2 − 1) / (x − 1) = x + 1
/// assert_eq!(divide_linear(&[-1, 0, 1], 1), vec![1, 1]);  // [1, 1] = x + 1
/// // (x^2 − 1) / (x + 1) = x − 1  (root = −1, factor = (x − (−1)) = x + 1)
/// assert_eq!(divide_linear(&[-1, 0, 1], -1), vec![-1, 1]);
/// ```
pub fn divide_linear(p: &[i64], root: i64) -> Poly {
    let p = normalize(p);
    if p.is_empty() {
        return vec![];
    }
    let n = p.len();
    let mut quotient = vec![0i64; n - 1];
    let mut remainder = 0i64;
    for i in (0..n).rev() {
        remainder = remainder.wrapping_mul(root).wrapping_add(p[i]);
        if i > 0 {
            quotient[i - 1] = remainder;
        }
    }
    normalize(&quotient)
}

// ---------------------------------------------------------------------------
// Divisors
// ---------------------------------------------------------------------------

/// All positive integer divisors of `|n|`, sorted ascending.
///
/// Returns an empty slice for `n == 0`.
///
/// ```
/// use cas_factor::polynomial::divisors;
/// assert_eq!(divisors(12),  vec![1, 2, 3, 4, 6, 12]);
/// assert_eq!(divisors(-12), vec![1, 2, 3, 4, 6, 12]);
/// assert_eq!(divisors(0),   vec![]);
/// ```
pub fn divisors(n: i64) -> Vec<i64> {
    let n = n.abs();
    if n == 0 {
        return vec![];
    }
    let mut out: Vec<i64> = Vec::new();
    let mut i = 1i64;
    while i * i <= n {
        if n % i == 0 {
            out.push(i);
            if i != n / i {
                out.push(n / i);
            }
        }
        i += 1;
    }
    out.sort_unstable();
    out
}

// ---------------------------------------------------------------------------
// GCD utility (internal)
// ---------------------------------------------------------------------------

/// Iterative Euclidean GCD for non-negative integers.
pub(crate) fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
