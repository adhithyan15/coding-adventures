// ============================================================================
// ActivationFunctions.kt — Non-Linear Transforms for Neural Networks
// ============================================================================
//
// Activation functions introduce non-linearity into neural networks.  Without
// them, stacking multiple linear layers collapses into a single linear
// transformation — the network could only learn straight lines.
//
// This module implements three fundamental activation functions:
//   • Sigmoid — maps any real number to a probability in (0, 1)
//   • ReLU   — kills negatives, passes positives unchanged
//   • Tanh   — like sigmoid but zero-centred, range (-1, 1)
//
// Each function has a companion derivative for use in backpropagation.
// All functions are pure scalar operations: one Double in, one Double out.
//
// Layer: ML04 (machine-learning layer 4 — leaf package, zero dependencies)
// Spec:  code/specs/ML04-activation-functions.md
// ============================================================================

package com.codingadventures.activationfunctions

import kotlin.math.exp
import kotlin.math.abs
import kotlin.math.ln1p
import kotlin.math.max
import kotlin.math.tanh

/**
 * Activation functions and their derivatives for neural networks.
 *
 * All functions are pure, stateless scalar operations.
 */
object ActivationFunctions {
    private const val LEAKY_RELU_SLOPE = 0.01

    // ========================================================================
    // Linear: x
    // ========================================================================

    /** Return the input unchanged. */
    fun linear(x: Double): Double = x

    /** Derivative of linear activation: 1 everywhere. */
    fun linearDerivative(x: Double): Double = 1.0

    // ========================================================================
    // Sigmoid: σ(x) = 1 / (1 + e^(-x))
    // ========================================================================

    /** Compute sigmoid: σ(x) = 1 / (1 + e^(-x)). Clamps for |x| > 709. */
    fun sigmoid(x: Double): Double {
        if (x < -709.0) return 0.0
        if (x > 709.0) return 1.0
        return 1.0 / (1.0 + exp(-x))
    }

    /** Derivative: σ'(x) = σ(x) · (1 − σ(x)) */
    fun sigmoidDerivative(x: Double): Double {
        val s = sigmoid(x)
        return s * (1.0 - s)
    }

    // ========================================================================
    // ReLU: max(0, x)
    // ========================================================================

    /** Compute ReLU: max(0, x) */
    fun relu(x: Double): Double = max(0.0, x)

    /** Derivative: 1 if x > 0, 0 otherwise (0 at x=0 by convention). */
    fun reluDerivative(x: Double): Double = if (x > 0.0) 1.0 else 0.0

    // ========================================================================
    // Leaky ReLU: x if x > 0, otherwise 0.01x
    // ========================================================================

    /** Compute Leaky ReLU with the spec default negative slope of 0.01. */
    fun leakyRelu(x: Double): Double = if (x > 0.0) x else LEAKY_RELU_SLOPE * x

    /** Derivative: 1 if x > 0, 0.01 otherwise. */
    fun leakyReluDerivative(x: Double): Double = if (x > 0.0) 1.0 else LEAKY_RELU_SLOPE

    // ========================================================================
    // Tanh: (e^x - e^(-x)) / (e^x + e^(-x))
    // ========================================================================

    /** Compute tanh(x). */
    fun tanh(x: Double): Double = kotlin.math.tanh(x)

    /** Derivative: tanh'(x) = 1 − tanh²(x) */
    fun tanhDerivative(x: Double): Double {
        val t = kotlin.math.tanh(x)
        return 1.0 - t * t
    }

    // ========================================================================
    // Softplus: log(1 + e^x)
    // ========================================================================

    /** Compute Softplus using a numerically stable log1p formulation. */
    fun softplus(x: Double): Double = ln1p(exp(-abs(x))) + max(x, 0.0)

    /** Derivative: sigmoid(x). */
    fun softplusDerivative(x: Double): Double = sigmoid(x)
}
