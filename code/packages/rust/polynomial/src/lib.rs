//! # polynomial — coefficient-array polynomial arithmetic over `f64`.
//!
//! A **polynomial** is a mathematical expression involving a variable *x*
//! with a finite number of terms, each being a constant times a power of x:
//!
//! ```text
//! p(x) = aₙxⁿ + aₙ₋₁xⁿ⁻¹ + … + a₁x + a₀
//! ```
//!
//! ## Representation: Index = Degree
//!
//! We store polynomials as `Vec<f64>` (or `&[f64]` slices) where the **array
//! index equals the degree of that term's coefficient**:
//!
//! ```text
//! [3.0, 0.0, 2.0]   →   3 + 0·x + 2·x²   =   3 + 2x²
//! [1.0, 2.0, 3.0]   →   1 + 2x + 3x²
//! []                 →   the zero polynomial (no terms)
//! ```
//!
//! This "little-endian" (lowest degree first) layout has two advantages:
//! 1. Addition is trivially position-aligned: `result[i] = a[i] + b[i]`.
//! 2. Horner's method for evaluation reads naturally from high index to low.
//!
//! ## Why This Module Exists
//!
//! Polynomial arithmetic is the foundation of three important algorithms in the
//! coding-adventures stack:
//!
//! - **GF(2^8) (MA01)** — The Galois Field used by Reed-Solomon and AES is defined
//!   as a polynomial ring modulo an irreducible polynomial. Every GF(256) element
//!   *is* a polynomial over GF(2).
//! - **Reed-Solomon error correction (MA02)** — A codeword is a polynomial; encoding
//!   is polynomial multiplication; decoding uses the extended Euclidean GCD algorithm.
//! - **CRCs and checksums** — A CRC is the remainder of polynomial division over GF(2).
//!
//! ## Normalization
//!
//! All functions that return a polynomial call [`normalize`] on their output.
//! Normalization strips trailing near-zero coefficients, so `[1.0, 0.0, 0.0]`
//! and `[1.0]` both represent the constant polynomial `1`.
//!
//! We use a threshold of `f64::EPSILON * 1e6` for "near-zero" to handle floating-point
//! rounding errors that accumulate during division.

/// The threshold below which a floating-point coefficient is considered zero.
///
/// We use a relative epsilon (not just `f64::EPSILON`) because polynomial
/// division accumulates rounding errors. A threshold of `f64::EPSILON * 1e6`
/// (≈ 2.22e-10) catches coefficients that are "zero by rounding" while still
/// distinguishing genuinely small but non-zero coefficients.
const ZERO_THRESHOLD: f64 = f64::EPSILON * 1e6;

// =============================================================================
// Fundamentals
// =============================================================================

/// Remove trailing near-zero coefficients from a polynomial.
///
/// "Trailing" means high-degree: we walk from the highest index downward
/// until we find a coefficient with absolute value greater than the zero
/// threshold.
///
/// ## Why Normalize?
///
/// Without normalization, `[1.0, 0.0, 0.0]` and `[1.0]` would have different
/// `degree()` values (2 vs 0) even though they represent the same polynomial.
/// The stopping condition in polynomial long division would break:
/// `degree(remainder) >= degree(divisor)` would be true for a phantom remainder.
///
/// ## Examples
///
/// ```text
/// normalize([1.0, 0.0, 0.0])  →  [1.0]   (strip two trailing zeros)
/// normalize([0.0])             →  []      (zero poly becomes empty)
/// normalize([])                →  []      (already empty)
/// normalize([1.0, 2.0, 3.0])  →  [1.0, 2.0, 3.0]   (no change)
/// ```
pub fn normalize(poly: &[f64]) -> Vec<f64> {
    // Find the highest index that is NOT near-zero.
    let mut len = poly.len();
    // Walk backwards, shrinking len for each trailing near-zero coefficient.
    while len > 0 && poly[len - 1].abs() <= ZERO_THRESHOLD {
        len -= 1;
    }
    // Return a copy truncated to `len` (or empty if all coefficients were zero).
    poly[..len].to_vec()
}

