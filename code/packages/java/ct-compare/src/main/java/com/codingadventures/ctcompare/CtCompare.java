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

    /**
     * Return a fresh copy of {@code left} when {@code choice} is 1 and of
     * {@code right} when it is 0, without branching on {@code choice}.
     *
     * <p><b>Why an int and not a boolean.</b> Java has no conversion from
     * {@code boolean} to an integer, so turning a boolean into a mask needs
     * {@code choice ? 1 : 0}, a branch on the secret bit that CT01 forbids.
     * The bit is therefore passed as an {@code int} that must be exactly 0 or
     * 1, as Go's {@code crypto/subtle} does.
     *
     * <pre>
     * choice = 0  -&gt;  mask = -0 = 0x00000000  -&gt;  result is right
     * choice = 1  -&gt;  mask = -1 = 0xFFFFFFFF  -&gt;  result is left
     * </pre>
     *
     * <p>Validation tests {@code choice & ~1}, which is 0 for both valid values,
     * so a valid call takes the same path whichever bit is secret.
     *
     * @throws IllegalArgumentException if the lengths differ or {@code choice}
     *     is not 0 or 1; thrown before either input is read
     */
    public static byte[] ctSelectBytes(byte[] left, byte[] right, int choice) {
        Objects.requireNonNull(left, "left");
        Objects.requireNonNull(right, "right");
        if (left.length != right.length) {
            throw new IllegalArgumentException("ctSelectBytes requires equal-length inputs");
        }
        if ((choice & ~1) != 0) {
            throw new IllegalArgumentException("ctSelectBytes requires choice to be 0 or 1");
        }

        int mask = -choice;
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

    // ── Bit-returning equality (CT01, "How a lane receives the selection bit") ──
    //
    // ctSelectBytes takes its selection bit as an int, because Java cannot
    // turn a boolean into an int without a branch. A caller holding only
    // ctEq's boolean would need `eq ? 1 : 0`, the very branch CT01 removes,
    // so each equality also returns the bit itself, computed arithmetically
    // from the difference accumulator:
    //
    //   acc = 0          (0 - 1) >>> 31 = 1        (equal)
    //   acc = 1 .. 255   (acc - 1) >>> 31 = 0      (different)
    //
    // Use it as ctSelectBytes(a, b, ctEqBit(x, y)).

    /** 1 when the arrays are equal, else 0; a length mismatch is 0. */
    public static int ctEqBit(byte[] left, byte[] right) {
        Objects.requireNonNull(left, "left");
        Objects.requireNonNull(right, "right");
        if (left.length != right.length) {
            return 0;
        }
        int accumulator = 0;
        for (int i = 0; i < left.length; i++) {
            accumulator |= (left[i] ^ right[i]) & 0xFF;
        }
        return (accumulator - 1) >>> 31;
    }

    /** ctEqBit for inputs of a declared public width. */
    public static int ctEqFixedBit(byte[] left, byte[] right) {
        return ctEqBit(left, right);
    }

    /**
     * 1 when left == right, else 0. {@code diff | -diff} has its top bit set
     * exactly when diff != 0, so shifting it down and flipping gives the bit.
     */
    public static int ctEqU64Bit(long left, long right) {
        long diff = left ^ right;
        return (int) (((diff | -diff) >>> 63) ^ 1L);
    }
}
