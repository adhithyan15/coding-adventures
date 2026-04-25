/**
 * Polynomial arithmetic over GF(2^8).
 *
 * This package implements polynomials whose coefficients are elements of the
 * finite field GF(256) — the integers 0..255 under XOR-addition and
 * log/exp-table multiplication.
 *
 * ## Representation
 *
 * A polynomial is a little-endian IntArray:
 *
 *   index i  =  coefficient of x^i
 *
 *   [3, 0, 2]  →  3 + 0·x + 2·x²  =  3 + 2x²
 *   [1, 2, 3]  →  1 + 2x + 3x²
 *   []         →  the zero polynomial
 *
 * "Little-endian" keeps addition trivially position-aligned and makes Horner's
 * method natural (iterate from high index to 0).
 *
 * All functions return normalised polynomials: trailing zeros are stripped.
 * [1, 0, 0] and [1] both represent the constant polynomial 1.
 *
 * ## Why GF(256) coefficients?
 *
 * Reed-Solomon encoding and decoding require polynomial arithmetic over the
 * same field as the data bytes.  Real-number polynomials accumulate rounding
 * errors and cannot represent byte values exactly.  GF(256) arithmetic is
 * exact, closed, and fast.
 *
 * ## Key differences from real-number polynomials (MA00)
 *
 * | Operation | Real polynomials | GF(256) polynomials |
 * |-----------|-----------------|---------------------|
 * | add coeff | a + b           | a XOR b             |
 * | sub coeff | a − b           | a XOR b  (same!)    |
 * | mul coeff | a × b           | GF256.mul(a, b)     |
 * | div coeff | a / b           | GF256.div(a, b)     |
 *
 * Spec: MA00-polynomial.md (GF(256) variant used by MA02 reed-solomon)
 */
package com.codingadventures.polynomial

import com.codingadventures.gf256.GF256

/** Version of this package. */
const val VERSION = "0.1.0"

// =============================================================================
// Type alias
// =============================================================================

/**
 * A polynomial over GF(256), represented as a little-endian IntArray.
 * Index i holds the coefficient of x^i.  All coefficients must be in 0..255.
 * The zero polynomial is an empty array.
 */
typealias Poly = IntArray

// =============================================================================
// Fundamentals
// =============================================================================

/**
 * Return a normalised copy of [p], with all trailing zero coefficients removed.
 *
 * Trailing zeros represent zero-coefficient high-degree terms. They do not
 * change the polynomial's value but do affect degree comparisons and the loop
 * termination condition in polynomial long division.
 *
 * Examples:
 *   normalize([1, 0, 0]) → [1]   (constant polynomial 1)
 *   normalize([0])       → []    (zero polynomial)
 *   normalize([1, 2, 3]) → [1, 2, 3]  (already normalised)
 */
fun normalize(p: Poly): Poly {
    var len = p.size
    // Walk backward until we reach a non-zero coefficient.
    while (len > 0 && p[len - 1] == 0) len--
    return p.copyOfRange(0, len)
}

/**
 * Return the degree of polynomial [p].
 *
 * The degree is the index of the highest non-zero coefficient.
 * By convention, the zero polynomial has degree −1. This sentinel lets
 * polynomial long division terminate cleanly: the loop condition
 * `degree(remainder) >= degree(divisor)` is false when the remainder is zero.
 *
 * Examples:
 *   degree([3, 0, 2]) → 2
 *   degree([7])       → 0
 *   degree([])        → -1   (zero polynomial)
 *   degree([0, 0])    → -1   (normalises to []; same as zero polynomial)
 */
fun degree(p: Poly): Int = normalize(p).size - 1

/** The zero polynomial — the additive identity. */
fun zero(): Poly = IntArray(0)

/** The multiplicative identity polynomial [1]. */
fun one(): Poly = intArrayOf(1)

// =============================================================================
// Addition and Subtraction
// =============================================================================