/// Return the degree of a polynomial.
///
/// The degree is the index of the highest non-zero coefficient. For example:
///
/// ```text
/// degree([3.0, 0.0, 2.0])  =  2   (x² term is non-zero)
/// degree([7.0])             =  0   (constant polynomial)
/// degree([])                =  0   (zero polynomial — special case)
/// ```
///
/// ## The Zero Polynomial Convention
///
/// The zero polynomial has **no** non-zero terms, so mathematically it has no
/// degree. Many texts say "degree −∞" or "degree −1". We follow a pragmatic
/// convention: `degree([]) = 0`. This simplifies callers that just want a
/// natural number to represent size.
///
/// Note: In the internal [`divmod`] algorithm we check `normalized.is_empty()`
/// rather than comparing degrees, avoiding any ambiguity.
pub fn degree(poly: &[f64]) -> usize {
    let n = normalize(poly);
    // If empty (zero polynomial), return 0.
    // Otherwise the degree is the index of the last element.
    if n.is_empty() {
        0
    } else {
        n.len() - 1
    }
}

/// Return the zero polynomial, `[0.0]`.
///
/// Zero is the additive identity: `add(zero(), p) == p` for any polynomial `p`.
///
/// We return `vec![0.0]` rather than an empty vec to make the zero polynomial
/// explicit when printing or inspecting values. Both represent the same
/// mathematical object, but `[0.0]` is more readable.
pub fn zero() -> Vec<f64> {
    vec![0.0]
}

/// Return the multiplicative identity polynomial, `[1.0]`.
///
/// The constant polynomial 1 satisfies: `multiply(one(), p) == p` for all `p`.
pub fn one() -> Vec<f64> {
    vec![1.0]
}

// =============================================================================
// Addition and Subtraction
// =============================================================================

/// Add two polynomials term-by-term.
///
/// Addition is the simplest polynomial operation: add matching coefficients,
/// treating "missing" high-degree terms in the shorter polynomial as zero.
///
/// If `a` has degree `m` and `b` has degree `n`, the result has degree ≤ max(m, n).
///
/// ## Visual Example
///
/// ```text
///   [1.0, 2.0, 3.0]   =   1 + 2x + 3x²
/// + [4.0, 5.0]        =   4 + 5x
/// ──────────────────────────────────────
///   [5.0, 7.0, 3.0]   =   5 + 7x + 3x²
/// ```
///
/// Step through: `1+4=5`, `2+5=7`, `3+0=3`. The x² term in `a` had no partner
/// in `b`, so it carried through with coefficient 3.
pub fn add(a: &[f64], b: &[f64]) -> Vec<f64> {
    // Allocate result as long as the longer input.
    let len = a.len().max(b.len());
    let mut result = vec![0.0; len];

    for i in 0..len {
        // Use 0.0 for indices beyond the end of either slice.
        let ai = if i < a.len() { a[i] } else { 0.0 };
        let bi = if i < b.len() { b[i] } else { 0.0 };
        result[i] = ai + bi;
    }

    normalize(&result)
}

/// Subtract polynomial `b` from polynomial `a` term-by-term.
///
/// Equivalent to `add(a, negate(b))`, but implemented directly to avoid
/// allocating an intermediate negated copy.
///
/// ## Visual Example
///
/// ```text
///   [5.0, 7.0, 3.0]   =   5 + 7x + 3x²
/// - [1.0, 2.0, 3.0]   =   1 + 2x + 3x²
/// ──────────────────────────────────────
///   [4.0, 5.0, 0.0]   →   normalize   →   [4.0, 5.0]   =   4 + 5x
/// ```
///
/// The x² term cancels: `3x² - 3x² = 0`. `normalize` strips the trailing zero.
pub fn subtract(a: &[f64], b: &[f64]) -> Vec<f64> {
    let len = a.len().max(b.len());
    let mut result = vec![0.0; len];

    for i in 0..len {
        let ai = if i < a.len() { a[i] } else { 0.0 };
        let bi = if i < b.len() { b[i] } else { 0.0 };
        result[i] = ai - bi;
    }

    normalize(&result)
}

