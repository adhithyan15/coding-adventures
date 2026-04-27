package com.codingadventures.polynomial;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for polynomial arithmetic.
 *
 * <p>Tests are organized in three sections:
 * <ol>
 *   <li>Integer arithmetic — verifies the worked examples from spec MA00.</li>
 *   <li>GF(256) arithmetic — verifies field-specific behavior (XOR addition, etc.).</li>
 *   <li>Edge cases — zero polynomial, empty inputs, degree conventions.</li>
 * </ol>
 */
class PolynomialTest {

    private static final FieldOps INT = FieldOps.INTEGER_OPS;
    private static final FieldOps GF  = FieldOps.GF256_OPS;

    // =========================================================================
    // normalize
    // =========================================================================

    @Test
    void normalize_emptyIsZero() {
        assertArrayEquals(new int[0], Polynomial.normalize(new int[0]));
    }

    @Test
    void normalize_stripsTrailingZeros() {
        assertArrayEquals(new int[]{1}, Polynomial.normalize(new int[]{1, 0, 0}));
    }

    @Test
    void normalize_allZerosIsEmpty() {
        assertArrayEquals(new int[0], Polynomial.normalize(new int[]{0}));
        assertArrayEquals(new int[0], Polynomial.normalize(new int[]{0, 0, 0}));
    }

    @Test
    void normalize_alreadyNormalized() {
        int[] p = {1, 2, 3};
        // normalize should return the same array (no copy needed)
        assertArrayEquals(p, Polynomial.normalize(p));
    }

    // =========================================================================
    // degree
    // =========================================================================

    @Test
    void degree_zeroPolynomial() {
        assertEquals(-1, Polynomial.degree(new int[0]));
        assertEquals(-1, Polynomial.degree(new int[]{0}));
    }

    @Test
    void degree_constant() {
        assertEquals(0, Polynomial.degree(new int[]{7}));
    }

    @Test
    void degree_linear() {
        assertEquals(1, Polynomial.degree(new int[]{3, 2}));
    }

    @Test
    void degree_quadratic() {
        assertEquals(2, Polynomial.degree(new int[]{3, 0, 2}));
    }

    // =========================================================================
    // add (integer)
    // =========================================================================

    /**
     * From spec MA00: [1,2,3] + [4,5] = [5,7,3].
     */
    @Test
    void add_int_differentLengths() {
        int[] a = {1, 2, 3};
        int[] b = {4, 5};
        assertArrayEquals(new int[]{5, 7, 3}, Polynomial.add(a, b, INT));
    }

    @Test
    void add_int_addsToZero() {
        int[] a = {1, 2, 3};
        int[] b = {-1, -2, -3};
        assertArrayEquals(new int[0], Polynomial.add(a, b, INT));
    }

    @Test
    void add_int_identityZero() {
        int[] a = {1, 2, 3};
        assertArrayEquals(a, Polynomial.add(a, new int[0], INT));
        assertArrayEquals(a, Polynomial.add(new int[0], a, INT));
    }

    // =========================================================================
    // sub (integer)
    // =========================================================================

    /**
     * From spec MA00: [5,7,3] - [1,2,3] = [4,5].
     * The x² term cancels and is stripped by normalize.
     */
    @Test
    void sub_int_cancelLeadingTerm() {
        int[] a = {5, 7, 3};
        int[] b = {1, 2, 3};
        assertArrayEquals(new int[]{4, 5}, Polynomial.sub(a, b, INT));
    }

    // =========================================================================
    // mul (integer)
    // =========================================================================

    /**
     * From spec MA00: [1,2] × [3,4] = [3,10,8].
     * Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
     */
    @Test
    void mul_int_example() {
        int[] a = {1, 2};
        int[] b = {3, 4};
        assertArrayEquals(new int[]{3, 10, 8}, Polynomial.mul(a, b, INT));
    }

    @Test
    void mul_int_byZero() {
        int[] a = {1, 2, 3};
        assertArrayEquals(new int[0], Polynomial.mul(a, new int[0], INT));
        assertArrayEquals(new int[0], Polynomial.mul(new int[0], a, INT));
    }

    @Test
    void mul_int_byOne() {
        int[] a = {1, 2, 3};
        assertArrayEquals(a, Polynomial.mul(a, new int[]{1}, INT));
        assertArrayEquals(a, Polynomial.mul(new int[]{1}, a, INT));
    }

    /**
     * Verify degree: deg(a) + deg(b) = deg(a*b) for non-zero polynomials.
     */
    @Test
    void mul_int_degreeSumRule() {
        int[] a = {1, 2, 3};      // degree 2
        int[] b = {4, 5, 6, 7};   // degree 3
        int[] c = Polynomial.mul(a, b, INT);
        assertEquals(Polynomial.degree(a) + Polynomial.degree(b), Polynomial.degree(c));
    }

    // =========================================================================
    // divmod (integer)
    // =========================================================================

