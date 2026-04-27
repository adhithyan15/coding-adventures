package com.codingadventures.wasmleb128;

import java.util.ArrayList;
import java.util.List;

public final class WasmLeb128 {
    private static final int CONTINUATION_BIT = 0x80;
    private static final int PAYLOAD_MASK = 0x7F;
    private static final int MAX_LEB128_BYTES_32 = 5;

    private WasmLeb128() {}

    public record UnsignedDecoding(long value, int bytesConsumed) {}

    public record SignedDecoding(int value, int bytesConsumed) {}

    public static final class LEB128Error extends RuntimeException {
        public LEB128Error(String message) {
            super(message);
        }
    }

    public static UnsignedDecoding decodeUnsigned(byte[] data) {
        return decodeUnsigned(data, 0);
    }

    public static UnsignedDecoding decodeUnsigned(byte[] data, int offset) {
        long result = 0;
        int shift = 0;
        int bytesConsumed = 0;

        for (int index = offset; index < data.length; index++) {
            if (bytesConsumed >= MAX_LEB128_BYTES_32) {
                throw new LEB128Error(
                        "LEB128 sequence exceeds maximum " + MAX_LEB128_BYTES_32 + " bytes for a 32-bit value"
                );
            }

            int current = Byte.toUnsignedInt(data[index]);
            int payload = current & PAYLOAD_MASK;
            result |= (long) payload << shift;
            shift += 7;
            bytesConsumed++;

            if ((current & CONTINUATION_BIT) == 0) {
                return new UnsignedDecoding(result & 0xFFFF_FFFFL, bytesConsumed);
            }
        }

        throw new LEB128Error(
                "LEB128 sequence is unterminated: reached end of data at offset "
                        + (offset + bytesConsumed)
                        + " without finding a byte with continuation bit = 0"
        );
    }

    public static SignedDecoding decodeSigned(byte[] data) {
        return decodeSigned(data, 0);
    }

    public static SignedDecoding decodeSigned(byte[] data, int offset) {
        int result = 0;
        int shift = 0;
        int bytesConsumed = 0;
        int lastByte = 0;

        for (int index = offset; index < data.length; index++) {
            if (bytesConsumed >= MAX_LEB128_BYTES_32) {
                throw new LEB128Error(
                        "LEB128 sequence exceeds maximum " + MAX_LEB128_BYTES_32 + " bytes for a 32-bit value"
                );
            }

            int current = Byte.toUnsignedInt(data[index]);
            lastByte = current;
            int payload = current & PAYLOAD_MASK;
            result |= payload << shift;
            shift += 7;
            bytesConsumed++;

            if ((current & CONTINUATION_BIT) == 0) {
                if (shift < 32 && (lastByte & 0x40) != 0) {
                    result |= -(1 << shift);
                }
                return new SignedDecoding(result, bytesConsumed);
            }
        }

        throw new LEB128Error(
                "LEB128 sequence is unterminated: reached end of data at offset "
                        + (offset + bytesConsumed)
                        + " without finding a byte with continuation bit = 0"
        );
    }

    public static byte[] encodeUnsigned(long value) {
        long remaining = value & 0xFFFF_FFFFL;
        List<Byte> bytes = new ArrayList<>();

        do {
            int current = (int) (remaining & PAYLOAD_MASK);
            remaining >>>= 7;
            if (remaining != 0) {
                current |= CONTINUATION_BIT;
            }
            bytes.add((byte) current);
        } while (remaining != 0);

        return toByteArray(bytes);
    }

    public static byte[] encodeSigned(int value) {
        int remaining = value;
        List<Byte> bytes = new ArrayList<>();
        boolean done = false;

        while (!done) {
            int current = remaining & PAYLOAD_MASK;
            remaining >>= 7;

            if ((remaining == 0 && (current & 0x40) == 0) || (remaining == -1 && (current & 0x40) != 0)) {
                done = true;
            } else {
                current |= CONTINUATION_BIT;
            }

            bytes.add((byte) current);
        }

        return toByteArray(bytes);
    }

    private static byte[] toByteArray(List<Byte> bytes) {
        byte[] result = new byte[bytes.size()];
        for (int index = 0; index < bytes.size(); index++) {
            result[index] = bytes.get(index);
        }
        return result;
    }
}
