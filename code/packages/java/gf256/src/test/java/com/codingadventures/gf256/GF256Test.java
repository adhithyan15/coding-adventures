package com.codingadventures.gf256;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for GF(256) arithmetic.
 *
 * <p>These tests verify the fundamental properties of the Galois Field:
 * <ul>
 *   <li>Field axioms (commutativity, associativity, distributivity)</li>
 *   <li>Specific spot-check test vectors from the spec</li>
 *   <li>Edge cases (zero, one, overflow)</li>
 * </ul>
 *
 * <p>Cross-validated against the TypeScript, Python, Go, and Rust implementations.
 */
class GF256Test {

    // =========================================================================
    // Table construction
    // =========================================================================

    /**
     * Verify the very first few entries of the antilogarithm table.
     *
     * From the spec (MA01):
     *   ALOG[0] = 1, ALOG[1] = 2, ALOG[7] = 128, ALOG[8] = 29
     *
     * ALOG[8] = 29 because: 128 * 2 = 256 ≥ 256, so XOR with 0x11D:
     *   256 XOR 285 = 0x100 XOR 0x11D = 0x01D = 29. ✓
     */
    @Test
    void expTable_firstEntries() {
        assertEquals(1,   GF256.EXP_TABLE[0]);
        assertEquals(2,   GF256.EXP_TABLE[1]);
        assertEquals(4,   GF256.EXP_TABLE[2]);
        assertEquals(8,   GF256.EXP_TABLE[3]);
        assertEquals(16,  GF256.EXP_TABLE[4]);
        assertEquals(32,  GF256.EXP_TABLE[5]);
        assertEquals(64,  GF256.EXP_TABLE[6]);
        assertEquals(128, GF256.EXP_TABLE[7]);
        assertEquals(29,  GF256.EXP_TABLE[8]);   // first reduction step
        assertEquals(58,  GF256.EXP_TABLE[9]);
    }

    /**
     * ALOG[254] = 142 (the last entry before the cycle wraps, 0-indexed).
     *
     * Note: the spec's "Last 5 entries" table uses 1-based step counting from
     * the iteration loop, which starts i from 0. The actual 0-indexed values are:
     *   ALOG[250]=108, ALOG[251]=216, ALOG[252]=173, ALOG[253]=71, ALOG[254]=142.
     * Cross-validated against the TypeScript reference implementation.
     */
    @Test
    void expTable_lastEntry() {
        assertEquals(142, GF256.EXP_TABLE[254]);
    }

    /**
     * The doubled region of EXP_TABLE must duplicate indices 0..254.
     * This is the mechanism that lets mul() avoid an extra mod-255.
     */
    @Test
    void expTable_doubledRegion() {
        for (int i = 0; i < 255; i++) {
            assertEquals(GF256.EXP_TABLE[i], GF256.EXP_TABLE[i + 255],
                "EXP_TABLE doubling failed at index " + i);
        }
    }

    /**
     * LOG[0] is -1 (sentinel; zero is not a power of the generator).
     */
    @Test
    void logTable_zeroSentinel() {
        assertEquals(-1, GF256.LOG_TABLE[0]);
    }

    /**
     * LOG and EXP are inverses: EXP_TABLE[LOG_TABLE[x]] == x for all x in 1..255.
     */
    @Test
    void logAndExp_inverses() {
        for (int x = 1; x <= 255; x++) {
            assertEquals(x, GF256.EXP_TABLE[GF256.LOG_TABLE[x]],
                "EXP(LOG(x)) != x for x=" + x);
        }
    }

    // =========================================================================
    // add / sub
    // =========================================================================

    /**
     * Addition is XOR. A few spot checks.
     */
    @Test
    void add_isXor() {
        assertEquals(0x00, GF256.add(0x00, 0x00));
        assertEquals(0x53 ^ 0xCA, GF256.add(0x53, 0xCA));
        assertEquals(0xFF ^ 0xFF, GF256.add(0xFF, 0xFF)); // = 0
    }

