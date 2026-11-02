package com.codingadventures.ctcompare;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

/** The bit-returning equality variants (CT01): agree with the booleans, 0 or 1 only. */
class CtCompareBitTest {
    @Test
    void ctEqBitAgreesOnEveryBytePair() {
        for (int a = 0; a < 256; a++) {
            for (int b = 0; b < 256; b++) {
                byte[] left = {(byte) a};
                byte[] right = {(byte) b};
                int want = CtCompare.ctEq(left, right) ? 1 : 0;
                assertEquals(want, CtCompare.ctEqBit(left, right), a + " vs " + b);
                assertEquals(want, CtCompare.ctEqFixedBit(left, right), a + " vs " + b);
            }
        }
    }

    @Test
    void lengthsAndEveryBitPosition() {
        assertEquals(1, CtCompare.ctEqBit(new byte[0], new byte[0]));
        assertEquals(0, CtCompare.ctEqBit(new byte[] {1}, new byte[] {1, 2}));
        assertThrows(NullPointerException.class, () -> CtCompare.ctEqBit(null, new byte[0]));
        byte[] base = new byte[64];
        for (int i = 0; i < base.length; i++) {
            for (int bit = 0; bit < 8; bit++) {
                byte[] other = base.clone();
                other[i] ^= (byte) (1 << bit);
                assertEquals(0, CtCompare.ctEqBit(base, other), "byte " + i + " bit " + bit);
            }
        }
    }

    @Test
    void ctEqU64Bit() {
        long[] values = {0L, 1L, 2L, Long.MIN_VALUE, Long.MAX_VALUE, -1L, 0x0123456789abcdefL};
        for (long x : values) {
            for (long y : values) {
                assertEquals(x == y ? 1 : 0, CtCompare.ctEqU64Bit(x, y));
            }
            for (int bit = 0; bit < 64; bit++) {
                assertEquals(0, CtCompare.ctEqU64Bit(x, x ^ (1L << bit)));
            }
        }
    }

    @Test
    void equalityBitFeedsSelection() {
        byte[] a = "left".getBytes();
        byte[] b = "rite".getBytes();
        assertArrayEquals(a, CtCompare.ctSelectBytes(a, b, CtCompare.ctEqBit("k".getBytes(), "k".getBytes())));
        assertArrayEquals(b, CtCompare.ctSelectBytes(a, b, CtCompare.ctEqBit("k".getBytes(), "j".getBytes())));
        assertArrayEquals(a, CtCompare.ctSelectBytes(a, b, CtCompare.ctEqU64Bit(7L, 7L)));
    }
}
