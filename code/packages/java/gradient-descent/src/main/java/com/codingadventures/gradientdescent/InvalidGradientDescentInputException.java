package com.codingadventures.gradientdescent;

import java.util.Objects;

/**
 * Thrown when {@link GradientDescent#sgd} refuses its input.
 *
 * <p>The only constructor accepts a closed {@link GradientDescentErrorCode};
 * there is deliberately no constructor that takes a free-form message. The
 * exception therefore exposes nothing but the code, its id and its static
 * message: never the offending weights, gradients, learning rate, lengths or
 * indices (CWE-209).
 */
public final class InvalidGradientDescentInputException extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;

    private final GradientDescentErrorCode code;

    public InvalidGradientDescentInputException(GradientDescentErrorCode code) {
        super(Objects.requireNonNull(code).getMessage());
        this.code = code;
    }

    /** Stable, machine-readable id such as {@code "non-finite-input"}. */
    public String getErrorId() {
        return code.getErrorId();
    }

    /** The closed error code that caused this exception. */
    public GradientDescentErrorCode getCode() {
        return code;
    }
}
