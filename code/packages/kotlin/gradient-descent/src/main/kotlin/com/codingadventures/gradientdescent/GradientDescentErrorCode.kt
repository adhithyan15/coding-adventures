package com.codingadventures.gradientdescent

/**
 * Closed enumeration of every way [GradientDescent.sgd] can refuse its input.
 */
enum class GradientDescentErrorCode(val errorId: String, val message: String) {
    NULL_INPUT(GradientDescentMessages.ERROR_NULL_INPUT, GradientDescentMessages.MESSAGE_NULL_INPUT),
    LENGTH_MISMATCH(GradientDescentMessages.ERROR_LENGTH_MISMATCH, GradientDescentMessages.MESSAGE_LENGTH_MISMATCH),
    EMPTY_INPUT(GradientDescentMessages.ERROR_EMPTY_INPUT, GradientDescentMessages.MESSAGE_EMPTY_INPUT),
    INPUT_TOO_LARGE(GradientDescentMessages.ERROR_INPUT_TOO_LARGE, GradientDescentMessages.MESSAGE_INPUT_TOO_LARGE),
    NON_FINITE_LEARNING_RATE(GradientDescentMessages.ERROR_NON_FINITE_LEARNING_RATE, GradientDescentMessages.MESSAGE_NON_FINITE_LEARNING_RATE),
    NON_FINITE_INPUT(GradientDescentMessages.ERROR_NON_FINITE_INPUT, GradientDescentMessages.MESSAGE_NON_FINITE_INPUT),
    NUMERIC_OVERFLOW(GradientDescentMessages.ERROR_NUMERIC_OVERFLOW, GradientDescentMessages.MESSAGE_NUMERIC_OVERFLOW);
}
