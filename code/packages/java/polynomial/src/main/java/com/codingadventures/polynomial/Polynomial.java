package com.codingadventures.polynomial;

import java.util.Arrays;

/**
 * Polynomial arithmetic over an abstract coefficient field.
 *
 * <p>A polynomial is stored as a {@code int[]} where <strong>index equals degree</strong>:
 *
 * <pre>
 *   coeffs[0] = constant term (coefficient of x^0)
 *   coeffs[1] = coefficient of x^1
 *   coeffs[2] = coefficient of x^2
 *   ...
 * </pre>
 *
 * <p>This "little-endian" convention makes addition trivially position-aligned:
 * to add two polynomials, add coefficients at matching indices.
 * It also makes Horner's method natural to read (iterate from high index to low).
 *
 * <p>The <strong>zero polynomial</strong> is represented as an empty array {@code []}.
 * All arithmetic operations <em>normalize</em> their result: trailing zero
 * coefficients are stripped. So {@code [1, 0, 0]} and {@code [1]} both represent
 * the constant polynomial 1.
 *
 * <h2>Field-agnostic Design</h2>
 *
 * <p>All arithmetic operations accept a {@link FieldOps} instance that defines
 * coefficient addition, subtraction, multiplication, and division. Two built-in
 * instances are provided:
 * <ul>
 *   <li>{@link FieldOps#INTEGER_OPS} — ordinary integer arithmetic</li>
 *   <li>{@link FieldOps#GF256_OPS} — GF(2^8) Galois Field arithmetic</li>
 * </ul>
 *
 * <h2>Operations</h2>
 *
 * <ul>
 *   <li>{@link #normalize(int[])} — strip trailing zeros</li>
 *   <li>{@link #degree(int[])} — highest non-zero index, or -1 for zero polynomial</li>
 *   <li>{@link #add(int[], int[], FieldOps)} — term-by-term addition</li>
 *   <li>{@link #sub(int[], int[], FieldOps)} — term-by-term subtraction</li>
 *   <li>{@link #mul(int[], int[], FieldOps)} — polynomial convolution</li>
 *   <li>{@link #divmod(int[], int[], FieldOps)} — polynomial long division</li>
 *   <li>{@link #divide(int[], int[], FieldOps)} — quotient only</li>
 *   <li>{@link #mod(int[], int[], FieldOps)} — remainder only</li>
 *   <li>{@link #evaluate(int[], int, FieldOps)} — Horner's method evaluation</li>
 *   <li>{@link #gcd(int[], int[], FieldOps)} — Euclidean GCD</li>
 * </ul>
 */
public final class Polynomial {

    /** The empty array — canonical representation of the zero polynomial. */
    public static final int[] ZERO = new int[0];

    /** The constant polynomial 1. */
    public static final int[] ONE = new int[]{1};

    private Polynomial() {}

    // =========================================================================
    // Fundamentals
    // =========================================================================

    /**
     * Remove trailing zero coefficients from a polynomial.
     *
     * <p>Trailing zeros represent zero-coefficient high-degree terms. They do not
     * change the mathematical value, but they do affect degree comparisons and the
     * termination condition of polynomial long division.
     *
     * <p>Examples:
     * <pre>
     *   normalize([1, 0, 0]) → [1]   // constant polynomial 1
     *   normalize([0])       → []    // zero polynomial
     *   normalize([1, 2, 3]) → [1, 2, 3]  // already normalized
     * </pre>
     *
     * @param p the polynomial to normalize (must not be null)
     * @return a new array with trailing zeros removed; the empty array if all-zero
     */
    public static int[] normalize(int[] p) {
        int len = p.length;
        // Walk backwards until we find a non-zero coefficient.
        while (len > 0 && p[len - 1] == 0) {
            len--;
        }
        if (len == p.length) return p;  // already normalized — avoid copy
        return Arrays.copyOf(p, len);
    }

