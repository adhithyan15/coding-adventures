package com.codingadventures.sha512;

/**
 * Payload-blind exception thrown by SHA-512 operations with a closed error code (Standing Rule #20).
 */
public final class Sha512Exception extends RuntimeException {
    private static final long serialVersionUID = 1L;
    private final Sha512ErrorCode code;

    public Sha512Exception(Sha512ErrorCode code) {
        super(Sha512Messages.messageFor(code));
        this.code = code != null ? code : Sha512ErrorCode.UNKNOWN;
    }

    public Sha512ErrorCode getCode() {
        return code;
    }
}
