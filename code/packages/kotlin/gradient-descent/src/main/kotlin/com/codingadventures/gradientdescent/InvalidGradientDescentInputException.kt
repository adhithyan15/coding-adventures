package com.codingadventures.gradientdescent

/**
 * Thrown when [GradientDescent.sgd] refuses its input.
 */
class InvalidGradientDescentInputException(val code: GradientDescentErrorCode) :
    IllegalArgumentException(code.message) {
    val errorId: String
        get() = code.errorId
}
