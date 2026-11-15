package com.codingadventures.itf;

/**
 * Diagnostic and error messages for ITF encoding, strictly payload-blind.
 */
public final class ItfMessages {
    private ItfMessages() {}

    public static final String ERROR_INPUT_TOO_LONG = "input-too-long";
    public static final String ERROR_INVALID_LENGTH = "invalid-length";
    public static final String ERROR_INVALID_CHARACTER = "invalid-character";

    public static final String MESSAGE_INPUT_TOO_LONG = "ITF input must contain at most 4096 characters";
    public static final String MESSAGE_INVALID_LENGTH = "ITF input must contain an even number of digits";
    public static final String MESSAGE_INVALID_CHARACTER = "ITF input must contain digits only";
    public static final String MESSAGE_INVALID_SCALARS = "ITF input must contain valid Unicode scalar values";
    public static final String MESSAGE_NULL_INPUT = "ITF input must not be null";

    public static final String REDACTED_PLACEHOLDER = "<redacted>";
    public static final String INVALID_PLACEHOLDER = "<invalid>";

    public static String messageFor(ItfErrorCode code) {
        if (code == null) {
            return MESSAGE_INVALID_CHARACTER;
        }
        return switch (code) {
            case INPUT_TOO_LONG -> MESSAGE_INPUT_TOO_LONG;
            case INVALID_LENGTH -> MESSAGE_INVALID_LENGTH;
            case INVALID_CHARACTER -> MESSAGE_INVALID_CHARACTER;
        };
    }
}
