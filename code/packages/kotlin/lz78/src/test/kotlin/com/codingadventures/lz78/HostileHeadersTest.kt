// ============================================================================
// HostileHeadersTest.kt — security regression tests
//
// Each case is a few bytes whose header claims an enormous count or length.  An
// implementation that sized an allocation or loop from the claim (or copied the
// whole output per byte) would exhaust memory or run for minutes; these tests
// run under a preemptive timeout.  They also round-trip every input of length
// <= 2 and a seeded random sample.
// ============================================================================

package com.codingadventures.lz78

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

    private fun hdr(len: Long, count: Long): ByteArray = u32(len) + u32(count)

    @Test
    fun maxHeaderIsRejected() {
        assertFailsWith<IllegalArgumentException> { quick { Lz78.decompress(ByteArray(8) { 0xFF.toByte() }) } }
    }

    @Test
    fun maxLengthWithTooFewTokensIsRejected() {
        assertFailsWith<IllegalArgumentException> {
            quick { Lz78.decompress(hdr(0x7FFFFFFFL, 1) + b(0, 0, 104, 0)) }
        }
    }

    @Test
    fun countLargerThanDataIsRejected() {
        assertFailsWith<IllegalArgumentException> {
            quick { Lz78.decompress(hdr(2, 0x7FFFFFFFL) + b(0, 0, 104, 0, 0, 0, 105, 0)) }
        }
    }

    @Test
    fun danglingDictionaryIndexIsRejected() {
        assertFailsWith<IllegalArgumentException> {
            quick { Lz78.decompress(hdr(10, 2) + b(0, 0, 97, 0, 0x27, 0x0F, 98, 0)) }
        }
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
            assertContentEquals(d, Lz78.decompress(Lz78.compress(d)))
        }
    }

    @Test
    fun roundTripSeededRandomInputs() {
        val rng = Random(0x5EED)
        val alphabets = intArrayOf(2, 4, 16, 256)
        for (i in 0 until 300) {
            val d = ByteArray(3 + rng.nextInt(597)) { rng.nextInt(alphabets[i % 4]).toByte() }
            assertContentEquals(d, Lz78.decompress(Lz78.compress(d)))
        }
    }
}
