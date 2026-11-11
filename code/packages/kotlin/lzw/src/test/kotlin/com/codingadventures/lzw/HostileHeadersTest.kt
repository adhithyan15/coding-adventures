// ============================================================================
// HostileHeadersTest.kt — security regression tests
//
// Each case is a few bytes whose header claims an enormous count or length.  An
// implementation that sized an allocation or loop from the claim (or copied the
// whole output per byte) would exhaust memory or run for minutes; these tests
// run under a preemptive timeout.  They also round-trip every input of length
// <= 2 and a seeded random sample.
// ============================================================================

package com.codingadventures.lzw

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

    @Test
    fun maxHeaderOnly() {
        for (n in intArrayOf(4, 8, 64)) {
            try {
                quick { LZW.decompress(ByteArray(n) { 0xFF.toByte() }) }
            } catch (_: IllegalArgumentException) {
            }
        }
    }

    @Test
    fun maxDeclaredLengthOnSmallStream() {
        val c = LZW.compress(str("hello hello hello"))
        u32(0x7FFFFFFFL).copyInto(c, 0)
        assertContentEquals(str("hello hello hello"), quick { LZW.decompress(c) })
    }

    @Test
    fun declaredLengthTruncatesOutput() {
        val c = LZW.compress(str("hello hello hello"))
        u32(5).copyInto(c, 0)
        assertContentEquals(str("hello"), quick { LZW.decompress(c) })
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
            assertContentEquals(d, LZW.decompress(LZW.compress(d)))
        }
    }

    @Test
    fun roundTripSeededRandomInputs() {
        val rng = Random(0x5EED)
        val alphabets = intArrayOf(2, 4, 16, 256)
        for (i in 0 until 300) {
            val d = ByteArray(3 + rng.nextInt(597)) { rng.nextInt(alphabets[i % 4]).toByte() }
            assertContentEquals(d, LZW.decompress(LZW.compress(d)))
        }
    }
}
