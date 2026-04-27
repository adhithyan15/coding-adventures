// ============================================================================
// LossFunctions.java — Fundamental Error Functions for Machine Learning
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

package com.codingadventures.lossfunctions;

/**
 * Namespace for loss functions and their derivatives.
 *
 * <p>All methods are pure and stateless — they take two equal-length arrays
 * of {@code double} and return either a scalar loss or an array of gradients.</p>
 */
public final class LossFunctions {

    private LossFunctions() {}

    // ========================================================================
    // Epsilon Clamping
    // ========================================================================
    //
    // BCE and CCE use log(ŷ).  If ŷ is exactly 0 or 1, the logarithm is
    // undefined (−∞) or the denominator in the BCE derivative is zero.
    // We clamp predictions into [ε, 1−ε] to avoid this.  The standard
    // choice across frameworks is ε = 1e-7.
    // ========================================================================

    private static final double EPSILON = 1e-7;

    private static double clamp(double v) {
        return Math.min(Math.max(v, EPSILON), 1.0 - EPSILON);
    }

    // ========================================================================
    // Input Validation
    // ========================================================================

    private static void validate(double[] yTrue, double[] yPred) {
        if (yTrue.length == 0) {
            throw new IllegalArgumentException("yTrue must not be empty");
        }
        if (yTrue.length != yPred.length) {
            throw new IllegalArgumentException(
                "yTrue and yPred must have equal length (got "
                + yTrue.length + " vs " + yPred.length + ")");
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

    /**
     * Compute Mean Squared Error between true labels and predictions.
     *
     * @param yTrue ground truth values
     * @param yPred model predictions (same length as yTrue)
     * @return the average squared difference
     */
    public static double mse(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double sum = 0.0;
        for (int i = 0; i < yTrue.length; i++) {
            double diff = yTrue[i] - yPred[i];
            sum += diff * diff;
        }
        return sum / yTrue.length;
    }

    /**
     * Derivative of MSE with respect to each prediction ŷᵢ.
     *
     * <p>d(MSE)/d(ŷᵢ) = (2/n)(ŷᵢ − yᵢ)</p>
     */
    public static double[] mseDerivative(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double n = yTrue.length;
        double[] grad = new double[yTrue.length];
        for (int i = 0; i < yTrue.length; i++) {
            grad[i] = 2.0 * (yPred[i] - yTrue[i]) / n;
        }
        return grad;
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

    /**
     * Compute Mean Absolute Error between true labels and predictions.
     */
    public static double mae(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double sum = 0.0;
        for (int i = 0; i < yTrue.length; i++) {
            sum += Math.abs(yTrue[i] - yPred[i]);
        }
        return sum / yTrue.length;
    }

    /**
     * Derivative of MAE with respect to each prediction ŷᵢ.
     *
     * <p>d(MAE)/d(ŷᵢ) = (1/n) sign(ŷᵢ − yᵢ)</p>
     */
    public static double[] maeDerivative(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double n = yTrue.length;
        double[] grad = new double[yTrue.length];
        for (int i = 0; i < yTrue.length; i++) {
            double diff = yPred[i] - yTrue[i];
            if (diff > 0) grad[i] = 1.0 / n;
            else if (diff < 0) grad[i] = -1.0 / n;
            else grad[i] = 0.0;
        }
        return grad;
    }

    // ========================================================================
    // Binary Cross-Entropy (BCE)
    // ========================================================================
    //
    // BCE = −(1/n) Σ [ yᵢ·log(ŷᵢ) + (1−yᵢ)·log(1−ŷᵢ) ]
    //
    // Cross-entropy comes from information theory.  It measures how many
    // extra bits you'd need to encode outcomes from the true distribution
    // using a code optimised for the predicted distribution.
    // ========================================================================

    /**
     * Compute Binary Cross-Entropy loss.
     *
     * <p>Predictions are clamped to [ε, 1−ε] to avoid log(0).</p>
     */
    public static double bce(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double sum = 0.0;
        for (int i = 0; i < yTrue.length; i++) {
            double p = clamp(yPred[i]);
            sum += yTrue[i] * Math.log(p) + (1.0 - yTrue[i]) * Math.log(1.0 - p);
        }
        return -sum / yTrue.length;
    }

    /**
     * Derivative of BCE with respect to each prediction ŷᵢ.
     *
     * <p>d(BCE)/d(ŷᵢ) = (1/n) · (ŷᵢ − yᵢ) / (ŷᵢ · (1 − ŷᵢ))</p>
     */
    public static double[] bceDerivative(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double n = yTrue.length;
        double[] grad = new double[yTrue.length];
        for (int i = 0; i < yTrue.length; i++) {
            double p = clamp(yPred[i]);
            grad[i] = (p - yTrue[i]) / (p * (1.0 - p)) / n;
        }
        return grad;
    }

    // ========================================================================
    // Categorical Cross-Entropy (CCE)
    // ========================================================================
    //
    // CCE = −(1/n) Σ yᵢ·log(ŷᵢ)
    //
    // CCE generalises BCE to more than two classes.  yTrue is a one-hot
    // vector and yPred is a probability distribution over all classes.
    // ========================================================================

    /**
     * Compute Categorical Cross-Entropy loss.
     *
     * <p>Predictions are clamped to [ε, 1−ε] to avoid log(0).</p>
     */
    public static double cce(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double sum = 0.0;
        for (int i = 0; i < yTrue.length; i++) {
            double p = clamp(yPred[i]);
            sum += yTrue[i] * Math.log(p);
        }
        return -sum / yTrue.length;
    }

    /**
     * Derivative of CCE with respect to each prediction ŷᵢ.
     *
     * <p>d(CCE)/d(ŷᵢ) = −(1/n) · yᵢ / ŷᵢ</p>
     */
    public static double[] cceDerivative(double[] yTrue, double[] yPred) {
        validate(yTrue, yPred);
        double n = yTrue.length;
        double[] grad = new double[yTrue.length];
        for (int i = 0; i < yTrue.length; i++) {
            double p = clamp(yPred[i]);
            grad[i] = -yTrue[i] / p / n;
        }
        return grad;
    }
}
