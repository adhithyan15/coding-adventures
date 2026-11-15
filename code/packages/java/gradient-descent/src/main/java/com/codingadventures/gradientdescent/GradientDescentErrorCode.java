package com.codingadventures.gradientdescent;

/**
 * Closed enumeration of every way {@link GradientDescent#sgd} can refuse its
 * input. Each constant carries its stable id and its static message, both
 * sourced from {@link GradientDescentMessages}.
 *
 * <p>Because callers can only pick one of these constants, there is no way to
 * smuggle a payload-bearing string into an error (Standing Rule #20).
 */
public enum GradientDescentErrorCode {
    NULL_INPUT(GradientDescentMessages.ERROR_NULL_INPUT,
            GradientDescentMessages.MESSAGE_NULL_INPUT),
    LENGTH_MISMATCH(GradientDescentMessages.ERROR_LENGTH_MISMATCH,
            GradientDescentMessages.MESSAGE_LENGTH_MISMATCH),
    EMPTY_INPUT(GradientDescentMessages.ERROR_EMPTY_INPUT,
            GradientDescentMessages.MESSAGE_EMPTY_INPUT),
    INPUT_TOO_LARGE(GradientDescentMessages.ERROR_INPUT_TOO_LARGE,
            GradientDescentMessages.MESSAGE_INPUT_TOO_LARGE),
    NON_FINITE_LEARNING_RATE(GradientDescentMessages.ERROR_NON_FINITE_LEARNING_RATE,
            GradientDescentMessages.MESSAGE_NON_FINITE_LEARNING_RATE),
    NON_FINITE_INPUT(GradientDescentMessages.ERROR_NON_FINITE_INPUT,
            GradientDescentMessages.MESSAGE_NON_FINITE_INPUT),
    NUMERIC_OVERFLOW(GradientDescentMessages.ERROR_NUMERIC_OVERFLOW,
            GradientDescentMessages.MESSAGE_NUMERIC_OVERFLOW);

    private final String errorId;
    private final String message;

    GradientDescentErrorCode(String errorId, String message) {
        this.errorId = errorId;
        this.message = message;
    }

    /** Stable, machine-readable id such as {@code "length-mismatch"}. */
    public String getErrorId() {
        return errorId;
    }

    /** Static, payload-blind, human-readable message. */
    public String getMessage() {
        return message;
    }
}
