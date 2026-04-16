using System;
using System.Collections.Generic;

namespace CodingAdventures.LossFunctions;

// LossFunctions.cs -- Measuring how wrong a model is, one vector at a time
// =========================================================================
//
// Training a model is impossible without a scoring rule. The optimizer asks:
//
//   "Given the prediction we just made and the truth we wanted, how bad was it?"
//
// That scoring rule is the loss function. This package implements four of the
// most common ones:
//
//   MSE -- mean squared error, the classic regression loss
//   MAE -- mean absolute error, more robust to outliers
//   BCE -- binary cross-entropy, for yes/no classification
//   CCE -- categorical cross-entropy, for one-hot multi-class classification
//
// Each loss also has a derivative with respect to y_pred, because backpropagation
// needs to know not only the scalar error but the direction in which each
// prediction should move.

/// <summary>
/// Pure vector-to-scalar loss functions plus vector derivatives.
/// </summary>
public static class LossFunctions
{
    // log(0) is undefined, so probabilities are clamped away from the boundary.
    private const double Epsilon = 1e-7;

    /// <summary>
    /// Compute mean squared error.
    ///
    /// MSE = (1 / n) * sum((y_true - y_pred)^2)
    ///
    /// Squaring makes large mistakes disproportionately expensive, which is why
    /// MSE is the standard regression loss.
    /// </summary>
    public static double Mse(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var sum = 0.0;
        for (var i = 0; i < yTrue.Count; i++)
        {
            var diff = yTrue[i] - yPred[i];
            sum += diff * diff;
        }

        return sum / yTrue.Count;
    }

    /// <summary>
    /// Compute mean absolute error.
    ///
    /// MAE = (1 / n) * sum(abs(y_true - y_pred))
    ///
    /// Because the error is not squared, MAE reacts more gently to outliers.
    /// </summary>
    public static double Mae(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var sum = 0.0;
        for (var i = 0; i < yTrue.Count; i++)
        {
            sum += Math.Abs(yTrue[i] - yPred[i]);
        }

        return sum / yTrue.Count;
    }

    /// <summary>
    /// Compute binary cross-entropy for binary labels and probabilities.
    ///
    /// BCE = -(1 / n) * sum(y * log(p) + (1 - y) * log(1 - p))
    ///
    /// The prediction p is clamped to [epsilon, 1 - epsilon] so that log never
    /// sees an exact zero.
    /// </summary>
    public static double Bce(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var sum = 0.0;
        for (var i = 0; i < yTrue.Count; i++)
        {
            var probability = ClampProbability(yPred[i]);
            sum += yTrue[i] * Math.Log(probability) +
                   (1.0 - yTrue[i]) * Math.Log(1.0 - probability);
        }

        return -sum / yTrue.Count;
    }

    /// <summary>
    /// Compute categorical cross-entropy for a one-hot target vector.
    ///
    /// CCE = -(1 / n) * sum(y * log(p))
    ///
    /// Only the entries whose ground-truth value is non-zero contribute to the
    /// sum, which mirrors the "probability assigned to the correct class"
    /// intuition.
    /// </summary>
    public static double Cce(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var sum = 0.0;
        for (var i = 0; i < yTrue.Count; i++)
        {
            var probability = ClampProbability(yPred[i]);
            sum += yTrue[i] * Math.Log(probability);
        }

        return -sum / yTrue.Count;
    }

    /// <summary>
    /// Compute the gradient of mean squared error with respect to the prediction.
    ///
    /// d/dy_pred MSE = (2 / n) * (y_pred - y_true)
    /// </summary>
    public static double[] MseDerivative(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var gradient = new double[yTrue.Count];
        var scale = 2.0 / yTrue.Count;
        for (var i = 0; i < yTrue.Count; i++)
        {
            gradient[i] = scale * (yPred[i] - yTrue[i]);
        }

        return gradient;
    }

    /// <summary>
    /// Compute the gradient of mean absolute error.
    ///
    /// The derivative is the sign of (y_pred - y_true) divided by n. Exactly
    /// at zero the derivative is undefined, and the conventional subgradient is 0.
    /// </summary>
    public static double[] MaeDerivative(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var gradient = new double[yTrue.Count];
        var scale = 1.0 / yTrue.Count;
        for (var i = 0; i < yTrue.Count; i++)
        {
            if (yPred[i] > yTrue[i])
            {
                gradient[i] = scale;
            }
            else if (yPred[i] < yTrue[i])
            {
                gradient[i] = -scale;
            }
            else
            {
                gradient[i] = 0.0;
            }
        }

        return gradient;
    }

    /// <summary>
    /// Compute the gradient of binary cross-entropy.
    ///
    /// d/dy_pred BCE = (1 / n) * (p - y) / (p * (1 - p))
    ///
    /// We use the clamped probability in both the numerator and denominator so
    /// that the derivative stays finite at the probability boundaries.
    /// </summary>
    public static double[] BceDerivative(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var gradient = new double[yTrue.Count];
        var scale = 1.0 / yTrue.Count;
        for (var i = 0; i < yTrue.Count; i++)
        {
            var probability = ClampProbability(yPred[i]);
            gradient[i] = scale * ((probability - yTrue[i]) / (probability * (1.0 - probability)));
        }

        return gradient;
    }

    /// <summary>
    /// Compute the gradient of categorical cross-entropy.
    ///
    /// d/dy_pred CCE = -(1 / n) * y / p
    ///
    /// Again, p is clamped so that division by zero cannot occur.
    /// </summary>
    public static double[] CceDerivative(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ValidateVectors(yTrue, yPred);

        var gradient = new double[yTrue.Count];
        var scale = -1.0 / yTrue.Count;
        for (var i = 0; i < yTrue.Count; i++)
        {
            var probability = ClampProbability(yPred[i]);
            gradient[i] = scale * (yTrue[i] / probability);
        }

        return gradient;
    }

    private static void ValidateVectors(IReadOnlyList<double> yTrue, IReadOnlyList<double> yPred)
    {
        ArgumentNullException.ThrowIfNull(yTrue);
        ArgumentNullException.ThrowIfNull(yPred);

        if (yTrue.Count == 0 || yTrue.Count != yPred.Count)
        {
            throw new ArgumentException("Vectors must have the same non-zero length.");
        }
    }

    private static double ClampProbability(double probability) =>
        Math.Max(Epsilon, Math.Min(1.0 - Epsilon, probability));
}
