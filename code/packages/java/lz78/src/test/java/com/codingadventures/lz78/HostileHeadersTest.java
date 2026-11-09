package com.codingadventures.lz78;

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

    private static byte[] hdr(long len, long count) {
        return cat(u32(len), u32(count));
    }

    @Test
    void maxHeaderIsRejected() {
        byte[] d = new byte[8];
        java.util.Arrays.fill(d, (byte) 0xFF);
        org.junit.jupiter.api.Assertions.assertNull(quick(() -> Lz78.decompress(d)));
    }

    @Test
    void maxLengthWithTooFewTokensIsRejected() {
        org.junit.jupiter.api.Assertions.assertNull(
            quick(() -> Lz78.decompress(cat(hdr(0x7FFFFFFFL, 1), bytes(0, 0, 104, 0)))));
    }

    @Test
    void countLargerThanDataIsRejected() {
        org.junit.jupiter.api.Assertions.assertNull(
            quick(() -> Lz78.decompress(cat(hdr(2, 0x7FFFFFFFL), bytes(0, 0, 104, 0, 0, 0, 105, 0)))));
    }

    @Test
    void danglingDictionaryIndexIsRejected() {
        org.junit.jupiter.api.Assertions.assertNull(
            quick(() -> Lz78.decompress(cat(hdr(10, 2), bytes(0, 0, 97, 0, 0x27, 0x0F, 98, 0)))));
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
            assertArrayEquals(d, Lz78.decompress(Lz78.compress(d)));
        }
    }

    @Test
    void roundTripSeededRandomInputs() {
        Random rng = new Random(0x5EED);
        int[] alphabets = {2, 4, 16, 256};
        for (int i = 0; i < 300; i++) {
            byte[] d = new byte[3 + rng.nextInt(597)];
            for (int j = 0; j < d.length; j++) d[j] = (byte) rng.nextInt(alphabets[i % 4]);
            assertArrayEquals(d, Lz78.decompress(Lz78.compress(d)));
        }
    }
}