    /**
     * Return the degree of a polynomial.
     *
     * <p>The degree is the index of the highest non-zero coefficient.
     * By convention, the zero polynomial has degree {@code -1}. This sentinel
     * lets polynomial long division terminate cleanly: the loop condition
     * {@code degree(remainder) >= degree(divisor)} is {@code false} when the
     * remainder is zero.
     *
     * <p>Examples:
     * <pre>
     *   degree([3, 0, 2]) == 2   // x^2 term is highest non-zero
     *   degree([7])       == 0   // constant polynomial
     *   degree([])        == -1  // zero polynomial
     *   degree([0, 0])    == -1  // normalizes to []; still zero polynomial
     * </pre>
     *
     * @param p the polynomial (may be zero polynomial)
     * @return the degree, or -1 if p is the zero polynomial
     */
    public static int degree(int[] p) {
        int[] n = normalize(p);
        return n.length - 1;   // -1 when n is empty (zero polynomial)
    }

    // =========================================================================
    // Addition and Subtraction
    // =========================================================================

    /**
     * Add two polynomials term-by-term.
     *
     * <p>Addition aligns coefficients by degree and adds matching pairs,
     * extending the shorter polynomial with implicit zeros.
     *
     * <p>Visual example:
     * <pre>
     *   [1, 2, 3]   =  1 + 2x + 3x²
     * + [4, 5]       =  4 + 5x
     * ───────────────
     *   [5, 7, 3]   =  5 + 7x + 3x²
     * </pre>
     *
     * <p>In GF(256), addition is XOR: {@code add(a, b)} uses {@code FieldOps.GF256_OPS}.
     *
     * @param a    first addend
     * @param b    second addend
     * @param ops  field arithmetic to use for coefficient addition
     * @return normalized sum polynomial
     */
    public static int[] add(int[] a, int[] b, FieldOps ops) {
        int len = Math.max(a.length, b.length);
        int[] result = new int[len];
        for (int i = 0; i < len; i++) {
            int ai = (i < a.length) ? a[i] : 0;
            int bi = (i < b.length) ? b[i] : 0;
            result[i] = ops.add(ai, bi);
        }
        return normalize(result);
    }

    /**
     * Subtract polynomial {@code b} from polynomial {@code a} term-by-term.
     *
     * <p>Equivalent to {@code add(a, negate(b))}, but implemented directly.
     * In GF(256), subtraction equals addition (since {@code -1 = 1} in
     * characteristic 2), so this is identical to {@link #add}.
     *
     * <p>Visual example:
     * <pre>
     *   [5, 7, 3]  =  5 + 7x + 3x²
     * - [1, 2, 3]  =  1 + 2x + 3x²
     * ─────────────
     *   [4, 5, 0]  →  normalize  →  [4, 5]   (3x² - 3x² = 0, stripped)
     * </pre>
     *
     * @param a    minuend polynomial
     * @param b    subtrahend polynomial
     * @param ops  field arithmetic to use for coefficient subtraction
     * @return normalized difference polynomial
     */
    public static int[] sub(int[] a, int[] b, FieldOps ops) {
        int len = Math.max(a.length, b.length);
        int[] result = new int[len];
        for (int i = 0; i < len; i++) {
            int ai = (i < a.length) ? a[i] : 0;
            int bi = (i < b.length) ? b[i] : 0;
            result[i] = ops.sub(ai, bi);
        }
        return normalize(result);
    }

    // =========================================================================
    // Multiplication
    // =========================================================================

    /**
     * Multiply two polynomials using polynomial convolution.
     *
     * <p>Each term {@code a[i]·x^i} multiplies each term {@code b[j]·x^j},
     * contributing {@code a[i]*b[j]} to the result at index {@code i+j}.
     * If a has degree m and b has degree n, the result has degree m+n.
     *
     * <p>Visual example:
     * <pre>
     *   [1, 2]  =  1 + 2x
     * × [3, 4]  =  3 + 4x
     * ──────────────────────
     *   result[0] = 1*3 = 3
     *   result[1] = 1*4 + 2*3 = 10
     *   result[2] = 2*4 = 8
     *   → [3, 10, 8]  =  3 + 10x + 8x²
     * </pre>
     *
     * <p>In GF(256) the coefficient arithmetic uses XOR and log/antilog tables,
     * but the convolution structure is identical.
     *
     * @param a    first factor polynomial
     * @param b    second factor polynomial
     * @param ops  field arithmetic to use for coefficient operations
     * @return normalized product polynomial (zero polynomial if either input is zero)
     */
    public static int[] mul(int[] a, int[] b, FieldOps ops) {
        if (a.length == 0 || b.length == 0) return ZERO;

        int resultLen = a.length + b.length - 1;
        int[] result = new int[resultLen];

        for (int i = 0; i < a.length; i++) {
            for (int j = 0; j < b.length; j++) {
                result[i + j] = ops.add(result[i + j], ops.mul(a[i], b[j]));
            }
        }
        return normalize(result);
    }

