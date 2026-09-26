package com.codingadventures.derasn1;

import com.codingadventures.dertlv.DerTlv;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;

/** Bounded typed ASN.1 DER values over payload-blind DER framing. */
public final class DerAsn1 {
    public static final int DEFAULT_MAX_DEPTH = 32;
    public static final long DEFAULT_MAX_TOTAL_ELEMENTS = 16_384;
    public static final int DEFAULT_MAX_OID_ARCS = 128;
    private static final BigInteger U64_MAX = BigInteger.ONE.shiftLeft(64).subtract(BigInteger.ONE);

    private DerAsn1() {}

    /** Explicit framing, nesting, whole-document work, and OID limits. */
    public record Asn1Limits(
            DerTlv.Limits der,
            int maxDepth,
            long maxTotalElements,
            int maxOidArcs) {
        public Asn1Limits {
            Objects.requireNonNull(der, "der");
            if (maxDepth < 0 || maxTotalElements < 0 || maxOidArcs < 0) {
                throw new IllegalArgumentException("ASN.1 limits must be non-negative");
            }
        }

        public static Asn1Limits defaults() {
            return new Asn1Limits(
                DerTlv.Limits.defaults(),
                DEFAULT_MAX_DEPTH,
                DEFAULT_MAX_TOTAL_ELEMENTS,
                DEFAULT_MAX_OID_ARCS);
        }
    }

    /** Stable typed error identifiers. */
    public enum Asn1ErrorKind {
        FRAMING("framing"),
        UNEXPECTED_TAG("unexpected-tag"),
        DECODER_LIMIT_MISMATCH("decoder-limit-mismatch"),
        DEPTH_LIMIT_EXCEEDED("depth-limit-exceeded"),
        ELEMENT_LIMIT_EXCEEDED("element-limit-exceeded"),
        INVALID_BOOLEAN_LENGTH("invalid-boolean-length"),
        INVALID_BOOLEAN_VALUE("invalid-boolean-value"),
        EMPTY_INTEGER("empty-integer"),
        NON_MINIMAL_INTEGER("non-minimal-integer"),
        NEGATIVE_INTEGER("negative-integer"),
        INTEGER_OVERFLOW("integer-overflow"),
        MISSING_UNUSED_BIT_COUNT("missing-unused-bit-count"),
        INVALID_UNUSED_BIT_COUNT("invalid-unused-bit-count"),
        NON_ZERO_BIT_PADDING("non-zero-bit-padding"),
        BIT_LENGTH_OVERFLOW("bit-length-overflow"),
        NON_EMPTY_NULL("non-empty-null"),
        NON_ASCII_IA5_STRING("non-ascii-ia5-string"),
        EMPTY_OBJECT_IDENTIFIER("empty-object-identifier"),
        UNTERMINATED_OBJECT_IDENTIFIER("unterminated-object-identifier"),
        NON_MINIMAL_OBJECT_IDENTIFIER("non-minimal-object-identifier"),
        OBJECT_IDENTIFIER_OVERFLOW("object-identifier-overflow"),
        OID_ARC_LIMIT_EXCEEDED("oid-arc-limit-exceeded");

        private final String id;

        Asn1ErrorKind(String id) {
            this.id = id;
        }

        public String id() {
            return id;
        }
    }

    /** Payload-free typed DER failure at a local byte offset. */
    public static final class Error extends RuntimeException {
        private static final long serialVersionUID = 1L;
        private final Asn1ErrorKind kind;
        private final int offset;
        private final String framingKind;

        private Error(Asn1ErrorKind kind, int offset, String framingKind) {
            super("ASN.1 DER value error " + kind.id() + " at byte " + offset);
            this.kind = kind;
            this.offset = offset;
            this.framingKind = framingKind;
        }

        public Asn1ErrorKind kind() {
            return kind;
        }

        public int offset() {
            return offset;
        }

        public String framingKind() {
            return framingKind;
        }
    }

