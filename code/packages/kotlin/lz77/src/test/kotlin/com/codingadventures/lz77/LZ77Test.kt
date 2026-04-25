// ============================================================================
// LZ77Test.kt — CMP00: LZ77 Compression Tests
// ============================================================================

package com.codingadventures.lz77

import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import org.junit.jupiter.api.assertThrows

class LZ77Test {

    // =========================================================================
    // 1. Round-trip
    // =========================================================================

    @Test fun roundTrip_helloWorld()       = roundTrip("hello world".toByteArray())
    @Test fun roundTrip_aaabbc()           = roundTrip("AAABBC".toByteArray())
    @Test fun roundTrip_repeatedPattern()  = roundTrip(repeat("ABCABC".toByteArray(), 100))
    @Test fun roundTrip_loremIpsum()       = roundTrip(("Lorem ipsum dolor sit amet, " +
        "consectetur adipiscing elit. Sed do eiusmod tempor.").toByteArray())
    @Test fun roundTrip_all256Bytes()      = roundTrip(ByteArray(256) { it.toByte() })
    @Test fun roundTrip_binaryData()       = roundTrip(byteArrayOf(0, 1, 2, 3,
        255.toByte(), 254.toByte(), 253.toByte(), 128.toByte(), 64, 32))
    @Test fun roundTrip_longInput()        = roundTrip(repeat(
        "the quick brown fox jumps over the lazy dog ".toByteArray(), 200))
    @Test fun roundTrip_singleByte()       = roundTrip(byteArrayOf('A'.code.toByte()))
    @Test fun roundTrip_twoBytes()         = roundTrip(byteArrayOf('A'.code.toByte(), 'B'.code.toByte()))
    @Test fun roundTrip_singleRepeated()   = roundTrip(ByteArray(100) { 'A'.code.toByte() })
    @Test fun roundTrip_newlineHeavy()     = roundTrip(repeat("line\n".toByteArray(), 200))
    @Test fun roundTrip_nullBytes()        = roundTrip(ByteArray(100))

    // =========================================================================
    // 2. Token stream correctness
    // =========================================================================

    @Test fun encode_simpleABABAB() {
        val data   = "ABABABAB".toByteArray()
        val tokens = LZ77.encode(data)
        assertContentEquals(data, LZ77.decode(tokens))
    }

    @Test fun encode_noRepetition_allLiterals() {
        val tokens = LZ77.encode(byteArrayOf(1, 2, 3))
        assertTrue(tokens.all { it.isLiteral }, "Non-repeating 3-byte input should be all literals")
    }

    @Test fun encode_AAAAAAA_hasBackRef() {
        val data = "AAAAAAA".toByteArray()
        val tokens = LZ77.encode(data)
        assertTrue(tokens.any { !it.isLiteral }, "Repeated 'A' bytes should produce at least one back-reference")
        assertContentEquals(data, LZ77.decode(tokens))
    }

    @Test fun token_literal() {
        val t = Token.literal(65)
        assertTrue(t.isLiteral)
        assertEquals(0,  t.offset)
        assertEquals(0,  t.length)
        assertEquals(65, t.nextChar)
    }

    @Test fun token_match() {
        val t = Token.match(4, 5, 90)
        assertFalse(t.isLiteral)
        assertEquals(4,  t.offset)
        assertEquals(5,  t.length)
        assertEquals(90, t.nextChar)
    }

    // =========================================================================
    // 3. Wire format
    // =========================================================================