    // =========================================================================
    // Division
    // =========================================================================

    /**
     * Polynomial long division: return {@code [quotient, remainder]}.
     *
     * <p>Given polynomials {@code a} and {@code b} ({@code b ≠ zero}), finds
     * {@code q} and {@code r} such that:
     * <pre>
     *   a = b × q + r   and   degree(r) < degree(b)
     * </pre>
     *
     * <p>Algorithm (identical to school long division):
     * <ol>
     *   <li>Find the leading term of the current remainder.</li>
     *   <li>Divide it by the leading term of b to get the next quotient term.</li>
     *   <li>Subtract (quotient term × b) from the remainder.</li>
     *   <li>Repeat until degree(remainder) &lt; degree(b).</li>
     * </ol>
     *
     * <p>Detailed example: divide {@code [5, 1, 3, 2]} (= 5 + x + 3x² + 2x³)
     * by {@code [2, 1]} (= 2 + x), computed over integers:
     * <pre>
     *   Step 1: rem = [5,1,3,2], deg=3. Lead = 2x³, divisor lead = x.
     *           q_term = 2x² → q[2] = 2
     *           subtract 2x² × (2+x) = [0,0,4,2] → rem = [5,1,-1]
     *   Step 2: rem = [5,1,-1], deg=2. q_term = -x → q[1] = -1
     *           subtract -x × (2+x) = [0,-2,-1] → rem = [5,3]
     *   Step 3: rem = [5,3], deg=1. q_term = 3 → q[0] = 3
     *           subtract 3 × (2+x) = [6,3] → rem = [-1]
     *   Step 4: degree([-1]) = 0 < 1 = degree(b). STOP.
     *   q = [3,-1,2]  r = [-1]
     * </pre>
     *
     * @param a    dividend polynomial
     * @param b    divisor polynomial (must not be the zero polynomial)
     * @param ops  field arithmetic for coefficient operations
     * @return two-element array {@code {quotient, remainder}}, both normalized
     * @throws ArithmeticException if b is the zero polynomial
     */
    public static int[][] divmod(int[] a, int[] b, FieldOps ops) {
        int[] nb = normalize(b);
        if (nb.length == 0) {
            throw new ArithmeticException("polynomial division by zero");
        }

        int[] na = normalize(a);
        int degA = na.length - 1;
        int degB = nb.length - 1;

        // If dividend has lower degree than divisor, quotient = 0, remainder = a.
        if (degA < degB) {
            return new int[][]{ZERO, na};
        }

        // Work on a mutable copy of the remainder.
        int[] rem = Arrays.copyOf(na, na.length);
        // Allocate quotient with the correct degree.
        int[] quot = new int[degA - degB + 1];

        // Leading coefficient of the divisor (constant throughout the loop).
        int leadB = nb[degB];

        // degRem walks downward as we subtract leading terms.
        int degRem = degA;

        while (degRem >= degB) {
            // Leading term of the current remainder.
            int leadRem = rem[degRem];

            // Quotient coefficient at this power: leadRem / leadB.
            // In exact field arithmetic (GF(256), rationals) this always eliminates
            // the leading term completely.  INTEGER_OPS uses truncating division:
            // if leadRem < leadB (absolute value), coeff would be 0 and no progress
            // is made — the algorithm would loop forever.  Guard against that.
            int coeff = ops.div(leadRem, leadB);
            if (ops.isZero(coeff)) {
                // Cannot eliminate the leading term by this quotient coefficient.
                // This happens with non-field (integer truncation) division.
                // Treat the remaining polynomial as the final remainder.
                break;
            }
            int power = degRem - degB;
            quot[power] = coeff;

            // Subtract coeff * x^power * b from rem.
            // For each term b[j] at degree j, its shifted position is j + power.
            for (int j = 0; j <= degB; j++) {
                rem[power + j] = ops.sub(rem[power + j], ops.mul(coeff, nb[j]));
            }

            // The leading term is now zero (in exact arithmetic).
            // Decrement degRem past any new trailing zeros.
            degRem--;
            while (degRem >= 0 && ops.isZero(rem[degRem])) {
                degRem--;
            }
        }

        return new int[][]{normalize(quot), normalize(rem)};
    }

