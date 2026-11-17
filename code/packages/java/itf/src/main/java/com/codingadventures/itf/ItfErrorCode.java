package com.codingadventures.itf;

/**
 * Closed enumeration of valid ITF validation error codes.
 */
public enum ItfErrorCode {
    INPUT_TOO_LONG(ItfMessages.ERROR_INPUT_TOO_LONG),
    INVALID_LENGTH(ItfMessages.ERROR_INVALID_LENGTH),
    INVALID_CHARACTER(ItfMessages.ERROR_INVALID_CHARACTER);

    private final String errorId;

    ItfErrorCode(String errorId) {
        this.errorId = errorId;
    }

    public String getErrorId() {
        return errorId;
    }
}