    @Test fun wireFormat_empty() {
        val result = LZ77.compress(ByteArray(0))
        assertEquals(4, result.size)
        assertEquals(0, ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN).getInt())
    }

    @Test fun wireFormat_tokenCountField() {
        val compressed = LZ77.compress(byteArrayOf('A'.code.toByte()))
        val tokenCount = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt()
        assertEquals(1, tokenCount)
    }

    @Test fun wireFormat_offsetBigEndian() {
        val t    = Token.match(1000, 5, 65)
        val wire = LZ77.serialiseTokens(listOf(t))
        assertEquals(1, ByteBuffer.wrap(wire, 0, 4).order(ByteOrder.BIG_ENDIAN).getInt())
        val offset = ((wire[4].toInt() and 0xFF) shl 8) or (wire[5].toInt() and 0xFF)
        assertEquals(1000, offset)
        assertEquals(5,   wire[6].toInt() and 0xFF)
        assertEquals(65,  wire[7].toInt() and 0xFF)
    }

    // =========================================================================
    // 4. Edge cases
    // =========================================================================

    @Test fun compress_null()        = assertContentEquals(ByteArray(0), LZ77.decompress(LZ77.compress(null)))
    @Test fun compress_empty()       = assertContentEquals(ByteArray(0), LZ77.decompress(LZ77.compress(ByteArray(0))))
    @Test fun decompress_null()      = assertContentEquals(ByteArray(0), LZ77.decompress(null))
    @Test fun decompress_tooShort()  = assertContentEquals(ByteArray(0), LZ77.decompress(byteArrayOf(0, 0, 0)))

    @ParameterizedTest
    @ValueSource(ints = [0, 65, 127, 255])
    fun roundTrip_singleSymbol(sym: Int) = roundTrip(byteArrayOf(sym.toByte()))

    @Test fun roundTrip_highByteValues() = roundTrip(ByteArray(128) { (128 + it).toByte() })

    // =========================================================================
    // 5. Overlapping matches
    // =========================================================================

    @Test fun decode_overlappingMatch_runs() {
        // literal('A'), match(offset=1, length=6, nextChar='Z') → "AAAAAAAZ"
        val tokens = listOf(Token.literal(65), Token.match(1, 6, 90))
        assertContentEquals("AAAAAAAZ".toByteArray(), LZ77.decode(tokens))
    }

    @Test fun decode_abOverlap() {
        // literal('A'), literal('B'), match(2, 4, 'B') → "ABABABB"
        val tokens = listOf(
            Token.literal(65),
            Token.literal(66),
            Token.match(2, 4, 66)
        )
        assertContentEquals("ABABABB".toByteArray(), LZ77.decode(tokens))
    }

    // =========================================================================
    // 6. Compression effectiveness
    // =========================================================================

    @Test fun effectiveness_repeatedPattern_shrinks() {
        val data = repeat("ABCABCABC".toByteArray(), 100)
        assertTrue(LZ77.compress(data).size < data.size)
    }

    @Test fun effectiveness_repeatedByte_shrinks() {
        val data = ByteArray(1000) { 'X'.code.toByte() }
        assertTrue(LZ77.compress(data).size < data.size)
    }

    // =========================================================================
    // 7. Decode with initial buffer
    // =========================================================================

    @Test fun decode_withInitialBuffer() {
        val seed = "ABCD".toByteArray()
        val tokens = listOf(Token.match(4, 3, 90))  // 'Z'
        val result = LZ77.decode(tokens, seed)
        assertContentEquals("ABCDABCZ".toByteArray(), result)
    }

    // =========================================================================
    // 8. Security / robustness
    // =========================================================================

    @Test fun decode_craftedOffset_throwsOnOOB() {
        // A Match token whose offset exceeds the current output buffer size
        // must throw IllegalArgumentException, not ArrayIndexOutOfBoundsException.
        val tokens = listOf(Token.match(65535, 1, 0))
        assertThrows<IllegalArgumentException> { LZ77.decode(tokens) }
    }

    @Test fun decompress_craftedOffset_doesNotCrash() {
        val crafted = byteArrayOf(
            0, 0, 0, 1,                          // token_count = 1
            0xFF.toByte(), 0xFF.toByte(),         // offset = 65535
            1,                                    // length = 1
            0                                     // nextChar = 0
        )
        assertThrows<IllegalArgumentException> { LZ77.decompress(crafted) }
    }

    // =========================================================================
    // 9. Determinism
    // =========================================================================

    @Test fun deterministicCompression() {
        val data = "the quick brown fox jumps over the lazy dog".toByteArray()
        assertContentEquals(LZ77.compress(data), LZ77.compress(data))
    }

    @Test fun deterministicDecompression() {
        val compressed = LZ77.compress("hello world hello world".toByteArray())
        assertContentEquals(LZ77.decompress(compressed), LZ77.decompress(compressed))
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun roundTrip(data: ByteArray) =
        assertContentEquals(data, LZ77.decompress(LZ77.compress(data)))

    private fun repeat(src: ByteArray, times: Int): ByteArray {
        val out = ByteArray(src.size * times)
        repeat(times) { i -> src.copyInto(out, i * src.size) }
        return out
    }
}
