package com.codingadventures.canonicalcbor;

/** A stable, payload-blind CBR01 conformance error. */
public final class CborException extends Exception {
    private static final long serialVersionUID = 1L;
    private final String id;

    CborException(String id) {
        super(messageFor(id));
        this.id = id;
    }

    /** The language-neutral error identifier from CBR01. */
    public String id() {
        return id;
    }

    private static String messageFor(String id) {
        return switch (id) {
            case "unexpected-eof" -> "canonical-cbor: unexpected end of input";
            case "trailing-bytes" -> "canonical-cbor: trailing bytes after decoded item";
            case "reserved" -> "canonical-cbor: reserved additional-info value";
            case "indefinite" -> "canonical-cbor: indefinite item rejected";
            case "non-minimal-integer" -> "canonical-cbor: argument is not in smallest form";
            case "invalid-utf8" -> "canonical-cbor: text is not valid UTF-8";
            case "non-canonical-map-order" -> "canonical-cbor: map key order is not canonical";
            case "unsupported-simple" -> "canonical-cbor: unsupported simple value";
            case "float-not-supported" -> "canonical-cbor: floats are not supported";
            case "too-deep" -> "canonical-cbor: decoded nesting is too deep";
            case "length-too-large" -> "canonical-cbor: declared length is too large";
            case "duplicate-map-key" -> "canonical-cbor: duplicate canonical map key";
            case "encode-too-deep" -> "canonical-cbor: encoded nesting is too deep";
            case "encode-too-large" -> "canonical-cbor: encoded item is too large";
            default -> throw new IllegalArgumentException("unknown canonical CBOR error identifier");
        };
    }
}
