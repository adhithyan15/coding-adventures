package com.codingadventures.polynomial;

/**
 * A set of arithmetic operations over an abstract coefficient field.
 *
 * <p>The polynomial package is field-agnostic: the same long-division, GCD, and
 * evaluation algorithms work over the real numbers, over integers, or over
 * GF(256). The caller supplies a {@code FieldOps} instance that defines how
 * coefficients are added, subtracted, multiplied, and divided.
 *
 * <p>Two built-in implementations are provided:
 * <ul>
 *   <li>{@link FieldOps#REAL} — ordinary double arithmetic (suitable for
 *       polynomial long-division over ℝ)</li>
 *   <li>{@link FieldOps#GF256} — Galois Field GF(2^8) arithmetic (XOR for
 *       add/subtract, log-table multiply for multiply/divide)</li>
 * </ul>
 *
 * <h2>Why this matters</h2>
 *
 * <p>Reed-Solomon encoding and decoding perform polynomial long-division over
 * GF(256). Using GF256 field ops here lets the same {@link Polynomial} divide
 * and GCD code work for both ordinary (real-number) examples in the spec
 * tutorial and for actual RS arithmetic.
 *
 * <p>Example: "zero" in GF(256) is the integer 0, and "equality to zero" must
 * use {@code == 0} on the {@code int}-valued coefficients. The {@link #isZero}
 * method abstracts this check so it works the same for both {@code double}
 * (which has {@code -0.0 == 0.0} as a subtlety) and {@code int} GF(256).
 */
public interface FieldOps {

    /**
     * Add two coefficients.
     *
     * <p>Over ℝ: {@code a + b}. Over GF(256): {@code a ^ b} (XOR).
     */
    int add(int a, int b);

    /**
     * Subtract coefficient b from coefficient a.
     *
     * <p>Over ℝ: {@code a - b}. Over GF(256): {@code a ^ b} (same as add).
     */
    int sub(int a, int b);

    /**
     * Multiply two coefficients.
     *
     * <p>Over ℝ: {@code a * b}. Over GF(256): log/antilog table lookup.
     */
    int mul(int a, int b);

    /**
     * Divide coefficient a by coefficient b.
     *
     * <p>Over ℝ: {@code a / b}. Over GF(256): log/antilog table lookup.
     *
     * @throws ArithmeticException if b is zero
     */
    int div(int a, int b);

    /**
     * Return true if this coefficient is the zero element of the field.
     *
     * <p>For integer fields (GF(256)) this is {@code a == 0}.
     * For floating-point this should tolerate tiny rounding errors.
     */
    boolean isZero(int a);

    // =========================================================================
    // Built-in implementations
    // =========================================================================

    /**
     * Integer arithmetic for GF(256) polynomial operations.
     *
     * <p>Coefficients are bytes in the range [0, 255].
     * All four arithmetic operations call through to {@link com.codingadventures.gf256.GF256}.
     */
    FieldOps GF256_OPS = new FieldOps() {
        @Override public int add(int a, int b) { return com.codingadventures.gf256.GF256.add(a, b); }
        @Override public int sub(int a, int b) { return com.codingadventures.gf256.GF256.sub(a, b); }
        @Override public int mul(int a, int b) { return com.codingadventures.gf256.GF256.mul(a, b); }
        @Override public int div(int a, int b) { return com.codingadventures.gf256.GF256.div(a, b); }
        @Override public boolean isZero(int a) { return a == 0; }
    };

    /**
     * Integer arithmetic over the ordinary integers (no modular reduction).
     *
     * <p>Useful for tutorial examples and tests with small integers that do not
     * overflow a Java {@code int}. Not suitable for large polynomials where
     * integer overflow could occur.
     *
     * <p>Division performs integer (truncating) division — this is correct for
     * polynomial long division when the dividend always divides evenly (as it
     * must when performing exact polynomial division, e.g., dividing
     * {@code (x+2)(x+3)} by {@code (x+2)}).
     *
     * <p>For the spec's real-number long-division examples see the test suite,
     * which uses this ops object to verify the worked examples from MA00.
     */
    FieldOps INTEGER_OPS = new FieldOps() {
        @Override public int add(int a, int b) { return a + b; }
        @Override public int sub(int a, int b) { return a - b; }
        @Override public int mul(int a, int b) { return a * b; }
        @Override public int div(int a, int b) {
            if (b == 0) throw new ArithmeticException("polynomial division by zero coefficient");
            return a / b;
        }
        @Override public boolean isZero(int a) { return a == 0; }
    };
}
