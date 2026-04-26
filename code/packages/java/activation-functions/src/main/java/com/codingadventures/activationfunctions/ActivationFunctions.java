// ============================================================================
// ActivationFunctions.java — Non-Linear Transforms for Neural Networks
// ============================================================================
//
// Activation functions introduce non-linearity into neural networks.  Without
// them, stacking multiple linear layers collapses into a single linear
// transformation — the network could only learn straight lines.
//
// This module implements three fundamental activation functions:
//
//   ┌──────────┬──────────────┬──────────────────────────────────────────┐
//   │ Function │ Range        │ Intuition                                │
//   ├──────────┼──────────────┼──────────────────────────────────────────┤
//   │ Sigmoid  │ (0, 1)       │ Maps any real number to a probability    │
//   │ ReLU     │ [0, ∞)       │ Kills negatives, passes positives       │
//   │ Tanh     │ (-1, 1)      │ Like sigmoid but zero-centred            │
//   └──────────┴──────────────┴──────────────────────────────────────────┘
//
// Each function has a companion derivative for use in backpropagation.
// All functions are pure scalar operations: one double in, one double out.
//
// Layer: ML04 (machine-learning layer 4 — leaf package, zero dependencies)
// Spec:  code/specs/ML04-activation-functions.md
// ============================================================================

package com.codingadventures.activationfunctions;

/**
 * Activation functions and their derivatives for neural networks.
 *
 * <p>All methods are pure, stateless scalar operations.</p>
 */
public final class ActivationFunctions {

    private static final double LEAKY_RELU_SLOPE = 0.01;

    private ActivationFunctions() {}

    // ========================================================================
    // Linear: x
    // ========================================================================

    /** Return the input unchanged. */
    public static double linear(double x) {
        return x;
    }

    /** Derivative of linear activation: 1 everywhere. */
    public static double linearDerivative(double x) {
        return 1.0;
    }

    // ========================================================================
    // Sigmoid: σ(x) = 1 / (1 + e^(-x))
    // ========================================================================

    /**
     * Compute sigmoid: σ(x) = 1 / (1 + e^(-x))
     *
     * <p>Clamps to 0.0/1.0 for |x| > 709 to avoid overflow.</p>
     */
    public static double sigmoid(double x) {
        if (x < -709.0) return 0.0;
        if (x > 709.0) return 1.0;
        return 1.0 / (1.0 + Math.exp(-x));
    }

    /** Derivative: σ'(x) = σ(x) · (1 − σ(x)) */
    public static double sigmoidDerivative(double x) {
        double s = sigmoid(x);
        return s * (1.0 - s);
    }

    // ========================================================================
    // ReLU: max(0, x)
    // ========================================================================

    /** Compute ReLU: max(0, x) */
    public static double relu(double x) {
        return Math.max(0.0, x);
    }

    /** Derivative: 1 if x > 0, 0 otherwise (0 at x=0 by convention). */
    public static double reluDerivative(double x) {
        return x > 0.0 ? 1.0 : 0.0;
    }

    // ========================================================================
    // Leaky ReLU: x if x > 0, otherwise 0.01x
    // ========================================================================

    /** Compute Leaky ReLU with the spec default negative slope of 0.01. */
    public static double leakyRelu(double x) {
        return x > 0.0 ? x : LEAKY_RELU_SLOPE * x;
    }

    /** Derivative: 1 if x > 0, 0.01 otherwise. */
    public static double leakyReluDerivative(double x) {
        return x > 0.0 ? 1.0 : LEAKY_RELU_SLOPE;
    }

    // ========================================================================
    // Tanh: (e^x - e^(-x)) / (e^x + e^(-x))
    // ========================================================================

    /** Compute tanh(x). */
    public static double tanh(double x) {
        return Math.tanh(x);
    }

    /** Derivative: tanh'(x) = 1 − tanh²(x) */
    public static double tanhDerivative(double x) {
        double t = Math.tanh(x);
        return 1.0 - t * t;
    }

    // ========================================================================
    // Softplus: log(1 + e^x)
    // ========================================================================

    /** Compute Softplus using a numerically stable log1p formulation. */
    public static double softplus(double x) {
        return Math.log1p(Math.exp(-Math.abs(x))) + Math.max(x, 0.0);
    }

    /** Derivative: sigmoid(x). */
    public static double softplusDerivative(double x) {
        return sigmoid(x);
    }
}