/**
 * Add two GF(256) polynomials coefficient-by-coefficient.
 *
 * In GF(256), addition is XOR for each coefficient.  The shorter polynomial
 * is extended with implicit zero coefficients.
 *
 * Visual example (in GF(256)):
 *   [1, 2, 3]   =  1 + 2x + 3x²
 * + [4, 5]      =  4 + 5x
 * ────────────────────────────────
 *   [1⊕4, 2⊕5, 3]  =  [5, 7, 3]   (XOR of each position)
 */
fun add(a: Poly, b: Poly): Poly {
    val len = maxOf(a.size, b.size)
    val result = IntArray(len)
    for (i in 0 until len) {
        val ai = if (i < a.size) a[i] else 0
        val bi = if (i < b.size) b[i] else 0
        result[i] = GF256.add(ai, bi)  // XOR
    }
    return normalize(result)
}

/**
 * Subtract polynomial [b] from [a] coefficient-by-coefficient.
 *
 * In GF(256), subtraction equals addition (XOR), because −1 = 1 in
 * characteristic-2 fields.  This function is provided for clarity/symmetry
 * with the MA00 polynomial interface.
 */
fun sub(a: Poly, b: Poly): Poly = add(a, b)  // sub == add in GF(2^n)

// =============================================================================
// Multiplication
// =============================================================================

/**
 * Multiply two GF(256) polynomials by polynomial convolution.
 *
 * Each term a[i]·x^i of [a] multiplies each term b[j]·x^j of [b],
 * contributing GF256.mul(a[i], b[j]) to result[i+j] via XOR accumulation.
 *
 * If [a] has degree m and [b] has degree n, the result has degree m + n.
 *
 * Visual example (in GF(256)):
 *   [1, 2]  =  1 + 2x
 * × [3, 4]  =  3 + 4x
 * ────────────────────────────────────────
 *   i=0,j=0: result[0] ^= mul(1,3) = 3
 *   i=0,j=1: result[1] ^= mul(1,4) = 4
 *   i=1,j=0: result[1] ^= mul(2,3) = 6  → result[1] = 4^6 = 2
 *   i=1,j=1: result[2] ^= mul(2,4) = 8? let's just note it's GF multiplication
 *
 * Result: [mul(1,3), mul(1,4)^mul(2,3), mul(2,4)]
 */
fun mul(a: Poly, b: Poly): Poly {
    if (a.isEmpty() || b.isEmpty()) return zero()
    val resultLen = a.size + b.size - 1
    val result = IntArray(resultLen)
    for (i in a.indices) {
        for (j in b.indices) {
            result[i + j] = GF256.add(result[i + j], GF256.mul(a[i], b[j]))
        }
    }
    return normalize(result)
}

// =============================================================================
// Division
// =============================================================================

/**
 * Perform GF(256) polynomial long division, returning [quotient, remainder].
 *
 * Finds q and r such that:
 *   a = b × q + r   and   degree(r) < degree(b)
 *
 * The algorithm is the polynomial analog of long division:
 * 1. Find the leading term of the current remainder.
 * 2. Divide it by the leading term of [b] (using GF256.div) to get the next
 *    quotient term.
 * 3. Subtract (quotient term) × b from the remainder (XOR, because GF add=sub).
 * 4. Repeat until degree(remainder) < degree(b).
 *
 * This is the core operation used in:
 * - RS encoding: compute M(x)·x^nCheck mod g(x) to get parity bytes
 * - RS decoding: extended Euclidean algorithm for error-locator polynomials
 *
 * @throws ArithmeticException if [b] is the zero polynomial
 */
