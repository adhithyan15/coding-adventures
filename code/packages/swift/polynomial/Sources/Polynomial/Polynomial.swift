// Polynomial.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Polynomial Arithmetic Over Real Numbers
// ============================================================================
//
// A polynomial is a mathematical expression of the form:
//
//   p(x) = a₀ + a₁·x + a₂·x² + ... + aₙ·xⁿ
//
// where each aᵢ is a coefficient (a real number), and n is the degree.
//
// ============================================================================
// Representation
// ============================================================================
//
// We represent a polynomial as a Swift [Double] array in "ascending-degree"
// (little-endian) order: index i holds the coefficient of x^i.
//
//   [3.0, 0.0, 2.0]  →  3 + 0·x + 2·x²  =  3 + 2x²
//   [1.0, 2.0, 3.0]  →  1 + 2x + 3x²
//   [0.0]            →  the zero polynomial (constant zero)
//
// Why little-endian? Because:
//   - Addition is trivially index-aligned (no reversal needed)
//   - Horner's method reads naturally from the last element backward
//   - Polynomial long division is easier to express
//
// ============================================================================
// Normalization
// ============================================================================
//
// A polynomial is "normalized" when it has no trailing zero coefficients.
// For example, [1.0, 0.0, 0.0] is the constant polynomial 1, which in
// normalized form is just [1.0]. The degree-2 and degree-1 terms have zero
// coefficients and carry no information.
//
// We represent the zero polynomial as [0.0] (never empty), because:
//   - An empty array causes index-out-of-bounds issues in degree computation
//   - [0.0] has a clear, unambiguous meaning
//   - degree([0.0]) = 0, which simplifies edge cases
//
// ============================================================================
// This File vs. TypeScript Reference
// ============================================================================
//
// Our canonical reference implementation lives at:
//   code/packages/typescript/polynomial/src/index.ts
//
// The TypeScript version uses [] for the zero polynomial; we use [0.0].
// All the mathematical algorithms are identical.
//
// ============================================================================

// ============================================================================
// MARK: - Public Namespace
// ============================================================================
//
// We wrap everything in a `public enum Polynomial` (used as a namespace).
// Swift enums with no cases cannot be instantiated — they serve as pure
// namespaces, equivalent to a module or static class in other languages.
// This avoids name conflicts with the built-in Double operators.

/// Polynomial arithmetic over real numbers (Double).
///
/// A polynomial is represented as `[Double]` in ascending-degree order:
/// index `i` holds the coefficient of `x^i`.
///
/// The zero polynomial is `[0.0]` — never an empty array.
///
/// All returned polynomials are normalized (no trailing near-zero coefficients).
public enum Polynomial {

    // ========================================================================
    // MARK: - Near-Zero Threshold
    // ========================================================================
    //
    // Floating-point arithmetic accumulates rounding errors. For example,
    // computing (1/3) * 3 in Double does not yield exactly 1.0. When we
    // perform polynomial division with non-integer coefficients, we may get
    // coefficients like 1.0e-16 that are mathematically zero but are not
    // exactly zero in IEEE 754 Double.
    //
    // We declare a coefficient "near-zero" if its absolute value is below
    // this threshold and strip it during normalization. This keeps our
    // polynomials tidy without losing meaningful precision.

    /// Coefficients with absolute value below this threshold are treated as zero.
    public static let zeroThreshold: Double = 1e-10

    // ========================================================================
    // MARK: - Fundamentals
    // ========================================================================

