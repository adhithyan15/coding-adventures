package com.codingadventures.gradientdescent;

/**
 * Named constants for the gradient-descent package (Standing Rule #20: zero
 * magic numbers in operational logic).
 */
public final class GradientDescentConstants {
    private GradientDescentConstants() {}

    /** Package version, kept in lockstep with every other ML02 language lane. */
    public static final String VERSION = "0.1.0";

    /**
     * Largest vector length {@link GradientDescent#sgd} will accept (2^20).
     *
     * <p>The length is checked <em>before</em> the output buffer is allocated,
     * so a hostile caller cannot make the library reserve an unbounded amount
     * of memory on its behalf (CWE-400, uncontrolled resource consumption).
     */
    public static final int MAX_VECTOR_LENGTH = 1_048_576;
}