fun divmod(a: Poly, b: Poly): Pair<Poly, Poly> {
    val nb = normalize(b)
    if (nb.isEmpty()) throw ArithmeticException("polynomial division by zero")

    val na = normalize(a)
    val degA = na.size - 1
    val degB = nb.size - 1

    // If degree(a) < degree(b), quotient is 0, remainder is a.
    if (degA < degB) return Pair(zero(), na)

    // Work on a mutable copy of the remainder.
    val rem = na.copyOf()
    // Allocate the quotient array with the correct degree.
    val quot = IntArray(degA - degB + 1)

    // Leading coefficient of the divisor — used to normalise each quotient term.
    val leadB = nb[degB]

    // Current logical degree of the remainder (walks downward).
    var degRem = degA

    while (degRem >= degB) {
        val leadRem = rem[degRem]
        if (leadRem == 0) {
            degRem--
            continue
        }
        // Quotient coefficient: leadRem / leadB in GF(256).
        val coeff = GF256.div(leadRem, leadB)
        val power = degRem - degB
        quot[power] = coeff

        // Subtract coeff·x^power·b from rem (XOR-based subtraction).
        for (j in 0..degB) {
            rem[power + j] = GF256.add(rem[power + j], GF256.mul(coeff, nb[j]))
        }

        // The leading term is now zero by construction.  Decrement, skipping new
        // trailing zeros.
        degRem--
        while (degRem >= 0 && rem[degRem] == 0) degRem--
    }

    return Pair(normalize(quot), normalize(rem))
}

/**
 * Return the quotient of [divmod].
 *
 * @throws ArithmeticException if [b] is the zero polynomial
 */
fun divide(a: Poly, b: Poly): Poly = divmod(a, b).first

/**
 * Return the remainder of [divmod].
 *
 * This is the GF(256) polynomial "modulo" operation.  Reed-Solomon encoding
 * reduces the shifted message polynomial modulo the generator polynomial using
 * this function.
 *
 * @throws ArithmeticException if [b] is the zero polynomial
 */
fun mod(a: Poly, b: Poly): Poly = divmod(a, b).second

// =============================================================================
// Evaluation
// =============================================================================

/**
 * Evaluate a GF(256) polynomial at [x] using Horner's method.
 *
 * Horner's method rewrites the polynomial in nested form to minimise
 * multiplications:
 *
 *   a₀ + x(a₁ + x(a₂ + … + x·aₙ))
 *
 * Algorithm (iterating coefficients from high degree to low):
 *   acc = 0
 *   for i from n downto 0:
 *       acc = GF256.add(GF256.mul(acc, x), p[i])
 *   return acc
 *
 * This requires exactly n GF(256) multiplications and n additions (XORs).
 *
 * Used in Reed-Solomon syndrome computation:
 *   S_j = codeword(α^j)   for j = 1, …, nCheck
 *
 * @param p  polynomial to evaluate (little-endian GF(256) coefficients)
 * @param x  the field element at which to evaluate (0..255)
 * @return   the GF(256) value p(x)
 */
fun eval(p: Poly, x: Int): Int {
    val n = normalize(p)
    if (n.isEmpty()) return 0
    var acc = 0
    for (i in n.size - 1 downTo 0) {
        acc = GF256.add(GF256.mul(acc, x), n[i])
    }
    return acc
}

// =============================================================================
// Greatest Common Divisor
// =============================================================================

/**
 * Compute the GCD of two GF(256) polynomials using the Euclidean algorithm.
 *
 * Repeatedly replaces (a, b) with (b, a mod b) until b is the zero polynomial.
 * The last non-zero remainder is the GCD.
 *
 * Pseudocode:
 *   while b ≠ zero:
 *       a, b = b, a mod b
 *   return normalize(a)
 *
 * This is identical to the integer GCD algorithm, with polynomial mod in place
 * of integer mod.
 *
 * Use case: Reed-Solomon decoding uses the extended Euclidean algorithm on
 * polynomials to find the error-locator and error-evaluator polynomials.
 */
fun gcd(a: Poly, b: Poly): Poly {
    var u = normalize(a)
    var v = normalize(b)
    while (v.isNotEmpty()) {
        val r = mod(u, v)
        u = v
        v = r
    }
    return normalize(u)
}

// =============================================================================
// Convenience constructors
// =============================================================================

/**
 * Create a polynomial from a vararg list of GF(256) coefficients (little-endian).
 *
 * Example: `poly(3, 0, 2)` → 3 + 0·x + 2·x² in GF(256)
 */
fun poly(vararg coeffs: Int): Poly = normalize(IntArray(coeffs.size) { coeffs[it] })
