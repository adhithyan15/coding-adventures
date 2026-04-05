// ============================================================================
// LossFunctions.swift — Fundamental Error Functions for Machine Learning
// ============================================================================
//
// Loss functions measure how far a model's predictions are from the true
// values.  During training, the optimizer (gradient descent) uses the loss
// to decide which direction to adjust the model's weights.
//
// This module implements four standard loss functions plus their derivatives:
//
//   ┌───────────────┬────────────┬─────────────────────────────────────────┐
//   │ Function      │ Task       │ Intuition                               │
//   ├───────────────┼────────────┼─────────────────────────────────────────┤
//   │ MSE           │ Regression │ Squares errors — big mistakes hurt a lot│
//   │ MAE           │ Regression │ Absolute errors — treats all equally    │
//   │ BCE           │ Binary     │ Log-loss for yes/no predictions         │
//   │ CCE           │ Multi-cls  │ Log-loss for one-hot category vectors   │
//   └───────────────┴────────────┴─────────────────────────────────────────┘
//
// Why build loss from scratch?
// ----------------------------
// Every ML framework (PyTorch, TensorFlow, etc.) ships these as built-in
// functions.  By implementing them from first principles we see that they
// are just simple arithmetic over arrays — no magic.
//
// Layer: ML01 (machine-learning layer 1 — leaf package, zero dependencies)
// Spec:  code/specs/ML01-loss-functions.md
// ============================================================================

import Foundation

/// Namespace for loss functions and their derivatives.
///
/// All functions are pure and stateless — they take two equal-length arrays
/// of `Double` and return either a scalar loss or an array of gradients.
public struct LossFunctions {
    private init() {}

    // ========================================================================
    // MARK: - Epsilon Clamping
    // ========================================================================
    //
    // BCE and CCE use `log(ŷ)`.  If ŷ is exactly 0 or 1, the logarithm is
    // undefined (−∞) or the denominator in the BCE derivative is zero.
    // We clamp predictions into [ε, 1−ε] to avoid this.  The standard
    // choice across frameworks is ε = 1e-7.
    // ========================================================================

    private static let epsilon: Double = 1e-7

    /// Clamp a value to [ε, 1−ε] so logarithms and divisions stay finite.
    private static func clamp(_ v: Double) -> Double {
        Swift.min(Swift.max(v, epsilon), 1.0 - epsilon)
    }

    // ========================================================================
    // MARK: - Input Validation
    // ========================================================================

    /// Shared precondition: arrays must be non-empty and equal length.
    private static func validate(_ yTrue: [Double], _ yPred: [Double]) {
        precondition(!yTrue.isEmpty, "yTrue must not be empty")
        precondition(yTrue.count == yPred.count,
                     "yTrue and yPred must have equal length (got \(yTrue.count) vs \(yPred.count))")
    }

    // ========================================================================
    // MARK: - Mean Squared Error (MSE)
    // ========================================================================
    //
    // MSE = (1/n) Σ (yᵢ − ŷᵢ)²
    //
    // Squaring the difference means:
    //   • Errors are always positive (no cancellation between +/−).
    //   • Large errors are penalised quadratically — an error of 2 costs
    //     4× as much as an error of 1.
    //
    // This makes MSE sensitive to outliers, which is desirable when big
    // mistakes are truly worse (e.g. predicting house prices).
    // ========================================================================

    /// Compute Mean Squared Error between true labels and predictions.
    ///
    /// - Parameters:
    ///   - yTrue: Ground truth values.
    ///   - yPred: Model predictions (same length as `yTrue`).
    /// - Returns: The average squared difference.
    public static func mse(_ yTrue: [Double], _ yPred: [Double]) -> Double {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        var sum = 0.0
        for i in 0..<yTrue.count {
            let diff = yTrue[i] - yPred[i]
            sum += diff * diff
        }
        return sum / n
    }

    /// Derivative of MSE with respect to each prediction ŷᵢ.
    ///
    /// d(MSE)/d(ŷᵢ) = (2/n)(ŷᵢ − yᵢ)
    ///
    /// Notice the sign flip: the derivative is positive when the prediction
    /// overshoots (ŷ > y), telling gradient descent to decrease ŷ.
    public static func mseDerivative(_ yTrue: [Double], _ yPred: [Double]) -> [Double] {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        return (0..<yTrue.count).map { i in
            2.0 * (yPred[i] - yTrue[i]) / n
        }
    }

