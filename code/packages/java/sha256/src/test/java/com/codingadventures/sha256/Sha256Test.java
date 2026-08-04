// ============================================================================
// Sha256Test.java — Unit tests for Sha256 (FIPS 180-4 vectors + properties)
// ============================================================================

package com.codingadventures.sha256;

import org.junit.jupiter.api.Test;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Set;
import static org.junit.jupiter.api.Assertions.*;

class Sha256Test {

    private static byte[] a(String s) {
        return s.getBytes(StandardCharsets.UTF_8);
    }

    // ── FIPS 180-4 known-answer vectors ──────────────────────────────────────

    @Test
    void fipsEmptyString() {
        assertEquals(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            Sha256.sha256Hex(a("")));
    }

    @Test
    void fipsAbc() {
        assertEquals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Sha256.sha256Hex(a("abc")));
    }

    @Test
    void fips448BitMessage() {
        String msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assertEquals(56, msg.length());
        assertEquals(
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            Sha256.sha256Hex(a(msg)));
    }

    @Test
    void fipsMillionA() {
        byte[] data = new byte[1_000_000];
        Arrays.fill(data, (byte) 'a');
        assertEquals(
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            Sha256.sha256Hex(data));
    }

    // ── Output format ────────────────────────────────────────────────────────

    @Test
    void digestIs32Bytes() {
        assertEquals(32, Sha256.sha256(a("")).length);
        assertEquals(32, Sha256.sha256(a("hello world")).length);
        assertEquals(32, Sha256.sha256(new byte[1000]).length);
    }

    @Test
    void hexIs64LowercaseChars() {
        String h = Sha256.sha256Hex(a("abc"));
        assertEquals(64, h.length());
        assertTrue(h.matches("[0-9a-f]{64}"));
    }

    // ── Properties ───────────────────────────────────────────────────────────

    @Test
    void deterministic() {
        assertArrayEquals(Sha256.sha256(a("hello")), Sha256.sha256(a("hello")));
    }

    @Test
    void avalanche() {
        byte[] h1 = Sha256.sha256(a("hello"));
        byte[] h2 = Sha256.sha256(a("helo"));
        assertFalse(Arrays.equals(h1, h2));
        int bits = 0;
        for (int i = 0; i < 32; i++) {
            bits += Integer.bitCount((h1[i] ^ h2[i]) & 0xff);
        }
        assertTrue(bits > 40, "only " + bits + " bits differed");
    }

    @Test
    void nullByteDiffersFromEmpty() {
        assertFalse(Arrays.equals(Sha256.sha256(new byte[]{0}), Sha256.sha256(a(""))));
    }

    @Test
    void everyByteValueHashesDistinctly() {
        Set<String> seen = new HashSet<>();
        for (int i = 0; i <= 255; i++) {
            seen.add(Sha256.sha256Hex(new byte[]{(byte) i}));
        }
        assertEquals(256, seen.size());
    }

    // ── Padding / block boundaries ───────────────────────────────────────────

    @Test
    void blockBoundariesProduce32ByteDigests() {
        for (int n : new int[]{55, 56, 63, 64, 127, 128}) {
            assertEquals(32, Sha256.sha256(new byte[n]).length);
        }
    }

    @Test
    void boundary55And56Differ() {
        assertFalse(Arrays.equals(Sha256.sha256(new byte[55]), Sha256.sha256(new byte[56])));
    }

    @Test
    void allBoundarySizesDistinct() {
        Set<String> seen = new HashSet<>();
        for (int n : new int[]{55, 56, 63, 64, 127, 128}) {
            seen.add(Sha256.sha256Hex(new byte[n]));
        }
        assertEquals(6, seen.size());
    }

    // ── Streaming hasher ─────────────────────────────────────────────────────

    @Test
    void streamingSingleWriteMatchesOneShot() {
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(a("abc"));
        assertArrayEquals(Sha256.sha256(a("abc")), h.digest());
    }

    @Test
    void streamingSplitMatchesOneShot() {
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(a("ab"));
        h.update(a("c"));
        assertArrayEquals(Sha256.sha256(a("abc")), h.digest());
    }

    @Test
    void streamingBlockSplitMatchesOneShot() {
        byte[] data = new byte[128];
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(Arrays.copyOfRange(data, 0, 64));
        h.update(Arrays.copyOfRange(data, 64, 128));
        assertArrayEquals(Sha256.sha256(data), h.digest());
    }

    @Test
    void streamingByteAtATimeMatchesOneShot() {
        byte[] data = new byte[100];
        for (int i = 0; i < 100; i++) data[i] = (byte) i;
        Sha256.Hasher h = new Sha256.Hasher();
        for (byte b : data) h.update(new byte[]{b});
        assertArrayEquals(Sha256.sha256(data), h.digest());
    }

    @Test
    void streamingEmptyMatchesEmptyOneShot() {
        assertArrayEquals(Sha256.sha256(a("")), new Sha256.Hasher().digest());
    }

    @Test
    void streamingDigestIsNonDestructive() {
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(a("abc"));
        assertArrayEquals(h.digest(), h.digest());
        h.update(a("d"));
        assertArrayEquals(Sha256.sha256(a("abcd")), h.digest());
    }

    @Test
    void streamingHexDigestMatches() {
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(a("abc"));
        assertEquals(Sha256.sha256Hex(a("abc")), h.hexDigest());
    }

    @Test
    void streamingCopyIsIndependent() {
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(a("ab"));
        Sha256.Hasher h2 = h.copy();
        h2.update(a("c"));
        h.update(a("x"));
        assertArrayEquals(Sha256.sha256(a("abc")), h2.digest());
        assertArrayEquals(Sha256.sha256(a("abx")), h.digest());
    }

    @Test
    void streamingMillionAInTwoHalves() {
        byte[] data = new byte[1_000_000];
        Arrays.fill(data, (byte) 'a');
        Sha256.Hasher h = new Sha256.Hasher();
        h.update(Arrays.copyOfRange(data, 0, 500_000));
        h.update(Arrays.copyOfRange(data, 500_000, 1_000_000));
        assertEquals(
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            h.hexDigest());
    }
}
