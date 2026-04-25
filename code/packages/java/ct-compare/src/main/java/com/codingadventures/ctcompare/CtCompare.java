package com.codingadventures.ctcompare;

import java.util.Objects;

public final class CtCompare {
    private CtCompare() {
    }

    public static boolean ctEq(byte[] left, byte[] right) {
        Objects.requireNonNull(left, "left");
        Objects.requireNonNull(right, "right");
        if (left.length != right.length) {
            return false;
        }

        int accumulator = 0;
        for (int i = 0; i < left.length; i++) {
            accumulator |= (left[i] ^ right[i]) & 0xFF;
        }
        return accumulator == 0;
    }

    public static boolean ctEqFixed(byte[] left, byte[] right) {
        return ctEq(left, right);
    }

    public static byte[] ctSelectBytes(byte[] left, byte[] right, boolean choice) {
        Objects.requireNonNull(left, "left");
        Objects.requireNonNull(right, "right");
        if (left.length != right.length) {
            throw new IllegalArgumentException("ctSelectBytes requires equal-length inputs");
        }

        int mask = 0 - (choice ? 1 : 0);
        byte[] output = new byte[left.length];
        for (int i = 0; i < left.length; i++) {
            output[i] = (byte) (right[i] ^ ((left[i] ^ right[i]) & mask));
        }
        return output;
    }

    public static boolean ctEqU64(long left, long right) {
        long diff = left ^ right;
        long folded = (diff | -diff) >>> 63;
        return folded == 0L;
    }
}
