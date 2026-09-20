package com.codingadventures.dertlv;

import java.nio.ByteBuffer;
import java.util.Objects;

/** Bounded, payload-blind DER tag-length-value framing. */
public final class DerTlv {
    private static final String[] TAG_CLASSES = {
        "universal", "application", "context-specific", "private"
    };

    private DerTlv() {}

    /** Explicit resource and representation limits. */
    public record Limits(long maxInputLength, long maxValueLength, long maxElements, long maxTagNumber) {
        public Limits {
            if (maxInputLength < 0 || maxValueLength < 0 || maxElements < 0 || maxTagNumber < 0) {
                throw new IllegalArgumentException("limits must be non-negative");
            }
            if (maxTagNumber > 0xffff_ffffL) {
                throw new IllegalArgumentException("maxTagNumber must fit u32");
            }
        }

        public static Limits defaults() {
            return new Limits(1_048_576, 1_048_576, 4_096, 0xffff_ffffL);
        }
    }

    /** A stable payload-blind framing failure. */
    public static final class Error extends RuntimeException {
        private static final long serialVersionUID = 1L;
        private final String kind;
        private final int offset;

        private Error(String kind, int offset) {
            super("DER framing error " + kind + " at byte " + offset);
            this.kind = kind;
            this.offset = offset;
        }

        public String kind() {
            return kind;
        }

        public int offset() {
            return offset;
        }
    }

    public record Tag(String tagClass, boolean constructed, long number) {}

    /** A frame represented by ranges into the caller-supplied byte array. */
    public static final class Element {
        private final Tag tag;
        private final byte[] input;
        private final int start;
        private final int headerLength;
        private final int encodedLength;

        private Element(Tag tag, byte[] input, int start, int headerLength, int encodedLength) {
            this.tag = tag;
            this.input = input;
            this.start = start;
            this.headerLength = headerLength;
            this.encodedLength = encodedLength;
        }

        public Tag tag() {
            return tag;
        }

        public ByteBuffer header() {
            return view(start, headerLength);
        }

        public ByteBuffer value() {
            return view(start + headerLength, encodedLength - headerLength);
        }

        public ByteBuffer encoded() {
            return view(start, encodedLength);
        }

        private ByteBuffer view(int offset, int length) {
            return ByteBuffer.wrap(input, offset, length).slice().asReadOnlyBuffer();
        }
    }

    public record Decoded(Element element, ByteBuffer remainder) {}

    public static Decoded decodeOne(byte[] input) {
        return decodeOne(input, Limits.defaults());
    }

    public static Decoded decodeOne(byte[] input, Limits limits) {
        Objects.requireNonNull(input, "input");
        var result = decodeAt(input, 0, input.length, Objects.requireNonNull(limits, "limits"));
        var remainder = ByteBuffer.wrap(input, result.nextOffset(), input.length - result.nextOffset())
            .slice()
            .asReadOnlyBuffer();
        return new Decoded(result.element(), remainder);
    }

    public static Element decodeExact(byte[] input) {
        return decodeExact(input, Limits.defaults());
    }

    public static Element decodeExact(byte[] input, Limits limits) {
        Objects.requireNonNull(input, "input");
        var result = decodeAt(input, 0, input.length, Objects.requireNonNull(limits, "limits"));
        if (result.nextOffset() != input.length) {
            throw error("trailing-data", result.nextOffset());
        }
        return result.element();
    }

    /** Iterative sibling decoder whose failed reads preserve state. */
    public static final class Cursor {
        private final byte[] input;
        private final Limits limits;
        private int offset;
        private long elementsRead;

        public Cursor(byte[] input) {
            this(input, Limits.defaults());
        }

        public Cursor(byte[] input, Limits limits) {
            this.input = Objects.requireNonNull(input, "input");
            this.limits = Objects.requireNonNull(limits, "limits");
            if (input.length > limits.maxInputLength()) {
                throw error("input-limit-exceeded", 0);
            }
        }

        public Element read() {
            if (offset == input.length) {
                return null;
            }
            if (elementsRead >= limits.maxElements()) {
                throw error("element-limit-exceeded", offset);
            }
            var result = decodeAt(input, offset, input.length - offset, limits);
            offset = result.nextOffset();
            elementsRead++;
            return result.element();
        }

        public void finish() {
            if (offset != input.length) {
                throw error("trailing-data", offset);
            }
        }

