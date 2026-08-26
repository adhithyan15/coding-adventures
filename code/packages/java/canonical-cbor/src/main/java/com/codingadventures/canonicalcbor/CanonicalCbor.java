package com.codingadventures.canonicalcbor;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;

/**
 * A zero-production-dependency RFC 8949 section 4.2.3 codec.
 *
 * <p>The implementation is intentionally two small machines. The encoder
 * chooses the shortest header and sorts already-encoded map keys. The decoder
 * advances one checked cursor and rejects every spelling the encoder would not
 * create. Keeping those rules symmetric is what turns flexible CBOR into one
 * deterministic byte representation.</p>
 */
public final class CanonicalCbor {
    public static final int MAX_NESTING_DEPTH = 128;
    public static final int MAX_ENCODED_BYTES = 1_048_576;

    private CanonicalCbor() { }

    /** Encode one value without publishing partial bytes on failure. */
    public static byte[] encodeChecked(CborValue value) throws CborException {
        Encoder encoder = new Encoder();
        encoder.writeValue(value, 0);
        return encoder.bytes();
    }

    /** Append one complete encoding, leaving {@code destination} unchanged on failure. */
    public static void encodeIntoChecked(CborValue value, ByteArrayOutputStream destination)
            throws CborException {
        byte[] encoded = encodeChecked(value);
        destination.writeBytes(encoded);
    }

    /** Decode exactly one canonical item. */
    public static CborValue decode(byte[] bytes) throws CborException {
        Cursor cursor = new Cursor(bytes);
        CborValue value = cursor.readValue(0);
        if (cursor.remaining() != 0) {
            throw error("trailing-bytes");
        }
        return value;
    }

    private static CborException error(String id) {
        return new CborException(id);
    }

    private static final class Encoder {
        private final ByteArrayOutputStream output = new ByteArrayOutputStream();

        byte[] bytes() {
            return output.toByteArray();
        }

        void writeValue(CborValue value, int depth) throws CborException {
            if (depth > MAX_NESTING_DEPTH) {
                throw error("encode-too-deep");
            }
            if (value instanceof CborValue.Unsigned integer) {
                writeArgument(0, integer.value());
            } else if (value instanceof CborValue.Negative integer) {
                writeArgument(1, integer.value());
            } else if (value instanceof CborValue.Bytes bytes) {
                byte[] payload = bytes.rawValue();
                writeArgument(2, payload.length);
                writeBytes(payload);
            } else if (value instanceof CborValue.Text text) {
                byte[] payload = text.value().getBytes(StandardCharsets.UTF_8);
                writeArgument(3, payload.length);
                writeBytes(payload);
            } else if (value instanceof CborValue.Array array) {
                writeArgument(4, array.values().size());
                for (CborValue item : array.values()) {
                    writeValue(item, depth + 1);
                }
            } else if (value instanceof CborValue.Map map) {
                writeMap(map, depth);
            } else if (value instanceof CborValue.Tag tag) {
                writeArgument(6, tag.number());
                writeValue(tag.value(), depth + 1);
            } else if (value instanceof CborValue.Bool bool) {
                writeByte(bool.value() ? 0xf5 : 0xf4);
            } else if (value == CborValue.Null.INSTANCE) {
                writeByte(0xf6);
            } else {
                throw new IllegalArgumentException("unknown CborValue implementation");
            }
        }

        private void writeMap(CborValue.Map map, int depth) throws CborException {
            List<EncodedEntry> entries = new ArrayList<>(map.entries().size());
            for (CborValue.MapEntry entry : map.entries()) {
                Encoder keyEncoder = new Encoder();
                keyEncoder.writeValue(entry.key(), depth + 1);
                entries.add(new EncodedEntry(keyEncoder.bytes(), entry.value()));
            }
            entries.sort(Comparator.comparingInt((EncodedEntry entry) -> entry.key.length)
                    .thenComparing((left, right) -> compareUnsigned(left.key, right.key)));
            for (int i = 1; i < entries.size(); i++) {
                if (Arrays.equals(entries.get(i - 1).key, entries.get(i).key)) {
                    throw error("duplicate-map-key");
                }
            }
            writeArgument(5, entries.size());
            for (EncodedEntry entry : entries) {
                writeBytes(entry.key);
                writeValue(entry.value, depth + 1);
            }
        }

        private void writeArgument(int major, long argument) throws CborException {
            int prefix = major << 5;
            if (Long.compareUnsigned(argument, 23) <= 0) {
                writeByte(prefix | (int) argument);
            } else if (Long.compareUnsigned(argument, 0xff) <= 0) {
                writeByte(prefix | 24);
                writeByte((int) argument);
            } else if (Long.compareUnsigned(argument, 0xffff) <= 0) {
                writeByte(prefix | 25);
                writeByte((int) (argument >>> 8));
                writeByte((int) argument);
            } else if (Long.compareUnsigned(argument, 0xffff_ffffL) <= 0) {
                writeByte(prefix | 26);
                for (int shift = 24; shift >= 0; shift -= 8) {
                    writeByte((int) (argument >>> shift));
                }
            } else {
                writeByte(prefix | 27);
                for (int shift = 56; shift >= 0; shift -= 8) {
                    writeByte((int) (argument >>> shift));
                }
            }
        }

        private void writeByte(int value) throws CborException {
            if (output.size() >= MAX_ENCODED_BYTES) {
                throw error("encode-too-large");
            }
            output.write(value);
        }

        private void writeBytes(byte[] bytes) throws CborException {
            if (bytes.length > MAX_ENCODED_BYTES - output.size()) {
                throw error("encode-too-large");
            }
            output.writeBytes(bytes);
        }
    }

