package com.codingadventures.gradientdescent

/**
 * ML02 gradient-descent optimizers.
 */
object GradientDescent {
    /**
     * One step of vanilla Stochastic Gradient Descent.
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
        if (weights.size > GradientDescentConstants.MAX_VECTOR_LENGTH) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.INPUT_TOO_LARGE)
        }
        if (!learningRate.isFinite()) {
            throw InvalidGradientDescentInputException(GradientDescentErrorCode.NON_FINITE_LEARNING_RATE)
        }
        for (i in weights.indices) {
            if (!weights[i].isFinite() || !gradients[i].isFinite()) {
                throw InvalidGradientDescentInputException(GradientDescentErrorCode.NON_FINITE_INPUT)
            }
        }

        val updatedWeights = DoubleArray(weights.size)
        for (i in weights.indices) {
            val weight = weights[i]
            val gradient = gradients[i]
            val step = learningRate * gradient
            val updated = weight - step
            if (!updated.isFinite()) {
                throw InvalidGradientDescentInputException(GradientDescentErrorCode.NUMERIC_OVERFLOW)
            }
            updatedWeights[i] = updated
        }
        return updatedWeights
    }
}
