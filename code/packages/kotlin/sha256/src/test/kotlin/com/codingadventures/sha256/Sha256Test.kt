// ============================================================================
// Sha256Test.kt — Unit tests for Sha256 (FIPS 180-4 vectors + properties)
// ============================================================================

package com.codingadventures.sha256

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class Sha256Test {

    private fun a(s: String): ByteArray = s.toByteArray(Charsets.UTF_8)

    // ── FIPS 180-4 known-answer vectors ──────────────────────────────────────

    @Test
    fun fipsVectors() {
        assertEquals(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            Sha256.sha256Hex(a("")))
        assertEquals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Sha256.sha256Hex(a("abc")))
        val msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        assertEquals(56, msg.length)
        assertEquals(
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            Sha256.sha256Hex(a(msg)))
    }

    @Test
    fun fipsMillionA() {
        val data = ByteArray(1_000_000) { 'a'.code.toByte() }
        assertEquals(
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            Sha256.sha256Hex(data))
    }

    // ── Output format ────────────────────────────────────────────────────────

    @Test
    fun digestIs32Bytes() {
        assertEquals(32, Sha256.sha256(a("")).size)
        assertEquals(32, Sha256.sha256(a("hello world")).size)
        assertEquals(32, Sha256.sha256(ByteArray(1000)).size)
    }

    @Test
    fun hexIs64LowercaseChars() {
        val h = Sha256.sha256Hex(a("abc"))
        assertEquals(64, h.length)
        assertTrue(Regex("[0-9a-f]{64}").matches(h))
    }

    // ── Properties ───────────────────────────────────────────────────────────

    @Test
    fun deterministic() {
        assertContentEquals(Sha256.sha256(a("hello")), Sha256.sha256(a("hello")))
    }

    @Test
    fun avalanche() {
        val h1 = Sha256.sha256(a("hello"))
        val h2 = Sha256.sha256(a("helo"))
        assertFalse(h1.contentEquals(h2))
        var bits = 0
        for (i in 0 until 32) bits += Integer.bitCount((h1[i].toInt() xor h2[i].toInt()) and 0xff)
        assertTrue(bits > 40, "only $bits bits differed")
    }

    @Test
    fun nullByteDiffersFromEmpty() {
        assertFalse(Sha256.sha256(byteArrayOf(0)).contentEquals(Sha256.sha256(a(""))))
    }

    @Test
    fun everyByteValueHashesDistinctly() {
        val seen = HashSet<String>()
        for (i in 0..255) seen.add(Sha256.sha256Hex(byteArrayOf(i.toByte())))
        assertEquals(256, seen.size)
    }

    // ── Padding / block boundaries ───────────────────────────────────────────

    @Test
    fun blockBoundariesProduce32ByteDigests() {
        for (n in intArrayOf(55, 56, 63, 64, 127, 128)) {
            assertEquals(32, Sha256.sha256(ByteArray(n)).size)
        }
    }

    @Test
    fun boundary55And56Differ() {
        assertFalse(Sha256.sha256(ByteArray(55)).contentEquals(Sha256.sha256(ByteArray(56))))
    }

    @Test
    fun allBoundarySizesDistinct() {
        val seen = HashSet<String>()
        for (n in intArrayOf(55, 56, 63, 64, 127, 128)) seen.add(Sha256.sha256Hex(ByteArray(n)))
        assertEquals(6, seen.size)
    }

    // ── Streaming hasher ─────────────────────────────────────────────────────

    @Test
    fun streamingSingleWriteMatchesOneShot() {
        val h = Sha256.Hasher(); h.update(a("abc"))
        assertContentEquals(Sha256.sha256(a("abc")), h.digest())
    }

    @Test
    fun streamingSplitMatchesOneShot() {
        val h = Sha256.Hasher(); h.update(a("ab")); h.update(a("c"))
        assertContentEquals(Sha256.sha256(a("abc")), h.digest())
    }

    @Test
    fun streamingBlockSplitMatchesOneShot() {
        val data = ByteArray(128)
        val h = Sha256.Hasher()
        h.update(data.copyOfRange(0, 64)); h.update(data.copyOfRange(64, 128))
        assertContentEquals(Sha256.sha256(data), h.digest())
    }

    @Test
    fun streamingByteAtATimeMatchesOneShot() {
        val data = ByteArray(100) { it.toByte() }
        val h = Sha256.Hasher()
        for (b in data) h.update(byteArrayOf(b))
        assertContentEquals(Sha256.sha256(data), h.digest())
    }

    @Test
    fun streamingEmptyMatchesEmptyOneShot() {
        assertContentEquals(Sha256.sha256(a("")), Sha256.Hasher().digest())
    }

    @Test
    fun streamingDigestIsNonDestructive() {
        val h = Sha256.Hasher(); h.update(a("abc"))
        assertContentEquals(h.digest(), h.digest())
        h.update(a("d"))
        assertContentEquals(Sha256.sha256(a("abcd")), h.digest())
    }

    @Test
    fun streamingHexDigestMatches() {
        val h = Sha256.Hasher(); h.update(a("abc"))
        assertEquals(Sha256.sha256Hex(a("abc")), h.hexDigest())
    }

    @Test
    fun streamingCopyIsIndependent() {
        val h = Sha256.Hasher(); h.update(a("ab"))
        val h2 = h.copy()
        h2.update(a("c")); h.update(a("x"))
        assertContentEquals(Sha256.sha256(a("abc")), h2.digest())
        assertContentEquals(Sha256.sha256(a("abx")), h.digest())
    }

    @Test
    fun streamingMillionAInTwoHalves() {
        val data = ByteArray(1_000_000) { 'a'.code.toByte() }
        val h = Sha256.Hasher()
        h.update(data.copyOfRange(0, 500_000)); h.update(data.copyOfRange(500_000, 1_000_000))
        assertEquals(
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            h.hexDigest())
    }
}