    /** One framed element plus its depth in the typed document walk. */
    public static final class Asn1Element {
        private final DerTlv.Element element;
        private final int depth;

        private Asn1Element(DerTlv.Element element, int depth) {
            this.element = element;
            this.depth = depth;
        }

        public DerTlv.Tag tag() {
            return element.tag();
        }

        public ByteBuffer header() {
            return element.header();
        }

        public ByteBuffer value() {
            return element.value();
        }

        public ByteBuffer encoded() {
            return element.encoded();
        }

        public int depth() {
            return depth;
        }

        private int valueOffset() {
            return header().remaining();
        }
    }

    /** Shared depth and total-element authority for one schema walk. */
    public static final class Asn1Decoder {
        private final Asn1Limits limits;
        private long elementsRead;

        public Asn1Decoder() {
            this(Asn1Limits.defaults());
        }

        public Asn1Decoder(Asn1Limits limits) {
            this.limits = Objects.requireNonNull(limits, "limits");
        }

        public Asn1Limits limits() {
            return limits;
        }

        public long elementsRead() {
            return elementsRead;
        }

        public Asn1Element decodeExact(byte[] input) {
            Objects.requireNonNull(input, "input");
            if (limits.maxDepth() == 0) {
                throw error(Asn1ErrorKind.DEPTH_LIMIT_EXCEEDED, 0);
            }
            requireElementCapacity(0);
            try {
                var element = DerTlv.decodeExact(input, limits.der());
                elementsRead++;
                return new Asn1Element(element, 0);
            } catch (DerTlv.Error failure) {
                throw framing(failure);
            }
        }

        public Asn1Cursor sequence(Asn1Element element) {
            return constructed(element, "universal", 16);
        }

        public Asn1Cursor set(Asn1Element element) {
            return constructed(element, "universal", 17);
        }

        public Asn1Element explicit(Asn1Element element, long tagNumber) {
            expectTag(element, "context-specific", true, tagNumber);
            int childDepth = childDepth(element);
            requireElementCapacity(element.valueOffset());
            try {
                var child = DerTlv.decodeExact(bytes(element.value()), limits.der());
                elementsRead++;
                return new Asn1Element(child, childDepth);
            } catch (DerTlv.Error failure) {
                throw framing(failure);
            }
        }

        private Asn1Cursor constructed(Asn1Element element, String tagClass, long number) {
            expectTag(element, tagClass, true, number);
            int childDepth = childDepth(element);
            try {
                return new Asn1Cursor(
                    new DerTlv.Cursor(bytes(element.value()), limits.der()),
                    childDepth,
                    limits);
            } catch (DerTlv.Error failure) {
                throw framing(failure);
            }
        }

        private int childDepth(Asn1Element element) {
            int value = element.depth() + 1;
            if (value >= limits.maxDepth()) {
                throw error(Asn1ErrorKind.DEPTH_LIMIT_EXCEEDED, 0);
            }
            return value;
        }

        private void requireElementCapacity(int offset) {
            if (elementsRead >= limits.maxTotalElements()) {
                throw error(Asn1ErrorKind.ELEMENT_LIMIT_EXCEEDED, offset);
            }
        }
    }

    /** Iterative sibling cursor sharing an {@link Asn1Decoder} work budget. */
    public static final class Asn1Cursor {
        private final DerTlv.Cursor cursor;
        private final int childDepth;
        private final Asn1Limits limits;

        private Asn1Cursor(DerTlv.Cursor cursor, int childDepth, Asn1Limits limits) {
            this.cursor = cursor;
            this.childDepth = childDepth;
            this.limits = limits;
        }

        public ByteBuffer remaining() {
            return cursor.remaining();
        }

