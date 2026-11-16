package com.codingadventures.sha512;

/**
 * Closed enumeration of error codes for SHA-512 operations (Standing Rule #20).
 */
public enum Sha512ErrorCode {
    INVALID_BYTE,
    INDEX_OUT_OF_BOUNDS,
    MESSAGE_TOO_LONG,
    NULL_DATA,
    CORPUS_MALFORMED,
    CORPUS_OVERSIZED,
    UNKNOWN
}
