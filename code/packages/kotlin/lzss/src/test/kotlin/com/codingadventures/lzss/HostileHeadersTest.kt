// ============================================================================
// HostileHeadersTest.kt — security regression tests
//
// Each case is a few bytes whose header claims an enormous count or length.  An
// implementation that sized an allocation or loop from the claim (or copied the
// whole output per byte) would exhaust memory or run for minutes; these tests
// run under a preemptive timeout.  They also round-trip every input of length
// <= 2 and a seeded random sample.
// ============================================================================

package com.codingadventures.lzss

import org.junit.jupiter.api.Assertions.assertTimeoutPreemptively
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.function.ThrowingSupplier
import java.time.Duration
import java.util.Random
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class HostileHeadersTest {

    private val limit: Duration = Duration.ofSeconds(20)

    private fun b(vararg v: Int): ByteArray = ByteArray(v.size) { v[it].toByte() }

    private fun u32(v: Long): ByteArray =
        b(((v shr 24) and 255).toInt(), ((v shr 16) and 255).toInt(), ((v shr 8) and 255).toInt(), (v and 255).toInt())

    private fun str(s: String): ByteArray = s.toByteArray(Charsets.ISO_8859_1)

    private fun <T> quick(body: () -> T): T = assertTimeoutPreemptively(limit, ThrowingSupplier { body() })

    private fun hdr(len: Long, blocks: Long): ByteArray = u32(len) + u32(blocks)

    @Test
    fun maxHeaderOnly() {
        assertEquals(0, quick { LZSS.decompress(ByteArray(8) { 0xFF.toByte() }) }.size)
    }

    @Test
    fun maxLengthOneLiteral() {
        assertContentEquals(str("x"), quick { LZSS.decompress(hdr(0x7FFFFFFFL, 1) + b(0, 120)) })
        assertContentEquals(str("x"), quick { LZSS.decompress(hdr(0xFFFFFFFFL, 1) + b(0, 120)) })
    }

    @Test
    fun blockCountLargerThanRemainingBytes() {
        assertContentEquals(str("hi"), quick { LZSS.decompress(hdr(2, 0xFFFFFFFFL) + b(0, 104, 105)) })
    }

    @Test
    fun lengthInconsistentWithTokens() {
        assertContentEquals(str("h"), LZSS.decompress(hdr(1, 1) + b(0, 104, 105)))
    }

    @Test
    fun invalidMatchOffsetsAreRejected() {
        assertFailsWith<IllegalArgumentException> { quick { LZSS.decompress(hdr(100, 1) + b(1, 0, 0, 5)) } }
        assertFailsWith<IllegalArgumentException> { quick { LZSS.decompress(hdr(100, 1) + b(1, 255, 255, 255)) } }
    }

    @Test
    fun longOverlappingMatchesDecodeInLinearTime() {
        val d = ByteArray(2_000_000) { 'A'.code.toByte() }
        val c = LZSS.compress(d)
        assertContentEquals(d, quick { LZSS.decompress(c) })
    }

    @Test
    fun roundTripEveryInputUpToLength2() {
        val inputs = ArrayList<ByteArray>()
        inputs.add(ByteArray(0))
        for (a in 0..255) {
            inputs.add(b(a))
            for (c in 0..255) inputs.add(b(a, c))
        }
        for (d in inputs) {
            assertContentEquals(d, LZSS.decompress(LZSS.compress(d)))
        }
    }

    @Test
    fun roundTripSeededRandomInputs() {
        val rng = Random(0x5EED)
        val alphabets = intArrayOf(2, 4, 16, 256)
        for (i in 0 until 300) {
            val d = ByteArray(3 + rng.nextInt(597)) { rng.nextInt(alphabets[i % 4]).toByte() }
            assertContentEquals(d, LZSS.decompress(LZSS.compress(d)))
        }
    }
}