        public Asn1Element read(Asn1Decoder decoder) {
            Objects.requireNonNull(decoder, "decoder");
            if (!cursor.remaining().hasRemaining()) {
                return null;
            }
            if (!decoder.limits().equals(limits)) {
                throw error(Asn1ErrorKind.DECODER_LIMIT_MISMATCH, 0);
            }
            decoder.requireElementCapacity(0);
            try {
                var element = cursor.read();
                if (element == null) {
                    return null;
                }
                decoder.elementsRead++;
                return new Asn1Element(element, childDepth);
            } catch (DerTlv.Error failure) {
                throw framing(failure);
            }
        }

        public void finish() {
            try {
                cursor.finish();
            } catch (DerTlv.Error failure) {
                throw framing(failure);
            }
        }
    }

    /** Canonical signed INTEGER contents with checked unsigned conversion. */
    public static final class DerInteger {
        private final ByteBuffer signedBytes;
        private final int valueOffset;

        private DerInteger(ByteBuffer signedBytes, int valueOffset) {
            this.signedBytes = readOnly(signedBytes);
            this.valueOffset = valueOffset;
        }

        public ByteBuffer signedBytes() {
            return signedBytes.duplicate().asReadOnlyBuffer();
        }

        public boolean isNegative() {
            return (signedBytes.get(0) & 0x80) != 0;
        }

        public BigInteger toU64() {
            if (isNegative()) {
                throw error(Asn1ErrorKind.NEGATIVE_INTEGER, valueOffset);
            }
            byte[] magnitude = bytes(signedBytes());
            int start = magnitude[0] == 0 ? 1 : 0;
            int length = magnitude.length - start;
            if (length > 8) {
                throw error(Asn1ErrorKind.INTEGER_OVERFLOW, valueOffset);
            }
            if (length == 0) {
                return BigInteger.ZERO;
            }
            byte[] unsigned = new byte[length];
            System.arraycopy(magnitude, start, unsigned, 0, length);
            return new BigInteger(1, unsigned);
        }
    }

    /** Canonical BIT STRING payload plus its explicit bit length. */
    public static final class DerBitString {
        private final ByteBuffer bytes;
        private final int unusedBits;
        private final long bitLength;

        private DerBitString(ByteBuffer bytes, int unusedBits, long bitLength) {
            this.bytes = readOnly(bytes);
            this.unusedBits = unusedBits;
            this.bitLength = bitLength;
        }

        public ByteBuffer bytes() {
            return bytes.duplicate().asReadOnlyBuffer();
        }

        public int unusedBits() {
            return unusedBits;
        }

        public long bitLength() {
            return bitLength;
        }
    }

    /** Fully validated OBJECT IDENTIFIER contents and unsigned arcs. */
    public static final class ObjectIdentifier {
        private final ByteBuffer encoded;
        private final List<BigInteger> arcs;

        private ObjectIdentifier(ByteBuffer encoded, List<BigInteger> arcs) {
            this.encoded = readOnly(encoded);
            this.arcs = List.copyOf(arcs);
        }

        public ByteBuffer encoded() {
            return encoded.duplicate().asReadOnlyBuffer();
        }

        public List<BigInteger> arcs() {
            return arcs;
        }

        public int arcCount() {
            return arcs.size();
        }

        public boolean equalsArcs(List<BigInteger> expected) {
            return arcs.equals(expected);
        }
    }

    public static boolean decodeBoolean(Asn1Element element) {
        expectUniversalPrimitive(element, 1);
        var value = element.value();
        if (value.remaining() != 1) {
            throw error(Asn1ErrorKind.INVALID_BOOLEAN_LENGTH, element.valueOffset());
        }
        int octet = value.get(0) & 0xff;
        if (octet == 0) {
            return false;
        }
        if (octet == 0xff) {
            return true;
        }
        throw error(Asn1ErrorKind.INVALID_BOOLEAN_VALUE, element.valueOffset());
    }