// =============================================================================
// Multiplication
// =============================================================================

/// Multiply two polynomials using polynomial convolution.
///
/// Each term `a[i]·xⁱ` of `a` multiplies each term `b[j]·xʲ` of `b`,
/// contributing `a[i]·b[j]` to the result coefficient at index `i + j`.
///
/// If `a` has degree `m` and `b` has degree `n`, the result has degree `m + n`.
/// The result array has length `a.len() + b.len() - 1`.
///
/// ## Visual Example
///
/// ```text
///   a = [1.0, 2.0]   =   1 + 2x
/// × b = [3.0, 4.0]   =   3 + 4x
/// ─────────────────────────────────────────────
/// result = [0.0, 0.0, 0.0]   (length = 2 + 2 - 1 = 3)
///
///   i=0, j=0: result[0] += 1·3 = 3   →  [3.0, 0.0, 0.0]
///   i=0, j=1: result[1] += 1·4 = 4   →  [3.0, 4.0, 0.0]
///   i=1, j=0: result[1] += 2·3 = 6   →  [3.0, 10.0, 0.0]
///   i=1, j=1: result[2] += 2·4 = 8   →  [3.0, 10.0, 8.0]
///
/// Result: [3.0, 10.0, 8.0]   =   3 + 10x + 8x²
///
/// Verify: (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²  ✓
/// ```
pub fn multiply(a: &[f64], b: &[f64]) -> Vec<f64> {
    // Multiplying by the zero polynomial yields zero.
    if a.is_empty() || b.is_empty() {
        return vec![];
    }

    // Result degree = deg(a) + deg(b), so result length = a.len() + b.len() - 1.
    let result_len = a.len() + b.len() - 1;
    let mut result = vec![0.0; result_len];

    for i in 0..a.len() {
        for j in 0..b.len() {
            // The product of the degree-i term of a and the degree-j term of b
            // contributes to the degree-(i+j) term of the result.
            result[i + j] += a[i] * b[j];
        }
    }

    normalize(&result)
}

// =============================================================================
// Division
// =============================================================================

