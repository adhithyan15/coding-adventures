package com.codingadventures.logicgates;

/**
 * Thrown when an N-ary gate receives an input array of the wrong shape:
 * null, too short (AND_N / OR_N need at least two inputs), or longer than
 * {@link LogicGates#MAX_INPUTS}.
 *
 * <p>It extends {@link IllegalArgumentException}, so existing callers keep
 * working. It is deliberately distinct from {@link InvalidBitException},
 * which means "an element was not 0 or 1".
 */
public final class InvalidShapeException extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;

    private final String parameter;

    /**
     * @param parameter name of the offending parameter ("inputs")
     * @param message   human-readable description of the shape problem
     */
    public InvalidShapeException(String parameter, String message) {
        super(parameter + ": " + message);
        this.parameter = parameter;
    }

    /** @return the name of the offending parameter. */
    public String getParameter() {
        return parameter;
    }
}