    public static DerInteger decodeInteger(Asn1Element element) {
        expectUniversalPrimitive(element, 2);
        var value = element.value();
        if (!value.hasRemaining()) {
            throw error(Asn1ErrorKind.EMPTY_INTEGER, element.valueOffset());
        }
        int first = value.get(0) & 0xff;
        if (value.remaining() > 1) {
            int second = value.get(1) & 0xff;
            if ((first == 0 && (second & 0x80) == 0)
                    || (first == 0xff && (second & 0x80) != 0)) {
                throw error(Asn1ErrorKind.NON_MINIMAL_INTEGER, element.valueOffset());
            }
        }
        return new DerInteger(value, element.valueOffset());
    }

    public static DerBitString decodeBitString(Asn1Element element) {
        expectUniversalPrimitive(element, 3);
        var value = element.value();
        if (!value.hasRemaining()) {
            throw error(Asn1ErrorKind.MISSING_UNUSED_BIT_COUNT, element.valueOffset());
        }
        int unusedBits = value.get(0) & 0xff;
        var payload = value.duplicate();
        payload.position(1);
        payload = payload.slice().asReadOnlyBuffer();
        if (unusedBits > 7 || (!payload.hasRemaining() && unusedBits != 0)) {
            throw error(Asn1ErrorKind.INVALID_UNUSED_BIT_COUNT, element.valueOffset());
        }
        if (unusedBits != 0
                && ((payload.get(payload.remaining() - 1) & 0xff)
                    & ((1 << unusedBits) - 1)) != 0) {
            throw error(
                Asn1ErrorKind.NON_ZERO_BIT_PADDING,
                element.valueOffset() + value.remaining() - 1);
        }
        long bitLength = Math.multiplyExact((long) payload.remaining(), 8L) - unusedBits;
        return new DerBitString(payload, unusedBits, bitLength);
    }

    public static ByteBuffer decodeOctetString(Asn1Element element) {
        expectUniversalPrimitive(element, 4);
        return element.value();
    }

    public static ByteBuffer decodeImplicitOctetString(Asn1Element element, long tagNumber) {
        expectContextPrimitive(element, tagNumber);
        return element.value();
    }

    public static String decodeIA5String(Asn1Element element) {
        expectUniversalPrimitive(element, 22);
        return decodeIa5Contents(element);
    }

    public static String decodeImplicitIA5String(Asn1Element element, long tagNumber) {
        expectContextPrimitive(element, tagNumber);
        return decodeIa5Contents(element);
    }

    public static void decodeNull(Asn1Element element) {
        expectUniversalPrimitive(element, 5);
        if (element.value().hasRemaining()) {
            throw error(Asn1ErrorKind.NON_EMPTY_NULL, element.valueOffset());
        }
    }

    public static ObjectIdentifier decodeObjectIdentifier(Asn1Element element) {
        return decodeObjectIdentifier(element, Asn1Limits.defaults());
    }

    public static ObjectIdentifier decodeObjectIdentifier(
            Asn1Element element, Asn1Limits limits) {
        expectUniversalPrimitive(element, 6);
        return decodeOidContents(element, Objects.requireNonNull(limits, "limits"));
    }

    public static ObjectIdentifier decodeImplicitObjectIdentifier(
            Asn1Element element, long tagNumber) {
        return decodeImplicitObjectIdentifier(element, tagNumber, Asn1Limits.defaults());
    }

    public static ObjectIdentifier decodeImplicitObjectIdentifier(
            Asn1Element element, long tagNumber, Asn1Limits limits) {
        expectContextPrimitive(element, tagNumber);
        return decodeOidContents(element, Objects.requireNonNull(limits, "limits"));
    }

    private static String decodeIa5Contents(Asn1Element element) {
        var value = element.value();
        for (int index = 0; index < value.remaining(); index++) {
            if ((value.get(index) & 0xff) > 0x7f) {
                throw error(Asn1ErrorKind.NON_ASCII_IA5_STRING, element.valueOffset() + index);
            }
        }
        return new String(bytes(value), StandardCharsets.US_ASCII);
    }