    /// Normalize a polynomial by stripping trailing near-zero coefficients.
    ///
    /// Trailing zeros represent zero-coefficient high-degree terms. They do
    /// not change the polynomial's value but do affect degree comparisons and
    /// the stopping condition in polynomial long division.
    ///
    /// The result is NEVER empty — the zero polynomial is returned as `[0.0]`.
    ///
    /// Examples:
    ///   normalize([1.0, 0.0, 0.0]) → [1.0]   (constant polynomial 1)
    ///   normalize([0.0])           → [0.0]   (zero polynomial — stays [0.0])
    ///   normalize([1.0, 2.0, 3.0]) → [1.0, 2.0, 3.0]  (already normalized)
    ///   normalize([])              → [0.0]   (empty input → zero polynomial)
    ///
    /// - Parameter poly: The polynomial to normalize (may have trailing zeros).
    /// - Returns: The normalized polynomial; always at least `[0.0]`.
    public static func normalize(_ poly: [Double]) -> [Double] {
        var len = poly.count
        // Walk backwards until we find a coefficient above the threshold.
        while len > 0 && abs(poly[len - 1]) < zeroThreshold {
            len -= 1
        }
        // If all coefficients were near-zero (or input was empty), return
        // the zero polynomial [0.0].
        if len == 0 {
            return [0.0]
        }
        return Array(poly.prefix(len))
    }

    /// Return the degree of a polynomial.
    ///
    /// The degree is the index of the highest non-zero coefficient.
    /// - The zero polynomial `[0.0]` has degree 0 (constant term only).
    ///
    /// Examples:
    ///   degree([3.0, 0.0, 2.0]) → 2   (highest non-zero: index 2, the x² term)
    ///   degree([7.0])           → 0   (constant polynomial; degree 0)
    ///   degree([0.0])           → 0   (zero polynomial; degree 0)
    ///
    /// - Parameter poly: The polynomial (need not be normalized).
    /// - Returns: The degree as a non-negative integer.
    public static func degree(_ poly: [Double]) -> Int {
        let n = normalize(poly)
        // After normalize, n is at least [0.0], so n.count >= 1.
        // The degree is the index of the last (highest) element.
        return n.count - 1
    }

    /// Return the zero polynomial `[0.0]`.
    ///
    /// Zero is the additive identity: `add(zero(), p) == p` for any `p`.
    ///
    /// - Returns: The zero polynomial as `[0.0]`.
    public static func zero() -> [Double] {
        return [0.0]
    }

    /// Return the multiplicative identity polynomial `[1.0]`.
    ///
    /// Multiplying any polynomial by `one()` leaves it unchanged.
    ///
    /// - Returns: The unit polynomial as `[1.0]`.
    public static func one() -> [Double] {
        return [1.0]
    }

    // ========================================================================
    // MARK: - Addition and Subtraction
    // ========================================================================

    /// Add two polynomials term-by-term.
    ///
    /// Addition is the simplest polynomial operation: we add matching
    /// coefficients, treating missing coefficients as zero.
    ///
    /// Visual example (ascending degree, so index 0 = constant term):
    ///
    ///   [1.0, 2.0, 3.0]  =  1 + 2x + 3x²
    /// + [4.0, 5.0]       =  4 + 5x
    /// ─────────────────────────────────
    ///   [5.0, 7.0, 3.0]  =  5 + 7x + 3x²
    ///
    /// The degree-2 term of `b` is implicitly zero, so 3x² carries through.
    ///
    /// - Parameters:
    ///   - a: First polynomial.
    ///   - b: Second polynomial.
    /// - Returns: The sum `a + b`, normalized.
    public static func add(_ a: [Double], _ b: [Double]) -> [Double] {
        let na = normalize(a)
        let nb = normalize(b)
        let len = max(na.count, nb.count)
        var result = [Double](repeating: 0.0, count: len)
        for i in 0..<len {
            let ai = i < na.count ? na[i] : 0.0
            let bi = i < nb.count ? nb[i] : 0.0
            result[i] = ai + bi
        }
        return normalize(result)
    }