/// Perform polynomial long division, returning `(quotient, remainder)`.
///
/// Given polynomials `dividend` and `divisor` (divisor ≠ zero), finds `q` and
/// `r` such that:
///
/// ```text
/// dividend = divisor × q + r    and    degree(r) < degree(divisor)
/// ```
///
/// The algorithm is the direct analog of grade-school long division, but for
/// polynomials:
/// 1. Find the leading term of the current remainder.
/// 2. Divide it by the leading term of the divisor to get the next quotient term.
/// 3. Subtract `(quotient term) × divisor` from the remainder.
/// 4. Repeat until `degree(remainder) < degree(divisor)`.
///
/// ## Detailed Example
///
/// Divide `5 + x + 3x² + 2x³` (array `[5, 1, 3, 2]`) by `2 + x` (array `[2, 1]`):
///
/// ```text
/// Step 1: remainder = [5, 1, 3, 2], deg=3. Divisor leading = 1x¹.
///         Quotient term: 2x³ / x = 2x² → q[2] = 2
///         Subtract 2x² × (2 + x) = 4x² + 2x³ = [0,0,4,2]:
///         [5, 1, 3-4, 2-2] = [5, 1, -1, 0] → normalize → [5, 1, -1]
///
/// Step 2: remainder = [5, 1, -1], deg=2.
///         Quotient term: -x² / x = -x → q[1] = -1
///         Subtract -x × (2 + x) = -2x - x² = [0, -2, -1]:
///         [5-0, 1-(-2), -1-(-1)] = [5, 3, 0] → [5, 3]
///
/// Step 3: remainder = [5, 3], deg=1.
///         Quotient term: 3x / x = 3 → q[0] = 3
///         Subtract 3 × (2 + x) = 6 + 3x = [6, 3]:
///         [5-6, 3-3] = [-1, 0] → [-1]
///
/// Step 4: degree([-1]) = 0 < 1 = degree(divisor). STOP.
///
/// Result: quotient = [3, -1, 2]  (3 - x + 2x²)
///         remainder = [-1]
///
/// Verify: (2+x)(3-x+2x²) + (-1) = 6 - 2x + 4x² + 3x - x² + 2x³ - 1
///                                 = 5 + x + 3x² + 2x³  ✓
/// ```
///
/// ## Panics
///
/// Panics if `divisor` is the zero polynomial (i.e., normalizes to empty or all-zero).
pub fn divmod(dividend: &[f64], divisor: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let nb = normalize(divisor);
    // Division by the zero polynomial is undefined.
    assert!(!nb.is_empty(), "polynomial division by zero");

    let na = normalize(dividend);

    // If dividend has lower degree than divisor, quotient is 0, remainder is dividend.
    if na.len() < nb.len() {
        return (vec![], na);
    }

    let deg_a = na.len() - 1; // degree of dividend
    let deg_b = nb.len() - 1; // degree of divisor

    // Work on a mutable copy of the remainder, starting as the dividend.
    let mut rem = na.clone();

    // The quotient has degree (deg_a - deg_b), so its array has that many + 1 elements.
    let mut quot = vec![0.0; deg_a - deg_b + 1];

    // Leading coefficient of the divisor — used to compute each quotient term.
    let lead_b = nb[deg_b];

    // Current "active" degree of the remainder. It shrinks by at least 1 each iteration.
    let mut deg_rem = deg_a;

    while deg_rem >= deg_b {
        // Leading coefficient of the current remainder.
        let lead_rem = rem[deg_rem];

        // The quotient term that cancels this leading term:
        //   coeff · x^power × (lead_b · x^deg_b) = lead_rem · x^deg_rem
        //   → coeff = lead_rem / lead_b,  power = deg_rem - deg_b
        let coeff = lead_rem / lead_b;
        let power = deg_rem - deg_b;

        // Record this quotient term.
        quot[power] = coeff;

        // Subtract coeff·x^power·divisor from the remainder.
        // This zeroes out rem[deg_rem] and adjusts lower-degree terms.
        for j in 0..=deg_b {
            rem[power + j] -= coeff * nb[j];
        }

        // The leading term of rem is now zero (or near-zero) by construction.
        // Walk deg_rem downward past any additional near-zero terms that appeared.
        // Using a signed intermediate to avoid usize underflow below zero.
        let mut signed_deg = deg_rem as isize - 1;
        while signed_deg >= 0 && rem[signed_deg as usize].abs() <= ZERO_THRESHOLD {
            signed_deg -= 1;
        }
        // If signed_deg < deg_b, the while condition will fail next iteration.
        // We use saturating cast: if signed_deg < 0, the loop exits anyway.
        if signed_deg < 0 {
            break;
        }
        deg_rem = signed_deg as usize;
    }

    (normalize(&quot), normalize(&rem))
}

/// Return the quotient of `dividend / divisor`.
///
/// This is `divmod(dividend, divisor).0`.
///
/// ## Panics
///
/// Panics if `divisor` is the zero polynomial.
pub fn divide(a: &[f64], b: &[f64]) -> Vec<f64> {
    divmod(a, b).0
}

