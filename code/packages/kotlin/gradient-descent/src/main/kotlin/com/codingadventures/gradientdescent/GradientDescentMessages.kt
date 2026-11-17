package com.codingadventures.gradientdescent

/**
 * Every error id, error message and placeholder used by the gradient-descent
 * package, collected in one place (Standing Rule #20: zero string literals in
 * operational logic).
 */
object GradientDescentMessages {
    const val ERROR_NULL_INPUT: String = "null-input"
    const val ERROR_LENGTH_MISMATCH: String = "length-mismatch"
    const val ERROR_EMPTY_INPUT: String = "empty-input"
    const val ERROR_INPUT_TOO_LARGE: String = "input-too-large"
    const val ERROR_NON_FINITE_LEARNING_RATE: String = "non-finite-learning-rate"
    const val ERROR_NON_FINITE_INPUT: String = "non-finite-input"
    const val ERROR_NUMERIC_OVERFLOW: String = "numeric-overflow"

    const val MESSAGE_NULL_INPUT: String = "weights and gradients must not be null"
    const val MESSAGE_LENGTH_MISMATCH: String = "weights and gradients must have the same length"
    const val MESSAGE_EMPTY_INPUT: String = "weights and gradients must not be empty"
    const val MESSAGE_INPUT_TOO_LARGE: String = "input vectors exceed the maximum supported length"
    const val MESSAGE_NON_FINITE_LEARNING_RATE: String = "learning rate must be a finite number"
    const val MESSAGE_NON_FINITE_INPUT: String = "weights and gradients must be finite numbers"
    const val MESSAGE_NUMERIC_OVERFLOW: String = "updated weight is not a finite number"

    const val REDACTED_PLACEHOLDER: String = "<redacted>"
    const val INVALID_PLACEHOLDER: String = "<invalid>"
}