    /// Subtract polynomial `b` from polynomial `a` term-by-term.
    ///
    /// This is equivalent to `add(a, negate(b))` but avoids creating an
    /// intermediate negated polynomial.
    ///
    /// Visual example:
    ///
    ///   [5.0, 7.0, 3.0]  =  5 + 7x + 3x²
    /// - [1.0, 2.0, 3.0]  =  1 + 2x + 3x²
    /// ─────────────────────────────────
    ///   [4.0, 5.0, 0.0]  →  normalize  →  [4.0, 5.0]  =  4 + 5x
    ///
    /// Note: 3x² − 3x² = 0; normalize strips the trailing zero.
    ///
    /// - Parameters:
    ///   - a: The minuend.
    ///   - b: The subtrahend.
    /// - Returns: The difference `a - b`, normalized.
    public static func subtract(_ a: [Double], _ b: [Double]) -> [Double] {
        let na = normalize(a)
        let nb = normalize(b)
        let len = max(na.count, nb.count)
        var result = [Double](repeating: 0.0, count: len)
        for i in 0..<len {
            let ai = i < na.count ? na[i] : 0.0
            let bi = i < nb.count ? nb[i] : 0.0
            result[i] = ai - bi
        }
        return normalize(result)
    }

    // ========================================================================
    // MARK: - Multiplication
    // ========================================================================

    /// Multiply two polynomials using polynomial convolution.
    ///
    /// Each term `a[i]·xⁱ` of `a` multiplies each term `b[j]·xʲ` of `b`,
    /// contributing `a[i]·b[j]` to the coefficient at index `i+j` of the result.
    ///
    /// If `a` has degree `m` and `b` has degree `n`, the result has degree `m+n`.
    ///
    /// Visual example:
    ///
    ///   [1.0, 2.0]  =  1 + 2x
    /// × [3.0, 4.0]  =  3 + 4x
    /// ──────────────────────────────────
    /// result initialized to [0, 0, 0]:
    ///   i=0, j=0: result[0] += 1·3 = 3   → [3, 0, 0]
    ///   i=0, j=1: result[1] += 1·4 = 4   → [3, 4, 0]
    ///   i=1, j=0: result[1] += 2·3 = 6   → [3, 10, 0]
    ///   i=1, j=1: result[2] += 2·4 = 8   → [3, 10, 8]
    ///
    /// Result: [3.0, 10.0, 8.0]  =  3 + 10x + 8x²
    /// Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
    ///
    /// - Parameters:
    ///   - a: First polynomial.
    ///   - b: Second polynomial.
    /// - Returns: The product `a × b`, normalized.
    public static func multiply(_ a: [Double], _ b: [Double]) -> [Double] {
        let na = normalize(a)
        let nb = normalize(b)

        // Multiplying by zero yields zero.
        let aIsZero = na.count == 1 && abs(na[0]) < zeroThreshold
        let bIsZero = nb.count == 1 && abs(nb[0]) < zeroThreshold
        if aIsZero || bIsZero {
            return [0.0]
        }

        // Result degree = deg(a) + deg(b), so length = a.count + b.count - 1.
        let resultLen = na.count + nb.count - 1
        var result = [Double](repeating: 0.0, count: resultLen)

        for i in 0..<na.count {
            for j in 0..<nb.count {
                result[i + j] += na[i] * nb[j]
            }
        }

        return normalize(result)
    }

    // ========================================================================
    // MARK: - Division
    // ========================================================================

