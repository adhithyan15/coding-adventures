package com.codingadventures.x509extension;

import com.codingadventures.derasn1.DerAsn1;
import java.nio.ByteBuffer;
import java.util.Objects;

/** Bounded decoding for one generic RFC 5280 Extension. */
public final class X509Extension {
    private static final long BOOLEAN_TAG = 1;

    private X509Extension() {}

    /** Stable payload-free Extension failure identifiers. */
    public enum ErrorKind {
        STRUCTURE("structure"),
        MISSING_EXTENSION_ID("missing-extension-id"),
        INVALID_EXTENSION_ID("invalid-extension-id"),
        INVALID_CRITICAL("invalid-critical"),
        ENCODED_DEFAULT_CRITICAL("encoded-default-critical"),
        MISSING_EXTENSION_VALUE("missing-extension-value"),
        INVALID_EXTENSION_VALUE("invalid-extension-value"),
        TRAILING_ELEMENT("trailing-element");

        private final String id;

        ErrorKind(String id) {
            this.id = id;
        }

        public String id() {
            return id;
        }
    }

    /** Redacted Extension failure at an Extension-local byte offset. */
    public static final class Error extends RuntimeException {
        private static final long serialVersionUID = 1L;
        private final ErrorKind kind;
        private final int offset;
        private final DerAsn1.Asn1ErrorKind asn1Kind;
        private final String framingKind;

        private Error(
                ErrorKind kind,
                int offset,
                DerAsn1.Asn1ErrorKind asn1Kind,
                String framingKind) {
            super("X.509 extension error " + kind.id() + " at byte " + offset);
            this.kind = kind;
            this.offset = offset;
            this.asn1Kind = asn1Kind;
            this.framingKind = framingKind;
        }

        public ErrorKind kind() {
            return kind;
        }

        public int offset() {
            return offset;
        }

        public DerAsn1.Asn1ErrorKind asn1Kind() {
            return asn1Kind;
        }

        public String framingKind() {
            return framingKind;
        }
    }

    /** Validated immutable Extension value. */
    public static final class Value {
        private final DerAsn1.ObjectIdentifier extensionId;
        private final boolean critical;
        private final ByteBuffer extensionValue;

        private Value(
                DerAsn1.ObjectIdentifier extensionId,
                boolean critical,
                ByteBuffer extensionValue) {
            this.extensionId = extensionId;
            this.critical = critical;
            this.extensionValue = ByteBuffer.wrap(bytes(extensionValue)).asReadOnlyBuffer();
        }

        public DerAsn1.ObjectIdentifier extensionId() {
            return extensionId;
        }

        public boolean critical() {
            return critical;
        }

        public ByteBuffer extensionValue() {
            return extensionValue.duplicate().asReadOnlyBuffer();
        }
    }

    /** Decode one generic Extension under the decoder's shared budgets. */
    public static Value decodeX509Extension(
            DerAsn1.Asn1Decoder decoder, DerAsn1.Asn1Element element) {
        Objects.requireNonNull(decoder, "decoder");
        Objects.requireNonNull(element, "element");
        int valueOffset = element.header().remaining();
        int valueLength = element.value().remaining();
        final DerAsn1.Asn1Cursor fields;
        try {
            fields = decoder.sequence(element);
        } catch (DerAsn1.Error error) {
            throw structure(error, error.offset());
        }

        int extensionIdOffset = childOffset(valueOffset, valueLength, fields);
        var extensionIdElement = readChild(
            decoder, fields, valueOffset, extensionIdOffset);
        if (extensionIdElement == null) {
            throw error(ErrorKind.MISSING_EXTENSION_ID, extensionIdOffset);
        }
        final DerAsn1.ObjectIdentifier extensionId;
        try {
            extensionId = DerAsn1.decodeObjectIdentifier(
                extensionIdElement, decoder.limits());
        } catch (DerAsn1.Error error) {
            throw semantic(ErrorKind.INVALID_EXTENSION_ID, error, extensionIdOffset);
        }

        int secondOffset = childOffset(valueOffset, valueLength, fields);
        var second = readChild(decoder, fields, valueOffset, secondOffset);
        if (second == null) {
            throw error(ErrorKind.MISSING_EXTENSION_VALUE, secondOffset);
        }

        boolean critical;
        DerAsn1.Asn1Element extensionValueElement;
        int extensionValueOffset;
        if (second.tag().number() == BOOLEAN_TAG) {
            try {
                critical = DerAsn1.decodeBoolean(second);
            } catch (DerAsn1.Error error) {
                throw semantic(ErrorKind.INVALID_CRITICAL, error, secondOffset);
            }
            if (!critical) {
                throw error(ErrorKind.ENCODED_DEFAULT_CRITICAL, secondOffset);
            }
            extensionValueOffset = childOffset(valueOffset, valueLength, fields);
            extensionValueElement = readChild(
                decoder, fields, valueOffset, extensionValueOffset);
            if (extensionValueElement == null) {
                throw error(ErrorKind.MISSING_EXTENSION_VALUE, extensionValueOffset);
            }
        } else {
            critical = false;
            extensionValueElement = second;
            extensionValueOffset = secondOffset;
        }

        final ByteBuffer extensionValue;
        try {
            extensionValue = DerAsn1.decodeOctetString(extensionValueElement);
        } catch (DerAsn1.Error error) {
            throw semantic(ErrorKind.INVALID_EXTENSION_VALUE, error, extensionValueOffset);
        }

        int trailingOffset = childOffset(valueOffset, valueLength, fields);
        if (readChild(decoder, fields, valueOffset, trailingOffset) != null) {
            throw error(ErrorKind.TRAILING_ELEMENT, trailingOffset);
        }
        return new Value(extensionId, critical, extensionValue);
    }

    private static DerAsn1.Asn1Element readChild(
            DerAsn1.Asn1Decoder decoder,
            DerAsn1.Asn1Cursor fields,
            int valueOffset,
            int childOffset) {
        try {
            return fields.read(decoder);
        } catch (DerAsn1.Error error) {
            int offset = error.kind() == DerAsn1.Asn1ErrorKind.FRAMING
                ? valueOffset + error.offset()
                : childOffset + error.offset();
            throw structure(error, offset);
        }
    }

    private static int childOffset(
            int valueOffset, int valueLength, DerAsn1.Asn1Cursor fields) {
        return valueOffset + valueLength - fields.remaining().remaining();
    }

    private static Error semantic(
            ErrorKind kind, DerAsn1.Error error, int childOffset) {
        return new Error(
            kind,
            childOffset + error.offset(),
            error.kind(),
            error.framingKind());
    }

    private static Error structure(DerAsn1.Error error, int offset) {
        return new Error(
            ErrorKind.STRUCTURE,
            offset,
            error.kind(),
            error.framingKind());
    }

    private static Error error(ErrorKind kind, int offset) {
        return new Error(kind, offset, null, null);
    }

    private static byte[] bytes(ByteBuffer source) {
        var view = source.duplicate();
        byte[] output = new byte[view.remaining()];
        view.get(output);
        return output;
    }
}