    /**
     * From spec MA00 detailed example:
     *   divide [5,1,3,2] (= 5 + x + 3x² + 2x³)  by  [2,1] (= 2 + x)
     *   → quotient [3,-1,2]  remainder [-1]
     *
     * Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³  ✓
     */
    @Test
    void divmod_int_specExample() {
        int[] a = {5, 1, 3, 2};
        int[] b = {2, 1};
        int[][] result = Polynomial.divmod(a, b, INT);
        assertArrayEquals(new int[]{3, -1, 2}, result[0], "quotient");
        assertArrayEquals(new int[]{-1},        result[1], "remainder");
    }

    @Test
    void divmod_int_exactDivision() {
        // (x+2)(x+3) = 6 + 5x + x² → dividing by (x+2) = [2,1]
        int[] product = {6, 5, 1};  // x² + 5x + 6
        int[] b = {2, 1};           // x + 2
        int[][] result = Polynomial.divmod(product, b, INT);
        assertArrayEquals(new int[]{3, 1}, result[0], "quotient = x+3");
        assertArrayEquals(new int[0],      result[1], "remainder = 0");
    }

    @Test
    void divmod_int_dividendLowerDegree() {
        // [1,2] divided by [3,4,5]: quotient=0, remainder=[1,2]
        int[] a = {1, 2};
        int[] b = {3, 4, 5};
        int[][] result = Polynomial.divmod(a, b, INT);
        assertArrayEquals(new int[0], result[0], "quotient");
        assertArrayEquals(new int[]{1, 2}, result[1], "remainder");
    }

    @Test
    void divmod_int_byZeroThrows() {
        assertThrows(ArithmeticException.class,
            () -> Polynomial.divmod(new int[]{1, 2}, new int[0], INT));
    }

    /**
     * Division theorem: for all a, b (b non-zero): a = b*q + r.
     */
    @Test
    void divmod_int_divisionTheorem() {
        int[] a = {7, 3, 5, 2};
        int[] b = {3, 2, 1};
        int[][] qr = Polynomial.divmod(a, b, INT);
        int[] q = qr[0];
        int[] r = qr[1];
        // Reconstruct: b*q + r should equal a.
        int[] reconstructed = Polynomial.add(Polynomial.mul(b, q, INT), r, INT);
        assertArrayEquals(Polynomial.normalize(a), reconstructed);
    }

    // =========================================================================
    // evaluate (integer)
    // =========================================================================

    /**
     * From spec MA00: evaluate [3,1,2] (= 3 + x + 2x²) at x=4 → 39.
     * Horner: acc=0 → 2 → 9 → 39.  Verify: 3+4+32=39  ✓
     */
    @Test
    void evaluate_int_specExample() {
        int[] p = {3, 1, 2};
        assertEquals(39, Polynomial.evaluate(p, 4, INT));
    }

    @Test
    void evaluate_int_zeroPolynomial() {
        assertEquals(0, Polynomial.evaluate(new int[0], 99, INT));
    }

    @Test
    void evaluate_int_constant() {
        assertEquals(7, Polynomial.evaluate(new int[]{7}, 42, INT));
    }

    @Test
    void evaluate_int_atZero() {
        // p(0) = constant term
        int[] p = {5, 3, 2};
        assertEquals(5, Polynomial.evaluate(p, 0, INT));
    }

    // =========================================================================
    // gcd (GF(256))
    // =========================================================================
    //
    // NOTE: The Euclidean polynomial GCD algorithm requires *exact* field division
    // at each step (the quotient must fully eliminate the leading term).
    // INTEGER_OPS uses truncating integer division, which is not exact for
    // non-monic divisors.  GCD tests therefore use GF256_OPS where every non-zero
    // element has a multiplicative inverse and division is always exact.

    /**
     * gcd(p, zero) = p and gcd(zero, p) = p (zero polynomial is the identity for gcd).
     */
    @Test
    void gcd_gf256_withZero() {
        int[] a = {1, 2, 3};
        assertArrayEquals(Polynomial.normalize(a),
            Polynomial.gcd(a, new int[0], GF));
        assertArrayEquals(Polynomial.normalize(a),
            Polynomial.gcd(new int[0], a, GF));
    }

    /**
     * gcd(g, factor) where factor divides g exactly: result must divide g with zero remainder.
     *
     * g(x) = [8,6,1] = (x+2)(x+4).  factor = [2,1] = (x+2).
     * The GCD must be a multiple of [2,1].
     */
    @Test
    void gcd_gf256_exactFactor() {
        int[] g      = {8, 6, 1};   // (x+2)(x+4)
        int[] factor = {2, 1};      // (x+2)
        int[] gcdResult = Polynomial.gcd(g, factor, GF);
        // The GCD must divide g with zero remainder.
        assertArrayEquals(new int[0], Polynomial.mod(g, gcdResult, GF),
            "gcd must divide g");
        // The GCD must divide factor with zero remainder.
        assertArrayEquals(new int[0], Polynomial.mod(factor, gcdResult, GF),
            "gcd must divide factor");
        // The GCD must have degree >= 1.
        assertTrue(Polynomial.degree(gcdResult) >= 1,
            "gcd of polynomials with a shared linear factor must be at least degree 1");
    }

