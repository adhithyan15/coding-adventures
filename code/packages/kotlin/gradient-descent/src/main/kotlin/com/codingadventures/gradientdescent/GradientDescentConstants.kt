package com.codingadventures.gradientdescent

/**
 * Named constants for the gradient-descent package (Standing Rule #20: zero
 * magic numbers in operational logic).
 */
object GradientDescentConstants {
    /** Package version, kept in lockstep with every other ML02 language lane. */
    const val VERSION: String = "0.1.0"

    /**
     * Largest vector length [GradientDescent.sgd] will accept (2^20).
     *
     * The length is checked *before* the output buffer is allocated,
     * so a hostile caller cannot make the library reserve an unbounded amount
     * of memory on its behalf (CWE-400, uncontrolled resource consumption).
     */
    const val MAX_VECTOR_LENGTH: Int = 1_048_576
}