    private static ObjectIdentifier decodeOidContents(Asn1Element element, Asn1Limits limits) {
        var encoded = element.value();
        if (!encoded.hasRemaining()) {
            throw error(Asn1ErrorKind.EMPTY_OBJECT_IDENTIFIER, element.valueOffset());
        }
        var first = parseBase128(encoded, 0, element.valueOffset());
        var arcs = new ArrayList<BigInteger>();
        if (first.value().compareTo(BigInteger.valueOf(40)) < 0) {
            arcs.add(BigInteger.ZERO);
            arcs.add(first.value());
        } else if (first.value().compareTo(BigInteger.valueOf(80)) < 0) {
            arcs.add(BigInteger.ONE);
            arcs.add(first.value().subtract(BigInteger.valueOf(40)));
        } else {
            arcs.add(BigInteger.TWO);
            arcs.add(first.value().subtract(BigInteger.valueOf(80)));
        }
        if (arcs.size() > limits.maxOidArcs()) {
            throw error(Asn1ErrorKind.OID_ARC_LIMIT_EXCEEDED, element.valueOffset());
        }
        int offset = first.nextOffset();
        while (offset < encoded.remaining()) {
            int arcStart = offset;
            var parsed = parseBase128(encoded, offset, element.valueOffset());
            arcs.add(parsed.value());
            if (arcs.size() > limits.maxOidArcs()) {
                throw error(
                    Asn1ErrorKind.OID_ARC_LIMIT_EXCEEDED,
                    element.valueOffset() + arcStart);
            }
            offset = parsed.nextOffset();
        }
        return new ObjectIdentifier(encoded, arcs);
    }

    private record ParsedArc(BigInteger value, int nextOffset) {}

    private static ParsedArc parseBase128(ByteBuffer encoded, int start, int valueOffset) {
        if ((encoded.get(start) & 0xff) == 0x80) {
            throw error(Asn1ErrorKind.NON_MINIMAL_OBJECT_IDENTIFIER, valueOffset + start);
        }
        var value = BigInteger.ZERO;
        for (int offset = start; ; offset++) {
            if (offset >= encoded.remaining()) {
                throw error(Asn1ErrorKind.UNTERMINATED_OBJECT_IDENTIFIER, valueOffset + offset);
            }
            int octet = encoded.get(offset) & 0xff;
            var candidate = value.shiftLeft(7).add(BigInteger.valueOf(octet & 0x7f));
            if (candidate.compareTo(U64_MAX) > 0) {
                throw error(Asn1ErrorKind.OBJECT_IDENTIFIER_OVERFLOW, valueOffset + offset);
            }
            value = candidate;
            if ((octet & 0x80) == 0) {
                return new ParsedArc(value, offset + 1);
            }
        }
    }

    private static void expectUniversalPrimitive(Asn1Element element, long number) {
        expectTag(element, "universal", false, number);
    }

    private static void expectContextPrimitive(Asn1Element element, long number) {
        expectTag(element, "context-specific", false, number);
    }

    private static void expectTag(
            Asn1Element element, String tagClass, boolean constructed, long number) {
        Objects.requireNonNull(element, "element");
        var tag = element.tag();
        if (!tag.tagClass().equals(tagClass)
                || tag.constructed() != constructed
                || tag.number() != number) {
            throw error(Asn1ErrorKind.UNEXPECTED_TAG, 0);
        }
    }

    private static ByteBuffer readOnly(ByteBuffer source) {
        return source.duplicate().asReadOnlyBuffer();
    }

    private static byte[] bytes(ByteBuffer source) {
        var view = source.duplicate();
        byte[] output = new byte[view.remaining()];
        view.get(output);
        return output;
    }

    private static Error error(Asn1ErrorKind kind, int offset) {
        return new Error(kind, offset, null);
    }

    private static Error framing(DerTlv.Error error) {
        return new Error(Asn1ErrorKind.FRAMING, error.offset(), error.kind());
    }
}