/// Return the remainder of `dividend / divisor`.
///
/// Named `modulo` rather than `mod` because `mod` is a reserved keyword in Rust.
///
/// This is `divmod(dividend, divisor).1`.
///
/// In GF(2^8) construction, we reduce a high-degree polynomial modulo the
/// primitive polynomial using this operation.
///
/// ## Panics
///
/// Panics if `divisor` is the zero polynomial.
pub fn modulo(a: &[f64], b: &[f64]) -> Vec<f64> {
    divmod(a, b).1
}

// =============================================================================
// Evaluation
// =============================================================================

/// Evaluate a polynomial at `x` using Horner's method.
///
/// **Naive evaluation** of `a₀ + a₁x + a₂x² + … + aₙxⁿ` needs n multiplications
/// for the powers of x, plus n more for the coefficients, and n additions — O(n²) work.
///
/// **Horner's method** rewrites the polynomial in nested form:
///
/// ```text
/// a₀ + x(a₁ + x(a₂ + … + x·aₙ))
/// ```
///
/// This only needs n multiplications and n additions — O(n) work, and no
/// exponentiation at all.
///
/// ## Algorithm
///
/// Read coefficients from the HIGH-degree end down to the constant:
///
/// ```text
/// acc = 0
/// for i from n downto 0:
///     acc = acc * x + poly[i]
/// return acc
/// ```
///
/// ## Example
///
/// Evaluate `3 + 0x + 1·x²` (array `[3, 0, 1]`) at `x = 2`:
///
/// ```text
/// Start: acc = 0
/// i=2: acc = 0 * 2 + 1 = 1
/// i=1: acc = 1 * 2 + 0 = 2
/// i=0: acc = 2 * 2 + 3 = 7
///
/// Result: 7
/// Verify: 3 + 0*2 + 1*4 = 3 + 0 + 4 = 7  ✓
/// ```
pub fn evaluate(poly: &[f64], x: f64) -> f64 {
    let n = normalize(poly);
    // The zero polynomial evaluates to 0 everywhere.
    if n.is_empty() {
        return 0.0;
    }

    let mut acc = 0.0;
    // Iterate from the highest-degree coefficient down to the constant term.
    for i in (0..n.len()).rev() {
        acc = acc * x + n[i];
    }
    acc
}

// =============================================================================
// Greatest Common Divisor
// =============================================================================

/// Compute the greatest common divisor of two polynomials.
///
/// The GCD of two polynomials is the highest-degree polynomial that divides
/// both of them with zero remainder.
///
/// We use the **Euclidean algorithm**, which is identical to the integer version
/// but with polynomial `modulo` in place of integer `%`:
///
/// ```text
/// gcd(a, b):
///     while b ≠ zero polynomial:
///         a, b = b, a mod b
///     return normalize(a)
/// ```
///
/// ## Why It Works
///
/// The Euclidean algorithm works for any **Euclidean domain** — a ring where you
/// can do division with remainder. The integers ℤ are one example; the polynomial
/// ring `ℝ[x]` is another. In both cases, each remainder has strictly smaller
/// "size" (absolute value / degree), so the algorithm terminates.
///
/// ## Example
///
/// ```text
/// gcd([1.0, -3.0, 2.0], [1.0, -1.0])
///   = gcd(x² - 3x + 2, x - 1)
///   = gcd((x-1)(x-2), (x-1))
///
/// Round 1: remainder of (x²-3x+2) / (x-1) = 0  (x-1 divides exactly)
/// Round 2: a = [1,-1], b = [] (zero). Loop exits.
/// Result: normalize([1,-1]) = [1,-1]   i.e.  (x - 1)
/// ```
///
/// This is the monic GCD: the polynomial `x - 1`, which divides both inputs.
pub fn gcd(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut u = normalize(a);
    let mut v = normalize(b);

    while !v.is_empty() {
        // The Euclidean step: replace (u, v) with (v, u mod v).
        let r = modulo(&u, &v);
        u = v;
        v = r;
    }

    normalize(&u)
}