    /// Perform polynomial long division, returning `(quotient, remainder)`.
    ///
    /// Given polynomials `dividend` and `divisor` (divisor ≠ zero), finds
    /// `quotient` and `remainder` such that:
    ///
    ///   dividend = divisor × quotient + remainder
    ///   degree(remainder) < degree(divisor)
    ///
    /// The algorithm is the polynomial analog of school long division:
    /// 1. Find the leading term of the current remainder.
    /// 2. Divide it by the leading term of `divisor` to get the next quotient term.
    /// 3. Subtract `(quotient term) × divisor` from the remainder.
    /// 4. Repeat until `degree(remainder) < degree(divisor)`.
    ///
    /// Detailed example: divide `[5, 1, 3, 2]` (= 5 + x + 3x² + 2x³) by
    ///                           `[2, 1]`       (= 2 + x)
    ///
    ///   Step 1: rem = [5,1,3,2], deg=3. Leading = 2x³, divisor leading = x.
    ///           Quotient term: 2x³ / x = 2x² → quot[2] = 2
    ///           Subtract 2x² × (2+x) = [0,0,4,2] from rem:
    ///           [5,1,3-4,2-2] = [5,1,-1,0] → [5,1,-1]
    ///
    ///   Step 2: rem = [5,1,-1], deg=2. Leading = -x², divisor leading = x.
    ///           Quotient term: -x² / x = -x → quot[1] = -1
    ///           Subtract -x × (2+x) = [0,-2,-1] from [5,1,-1]:
    ///           [5,3,0] → [5,3]
    ///
    ///   Step 3: rem = [5,3], deg=1. Leading = 3x, divisor leading = x.
    ///           Quotient term: 3x / x = 3 → quot[0] = 3
    ///           Subtract 3 × (2+x) = [6,3] from [5,3]:
    ///           [-1,0] → [-1]
    ///
    ///   Step 4: degree([-1]) = 0 < 1 = degree(divisor). STOP.
    ///   Result: quot = [3, -1, 2], rem = [-1]
    ///   Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³ ✓
    ///
    /// - Precondition: `divisor` is not the zero polynomial.
    /// - Parameters:
    ///   - dividend: The polynomial being divided.
    ///   - divisor: The polynomial to divide by.
    /// - Returns: A tuple `(quotient, remainder)`, both normalized.
    public static func divmod(_ dividend: [Double], _ divisor: [Double]) -> ([Double], [Double]) {
        let nb = normalize(divisor)
        // A divisor is zero if it normalizes to [0.0].
        let divisorIsZero = nb.count == 1 && abs(nb[0]) < zeroThreshold
        precondition(!divisorIsZero, "Polynomial division by zero polynomial")

        let na = normalize(dividend)
        let aIsZero = na.count == 1 && abs(na[0]) < zeroThreshold
        if aIsZero {
            return ([0.0], [0.0])
        }

        let degA = na.count - 1  // degree of dividend
        let degB = nb.count - 1  // degree of divisor

        // If dividend has lower degree than divisor, quotient is zero.
        if degA < degB {
            return ([0.0], na)
        }

        // Work on a mutable copy of the remainder.
        var rem = na

        // Allocate quotient with the right number of coefficients.
        // deg(quotient) = deg(dividend) - deg(divisor)
        var quot = [Double](repeating: 0.0, count: degA - degB + 1)

        // Leading coefficient of the divisor — every quotient term is divided by this.
        let leadB = nb[degB]

        // Current degree of the remainder (walks downward as we subtract terms).
        var degRem = degA

        while degRem >= degB {
            // Leading coefficient of the current remainder.
            let leadRem = rem[degRem]
            // Coefficient and degree of the next quotient term.
            let coeff = leadRem / leadB
            let power = degRem - degB
            quot[power] = coeff

            // Subtract coeff·x^power·divisor from rem.
            for j in 0...degB {
                rem[power + j] -= coeff * nb[j]
            }

            // The leading term is now zero (by construction). Step down.
            // Skip any new trailing near-zeros as well.
            degRem -= 1
            while degRem >= 0 && abs(rem[degRem]) < zeroThreshold {
                degRem -= 1
            }
        }

        return (normalize(quot), normalize(rem))
    }

    /// Return the quotient of polynomial long division.
    ///
    /// Equivalent to `divmod(a, b).0`.
    ///
    /// - Precondition: `b` is not the zero polynomial.
    /// - Parameters:
    ///   - a: The dividend.
    ///   - b: The divisor.
    /// - Returns: The quotient `a ÷ b`, normalized.
    public static func divide(_ a: [Double], _ b: [Double]) -> [Double] {
        return divmod(a, b).0
    }

    /// Return the remainder of polynomial long division (the "modulo" operation).
    ///
    /// Equivalent to `divmod(a, b).1`. In GF(2^8) construction, a high-degree
    /// polynomial is reduced modulo the primitive polynomial using this function.
    ///
    /// Note: Swift allows a function named `mod` in this context because it is
    /// scoped inside the `Polynomial` enum namespace.
    ///
    /// - Precondition: `b` is not the zero polynomial.
    /// - Parameters:
    ///   - a: The dividend.
    ///   - b: The divisor.
    /// - Returns: The remainder `a mod b`, normalized.
    public static func mod(_ a: [Double], _ b: [Double]) -> [Double] {
        return divmod(a, b).1
    }

