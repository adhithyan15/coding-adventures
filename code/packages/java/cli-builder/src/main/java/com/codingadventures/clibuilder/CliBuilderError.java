package com.codingadventures.clibuilder;

/** Base type for all CLI Builder failures. */
public class CliBuilderError extends RuntimeException {
    public CliBuilderError(String message) {
        super(message);
    }

    public CliBuilderError(String message, Throwable cause) {
        super(message, cause);
    }
}