    /**
     * Return just the quotient of polynomial long division.
     *
     * <p>Equivalent to {@code divmod(a, b, ops)[0]}.
     *
     * @param a    dividend
     * @param b    divisor (must not be zero)
     * @param ops  field arithmetic
     * @return quotient polynomial, normalized
     * @throws ArithmeticException if b is zero
     */
    public static int[] divide(int[] a, int[] b, FieldOps ops) {
        return divmod(a, b, ops)[0];
    }

    /**
     * Return just the remainder of polynomial long division.
     *
     * <p>This is the polynomial "modulo" operation. Equivalent to
     * {@code divmod(a, b, ops)[1]}.
     *
     * <p>In Reed-Solomon encoding, this is the key step: the check bytes are
     * the coefficients of {@code (message_shifted mod generator)}.
     *
     * @param a    dividend
     * @param b    divisor (must not be zero)
     * @param ops  field arithmetic
     * @return remainder polynomial, normalized
     * @throws ArithmeticException if b is zero
     */
    public static int[] mod(int[] a, int[] b, FieldOps ops) {
        return divmod(a, b, ops)[1];
    }

    // =========================================================================
    // Evaluation
    // =========================================================================

    /**
     * Evaluate a polynomial at a point {@code x} using Horner's method.
     *
     * <p>Horner's method rewrites the polynomial in nested form to eliminate
     * explicit powers:
     * <pre>
     *   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
     * </pre>
     *
     * <p>This requires only n additions and n multiplications — no {@code x^k}
     * exponentiation at all.
     *
     * <p>Algorithm (read from high degree to low):
     * <pre>
     *   acc = 0
     *   for i from degree(p) downto 0:
     *       acc = acc * x + p[i]
     *   return acc
     * </pre>
     *
     * <p>Example: evaluate {@code [3, 1, 2]} (= 3 + x + 2x²) at x = 4:
     * <pre>
     *   acc = 0
     *   i=2: acc = 0*4 + 2 = 2
     *   i=1: acc = 2*4 + 1 = 9
     *   i=0: acc = 9*4 + 3 = 39
     *   Verify: 3 + 4 + 2·16 = 39  ✓
     * </pre>
     *
     * @param p    the polynomial to evaluate
     * @param x    the point to evaluate at
     * @param ops  field arithmetic for coefficient operations
     * @return p(x) as a field element
     */
    public static int evaluate(int[] p, int x, FieldOps ops) {
        int[] n = normalize(p);
        if (n.length == 0) return 0;  // zero polynomial evaluates to 0 everywhere

        int acc = 0;
        // Iterate from the highest-degree coefficient downward.
        for (int i = n.length - 1; i >= 0; i--) {
            acc = ops.add(ops.mul(acc, x), n[i]);
        }
        return acc;
    }

    // =========================================================================
    // Greatest Common Divisor
    // =========================================================================

    /**
     * Compute the greatest common divisor of two polynomials using the
     * Euclidean algorithm.
     *
     * <p>The GCD of {@code a} and {@code b} is the highest-degree monic polynomial
     * that divides both with zero remainder.
     *
     * <p>The Euclidean algorithm for polynomials is identical to its integer
     * counterpart: repeatedly replace {@code (a, b)} with {@code (b, a mod b)}
     * until b is the zero polynomial.
     *
     * <p>Pseudocode:
     * <pre>
     *   while b ≠ zero:
     *       a, b = b, a mod b
     *   return normalize(a)
     * </pre>
     *
     * <p>Use case: GCD is used in Reed-Solomon decoding via the extended
     * Euclidean algorithm to find the error-locator and error-evaluator polynomials.
     *
     * @param a    first polynomial
     * @param b    second polynomial
     * @param ops  field arithmetic
     * @return GCD of a and b, normalized
     */
    public static int[] gcd(int[] a, int[] b, FieldOps ops) {
        int[] u = normalize(a);
        int[] v = normalize(b);

        while (v.length > 0) {
            int[] r = mod(u, v, ops);
            u = v;
            v = r;
        }

        return normalize(u);
    }
}
