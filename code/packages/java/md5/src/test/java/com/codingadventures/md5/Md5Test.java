// ============================================================================
// Md5Test.java — Unit tests for Md5 (RFC 1321 vectors + properties)
// ============================================================================

package com.codingadventures.md5;

import org.junit.jupiter.api.Test;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import static org.junit.jupiter.api.Assertions.*;

class Md5Test {

    private static byte[] a(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    // ── RFC 1321 Appendix A.5 known-answer vectors ───────────────────────────

    @Test
    void rfc1321Vectors() {
        assertEquals("d41d8cd98f00b204e9800998ecf8427e", Md5.hexString(a("")));
        assertEquals("0cc175b9c0f1b6a831c399e269772661", Md5.hexString(a("a")));
        assertEquals("900150983cd24fb0d6963f7d28e17f72", Md5.hexString(a("abc")));
        assertEquals("f96b697d7cb7938d525a2f31aaf161d0", Md5.hexString(a("message digest")));
        assertEquals("c3fcd3d76192e4007dfb496cca67e13b",
            Md5.hexString(a("abcdefghijklmnopqrstuvwxyz")));
        assertEquals("d174ab98d277d9f5a5611c2c9f419d9f",
            Md5.hexString(a("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")));
        assertEquals("57edf4a22be3c955ac49da2e2107b67a", Md5.hexString(a(
            "12345678901234567890123456789012345678901234567890123456789012345678901234567890")));
    }

    // ── Little-endian byte order ──────────────────────────────────────────────

    @Test
    void littleEndianByteOrderOfA() {
        byte[] d = Md5.sumMd5(a("a"));
        assertEquals(0x0c, d[0] & 0xff);
        assertEquals(0xc1, d[1] & 0xff);
        assertEquals(0x75, d[2] & 0xff);
        assertEquals(0xb9, d[3] & 0xff);
    }

    @Test
    void bytes0To255KnownDigest() {
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        assertEquals("e2c865db4162bed963bfaa9ef6ac18f0", Md5.hexString(data));
    }

    // ── Output format ────────────────────────────────────────────────────────

    @Test
    void digestIs16Bytes() {
        assertEquals(16, Md5.sumMd5(a("")).length);
        assertEquals(16, Md5.sumMd5(a("hello world")).length);
        assertEquals(16, Md5.sumMd5(new byte[1000]).length);
    }

    @Test
    void hexIs32LowercaseChars() {
        String h = Md5.hexString(a("abc"));
        assertEquals(32, h.length());
        assertTrue(h.matches("[0-9a-f]{32}"));
    }

    // ── Properties ───────────────────────────────────────────────────────────

    @Test
    void deterministic() {
        assertArrayEquals(Md5.sumMd5(a("hello")), Md5.sumMd5(a("hello")));
    }

    @Test
    void avalanche() {
        byte[] h1 = Md5.sumMd5(a("hello"));
        byte[] h2 = Md5.sumMd5(a("helo"));
        assertFalse(Arrays.equals(h1, h2));
        int bits = 0;
        for (int i = 0; i < 16; i++) bits += Integer.bitCount((h1[i] ^ h2[i]) & 0xff);
        assertTrue(bits > 20, "only " + bits + " bits differed");
    }

    @Test
    void nullByteDiffersFromEmpty() {
        assertFalse(Arrays.equals(Md5.sumMd5(new byte[]{0}), Md5.sumMd5(a(""))));
    }

    @Test
    void everyByteValueHashesDistinctly() {
        Set<String> seen = new HashSet<>();
        for (int i = 0; i <= 255; i++) seen.add(Md5.hexString(new byte[]{(byte) i}));
        assertEquals(256, seen.size());
    }

    // ── Padding / block boundaries ───────────────────────────────────────────

    @Test
    void blockBoundariesProduce16ByteDigests() {
        for (int n : new int[]{0, 55, 56, 63, 64, 127, 128}) {
            assertEquals(16, Md5.sumMd5(new byte[n]).length);
        }
    }

    @Test
    void boundary55And56Differ() {
        assertFalse(Arrays.equals(Md5.sumMd5(new byte[55]), Md5.sumMd5(new byte[56])));
    }

    @Test
    void allBoundarySizesDistinct() {
        Set<String> seen = new HashSet<>();
        for (int n : new int[]{0, 55, 56, 63, 64, 127, 128}) seen.add(Md5.hexString(new byte[n]));
        assertEquals(7, seen.size());
    }

    // ── Streaming digest ─────────────────────────────────────────────────────

    @Test
    void streamingSingleWriteMatchesOneShot() {
        Md5.Digest h = new Md5.Digest();
        h.update(a("abc"));
        assertArrayEquals(Md5.sumMd5(a("abc")), h.digest());
    }

    @Test
    void streamingSplitMatchesOneShot() {
        Md5.Digest h = new Md5.Digest();
        h.update(a("ab"));
        h.update(a("c"));
        assertArrayEquals(Md5.sumMd5(a("abc")), h.digest());
    }

    @Test
    void streamingBlockSplitMatchesOneShot() {
        byte[] data = new byte[128];
        Md5.Digest h = new Md5.Digest();
        h.update(Arrays.copyOfRange(data, 0, 64));
        h.update(Arrays.copyOfRange(data, 64, 128));
        assertArrayEquals(Md5.sumMd5(data), h.digest());
    }

    @Test
    void streamingByteAtATimeMatchesOneShot() {
        byte[] data = new byte[100];
        for (int i = 0; i < 100; i++) data[i] = (byte) i;
        Md5.Digest h = new Md5.Digest();
        for (byte b : data) h.update(new byte[]{b});
        assertArrayEquals(Md5.sumMd5(data), h.digest());
    }

    @Test
    void streamingEmptyMatchesEmptyOneShot() {
        assertArrayEquals(Md5.sumMd5(a("")), new Md5.Digest().digest());
    }

    @Test
    void streamingDigestIsNonDestructive() {
        Md5.Digest h = new Md5.Digest();
        h.update(a("hello"));
        assertArrayEquals(h.digest(), h.digest());
        h.update(a(" world"));
        assertArrayEquals(Md5.sumMd5(a("hello world")), h.digest());
    }

    @Test
    void streamingHexDigestMatches() {
        Md5.Digest h = new Md5.Digest();
        h.update(a("abc"));
        assertEquals("900150983cd24fb0d6963f7d28e17f72", h.hexDigest());
    }

    @Test
    void streamingCopyIsIndependent() {
        Md5.Digest h = new Md5.Digest();
        h.update(a("ab"));
        Md5.Digest h2 = h.copy();
        h2.update(a("c"));
        h.update(a("x"));
        assertArrayEquals(Md5.sumMd5(a("abc")), h2.digest());
        assertArrayEquals(Md5.sumMd5(a("abx")), h.digest());
    }

    @Test
    void streamingMillionAInTwoHalves() {
        byte[] data = new byte[1_000_000];
        Arrays.fill(data, (byte) 'a');
        Md5.Digest h = new Md5.Digest();
        h.update(Arrays.copyOfRange(data, 0, 500_000));
        h.update(Arrays.copyOfRange(data, 500_000, 1_000_000));
        assertArrayEquals(Md5.sumMd5(data), h.digest());
    }
}
