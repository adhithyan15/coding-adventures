package com.codingadventures.gradientdescent;

/**
 * ML02 gradient-descent optimizers.
 *
 * <p>An optimizer turns gradients (how much the loss changes when each weight
 * nudges upward) into updated weights. This package currently implements the
 * bedrock optimizer, vanilla Stochastic Gradient Descent:
 *
 * <pre>
 *     w_new = w_old - (learningRate * gradient)
 * </pre>
 *
 * <p>Every function is pure: inputs are never mutated and a fresh array is
 * always returned.
 */
public final class GradientDescent {
    private GradientDescent() {}

    /**
     * One step of vanilla Stochastic Gradient Descent.
     *
     * <p>Validation is fail-closed and runs in a fixed precedence order; the
     * first failing rule wins:
     * <ol>
     *   <li>{@code null-input}: either array is {@code null}</li>
     *   <li>{@code length-mismatch}: the arrays differ in length</li>
     *   <li>{@code empty-input}: the arrays are empty</li>
     *   <li>{@code input-too-large}: longer than
     *       {@link GradientDescentConstants#MAX_VECTOR_LENGTH}</li>
     *   <li>{@code non-finite-learning-rate}: learning rate is NaN or infinite</li>
     *   <li>{@code non-finite-input}: any weight or gradient is NaN or infinite</li>
     *   <li>{@code numeric-overflow}: any updated weight is not finite</li>
     * </ol>
     *
     * @param weights      current weights (not modified)
     * @param gradients    partial derivatives of the loss, one per weight (not modified)
     * @param learningRate step size, {@code alpha}
     * @return a new array of updated weights
     * @throws InvalidGradientDescentInputException if any rule above fails
     */
    public static double[] sgd(double[] weights, double[] gradients, double learningRate) {
        if (weights == null || gradients == null) {
            throw new InvalidGradientDescentInputException(GradientDescentErrorCode.NULL_INPUT);
        }
        if (weights.length != gradients.length) {
            throw new InvalidGradientDescentInputException(GradientDescentErrorCode.LENGTH_MISMATCH);
        }
        if (weights.length == 0) {
            throw new InvalidGradientDescentInputException(GradientDescentErrorCode.EMPTY_INPUT);
        }
        // CWE-400: refuse oversized vectors BEFORE allocating the output buffer,
        // so the caller cannot make us reserve unbounded memory.
        if (weights.length > GradientDescentConstants.MAX_VECTOR_LENGTH) {
            throw new InvalidGradientDescentInputException(GradientDescentErrorCode.INPUT_TOO_LARGE);
        }
        // NaN/Inf poisoning: a single NaN learning rate would silently turn every
        // weight into NaN, and NaN then spreads through every later training step.
        if (!Double.isFinite(learningRate)) {
            throw new InvalidGradientDescentInputException(
                    GradientDescentErrorCode.NON_FINITE_LEARNING_RATE);
        }
        // Full pre-pass over ALL inputs before computing anything, so the reported
        // error never depends on where in the vector the bad value happens to sit
        // relative to an overflowing element.
        for (int i = 0; i < weights.length; i++) {
            if (!Double.isFinite(weights[i]) || !Double.isFinite(gradients[i])) {
                throw new InvalidGradientDescentInputException(
                        GradientDescentErrorCode.NON_FINITE_INPUT);
            }
        }

        // Alias safety: the output is always a brand-new buffer, and each element
        // reads both inputs into locals before anything is written. sgd(x, x, lr)
        // is therefore correct and leaves x untouched.
        double[] updatedWeights = new double[weights.length];
        for (int i = 0; i < weights.length; i++) {
            double weight = weights[i];
            double gradient = gradients[i];
            // Unfused rounding determinism: two separately rounded IEEE-754
            // binary64 operations (multiply, then subtract). We deliberately do
            // NOT use Math.fma: a fused multiply-add rounds once, which would
            // produce different bits from every other language lane.
            double step = learningRate * gradient;
            double updated = weight - step;
            // Overflow refusal: finite inputs can still overflow (for example
            // 1e308 - (-1.0 * 1e308) = +Inf). Returning +/-Inf would poison the
            // model, so we fail closed instead. A non-finite step always yields a
            // non-finite update (finite weight minus infinity), so checking the
            // update alone also catches an overflowing step.
            if (!Double.isFinite(updated)) {
                throw new InvalidGradientDescentInputException(
                        GradientDescentErrorCode.NUMERIC_OVERFLOW);
            }
            updatedWeights[i] = updated;
        }
        return updatedWeights;
    }
}
