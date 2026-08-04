// ============================================================================
// Sha1Test.kt — Unit tests for Sha1 (FIPS 180-4 / RFC 3174 vectors)
// ============================================================================

package com.codingadventures.sha1

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class Sha1Test {

    private fun a(s: String): ByteArray = s.toByteArray(Charsets.UTF_8)

    @Test
    fun vectors() {
        assertEquals("da39a3ee5e6b4b0d3255bfef95601890afd80709", Sha1.hexString(a("")))
        assertEquals("a9993e364706816aba3e25717850c26c9cd0d89d", Sha1.hexString(a("abc")))
        val msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        assertEquals(56, msg.length)
        assertEquals("84983e441c3bd26ebaae4aa1f95129e5e54670f1", Sha1.hexString(a(msg)))
    }

    @Test
    fun millionA() {
        val data = ByteArray(1_000_000) { 'a'.code.toByte() }
        assertEquals("34aa973cd4c4daa4f61eeb2bdbad27316534016f", Sha1.hexString(data))
    }

    @Test
    fun digestIs20Bytes() {
        assertEquals(20, Sha1.sum1(a("")).size)
        assertEquals(20, Sha1.sum1(a("hello world")).size)
        assertEquals(20, Sha1.sum1(ByteArray(1000)).size)
    }

    @Test
    fun hexIs40LowercaseChars() {
        val h = Sha1.hexString(a("abc"))
        assertEquals(40, h.length)
        assertTrue(Regex("[0-9a-f]{40}").matches(h))
    }

    @Test
    fun deterministic() {
        assertContentEquals(Sha1.sum1(a("hello")), Sha1.sum1(a("hello")))
    }

    @Test
    fun avalanche() {
        val h1 = Sha1.sum1(a("hello"))
        val h2 = Sha1.sum1(a("helo"))
        assertFalse(h1.contentEquals(h2))
        var bits = 0
        for (i in 0 until 20) bits += Integer.bitCount((h1[i].toInt() xor h2[i].toInt()) and 0xff)
        assertTrue(bits > 30, "only $bits bits differed")
    }

    @Test
    fun nullByteDiffersFromEmpty() {
        assertFalse(Sha1.sum1(byteArrayOf(0)).contentEquals(Sha1.sum1(a(""))))
    }

    @Test
    fun everyByteValueHashesDistinctly() {
        val seen = HashSet<String>()
        for (i in 0..255) seen.add(Sha1.hexString(byteArrayOf(i.toByte())))
        assertEquals(256, seen.size)
    }

    @Test
    fun blockBoundariesProduce20ByteDigests() {
        for (n in intArrayOf(0, 55, 56, 63, 64, 127, 128)) {
            assertEquals(20, Sha1.sum1(ByteArray(n)).size)
        }
    }

    @Test
    fun boundary55And56Differ() {
        assertFalse(Sha1.sum1(ByteArray(55)).contentEquals(Sha1.sum1(ByteArray(56))))
    }

    @Test
    fun allBoundarySizesDistinct() {
        val seen = HashSet<String>()
        for (n in intArrayOf(0, 55, 56, 63, 64, 127, 128)) seen.add(Sha1.hexString(ByteArray(n)))
        assertEquals(7, seen.size)
    }

    @Test
    fun streamingSingleWriteMatchesOneShot() {
        val h = Sha1.Digest(); h.update(a("abc"))
        assertContentEquals(Sha1.sum1(a("abc")), h.digest())
    }

    @Test
    fun streamingSplitMatchesOneShot() {
        val h = Sha1.Digest(); h.update(a("ab")); h.update(a("c"))
        assertContentEquals(Sha1.sum1(a("abc")), h.digest())
    }

    @Test
    fun streamingBlockSplitMatchesOneShot() {
        val data = ByteArray(128)
        val h = Sha1.Digest()
        h.update(data.copyOfRange(0, 64)); h.update(data.copyOfRange(64, 128))
        assertContentEquals(Sha1.sum1(data), h.digest())
    }

    @Test
    fun streamingByteAtATimeMatchesOneShot() {
        val data = ByteArray(100) { it.toByte() }
        val h = Sha1.Digest()
        for (b in data) h.update(byteArrayOf(b))
        assertContentEquals(Sha1.sum1(data), h.digest())
    }

    @Test
    fun streamingEmptyMatchesEmptyOneShot() {
        assertContentEquals(Sha1.sum1(a("")), Sha1.Digest().digest())
    }

    @Test
    fun streamingDigestIsNonDestructive() {
        val h = Sha1.Digest(); h.update(a("hello"))
        assertContentEquals(h.digest(), h.digest())
        h.update(a(" world"))
        assertContentEquals(Sha1.sum1(a("hello world")), h.digest())
    }

    @Test
    fun streamingHexDigestMatches() {
        val h = Sha1.Digest(); h.update(a("abc"))
        assertEquals("a9993e364706816aba3e25717850c26c9cd0d89d", h.hexDigest())
    }

    @Test
    fun streamingCopyIsIndependent() {
        val h = Sha1.Digest(); h.update(a("ab"))
        val h2 = h.copy()
        h2.update(a("c")); h.update(a("x"))
        assertContentEquals(Sha1.sum1(a("abc")), h2.digest())
        assertContentEquals(Sha1.sum1(a("abx")), h.digest())
    }

    @Test
    fun streamingMillionAInTwoHalves() {
        val data = ByteArray(1_000_000) { 'a'.code.toByte() }
        val h = Sha1.Digest()
        h.update(data.copyOfRange(0, 500_000)); h.update(data.copyOfRange(500_000, 1_000_000))
        assertEquals("34aa973cd4c4daa4f61eeb2bdbad27316534016f", h.hexDigest())
    }
}
