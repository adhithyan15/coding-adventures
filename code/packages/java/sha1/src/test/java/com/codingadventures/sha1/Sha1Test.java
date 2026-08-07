// ============================================================================
// Sha1Test.java — Unit tests for Sha1 (FIPS 180-4 / RFC 3174 vectors)
// ============================================================================

package com.codingadventures.sha1;

import org.junit.jupiter.api.Test;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import static org.junit.jupiter.api.Assertions.*;

class Sha1Test {

    private static byte[] a(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    // ── Known-answer vectors ─────────────────────────────────────────────────

    @Test
    void vectors() {
        assertEquals("da39a3ee5e6b4b0d3255bfef95601890afd80709", Sha1.hexString(a("")));
        assertEquals("a9993e364706816aba3e25717850c26c9cd0d89d", Sha1.hexString(a("abc")));
        String msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assertEquals(56, msg.length());
        assertEquals("84983e441c3bd26ebaae4aa1f95129e5e54670f1", Sha1.hexString(a(msg)));
    }

    @Test
    void millionA() {
        byte[] data = new byte[1_000_000];
        Arrays.fill(data, (byte) 'a');
        assertEquals("34aa973cd4c4daa4f61eeb2bdbad27316534016f", Sha1.hexString(data));
    }

    // ── Output format ────────────────────────────────────────────────────────

    @Test
    void digestIs20Bytes() {
        assertEquals(20, Sha1.sum1(a("")).length);
        assertEquals(20, Sha1.sum1(a("hello world")).length);
        assertEquals(20, Sha1.sum1(new byte[1000]).length);
    }

    @Test
    void hexIs40LowercaseChars() {
        String h = Sha1.hexString(a("abc"));
        assertEquals(40, h.length());
        assertTrue(h.matches("[0-9a-f]{40}"));
    }

    // ── Properties ───────────────────────────────────────────────────────────

    @Test
    void deterministic() {
        assertArrayEquals(Sha1.sum1(a("hello")), Sha1.sum1(a("hello")));
    }

    @Test
    void avalanche() {
        byte[] h1 = Sha1.sum1(a("hello"));
        byte[] h2 = Sha1.sum1(a("helo"));
        assertFalse(Arrays.equals(h1, h2));
        int bits = 0;
        for (int i = 0; i < 20; i++) bits += Integer.bitCount((h1[i] ^ h2[i]) & 0xff);
        assertTrue(bits > 30, "only " + bits + " bits differed");
    }

    @Test
    void nullByteDiffersFromEmpty() {
        assertFalse(Arrays.equals(Sha1.sum1(new byte[]{0}), Sha1.sum1(a(""))));
    }

    @Test
    void everyByteValueHashesDistinctly() {
        Set<String> seen = new HashSet<>();
        for (int i = 0; i <= 255; i++) seen.add(Sha1.hexString(new byte[]{(byte) i}));
        assertEquals(256, seen.size());
    }

    // ── Padding / block boundaries ───────────────────────────────────────────

    @Test
    void blockBoundariesProduce20ByteDigests() {
        for (int n : new int[]{0, 55, 56, 63, 64, 127, 128}) {
            assertEquals(20, Sha1.sum1(new byte[n]).length);
        }
    }

    @Test
    void boundary55And56Differ() {
        assertFalse(Arrays.equals(Sha1.sum1(new byte[55]), Sha1.sum1(new byte[56])));
    }

    @Test
    void allBoundarySizesDistinct() {
        Set<String> seen = new HashSet<>();
        for (int n : new int[]{0, 55, 56, 63, 64, 127, 128}) seen.add(Sha1.hexString(new byte[n]));
        assertEquals(7, seen.size());
    }

    // ── Streaming digest ─────────────────────────────────────────────────────

    @Test
    void streamingSingleWriteMatchesOneShot() {
        Sha1.Digest h = new Sha1.Digest();
        h.update(a("abc"));
        assertArrayEquals(Sha1.sum1(a("abc")), h.digest());
    }

    @Test
    void streamingSplitMatchesOneShot() {
        Sha1.Digest h = new Sha1.Digest();
        h.update(a("ab"));
        h.update(a("c"));
        assertArrayEquals(Sha1.sum1(a("abc")), h.digest());
    }

    @Test
    void streamingBlockSplitMatchesOneShot() {
        byte[] data = new byte[128];
        Sha1.Digest h = new Sha1.Digest();
        h.update(Arrays.copyOfRange(data, 0, 64));
        h.update(Arrays.copyOfRange(data, 64, 128));
        assertArrayEquals(Sha1.sum1(data), h.digest());
    }

    @Test
    void streamingByteAtATimeMatchesOneShot() {
        byte[] data = new byte[100];
        for (int i = 0; i < 100; i++) data[i] = (byte) i;
        Sha1.Digest h = new Sha1.Digest();
        for (byte b : data) h.update(new byte[]{b});
        assertArrayEquals(Sha1.sum1(data), h.digest());
    }

    @Test
    void streamingEmptyMatchesEmptyOneShot() {
        assertArrayEquals(Sha1.sum1(a("")), new Sha1.Digest().digest());
    }

    @Test
    void streamingDigestIsNonDestructive() {
        Sha1.Digest h = new Sha1.Digest();
        h.update(a("hello"));
        assertArrayEquals(h.digest(), h.digest());
        h.update(a(" world"));
        assertArrayEquals(Sha1.sum1(a("hello world")), h.digest());
    }

    @Test
    void streamingHexDigestMatches() {
        Sha1.Digest h = new Sha1.Digest();
        h.update(a("abc"));
        assertEquals("a9993e364706816aba3e25717850c26c9cd0d89d", h.hexDigest());
    }

    @Test
    void streamingCopyIsIndependent() {
        Sha1.Digest h = new Sha1.Digest();
        h.update(a("ab"));
        Sha1.Digest h2 = h.copy();
        h2.update(a("c"));
        h.update(a("x"));
        assertArrayEquals(Sha1.sum1(a("abc")), h2.digest());
        assertArrayEquals(Sha1.sum1(a("abx")), h.digest());
    }

    @Test
    void streamingMillionAInTwoHalves() {
        byte[] data = new byte[1_000_000];
        Arrays.fill(data, (byte) 'a');
        Sha1.Digest h = new Sha1.Digest();
        h.update(Arrays.copyOfRange(data, 0, 500_000));
        h.update(Arrays.copyOfRange(data, 500_000, 1_000_000));
        assertEquals("34aa973cd4c4daa4f61eeb2bdbad27316534016f", h.hexDigest());
    }
}
