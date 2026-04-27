package com.codingadventures.wasmexecution;

import java.util.ArrayList;
import java.util.List;

final class WasmExecutionTestSupport {
    private WasmExecutionTestSupport() {}

    static byte[] encodeUnsigned(long value) {
        long remaining = value;
        List<Byte> bytes = new ArrayList<>();
        do {
            int current = (int) (remaining & 0x7F);
            remaining >>>= 7;
            if (remaining != 0) {
                current |= 0x80;
            }
            bytes.add((byte) current);
        } while (remaining != 0);
        return toByteArray(bytes);
    }

    static byte[] encodeSigned32(int value) {
        int remaining = value;
        List<Byte> bytes = new ArrayList<>();
        boolean done = false;
        while (!done) {
            int current = remaining & 0x7F;
            remaining >>= 7;
            done = (remaining == 0 && (current & 0x40) == 0) || (remaining == -1 && (current & 0x40) != 0);
            if (!done) {
                current |= 0x80;
            }
            bytes.add((byte) current);
        }
        return toByteArray(bytes);
    }

    static byte[] encodeSigned64(long value) {
        long remaining = value;
        List<Byte> bytes = new ArrayList<>();
        boolean done = false;
        while (!done) {
            int current = (int) (remaining & 0x7F);
            remaining >>= 7;
            done = (remaining == 0 && (current & 0x40) == 0) || (remaining == -1 && (current & 0x40) != 0);
            if (!done) {
                current |= 0x80;
            }
            bytes.add((byte) current);
        }
        return toByteArray(bytes);
    }

    private static byte[] toByteArray(List<Byte> bytes) {
        byte[] output = new byte[bytes.size()];
        for (int index = 0; index < bytes.size(); index++) {
            output[index] = bytes.get(index);
        }
        return output;
    }
}
