// ============================================================================
// Md5Test.kt — Unit tests for Md5 (RFC 1321 vectors + properties)
// ============================================================================

package com.codingadventures.md5

import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class Md5Test {

    private fun a(s: String): ByteArray = s.toByteArray(Charsets.UTF_8)

    @Test
    fun rfc1321Vectors() {
        assertEquals("d41d8cd98f00b204e9800998ecf8427e", Md5.hexString(a("")))
        assertEquals("0cc175b9c0f1b6a831c399e269772661", Md5.hexString(a("a")))
        assertEquals("900150983cd24fb0d6963f7d28e17f72", Md5.hexString(a("abc")))
        assertEquals("f96b697d7cb7938d525a2f31aaf161d0", Md5.hexString(a("message digest")))
        assertEquals("c3fcd3d76192e4007dfb496cca67e13b",
            Md5.hexString(a("abcdefghijklmnopqrstuvwxyz")))
        assertEquals("d174ab98d277d9f5a5611c2c9f419d9f",
            Md5.hexString(a("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")))
        assertEquals("57edf4a22be3c955ac49da2e2107b67a", Md5.hexString(a(
            "12345678901234567890123456789012345678901234567890123456789012345678901234567890")))
    }

    @Test
    fun littleEndianByteOrderOfA() {
        val d = Md5.sumMd5(a("a"))
        assertEquals(0x0c, d[0].toInt() and 0xff)
        assertEquals(0xc1, d[1].toInt() and 0xff)
        assertEquals(0x75, d[2].toInt() and 0xff)
        assertEquals(0xb9, d[3].toInt() and 0xff)
    }

    @Test
    fun bytes0To255KnownDigest() {
        val data = ByteArray(256) { it.toByte() }
        assertEquals("e2c865db4162bed963bfaa9ef6ac18f0", Md5.hexString(data))
    }

    @Test
    fun digestIs16Bytes() {
        assertEquals(16, Md5.sumMd5(a("")).size)
        assertEquals(16, Md5.sumMd5(a("hello world")).size)
        assertEquals(16, Md5.sumMd5(ByteArray(1000)).size)
    }

    @Test
    fun hexIs32LowercaseChars() {
        val h = Md5.hexString(a("abc"))
        assertEquals(32, h.length)
        assertTrue(Regex("[0-9a-f]{32}").matches(h))
    }

    @Test
    fun deterministic() {
        assertContentEquals(Md5.sumMd5(a("hello")), Md5.sumMd5(a("hello")))
    }

    @Test
    fun avalanche() {
        val h1 = Md5.sumMd5(a("hello"))
        val h2 = Md5.sumMd5(a("helo"))
        assertFalse(h1.contentEquals(h2))
        var bits = 0
        for (i in 0 until 16) bits += Integer.bitCount((h1[i].toInt() xor h2[i].toInt()) and 0xff)
        assertTrue(bits > 20, "only $bits bits differed")
    }

    @Test
    fun nullByteDiffersFromEmpty() {
        assertFalse(Md5.sumMd5(byteArrayOf(0)).contentEquals(Md5.sumMd5(a(""))))
    }

    @Test
    fun everyByteValueHashesDistinctly() {
        val seen = HashSet<String>()
        for (i in 0..255) seen.add(Md5.hexString(byteArrayOf(i.toByte())))
        assertEquals(256, seen.size)
    }

    @Test
    fun blockBoundariesProduce16ByteDigests() {
        for (n in intArrayOf(0, 55, 56, 63, 64, 127, 128)) {
            assertEquals(16, Md5.sumMd5(ByteArray(n)).size)
        }
    }

    @Test
    fun boundary55And56Differ() {
        assertFalse(Md5.sumMd5(ByteArray(55)).contentEquals(Md5.sumMd5(ByteArray(56))))
    }

    @Test
    fun allBoundarySizesDistinct() {
        val seen = HashSet<String>()
        for (n in intArrayOf(0, 55, 56, 63, 64, 127, 128)) seen.add(Md5.hexString(ByteArray(n)))
        assertEquals(7, seen.size)
    }

    @Test
    fun streamingSingleWriteMatchesOneShot() {
        val h = Md5.Digest(); h.update(a("abc"))
        assertContentEquals(Md5.sumMd5(a("abc")), h.digest())
    }

    @Test
    fun streamingSplitMatchesOneShot() {
        val h = Md5.Digest(); h.update(a("ab")); h.update(a("c"))
        assertContentEquals(Md5.sumMd5(a("abc")), h.digest())
    }

    @Test
    fun streamingBlockSplitMatchesOneShot() {
        val data = ByteArray(128)
        val h = Md5.Digest()
        h.update(data.copyOfRange(0, 64)); h.update(data.copyOfRange(64, 128))
        assertContentEquals(Md5.sumMd5(data), h.digest())
    }

    @Test
    fun streamingByteAtATimeMatchesOneShot() {
        val data = ByteArray(100) { it.toByte() }
        val h = Md5.Digest()
        for (b in data) h.update(byteArrayOf(b))
        assertContentEquals(Md5.sumMd5(data), h.digest())
    }

    @Test
    fun streamingEmptyMatchesEmptyOneShot() {
        assertContentEquals(Md5.sumMd5(a("")), Md5.Digest().digest())
    }

    @Test
    fun streamingDigestIsNonDestructive() {
        val h = Md5.Digest(); h.update(a("hello"))
        assertContentEquals(h.digest(), h.digest())
        h.update(a(" world"))
        assertContentEquals(Md5.sumMd5(a("hello world")), h.digest())
    }

    @Test
    fun streamingHexDigestMatches() {
        val h = Md5.Digest(); h.update(a("abc"))
        assertEquals("900150983cd24fb0d6963f7d28e17f72", h.hexDigest())
    }

    @Test
    fun streamingCopyIsIndependent() {
        val h = Md5.Digest(); h.update(a("ab"))
        val h2 = h.copy()
        h2.update(a("c")); h.update(a("x"))
        assertContentEquals(Md5.sumMd5(a("abc")), h2.digest())
        assertContentEquals(Md5.sumMd5(a("abx")), h.digest())
    }

    @Test
    fun streamingMillionAInTwoHalves() {
        val data = ByteArray(1_000_000) { 'a'.code.toByte() }
        val h = Md5.Digest()
        h.update(data.copyOfRange(0, 500_000)); h.update(data.copyOfRange(500_000, 1_000_000))
        assertContentEquals(Md5.sumMd5(data), h.digest())
    }
}