    private record EncodedEntry(byte[] key, CborValue value) { }

    private record Header(int major, int info, long argument) { }

    private static final class Cursor {
        private final byte[] bytes;
        private int position;

        Cursor(byte[] bytes) {
            this.bytes = bytes.clone();
        }

        int remaining() {
            return bytes.length - position;
        }

        int readByte() throws CborException {
            if (position >= bytes.length) {
                throw error("unexpected-eof");
            }
            return bytes[position++] & 0xff;
        }

        byte[] readBytes(int length) throws CborException {
            if (length > remaining()) {
                throw error("unexpected-eof");
            }
            int start = position;
            position += length;
            return Arrays.copyOfRange(bytes, start, position);
        }

        Header readHeader() throws CborException {
            int initial = readByte();
            int major = initial >>> 5;
            int info = initial & 0x1f;
            boolean enforceMinimal = major != 7;
            long argument;
            if (info <= 23) {
                argument = info;
            } else if (info == 24) {
                argument = readByte();
                ensureMinimal(argument, 23, enforceMinimal);
            } else if (info == 25) {
                argument = readUnsigned(2);
                ensureMinimal(argument, 0xff, enforceMinimal);
            } else if (info == 26) {
                argument = readUnsigned(4);
                ensureMinimal(argument, 0xffff, enforceMinimal);
            } else if (info == 27) {
                argument = readUnsigned(8);
                ensureMinimal(argument, 0xffff_ffffL, enforceMinimal);
            } else if (info <= 30) {
                throw error("reserved");
            } else {
                throw error("indefinite");
            }
            return new Header(major, info, argument);
        }

        private long readUnsigned(int width) throws CborException {
            long value = 0;
            for (int i = 0; i < width; i++) {
                value = (value << 8) | readByte();
            }
            return value;
        }

        private void ensureMinimal(long argument, long previousMaximum, boolean enabled)
                throws CborException {
            if (enabled && Long.compareUnsigned(argument, previousMaximum) <= 0) {
                throw error("non-minimal-integer");
            }
        }

        CborValue readValue(int depth) throws CborException {
            if (depth > MAX_NESTING_DEPTH) {
                throw error("too-deep");
            }
            Header header = readHeader();
            return switch (header.major) {
                case 0 -> new CborValue.Unsigned(header.argument);
                case 1 -> new CborValue.Negative(header.argument);
                case 2 -> new CborValue.Bytes(readBytes(checkedLength(header.argument, 1)));
                case 3 -> new CborValue.Text(readText(checkedLength(header.argument, 1)));
                case 4 -> readArray(checkedLength(header.argument, 1), depth);
                case 5 -> readMap(checkedLength(header.argument, 2), depth);
                case 6 -> new CborValue.Tag(header.argument, readValue(depth + 1));
                case 7 -> readSimple(header.info);
                default -> throw new IllegalStateException("three-bit major type escaped range");
            };
        }

        private int checkedLength(long declared, int minimumBytesPerUnit) throws CborException {
            int maximum = remaining() / minimumBytesPerUnit;
            if (Long.compareUnsigned(declared, maximum) > 0) {
                throw error("length-too-large");
            }
            return (int) declared;
        }

        private String readText(int length) throws CborException {
            byte[] payload = readBytes(length);
            try {
                return StandardCharsets.UTF_8.newDecoder()
                        .onMalformedInput(CodingErrorAction.REPORT)
                        .onUnmappableCharacter(CodingErrorAction.REPORT)
                        .decode(ByteBuffer.wrap(payload)).toString();
            } catch (CharacterCodingException exception) {
                throw error("invalid-utf8");
            }
        }

        private CborValue.Array readArray(int count, int depth) throws CborException {
            List<CborValue> values = new ArrayList<>(count);
            for (int i = 0; i < count; i++) {
                values.add(readValue(depth + 1));
            }
            return new CborValue.Array(values);
        }

        private CborValue.Map readMap(int count, int depth) throws CborException {
            List<CborValue.MapEntry> entries = new ArrayList<>(count);
            byte[] previousKey = null;
            for (int i = 0; i < count; i++) {
                int keyStart = position;
                CborValue key = readValue(depth + 1);
                byte[] encodedKey = Arrays.copyOfRange(bytes, keyStart, position);
                if (previousKey != null && compareLengthFirst(previousKey, encodedKey) >= 0) {
                    throw error("non-canonical-map-order");
                }
                previousKey = encodedKey;
                entries.add(new CborValue.MapEntry(key, readValue(depth + 1)));
            }
            return new CborValue.Map(entries);
        }

        private CborValue readSimple(int info) throws CborException {
            return switch (info) {
                case 20 -> new CborValue.Bool(false);
                case 21 -> new CborValue.Bool(true);
                case 22 -> CborValue.Null.INSTANCE;
                case 25, 26, 27 -> throw error("float-not-supported");
                default -> throw error("unsupported-simple");
            };
        }
    }

    private static int compareLengthFirst(byte[] left, byte[] right) {
        int length = Integer.compare(left.length, right.length);
        return length != 0 ? length : compareUnsigned(left, right);
    }

    private static int compareUnsigned(byte[] left, byte[] right) {
        for (int i = 0; i < Math.min(left.length, right.length); i++) {
            int comparison = Integer.compare(left[i] & 0xff, right[i] & 0xff);
            if (comparison != 0) {
                return comparison;
            }
        }
        return Integer.compare(left.length, right.length);
    }
}