    /**
     * Every element is its own additive inverse: add(x, x) = 0 for all x.
     * This is the characteristic-2 property: x + x = 2x = 0.
     */
    @Test
    void add_selfIsZero() {
        for (int x = 0; x <= 255; x++) {
            assertEquals(0, GF256.add(x, x), "add(x,x) != 0 for x=" + x);
        }
    }

    /**
     * Subtraction is identical to addition in characteristic 2.
     */
    @Test
    void sub_equalsAdd() {
        for (int a = 0; a < 256; a += 17) {
            for (int b = 0; b < 256; b += 13) {
                assertEquals(GF256.add(a, b), GF256.sub(a, b),
                    "sub != add for a=" + a + ", b=" + b);
            }
        }
    }

    // =========================================================================
    // mul
    // =========================================================================

    /**
     * Multiply by zero always returns zero.
     */
    @Test
    void mul_zeroAbsorbing() {
        for (int x = 0; x <= 255; x++) {
            assertEquals(0, GF256.mul(0, x), "0 * x != 0 for x=" + x);
            assertEquals(0, GF256.mul(x, 0), "x * 0 != 0 for x=" + x);
        }
    }

    /**
     * Multiply by one is the identity: x * 1 = x.
     */
    @Test
    void mul_identityOne() {
        for (int x = 0; x <= 255; x++) {
            assertEquals(x, GF256.mul(1, x), "1 * x != x for x=" + x);
            assertEquals(x, GF256.mul(x, 1), "x * 1 != x for x=" + x);
        }
    }

    /**
     * Commutativity: a * b = b * a for all a, b.
     */
    @Test
    void mul_commutative() {
        int[] vals = {0, 1, 2, 3, 29, 57, 128, 200, 255};
        for (int a : vals) {
            for (int b : vals) {
                assertEquals(GF256.mul(a, b), GF256.mul(b, a),
                    "mul not commutative: a=" + a + ", b=" + b);
            }
        }
    }

    /**
     * Spot check: 0x53 * 0x8C = 1 (multiplicative inverses under 0x11D polynomial).
     * From MA01 spec verification section.
     */
    @Test
    void mul_inverseSpotCheck() {
        assertEquals(1, GF256.mul(0x53, 0x8C));
    }

    /**
     * g^255 = 1: the generator has exact multiplicative order 255.
     * Verified by repeated multiplication: mul(result, 2) 255 times.
     */
    @Test
    void mul_generatorOrder255() {
        // 2^1, 2^2, ..., 2^255 must equal 1.
        int result = 1;
        for (int i = 0; i < 255; i++) {
            result = GF256.mul(result, 2);
        }
        assertEquals(1, result);
    }

    /**
     * Associativity: (a * b) * c = a * (b * c).
     */
    @Test
    void mul_associative() {
        int[] vals = {1, 2, 3, 7, 29, 53, 100, 200};
        for (int a : vals) {
            for (int b : vals) {
                for (int c : vals) {
                    assertEquals(
                        GF256.mul(GF256.mul(a, b), c),
                        GF256.mul(a, GF256.mul(b, c)),
                        "mul not associative: a=" + a + ", b=" + b + ", c=" + c
                    );
                }
            }
        }
    }

    /**
     * Distributivity: a * (b + c) = a*b + a*c.
     */
    @Test
    void mul_distributive() {
        int[] vals = {1, 2, 3, 7, 29, 53, 100, 200};
        for (int a : vals) {
            for (int b : vals) {
                for (int c : vals) {
                    int lhs = GF256.mul(a, GF256.add(b, c));
                    int rhs = GF256.add(GF256.mul(a, b), GF256.mul(a, c));
                    assertEquals(lhs, rhs,
                        "distributivity failed: a=" + a + ", b=" + b + ", c=" + c);
                }
            }
        }
    }

    // =========================================================================
    // div
    // =========================================================================

    /**
     * Division by zero throws ArithmeticException.
     */
    @Test
    void div_byZeroThrows() {
        assertThrows(ArithmeticException.class, () -> GF256.div(5, 0));
        assertThrows(ArithmeticException.class, () -> GF256.div(0, 0));
    }

