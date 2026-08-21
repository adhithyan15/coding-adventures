package com.codingadventures.imagecodecpng;

/** A stable, payload-blind IC18 PNG failure. */
public final class PngError extends RuntimeException {
    private final String code;

    /** Create an error whose message is exactly its portable code. */
    public PngError(String code) {
        super(code);
        this.code = code;
    }

    /** Return the stable portable failure identifier. */
    public String code() {
        return code;
    }
}
