package com.codingadventures.itf;

import java.util.Objects;

/**
 * Thrown when ITF input fails validation against the ITF specification.
 */
public final class InvalidItfInputException extends IllegalArgumentException {
    private final ItfErrorCode errorId;

    public InvalidItfInputException(ItfErrorCode errorId) {
        super(ItfMessages.messageFor(Objects.requireNonNull(errorId)));
        this.errorId = errorId;
    }

    public String getErrorId() {
        return errorId.getErrorId();
    }

    public ItfErrorCode getErrorCode() {
        return errorId;
    }
}
