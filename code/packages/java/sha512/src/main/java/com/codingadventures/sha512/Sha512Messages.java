package com.codingadventures.sha512;

/**
 * Centralized error and diagnostic messages for SHA-512 (Standing Rule #20).
 */
public final class Sha512Messages {
    public static final String ERR_NULL_DATA = "sha512: input data must not be null";
    public static final String ERR_INDEX_OUT_OF_BOUNDS = "sha512: offset or length out of bounds";
    public static final String ERR_INVALID_BYTE = "sha512: byte value out of range 0..255";
    public static final String ERR_MESSAGE_TOO_LONG = "sha512: message length must be less than 2^128 bits";
    public static final String ERR_CORPUS_MALFORMED = "sha512: malformed fixture corpus row";
    public static final String ERR_CORPUS_OVERSIZED = "sha512: fixture corpus file exceeds 4096 bytes";
    public static final String ERR_UNKNOWN = "sha512: unknown error";

    private Sha512Messages() {
    }

    public static String messageFor(Sha512ErrorCode code) {
        if (code == null) {
            return ERR_UNKNOWN;
        }
        return switch (code) {
            case NULL_DATA -> ERR_NULL_DATA;
            case INDEX_OUT_OF_BOUNDS -> ERR_INDEX_OUT_OF_BOUNDS;
            case INVALID_BYTE -> ERR_INVALID_BYTE;
            case MESSAGE_TOO_LONG -> ERR_MESSAGE_TOO_LONG;
            case CORPUS_MALFORMED -> ERR_CORPUS_MALFORMED;
            case CORPUS_OVERSIZED -> ERR_CORPUS_OVERSIZED;
            case UNKNOWN -> ERR_UNKNOWN;
        };
    }
}
