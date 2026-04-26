// ============================================================================
// ActivationFunctions.swift — Non-Linear Transforms for Neural Networks
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
//   ├��─────────┼���─────────────┼───────────���──────────────────��───────────┤
//   │ Sigmoid  │ (0, 1)       │ Maps any real number to a probability    │
//   │ ReLU     │ [0, ∞)       │ Kills negatives, passes positives       │
//   │ Tanh     │ (-1, 1)      │ Like sigmoid but zero-centred            │
//   └──────────┴��─────────────┴────────────────────────────��─────────────┘
//
// Each function has a companion derivative for use in backpropagation.
// All functions are pure scalar operations: one Double in, one Double out.
//
// Why build from scratch?
// -----------------------
// When you call a framework's `sigmoid()`, it runs the same formula we
// implement here.  By writing it ourselves, we see there's no magic —
// just exp(), max(), and basic arithmetic.
//
// Layer: ML04 (machine-learning layer 4 — leaf package, zero dependencies)
// Spec:  code/specs/ML04-activation-functions.md
// ============================================================================

import Foundation

/// Namespace for activation functions and their derivatives.
///
/// All functions are pure, stateless scalar operations.
public struct ActivationFunctions {
    private init() {}
    private static let leakyReluSlope = 0.01

    // ========================================================================
    // MARK: - Linear
    // ========================================================================

    /// Return the input unchanged.
    public static func linear(_ x: Double) -> Double {
        return x
    }

    /// Derivative of linear activation: 1 everywhere.
    public static func linearDerivative(_ x: Double) -> Double {
        return 1.0
    }

    // ========================================================================
    // MARK: - Sigmoid
    // ========================================================================
    //
    // σ(x) = 1 / (1 + e^(-x))
    //
    // The sigmoid function squashes any real number into (0, 1).  For very
    // negative x the output approaches 0; for very positive x it approaches
    // 1; at x = 0 it returns exactly 0.5.
    //
    // Historical note: sigmoid was the original activation function, inspired
    // by the firing rate of biological neurons.  It fell out of favour for
    // hidden layers (vanishing gradients) but remains standard for binary
    // classification output layers.
    //
    // Overflow protection: exp(-x) overflows Double for x < -709, so we
    // clamp to 0.0 / 1.0 at the extremes.
    // ========================================================================

    /// Compute the sigmoid function: σ(x) = 1 / (1 + e^(-x))
    ///
    /// - Parameter x: any real number
    /// - Returns: a value in (0, 1)
    public static func sigmoid(_ x: Double) -> Double {
        if x < -709.0 { return 0.0 }
        if x > 709.0 { return 1.0 }
        return 1.0 / (1.0 + exp(-x))
    }

    /// Derivative of sigmoid: σ'(x) = σ(x) · (1 − σ(x))
    ///
    /// The maximum derivative is 0.25, occurring at x = 0.  This means
    /// gradients shrink by at least 4× per layer — the "vanishing gradient"
    /// problem that makes sigmoid unsuitable for deep hidden layers.
    public static func sigmoidDerivative(_ x: Double) -> Double {
        let s = sigmoid(x)
        return s * (1.0 - s)
    }

    // ========================================================================
    // MARK: - ReLU (Rectified Linear Unit)
    // ========================================================================
    //
    // ReLU(x) = max(0, x)
    //
    // ReLU is the simplest and most widely used activation in modern deep
    // learning.  It passes positive values unchanged and zeros out negatives.
    //
    // Why ReLU dominates:
    //   • Gradient is 1 for positive inputs — no vanishing gradient
    //   • Computationally trivial (just a comparison)
    //   • Induces sparsity (many neurons output exactly 0)
    //
    // The "dying ReLU" problem: if a neuron's input is always negative,
    // its gradient is always 0 and it never updates.  Variants like Leaky
    // ReLU address this, but standard ReLU remains the default starting
    // point.
    // ========================================================================

    /// Compute ReLU: max(0, x)
    ///
    /// - Parameter x: any real number
    /// - Returns: x if positive, 0 otherwise
    public static func relu(_ x: Double) -> Double {
        return max(0.0, x)
    }

    /// Derivative of ReLU: 1 if x > 0, 0 otherwise.
    ///
    /// Technically undefined at x = 0; by convention we return 0.
    public static func reluDerivative(_ x: Double) -> Double {
        return x > 0.0 ? 1.0 : 0.0
    }

    // ========================================================================
    // MARK: - Leaky ReLU
    // ========================================================================

    /// Compute Leaky ReLU with the spec default negative slope of 0.01.
    public static func leakyRelu(_ x: Double) -> Double {
        return x > 0.0 ? x : leakyReluSlope * x
    }

    /// Derivative of Leaky ReLU: 1 if x > 0, 0.01 otherwise.
    public static func leakyReluDerivative(_ x: Double) -> Double {
        return x > 0.0 ? 1.0 : leakyReluSlope
    }

    // ========================================================================
    // MARK: - Tanh (Hyperbolic Tangent)
    // ========================================================================
    //
    // tanh(x) = (e^x − e^(-x)) / (e^x + e^(-x))
    //
    // Tanh is a rescaled sigmoid: tanh(x) = 2σ(2x) − 1.  Its output is
    // centred at zero (unlike sigmoid's 0.5), which often leads to faster
    // convergence because gradients are better balanced.
    //
    // Maximum derivative is 1.0 at x = 0, which is better than sigmoid's
    // 0.25 but still causes vanishing gradients in very deep networks.
    // ========================================================================

    /// Compute tanh(x) using the standard library.
    ///
    /// - Parameter x: any real number
    /// - Returns: a value in (-1, 1)
    public static func tanh(_ x: Double) -> Double {
        return Foundation.tanh(x)
    }

    /// Derivative of tanh: tanh'(x) = 1 − tanh²(x)
    public static func tanhDerivative(_ x: Double) -> Double {
        let t = Foundation.tanh(x)
        return 1.0 - t * t
    }

    // ========================================================================
    // MARK: - Softplus
    // ========================================================================

    /// Compute Softplus using a numerically stable log1p formulation.
    public static func softplus(_ x: Double) -> Double {
        return log1p(exp(-abs(x))) + max(x, 0.0)
    }

    /// Derivative of Softplus: sigmoid(x).
    public static func softplusDerivative(_ x: Double) -> Double {
        return sigmoid(x)
    }
}
