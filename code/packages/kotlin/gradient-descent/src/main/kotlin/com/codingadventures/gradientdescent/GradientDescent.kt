package com.codingadventures.gradientdescent

/**
 * ML02 gradient-descent optimizers.
 *
 * An optimizer turns gradients (how much the loss changes when each weight
 * nudges upward) into updated weights. This package currently implements the
 * bedrock optimizer, vanilla Stochastic Gradient Descent:
 *
 *     w_new = w_old - (learningRate * gradient)
 *
 * Every function is pure: inputs are never mutated and a fresh array is
 * always returned.
 */
object GradientDescent {
    /**
     * One step of vanilla Stochastic Gradient Descent.
     *
     * Validation is fail-closed and runs in a fixed precedence order; the
     * first failing rule wins:
     * 1. null-input: either array is null
     * 2. length-mismatch: the arrays differ in length
     * 3. empty-input: the arrays are empty
     * 4. input-too-large: longer than [GradientDescentConstants.MAX_VECTOR_LENGTH]
     * 5. non-finite-learning-rate: learning rate is NaN or infinite
     * 6. non-finite-input: any weight or gradient is NaN or infinite
     * 7. numeric-overflow: any updated weight is not finite
     *
     * Note: weights and gradients are typed as DoubleArray? so callers passing null
     * trigger NULL_INPUT instead of a standard Kotlin NullPointerException.
     */
    @JvmStatic
    fun sgd(weights: DoubleArray?, gradients: DoubleArray?, learningRate: Double): DoubleArray {
        if (weights == null || gradients == null) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.NULL_INPUT)
        }
        if (weights.size != gradients.size) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.LENGTH_MISMATCH)
        }
        if (weights.isEmpty()) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.EMPTY_INPUT)
        }
        // CWE-400: refuse oversized vectors BEFORE allocating the output buffer,
        // so the caller cannot make us reserve unbounded memory.
        if (weights.size > GradientDescentConstants.MAX_VECTOR_LENGTH) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.INPUT_TOO_LARGE)
        }
        // NaN/Inf poisoning: a single NaN learning rate would silently turn every
        // weight into NaN, and NaN then spreads through every later training step.
        if (!learningRate.isFinite()) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE)
        }
        // Full pre-pass over ALL inputs before computing anything, so the reported
        // error never depends on where in the vector the bad value happens to sit
        // relative to an overflowing element.
        for (i in weights.indices) {
            if (!weights[i].isFinite() || !gradients[i].isFinite()) {
                throw InvalidGradientDescentInputException(GradientDescentErrorCode.NON_FINITE_INPUT)
            }
        }

        // Alias safety: the output is always a brand-new buffer, and each element
        // reads both inputs into locals before anything is written. sgd(x, x, lr)
        // is therefore correct and leaves x untouched.
        val updatedWeights = DoubleArray(weights.size)
        for (i in weights.indices) {
            val weight = weights[i]
            val gradient = gradients[i]
            // Unfused rounding determinism: two separately rounded IEEE-754
            // binary64 operations (multiply, then subtract). We deliberately do
            // NOT use Math.fma: a fused multiply-add rounds once, which would
            // produce different bits from every other language lane.
            val step = learningRate * gradient
            val updated = weight - step
            // Overflow refusal: finite inputs can still overflow (for example
            // 1e308 - (-1.0 * 1e308) = +Inf). Returning +/-Inf would poison the
            // model, so we fail closed instead. A non-finite step always yields a
            // non-finite update (finite weight minus infinity), so checking the
            // update alone also catches an overflowing step.
            if (!updated.isFinite()) {
                throw InvalidGradientDescentInputException(GradientDescentErrorCode.NUMERIC_OVERFLOW)
            }
            updatedWeights[i] = updated
        }
        return updatedWeights
    }
}
