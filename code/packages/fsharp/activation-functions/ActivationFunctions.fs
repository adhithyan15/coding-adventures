namespace CodingAdventures.ActivationFunctions

open System

// ActivationFunctions.fs -- The non-linear turns in an otherwise linear road
// ===========================================================================
//
// A neural network alternates between:
//
//   1. linear mixing   -- weighted sums
//   2. non-linear bend -- activation functions
//
// Without step 2, the entire network collapses into a single linear transform.
// This file implements the three scalar activations that show up over and over
// in introductory machine learning:
//
//   sigmoid -- probability-like squashing to (0, 1)
//   relu    -- keep positives, discard negatives
//   tanh    -- zero-centered squashing to (-1, 1)
//
// Each function also exposes its derivative so the same package is useful in
// both the forward pass and the backward pass of training.

[<RequireQualifiedAccess>]
module ActivationFunctions =
    // exp(710) is already too large for float, so we clamp before calling Exp.
    let private sigmoidOverflowClamp = 709.0
    let private leakyReluSlope = 0.01

    /// Return the input unchanged.
    let linear (x: float) = x

    /// Return the constant slope of the identity function.
    let linearDerivative (_x: float) = 1.0

    /// Compute the logistic sigmoid.
    ///
    /// sigma(x) = 1 / (1 + e^-x)
    ///
    /// The result is always between zero and one, which is why sigmoid often
    /// appears in binary-classification output layers.
    let sigmoid (x: float) =
        if x < -sigmoidOverflowClamp then
            0.0
        elif x > sigmoidOverflowClamp then
            1.0
        else
            1.0 / (1.0 + Math.Exp(-x))

    /// Compute sigma(x) * (1 - sigma(x)).
    ///
    /// The derivative is largest in the middle and smallest in the saturated
    /// tails, which is the textbook source of vanishing gradients.
    let sigmoidDerivative (x: float) =
        let s = sigmoid x
        s * (1.0 - s)

    /// Compute the rectified linear unit: max(0, x).
    ///
    /// ReLU became the default hidden-layer choice because it is cheap and
    /// preserves positive magnitudes instead of compressing them.
    let relu (x: float) = max 0.0 x

    /// Return the slope of ReLU.
    ///
    /// The function is piecewise linear: slope 1 to the right of zero, slope 0
    /// to the left. At exactly zero we follow the common convention of 0.
    let reluDerivative (x: float) =
        if x > 0.0 then
            1.0
        else
            0.0

    /// Compute Leaky ReLU with the spec default negative slope of 0.01.
    let leakyRelu (x: float) =
        if x > 0.0 then
            x
        else
            leakyReluSlope * x

    /// Return the slope of Leaky ReLU.
    let leakyReluDerivative (x: float) =
        if x > 0.0 then
            1.0
        else
            leakyReluSlope

    /// Compute tanh(x), the zero-centered sibling of sigmoid.
    let tanh (x: float) = Math.Tanh(x)

    /// Compute 1 - tanh(x)^2.
    ///
    /// This derivative is convenient because it can be expressed directly in
    /// terms of the activation value.
    let tanhDerivative (x: float) =
        let t = Math.Tanh(x)
        1.0 - (t * t)

    /// Compute Softplus using a stable absolute-value formulation.
    let softplus (x: float) =
        Math.Log(1.0 + Math.Exp(-abs x)) + max x 0.0

    /// Compute the derivative of Softplus, which is sigmoid.
    let softplusDerivative (x: float) = sigmoid x
