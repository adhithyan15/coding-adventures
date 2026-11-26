package com.codingadventures.logicgates;

/**
 * Thrown when a gate receives an argument that is not the bit 0 or 1.
 *
 * <p>It extends {@link IllegalArgumentException}, so existing callers that
 * catch that type keep working; callers who want to tell "bad bit" apart
 * from unrelated argument errors (e.g. AND_N with fewer than two inputs)
 * can catch this class instead.
 *
 * <p>Java widens char, byte and short to int, so {@code AND('a', 1)}
 * compiles and is rejected at runtime with value 97.
 */
public final class InvalidBitException extends IllegalArgumentException {
    private static final long serialVersionUID = 1L;

    private final String parameter;
    private final int value;

    /**
     * @param parameter name of the offending parameter ("a", "b", "bit[2]")
     * @param value     the rejected value
     */
    public InvalidBitException(String parameter, int value) {
        super(parameter + " must be 0 or 1, got: " + value);
        this.parameter = parameter;
        this.value = value;
    }

    /** @return the name of the first invalid parameter. */
    public String getParameter() {
        return parameter;
    }

    /** @return the rejected value. */
    public int getValue() {
        return value;
    }
}
