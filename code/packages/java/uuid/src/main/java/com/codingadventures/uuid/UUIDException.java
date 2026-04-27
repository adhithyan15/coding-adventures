package com.codingadventures.uuid;

/**
 * Thrown when a UUID string cannot be parsed or a UUID value is invalid.
 *
 * <p>Extends {@link IllegalArgumentException} so callers that don't explicitly
 * catch this subclass still get a reasonable unchecked exception.
 */
public class UUIDException extends IllegalArgumentException {

    public UUIDException(String message) {
        super(message);
    }

    public UUIDException(String message, Throwable cause) {
        super(message, cause);
    }
}