    // ========================================================================
    // MARK: - Mean Absolute Error (MAE)
    // ========================================================================
    //
    // MAE = (1/n) Σ |yᵢ − ŷᵢ|
    //
    // Unlike MSE, MAE treats all errors linearly — an error of 2 costs
    // exactly 2× as much as an error of 1.  This makes MAE more robust
    // to outliers but harder to optimise (the gradient doesn't increase
    // near the solution, so convergence can be slower).
    // ========================================================================

    /// Compute Mean Absolute Error between true labels and predictions.
    public static func mae(_ yTrue: [Double], _ yPred: [Double]) -> Double {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        var sum = 0.0
        for i in 0..<yTrue.count {
            sum += abs(yTrue[i] - yPred[i])
        }
        return sum / n
    }

    /// Derivative of MAE with respect to each prediction ŷᵢ.
    ///
    /// d(MAE)/d(ŷᵢ) = (1/n) sign(ŷᵢ − yᵢ)
    ///
    /// The gradient is +1/n or −1/n everywhere (undefined at ŷ = y, where
    /// we return 0 by convention).
    public static func maeDerivative(_ yTrue: [Double], _ yPred: [Double]) -> [Double] {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        return (0..<yTrue.count).map { i in
            let diff = yPred[i] - yTrue[i]
            if diff > 0 { return 1.0 / n }
            if diff < 0 { return -1.0 / n }
            return 0.0
        }
    }

    // ========================================================================
    // MARK: - Binary Cross-Entropy (BCE)
    // ========================================================================
    //
    // BCE = −(1/n) Σ [ yᵢ·log(ŷᵢ) + (1−yᵢ)·log(1−ŷᵢ) ]
    //
    // Cross-entropy comes from information theory.  It measures how many
    // extra bits you'd need to encode outcomes from the true distribution
    // using a code optimised for the predicted distribution.  When the
    // prediction matches the truth perfectly, BCE equals zero.
    //
    // BCE is the standard loss for binary classification (spam/not-spam,
    // sick/healthy).  yTrue is 0 or 1, and yPred is a probability in (0,1).
    // ========================================================================

    /// Compute Binary Cross-Entropy loss.
    ///
    /// Predictions are clamped to [ε, 1−ε] to avoid log(0).
    public static func bce(_ yTrue: [Double], _ yPred: [Double]) -> Double {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        var sum = 0.0
        for i in 0..<yTrue.count {
            let p = clamp(yPred[i])
            sum += yTrue[i] * log(p) + (1.0 - yTrue[i]) * log(1.0 - p)
        }
        return -sum / n
    }

    /// Derivative of BCE with respect to each prediction ŷᵢ.
    ///
    /// d(BCE)/d(ŷᵢ) = (1/n) · (ŷᵢ − yᵢ) / (ŷᵢ · (1 − ŷᵢ))
    public static func bceDerivative(_ yTrue: [Double], _ yPred: [Double]) -> [Double] {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        return (0..<yTrue.count).map { i in
            let p = clamp(yPred[i])
            return (p - yTrue[i]) / (p * (1.0 - p)) / n
        }
    }

    // ========================================================================
    // MARK: - Categorical Cross-Entropy (CCE)
    // ========================================================================
    //
    // CCE = −(1/n) Σ yᵢ·log(ŷᵢ)
    //
    // CCE generalises BCE to more than two classes.  yTrue is a one-hot
    // vector (e.g. [0, 0, 1] for class 2 of 3) and yPred is a probability
    // distribution over all classes (e.g. [0.1, 0.1, 0.8]).
    //
    // Only the term where yᵢ = 1 contributes, so CCE effectively measures
    // −log(predicted probability of the correct class).
    // ========================================================================

    /// Compute Categorical Cross-Entropy loss.
    ///
    /// Predictions are clamped to [ε, 1−ε] to avoid log(0).
    public static func cce(_ yTrue: [Double], _ yPred: [Double]) -> Double {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        var sum = 0.0
        for i in 0..<yTrue.count {
            let p = clamp(yPred[i])
            sum += yTrue[i] * log(p)
        }
        return -sum / n
    }

    /// Derivative of CCE with respect to each prediction ŷᵢ.
    ///
    /// d(CCE)/d(ŷᵢ) = −(1/n) · yᵢ / ŷᵢ
    public static func cceDerivative(_ yTrue: [Double], _ yPred: [Double]) -> [Double] {
        validate(yTrue, yPred)
        let n = Double(yTrue.count)
        return (0..<yTrue.count).map { i in
            let p = clamp(yPred[i])
            return -yTrue[i] / p / n
        }
    }
}
