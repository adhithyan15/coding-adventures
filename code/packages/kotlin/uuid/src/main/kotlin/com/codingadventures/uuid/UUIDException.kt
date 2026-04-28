package com.codingadventures.uuid

/**
 * Thrown when a UUID string cannot be parsed or a UUID value is invalid.
 *
 * Extends [IllegalArgumentException] so callers that don't explicitly catch
 * this subclass still get a reasonable unchecked exception.
 */
class UUIDException(message: String, cause: Throwable? = null) :
    IllegalArgumentException(message, cause)
