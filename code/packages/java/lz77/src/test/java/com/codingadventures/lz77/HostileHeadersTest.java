package com.codingadventures.lz77;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTimeoutPreemptively;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.Random;
import java.util.function.Supplier;
import org.junit.jupiter.api.Test;

/**
 * Security regression tests: hostile headers must not drive work or memory.
 *
 * <p>Each case is a few bytes of input whose header claims an enormous count or
 * length.  An implementation that sized an allocation or loop from the claim
 * (or copied the whole output per byte) would exhaust memory or run for
 * minutes; these tests run each case under a preemptive timeout.  They also
 * round-trip every input of length &lt;= 2 and a seeded random sample.
 */
class HostileHeadersTest {

    private static final Duration LIMIT = Duration.ofSeconds(20);

    private static byte[] bytes(int... v) {
        byte[] b = new byte[v.length];
        for (int i = 0; i < v.length; i++) b[i] = (byte) v[i];
        return b;
    }

    private static byte[] cat(byte[]... parts) {
        int n = 0;
        for (byte[] p : parts) n += p.length;
        ByteBuffer buf = ByteBuffer.allocate(n);
        for (byte[] p : parts) buf.put(p);
        return buf.array();
    }

    private static byte[] u32(long v) {
        return bytes((int) (v >>> 24) & 255, (int) (v >>> 16) & 255, (int) (v >>> 8) & 255, (int) v & 255);
    }

    private static byte[] str(String s) {
        return s.getBytes(StandardCharsets.ISO_8859_1);
    }

    /** Run {@code body} with a timeout; malformed input may throw IllegalArgumentException. */
    private static byte[] quick(Supplier<byte[]> body) {
        return assertTimeoutPreemptively(LIMIT, () -> {
            try {
                return body.get();
            } catch (IllegalArgumentException e) {
                return null; // rejected cleanly
            }
        });
    }

    @Test
    void maxCountHeaderOnly() {
        assertEquals(0, quick(() -> LZ77.decompress(bytes(255, 255, 255, 255))).length);
    }

    @Test
    void countLargerThanRemainingBytesYieldsPresentTokens() {
        byte[] d = bytes(255, 255, 255, 255, 0, 0, 0, 104, 0, 0, 0, 105);
        assertArrayEquals(str("hi"), quick(() -> LZ77.decompress(d)));
        assertArrayEquals(str("hi"), LZ77.decompress(cat(d, bytes(0, 0, 0))));
    }

    @Test
    void countSmallerThanRemainingBytesStopsAtCount() {
        assertArrayEquals(str("h"), LZ77.decompress(bytes(0, 0, 0, 1, 0, 0, 0, 104, 0, 0, 0, 105)));
    }

    @Test
    void invalidOffsetsAreRejectedNotCrashed() {
        // Rejected with IllegalArgumentException (quick() maps that to null).
        org.junit.jupiter.api.Assertions.assertNull(quick(() -> LZ77.decompress(bytes(0, 0, 0, 1, 0, 5, 3, 120))));
        org.junit.jupiter.api.Assertions.assertNull(quick(() -> LZ77.decompress(bytes(0, 0, 0, 1, 0, 0, 3, 120))));
    }

    @Test
    void longOverlappingMatchesDecodeInLinearTime() {
        // Regression: decode used out.toByteArray() per copied byte (quadratic).
        byte[] d = new byte[2_000_000];
        java.util.Arrays.fill(d, (byte) 'A');
        byte[] c = LZ77.compress(d);
        assertArrayEquals(d, quick(() -> LZ77.decompress(c)));
    }

    @Test
    void roundTripEveryInputUpToLength2() {
        byte[][] inputs = new byte[1 + 256 + 65536][];
        int k = 0;
        inputs[k++] = new byte[0];
        for (int a = 0; a < 256; a++) {
            inputs[k++] = bytes(a);
            for (int b = 0; b < 256; b++) inputs[k++] = bytes(a, b);
        }
        for (byte[] d : inputs) {
            assertArrayEquals(d, LZ77.decompress(LZ77.compress(d)));
        }
    }

    @Test
    void roundTripSeededRandomInputs() {
        Random rng = new Random(0x5EED);
        int[] alphabets = {2, 4, 16, 256};
        for (int i = 0; i < 300; i++) {
            byte[] d = new byte[3 + rng.nextInt(597)];
            for (int j = 0; j < d.length; j++) d[j] = (byte) rng.nextInt(alphabets[i % 4]);
            assertArrayEquals(d, LZ77.decompress(LZ77.compress(d)));
        }
    }
}