    // ========================================================================
    // MARK: - Evaluation
    // ========================================================================

    /// Evaluate a polynomial at a given x using Horner's method.
    ///
    /// Horner's method rewrites the polynomial in nested form:
    ///
    ///   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
    ///
    /// This requires only n additions and n multiplications — no calls to `pow`.
    /// It is both faster and more numerically stable than the naive approach.
    ///
    /// Algorithm (reading coefficients from high degree to low):
    ///   acc = 0.0
    ///   for i from n downto 0:
    ///       acc = acc * x + poly[i]
    ///   return acc
    ///
    /// Example: evaluate `[3.0, 1.0, 2.0]` (= 3 + x + 2x²) at x = 4:
    ///
    ///   Start: acc = 0.0
    ///   i=2: acc = 0.0 * 4 + 2.0 = 2.0
    ///   i=1: acc = 2.0 * 4 + 1.0 = 9.0
    ///   i=0: acc = 9.0 * 4 + 3.0 = 39.0
    ///   Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓
    ///
    /// - Parameters:
    ///   - poly: The polynomial to evaluate.
    ///   - x: The point at which to evaluate.
    /// - Returns: The numeric value `poly(x)`.
    public static func evaluate(_ poly: [Double], _ x: Double) -> Double {
        let n = normalize(poly)
        // The zero polynomial evaluates to 0 everywhere.
        if n.count == 1 && abs(n[0]) < zeroThreshold {
            return 0.0
        }
        var acc = 0.0
        // Iterate from the highest-degree coefficient down to the constant.
        for i in stride(from: n.count - 1, through: 0, by: -1) {
            acc = acc * x + n[i]
        }
        return acc
    }

    // ========================================================================
    // MARK: - Greatest Common Divisor
    // ========================================================================

    /// Compute the GCD of two polynomials using the Euclidean algorithm.
    ///
    /// Uses the Euclidean algorithm: repeatedly replace `(a, b)` with
    /// `(b, a mod b)` until `b` is the zero polynomial. The last non-zero
    /// polynomial is the GCD.
    ///
    /// This is identical to the integer GCD algorithm, with polynomial `mod`
    /// replacing integer `mod`.
    ///
    /// Pseudocode:
    ///   while b ≠ zero:
    ///       (a, b) = (b, a mod b)
    ///   return normalize(a)
    ///
    /// The result is made monic (leading coefficient 1) so the GCD is unique.
    /// A monic GCD is conventional — the GCD of two polynomials is only unique
    /// up to scalar multiples, so we pick the monic representative.
    ///
    /// Use case: GCD is used in Reed-Solomon decoding (extended Euclidean
    /// algorithm) to find the error-locator and error-evaluator polynomials.
    ///
    /// Example:
    ///   gcd([x² - 1], [x - 1]) = [x - 1]
    ///   gcd([2, 3, 1], [2, 1]) = [1, 1] (monic form of x + 1... × scalar)
    ///
    /// - Parameters:
    ///   - a: First polynomial.
    ///   - b: Second polynomial.
    /// - Returns: The monic GCD of `a` and `b`, normalized.
    public static func gcd(_ a: [Double], _ b: [Double]) -> [Double] {
        var u = normalize(a)
        var v = normalize(b)

        let uIsZero = u.count == 1 && abs(u[0]) < zeroThreshold
        let vIsZero = v.count == 1 && abs(v[0]) < zeroThreshold

        // GCD(0, 0) = 0 by convention
        if uIsZero && vIsZero {
            return [0.0]
        }

        while true {
            let vIsZeroNow = v.count == 1 && abs(v[0]) < zeroThreshold
            if vIsZeroNow { break }
            let r = mod(u, v)
            u = v
            v = r
        }

        // Make monic: divide all coefficients by the leading coefficient.
        let lead = u[u.count - 1]
        if abs(lead) < zeroThreshold || abs(lead - 1.0) < zeroThreshold {
            return normalize(u)
        }
        let monic = u.map { $0 / lead }
        return normalize(monic)
    }
}
