// ============================================================================
// LossFunctions.kt — Fundamental Error Functions for Machine Learning
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

package com.codingadventures.lossfunctions

import kotlin.math.abs
import kotlin.math.ln
import kotlin.math.max
import kotlin.math.min

/**
 * Namespace for loss functions and their derivatives.
 *
 * All functions are pure and stateless — they take two equal-length arrays
 * of [Double] and return either a scalar loss or an array of gradients.
 */
object LossFunctions {

    // ========================================================================
    // Epsilon Clamping
    // ========================================================================
    //
    // BCE and CCE use ln(ŷ).  If ŷ is exactly 0 or 1, the logarithm is
    // undefined (−∞) or the denominator in the BCE derivative is zero.
    // We clamp predictions into [ε, 1−ε] to avoid this.  The standard
    // choice across frameworks is ε = 1e-7.
    // ========================================================================

    private const val EPSILON: Double = 1e-7

    private fun clamp(v: Double): Double = min(max(v, EPSILON), 1.0 - EPSILON)

    // ========================================================================
    // Input Validation
    // ========================================================================

    private fun validate(yTrue: DoubleArray, yPred: DoubleArray) {
        require(yTrue.isNotEmpty()) { "yTrue must not be empty" }
        require(yTrue.size == yPred.size) {
            "yTrue and yPred must have equal length (got ${yTrue.size} vs ${yPred.size})"
        }
    }

    // ========================================================================
    // Mean Squared Error (MSE)
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

    /** Compute Mean Squared Error between true labels and predictions. */
    fun mse(yTrue: DoubleArray, yPred: DoubleArray): Double {
        validate(yTrue, yPred)
        var sum = 0.0
        for (i in yTrue.indices) {
            val diff = yTrue[i] - yPred[i]
            sum += diff * diff
        }
        return sum / yTrue.size
    }

    /** Derivative of MSE: d(MSE)/d(ŷᵢ) = (2/n)(ŷᵢ − yᵢ) */
    fun mseDerivative(yTrue: DoubleArray, yPred: DoubleArray): DoubleArray {
        validate(yTrue, yPred)
        val n = yTrue.size.toDouble()
        return DoubleArray(yTrue.size) { i -> 2.0 * (yPred[i] - yTrue[i]) / n }
    }

    // ========================================================================
    // Mean Absolute Error (MAE)
    // ========================================================================
    //
    // MAE = (1/n) Σ |yᵢ − ŷᵢ|
    //
    // Unlike MSE, MAE treats all errors linearly — an error of 2 costs
    // exactly 2× as much as an error of 1.  This makes MAE more robust
    // to outliers but harder to optimise.
    // ========================================================================

    /** Compute Mean Absolute Error between true labels and predictions. */
    fun mae(yTrue: DoubleArray, yPred: DoubleArray): Double {
        validate(yTrue, yPred)
        var sum = 0.0
        for (i in yTrue.indices) {
            sum += abs(yTrue[i] - yPred[i])
        }
        return sum / yTrue.size
    }

    /** Derivative of MAE: d(MAE)/d(ŷᵢ) = (1/n) sign(ŷᵢ − yᵢ) */
    fun maeDerivative(yTrue: DoubleArray, yPred: DoubleArray): DoubleArray {
        validate(yTrue, yPred)
        val n = yTrue.size.toDouble()
        return DoubleArray(yTrue.size) { i ->
            val diff = yPred[i] - yTrue[i]
            when {
                diff > 0 -> 1.0 / n
                diff < 0 -> -1.0 / n
                else -> 0.0
            }
        }
    }

    // ========================================================================
    // Binary Cross-Entropy (BCE)
    // ========================================================================
    //
    // BCE = −(1/n) Σ [ yᵢ·ln(ŷᵢ) + (1−yᵢ)·ln(1−ŷᵢ) ]
    //
    // Cross-entropy comes from information theory.  It measures how many
    // extra bits you'd need to encode outcomes from the true distribution
    // using a code optimised for the predicted distribution.
    // ========================================================================

    /** Compute Binary Cross-Entropy loss. Predictions clamped to [ε, 1−ε]. */
    fun bce(yTrue: DoubleArray, yPred: DoubleArray): Double {
        validate(yTrue, yPred)
        var sum = 0.0
        for (i in yTrue.indices) {
            val p = clamp(yPred[i])
            sum += yTrue[i] * ln(p) + (1.0 - yTrue[i]) * ln(1.0 - p)
        }
        return -sum / yTrue.size
    }

    /** Derivative of BCE: d(BCE)/d(ŷᵢ) = (1/n) · (ŷᵢ − yᵢ) / (ŷᵢ · (1 − ŷᵢ)) */
    fun bceDerivative(yTrue: DoubleArray, yPred: DoubleArray): DoubleArray {
        validate(yTrue, yPred)
        val n = yTrue.size.toDouble()
        return DoubleArray(yTrue.size) { i ->
            val p = clamp(yPred[i])
            (p - yTrue[i]) / (p * (1.0 - p)) / n
        }
    }

    // ========================================================================
    // Categorical Cross-Entropy (CCE)
    // ========================================================================
    //
    // CCE = −(1/n) Σ yᵢ·ln(ŷᵢ)
    //
    // CCE generalises BCE to more than two classes.  yTrue is a one-hot
    // vector and yPred is a probability distribution over all classes.
    // ========================================================================

    /** Compute Categorical Cross-Entropy loss. Predictions clamped to [ε, 1−ε]. */
    fun cce(yTrue: DoubleArray, yPred: DoubleArray): Double {
        validate(yTrue, yPred)
        var sum = 0.0
        for (i in yTrue.indices) {
            val p = clamp(yPred[i])
            sum += yTrue[i] * ln(p)
        }
        return -sum / yTrue.size
    }

    /** Derivative of CCE: d(CCE)/d(ŷᵢ) = −(1/n) · yᵢ / ŷᵢ */
    fun cceDerivative(yTrue: DoubleArray, yPred: DoubleArray): DoubleArray {
        validate(yTrue, yPred)
        val n = yTrue.size.toDouble()
        return DoubleArray(yTrue.size) { i ->
            val p = clamp(yPred[i])
            -yTrue[i] / p / n
        }
    }
}