    /**
     * Zero divided by anything (non-zero) is zero.
     */
    @Test
    void div_zeroNumerator() {
        for (int b = 1; b <= 255; b++) {
            assertEquals(0, GF256.div(0, b), "0 / b != 0 for b=" + b);
        }
    }

    /**
     * Division is the inverse of multiplication: div(mul(a, b), b) = a.
     */
    @Test
    void div_inverseMul() {
        for (int a = 1; a <= 255; a += 7) {
            for (int b = 1; b <= 255; b += 11) {
                int product = GF256.mul(a, b);
                assertEquals(a, GF256.div(product, b),
                    "div(mul(a,b), b) != a for a=" + a + ", b=" + b);
            }
        }
    }

    /**
     * Dividing by one is identity: div(x, 1) = x.
     */
    @Test
    void div_byOneIdentity() {
        for (int x = 1; x <= 255; x++) {
            assertEquals(x, GF256.div(x, 1), "div(x, 1) != x for x=" + x);
        }
    }

    // =========================================================================
    // pow
    // =========================================================================

    /**
     * Any non-zero element to the 0th power is 1.
     */
    @Test
    void pow_zeroExponent() {
        for (int a = 0; a <= 255; a++) {
            assertEquals(1, GF256.pow(a, 0), "a^0 != 1 for a=" + a);
        }
    }

    /**
     * 0 to any positive power is 0.
     */
    @Test
    void pow_zeroBase() {
        for (int n = 1; n <= 10; n++) {
            assertEquals(0, GF256.pow(0, n), "0^" + n + " != 0");
        }
    }

    /**
     * pow(2, i) == EXP_TABLE[i] for all i in 0..254.
     * This directly validates the log-table implementation against the spec.
     */
    @Test
    void pow_matchesExpTable() {
        for (int i = 0; i < 255; i++) {
            assertEquals(GF256.EXP_TABLE[i], GF256.pow(2, i),
                "pow(2," + i + ") != EXP_TABLE[" + i + "]");
        }
    }

    /**
     * pow with negative exponent throws ArithmeticException.
     */
    @Test
    void pow_negativeExponentThrows() {
        assertThrows(ArithmeticException.class, () -> GF256.pow(2, -1));
    }

    // =========================================================================
    // inv
    // =========================================================================

    /**
     * inv(0) throws ArithmeticException (zero has no multiplicative inverse).
     */
    @Test
    void inv_zeroThrows() {
        assertThrows(ArithmeticException.class, () -> GF256.inv(0));
    }

    /**
     * a * inv(a) = 1 for all non-zero a.
     * This verifies every element has a valid inverse.
     */
    @Test
    void inv_timesOriginalIsOne() {
        for (int a = 1; a <= 255; a++) {
            assertEquals(1, GF256.mul(a, GF256.inv(a)),
                "a * inv(a) != 1 for a=" + a);
        }
    }

    /**
     * inv(1) = 1 (1 is its own inverse).
     */
    @Test
    void inv_oneIsOwnInverse() {
        assertEquals(1, GF256.inv(1));
    }

    /**
     * Spot check from the spec: inv(0x53) = 0x8C under 0x11D polynomial.
     */
    @Test
    void inv_spotCheck() {
        assertEquals(0x8C, GF256.inv(0x53));
        assertEquals(0x53, GF256.inv(0x8C));
    }

    // =========================================================================
    // Comprehensive field property verification
    // =========================================================================

    /**
     * Full round-trip: every non-zero element appears exactly once in EXP_TABLE[0..254].
     * This confirms the primitive polynomial generates the full multiplicative group.
     */
    @Test
    void expTable_allNonZeroElementsPresent() {
        boolean[] seen = new boolean[256];
        for (int i = 0; i < 255; i++) {
            int v = GF256.EXP_TABLE[i];
            assertFalse(seen[v], "Duplicate value " + v + " in EXP_TABLE at index " + i);
            assertTrue(v >= 1 && v <= 255, "EXP_TABLE[" + i + "] out of range: " + v);
            seen[v] = true;
        }
        // Every value 1..255 should have been seen exactly once.
        for (int v = 1; v <= 255; v++) {
            assertTrue(seen[v], "Value " + v + " never appears in EXP_TABLE");
        }
    }
}
