namespace CodingAdventures.LossFunctions

open System

// LossFunctions.fs -- Converting "that prediction was bad" into math
// ==================================================================
//
// A model can only improve if we can score its mistakes. Loss functions turn a
// pair of vectors:
//
//   yTrue -- what should have happened
//   yPred -- what the model predicted
//
// into either:
//
//   1. a single scalar error value, or
//   2. a gradient telling each prediction which direction to move next
//
// This module implements four standard losses:
//
//   mse -- mean squared error
//   mae -- mean absolute error
//   bce -- binary cross-entropy
//   cce -- categorical cross-entropy

[<RequireQualifiedAccess>]
module LossFunctions =
    // Cross-entropy uses logarithms, so we clamp away from the undefined point log(0).
    let private epsilon = 1e-7

    let private validateInputs (yTrue: float array) (yPred: float array) =
        if yTrue.Length = 0 || yTrue.Length <> yPred.Length then
            invalidArg "yPred" "Vectors must have the same non-zero length."

    let private clampProbability probability =
        max epsilon (min (1.0 - epsilon) probability)

    /// Compute mean squared error.
    ///
    /// MSE = (1 / n) * sum((y_true - y_pred)^2)
    ///
    /// Squaring makes large errors dominate the total, which is why MSE is so
    /// common in regression tasks.
    let mse (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let mutable sum = 0.0
        for i in 0 .. yTrue.Length - 1 do
            let diff = yTrue[i] - yPred[i]
            sum <- sum + (diff * diff)

        sum / float yTrue.Length

    /// Compute mean absolute error.
    ///
    /// MAE = (1 / n) * sum(abs(y_true - y_pred))
    ///
    /// Without the square, outliers matter less than they do under MSE.
    let mae (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let mutable sum = 0.0
        for i in 0 .. yTrue.Length - 1 do
            sum <- sum + abs (yTrue[i] - yPred[i])

        sum / float yTrue.Length

    /// Compute binary cross-entropy.
    ///
    /// BCE = -(1 / n) * sum(y * log(p) + (1 - y) * log(1 - p))
    ///
    /// Probabilities are clamped so that logarithms stay finite at the edges.
    let bce (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let mutable sum = 0.0
        for i in 0 .. yTrue.Length - 1 do
            let probability = clampProbability yPred[i]
            sum <-
                sum
                + (yTrue[i] * Math.Log(probability))
                + ((1.0 - yTrue[i]) * Math.Log(1.0 - probability))

        -sum / float yTrue.Length

    /// Compute categorical cross-entropy for a one-hot target vector.
    ///
    /// CCE = -(1 / n) * sum(y * log(p))
    let cce (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let mutable sum = 0.0
        for i in 0 .. yTrue.Length - 1 do
            let probability = clampProbability yPred[i]
            sum <- sum + (yTrue[i] * Math.Log(probability))

        -sum / float yTrue.Length

    /// Compute the gradient of mean squared error.
    ///
    /// d/dy_pred MSE = (2 / n) * (y_pred - y_true)
    let mseDerivative (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let scale = 2.0 / float yTrue.Length
        Array.init yTrue.Length (fun i -> scale * (yPred[i] - yTrue[i]))

    /// Compute the gradient of mean absolute error.
    ///
    /// The derivative is the sign of the error divided by n. At zero we use the
    /// conventional subgradient 0.
    let maeDerivative (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let scale = 1.0 / float yTrue.Length

        Array.init yTrue.Length (fun i ->
            if yPred[i] > yTrue[i] then
                scale
            elif yPred[i] < yTrue[i] then
                -scale
            else
                0.0)

    /// Compute the gradient of binary cross-entropy.
    ///
    /// d/dy_pred BCE = (1 / n) * (p - y) / (p * (1 - p))
    let bceDerivative (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let scale = 1.0 / float yTrue.Length

        Array.init yTrue.Length (fun i ->
            let probability = clampProbability yPred[i]
            scale * ((probability - yTrue[i]) / (probability * (1.0 - probability))))

    /// Compute the gradient of categorical cross-entropy.
    ///
    /// d/dy_pred CCE = -(1 / n) * y / p
    let cceDerivative (yTrue: float array) (yPred: float array) =
        validateInputs yTrue yPred

        let scale = -1.0 / float yTrue.Length

        Array.init yTrue.Length (fun i ->
            let probability = clampProbability yPred[i]
            scale * (yTrue[i] / probability))
