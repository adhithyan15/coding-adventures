package com.codingadventures.ctcompare;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class CtCompareTest {
    @Test
    void ctEqMatchesByteEquality() {
        assertTrue(CtCompare.ctEq("abcdef".getBytes(), "abcdef".getBytes()));
        assertTrue(CtCompare.ctEq(new byte[0], new byte[0]));
        assertFalse(CtCompare.ctEq("abcdef".getBytes(), "abcdeg".getBytes()));
        assertFalse(CtCompare.ctEq("abcdef".getBytes(), "bbcdef".getBytes()));
        assertFalse(CtCompare.ctEq("abc".getBytes(), "abcd".getBytes()));
    }

    @Test
    void ctEqDetectsEverySingleBitPosition() {
        byte[] baseline = new byte[32];
        java.util.Arrays.fill(baseline, (byte) 0x42);
        for (int index = 0; index < baseline.length; index++) {
            for (int bit = 0; bit < 8; bit++) {
                byte[] flipped = baseline.clone();
                flipped[index] ^= (byte) (1 << bit);
                assertFalse(CtCompare.ctEq(baseline, flipped));
            }
        }
    }

    @Test
    void ctEqFixedIsDynamicAlias() {
        assertTrue(CtCompare.ctEqFixed(new byte[16], new byte[16]));
        byte[] different = new byte[16];
        different[15] = 1;
        assertFalse(CtCompare.ctEqFixed(new byte[16], different));
    }

    @Test
    void ctSelectBytesChoosesWithoutMutatingInputs() {
        byte[] left = new byte[256];
        byte[] right = new byte[256];
        for (int i = 0; i < 256; i++) {
            left[i] = (byte) i;
            right[i] = (byte) (255 - i);
        }

        assertArrayEquals(left, CtCompare.ctSelectBytes(left, right, true));
        assertArrayEquals(right, CtCompare.ctSelectBytes(left, right, false));
        assertArrayEquals(new byte[0], CtCompare.ctSelectBytes(new byte[0], new byte[0], true));
        assertThrows(IllegalArgumentException.class, () -> CtCompare.ctSelectBytes(new byte[1], new byte[2], true));
    }

    @Test
    void ctEqU64HandlesEdges() {
        assertTrue(CtCompare.ctEqU64(0L, 0L));
        assertTrue(CtCompare.ctEqU64(-1L, -1L));
        assertFalse(CtCompare.ctEqU64(0L, Long.MIN_VALUE));

        long baseline = 0x1234_5678_9ABC_DEF0L;
        for (int bit = 0; bit < 64; bit++) {
            assertFalse(CtCompare.ctEqU64(baseline, baseline ^ (1L << bit)));
        }
    }
}