        public long elementsRead() {
            return elementsRead;
        }

        public ByteBuffer remaining() {
            return ByteBuffer.wrap(input, offset, input.length - offset).slice().asReadOnlyBuffer();
        }
    }

    private record DecodeResult(Element element, int nextOffset) {}

    private static DecodeResult decodeAt(byte[] input, int start, int available, Limits limits) {
        if (available > limits.maxInputLength()) {
            throw error("input-limit-exceeded", start);
        }
        if (available == 0) {
            throw error("empty-input", start);
        }

        int first = unsigned(input[start]);
        String tagClass = TAG_CLASSES[first >>> 6];
        boolean constructed = (first & 0x20) != 0;
        int low = first & 0x1f;
        long number;
        int identifierLength;
        if (low != 0x1f) {
            number = low;
            identifierLength = 1;
            if (number > limits.maxTagNumber()) {
                throw error("tag-limit-exceeded", start);
            }
        } else {
            long[] high = decodeHighTag(input, start, available, limits);
            number = high[0];
            identifierLength = Math.toIntExact(high[1]);
        }
        if (tagClass.equals("universal") && number == 0) {
            throw error("end-of-contents", start);
        }

        long[] length = decodeLength(input, start, available, identifierLength);
        long valueLength = length[0];
        int lengthLength = Math.toIntExact(length[1]);
        int lengthOffset = Math.toIntExact(length[2]);
        if (valueLength > limits.maxValueLength()) {
            throw error("value-limit-exceeded", lengthOffset);
        }
        int headerLength = identifierLength + lengthLength;
        if (valueLength > Integer.MAX_VALUE - (long) headerLength) {
            throw error("length-host-overflow", lengthOffset);
        }
        int encodedLength = headerLength + Math.toIntExact(valueLength);
        if (encodedLength > available) {
            throw error("truncated-value", start + available);
        }
        var tag = new Tag(tagClass, constructed, number);
        return new DecodeResult(new Element(tag, input, start, headerLength, encodedLength), start + encodedLength);
    }

    private static long[] decodeHighTag(byte[] input, int start, int available, Limits limits) {
        long number = 0;
        int index = 1;
        while (true) {
            if (index >= available) {
                throw error("truncated-high-tag", start + index);
            }
            int octet = unsigned(input[start + index]);
            int payload = octet & 0x7f;
            if (index == 1 && payload == 0) {
                throw error("non-minimal-tag", start + index);
            }
            if (number > (0xffff_ffffL - payload) / 128) {
                throw error("tag-overflow", start + index);
            }
            number = number * 128 + payload;
            if (number > limits.maxTagNumber()) {
                throw error("tag-limit-exceeded", start + index);
            }
            index++;
            if ((octet & 0x80) == 0) {
                break;
            }
        }
        if (number < 31) {
            throw error("non-minimal-tag", start);
        }
        return new long[] {number, index};
    }

    private static long[] decodeLength(byte[] input, int start, int available, int identifierLength) {
        int lengthOffset = start + identifierLength;
        if (identifierLength >= available) {
            throw error("truncated-length", lengthOffset);
        }
        int first = unsigned(input[lengthOffset]);
        if (first < 0x80) {
            return new long[] {first, 1, lengthOffset};
        }
        if (first == 0x80) {
            throw error("indefinite-length", lengthOffset);
        }
        if (first == 0xff) {
            throw error("reserved-length", lengthOffset);
        }
        int count = first & 0x7f;
        if (count > 8) {
            throw error("length-too-wide", lengthOffset);
        }
        if (identifierLength + 1 + count > available) {
            throw error("truncated-length", start + available);
        }
        int leading = unsigned(input[lengthOffset + 1]);
        if (leading == 0) {
            throw error("non-minimal-length", lengthOffset + 1);
        }
        if (count == 8 && (leading & 0x80) != 0) {
            throw error("length-host-overflow", lengthOffset);
        }
        long value = 0;
        for (int index = 0; index < count; index++) {
            value = value * 256 + unsigned(input[lengthOffset + 1 + index]);
        }
        if (value < 128) {
            throw error("non-minimal-length", lengthOffset);
        }
        return new long[] {value, count + 1, lengthOffset};
    }

    private static int unsigned(byte value) {
        return value & 0xff;
    }

    private static Error error(String kind, int offset) {
        return new Error(kind, offset);
    }
}
