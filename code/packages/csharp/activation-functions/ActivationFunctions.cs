using System;

namespace CodingAdventures.ActivationFunctions;

// ActivationFunctions.cs -- The three non-linear gates that make neural networks useful
// ======================================================================================
//
// A stack of linear layers is still just one big linear layer. That is the central
// reason activation functions exist. They bend the output of a neuron so that the next
// layer receives something richer than "just another weighted sum".
//
// This package starts with the three most common scalar activations:
//
//   sigmoid(x)  -- squeezes any real number into the probability-like range (0, 1)
//   relu(x)     -- keeps positive values, clips negative values to zero
//   tanh(x)     -- squeezes to (-1, 1) and is centered at zero
//
// Each function also exposes its derivative, because gradient descent needs both the
// forward pass ("what value do we emit?") and the backward pass ("how sensitive is the
// loss to a small change in this input?").

/// <summary>
/// Pure scalar activation functions plus their derivatives.
/// </summary>
public static class ActivationFunctions
{
    // exp(710) overflows a double, so we clamp before calling Math.Exp.
    private const double SigmoidOverflowClamp = 709.0;
    private const double LeakyReluSlope = 0.01;

    /// <summary>
    /// Return the input unchanged.
    /// </summary>
    public static double Linear(double x) => x;

    /// <summary>
    /// Return the constant slope of the identity function.
    /// </summary>
    public static double LinearDerivative(double x) => 1.0;

    /// <summary>
    /// Compute the logistic sigmoid.
    ///
    /// sigma(x) = 1 / (1 + e^-x)
    ///
    /// The output is useful as a probability because it always stays between
    /// zero and one. Large negative inputs saturate near 0, large positive
    /// inputs saturate near 1, and x = 0 maps to 0.5.
    /// </summary>
    public static double Sigmoid(double x)
    {
        if (x < -SigmoidOverflowClamp)
        {
            return 0.0;
        }

        if (x > SigmoidOverflowClamp)
        {
            return 1.0;
        }

        return 1.0 / (1.0 + Math.Exp(-x));
    }

    /// <summary>
    /// Compute sigma(x) * (1 - sigma(x)).
    ///
    /// This derivative is largest at x = 0 and shrinks near the saturated tails,
    /// which is the classic "vanishing gradient" behavior of sigmoid networks.
    /// </summary>
    public static double SigmoidDerivative(double x)
    {
        var sigmoid = Sigmoid(x);
        return sigmoid * (1.0 - sigmoid);
    }

    /// <summary>
    /// Compute the rectified linear unit: max(0, x).
    ///
    /// ReLU is popular because it is simple, cheap, and does not compress the
    /// positive half-line. Positive signals keep their scale; negative ones are
    /// treated as "the neuron did not fire".
    /// </summary>
    public static double Relu(double x) => Math.Max(0.0, x);

    /// <summary>
    /// Return the slope of ReLU.
    ///
    /// ReLU is piecewise linear: slope 1 on the positive side, slope 0 on the
    /// negative side. At exactly zero the mathematical derivative is undefined,
    /// and the common machine-learning convention is to return 0.
    /// </summary>
    public static double ReluDerivative(double x) => x > 0.0 ? 1.0 : 0.0;

    /// <summary>
    /// Compute Leaky ReLU with the spec default negative slope of 0.01.
    /// </summary>
    public static double LeakyRelu(double x) => x > 0.0 ? x : LeakyReluSlope * x;

    /// <summary>
    /// Return the slope of Leaky ReLU.
    /// </summary>
    public static double LeakyReluDerivative(double x) => x > 0.0 ? 1.0 : LeakyReluSlope;

    /// <summary>
    /// Compute tanh(x), the zero-centered sibling of sigmoid.
    ///
    /// tanh maps any real number to (-1, 1). Because its midpoint is 0 rather
    /// than 0.5, it often behaves better in hidden layers whose activations
    /// should stay balanced around zero.
    /// </summary>
    public static double Tanh(double x) => Math.Tanh(x);

    /// <summary>
    /// Compute 1 - tanh(x)^2.
    ///
    /// tanh has a convenient derivative because the answer can be written in
    /// terms of the activation itself rather than re-deriving the exponential
    /// quotient from scratch.
    /// </summary>
    public static double TanhDerivative(double x)
    {
        var tanh = Math.Tanh(x);
        return 1.0 - (tanh * tanh);
    }

    /// <summary>
    /// Compute Softplus using a stable absolute-value formulation.
    /// </summary>
    public static double Softplus(double x) => Math.Log(1.0 + Math.Exp(-Math.Abs(x))) + Math.Max(x, 0.0);

    /// <summary>
    /// Compute the derivative of Softplus, which is sigmoid.
    /// </summary>
    public static double SoftplusDerivative(double x) => Sigmoid(x);
}