    /**
     * Two distinct irreducible linear factors are coprime: their GCD is a unit (degree 0).
     * (x+2) and (x+8) have no common root and are coprime over GF(256).
     */
    @Test
    void gcd_gf256_coprime() {
        int[] a = {2, 1};   // x + α   (root = α  = 2)
        int[] b = {8, 1};   // x + α³  (root = α³ = 8)
        int[] g = Polynomial.gcd(a, b, GF);
        // Coprime polynomials have a constant GCD (degree 0).
        assertEquals(0, Polynomial.degree(g),
            "gcd of coprime polynomials must be a constant");
    }

    // =========================================================================
    // GF(256) polynomial arithmetic
    // =========================================================================

    /**
     * In GF(256), addition is XOR. [3,5] + [3,7] = [0,2] → normalize → [0,2].
     * 3^3=0, 5^7=2 → result = [0, 2].
     */
    @Test
    void add_gf256_isXor() {
        int[] a = {3, 5};
        int[] b = {3, 7};
        assertArrayEquals(new int[]{0, 2}, Polynomial.add(a, b, GF));
    }

    /**
     * In GF(256), subtraction equals addition (XOR).
     */
    @Test
    void sub_gf256_equalsAdd() {
        int[] a = {10, 20, 30};
        int[] b = {5, 15, 25};
        assertArrayEquals(Polynomial.add(a, b, GF), Polynomial.sub(a, b, GF));
    }

    /**
     * GF(256) polynomial multiply: [2,1] × [4,1] = [8,6,1].
     *
     * This is the RS generator polynomial for nCheck=2:
     *   g(x) = (x+α)(x+α²) = (x+2)(x+4)
     *
     * Coefficients:
     *   [0] = GF.mul(2,4) = 8
     *   [1] = GF.mul(2,1) XOR GF.mul(4,1) = 2 XOR 4 = 6
     *   [2] = 1
     *
     * So [2,1] × [4,1] = [8,6,1]. This is the nCheck=2 generator polynomial.
     */
    @Test
    void mul_gf256_generatorNCheck2() {
        int[] a = {2, 1};   // (x + 2) = (x + α)
        int[] b = {4, 1};   // (x + 4) = (x + α²)
        // Expected: [8, 6, 1]
        assertArrayEquals(new int[]{8, 6, 1}, Polynomial.mul(a, b, GF));
    }

    /**
     * GF(256) evaluate: the generator polynomial [8,6,1] has roots at α=2 and α²=4.
     */
    @Test
    void evaluate_gf256_generatorRoots() {
        int[] gen = {8, 6, 1};  // g(x) = x² + 6x + 8 (nCheck=2 generator)
        // g(2)  must be 0:  8 XOR mul(6,2) XOR mul(1,4) = 8 XOR 12 XOR 4 = 0
        assertEquals(0, Polynomial.evaluate(gen, 2, GF), "g(α¹) should be 0");
        // g(4)  must be 0:  8 XOR mul(6,4) XOR mul(1,16) = 8 XOR 24 XOR 16 = 0
        assertEquals(0, Polynomial.evaluate(gen, 4, GF), "g(α²) should be 0");
    }

    /**
     * GF(256) divmod: dividing a polynomial by a factor should give zero remainder.
     */
    @Test
    void divmod_gf256_exactDivision() {
        // g(x) = [8,6,1] is (x+2)(x+4).  Divide by (x+2) = [2,1].
        int[] g = {8, 6, 1};
        int[] factor = {2, 1};
        int[][] qr = Polynomial.divmod(g, factor, GF);
        // Remainder must be zero.
        assertArrayEquals(new int[0], qr[1], "remainder should be zero");
        // Quotient must be [4,1] = (x+4).
        assertArrayEquals(new int[]{4, 1}, qr[0], "quotient should be (x+4)");
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    @Test
    void mul_int_commutativity() {
        int[] a = {1, 2, 3};
        int[] b = {4, 5};
        assertArrayEquals(Polynomial.mul(a, b, INT), Polynomial.mul(b, a, INT));
    }

    @Test
    void add_int_commutativity() {
        int[] a = {1, 2, 3};
        int[] b = {4, 5};
        assertArrayEquals(Polynomial.add(a, b, INT), Polynomial.add(b, a, INT));
    }

    @Test
    void divmod_gf256_byZeroThrows() {
        assertThrows(ArithmeticException.class,
            () -> Polynomial.divmod(new int[]{1, 2}, new int[0], GF));
    }

    @Test
    void evaluate_gf256_zeroPolynomial() {
        assertEquals(0, Polynomial.evaluate(new int[0], 42, GF));
    }
}
