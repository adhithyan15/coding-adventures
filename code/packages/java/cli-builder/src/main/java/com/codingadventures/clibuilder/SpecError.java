package com.codingadventures.clibuilder;

/** Raised when a CLI Builder JSON specification is invalid. */
public final class SpecError extends CliBuilderError {
    public SpecError(String message) {
        super(message);
    }

    public SpecError(String message, Throwable cause) {
        super(message, cause);
    }

    @Override
    public String getMessage() {
        return "CliBuilder spec error: " + super.getMessage();
    }
}
