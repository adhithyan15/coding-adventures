// ============================================================================
// LZSSTest.kt — CMP02: LZSS Compression Tests
// ============================================================================

package com.codingadventures.lzss

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

class LZSSTest {

    // =========================================================================
    // 1. Round-trip
    // =========================================================================

    @Test fun roundTrip_helloWorld()      { roundTrip("hello world".toByteArray()) }
    @Test fun roundTrip_aaabbc()          { roundTrip("AAABBC".toByteArray()) }
    @Test fun roundTrip_repeatedPattern() { roundTrip(repeat("ABCABC".toByteArray(), 100)) }
    @Test fun roundTrip_loremIpsum()      { roundTrip(("Lorem ipsum dolor sit amet, consectetur " +
        "adipiscing elit.").toByteArray()) }
    @Test fun roundTrip_all256Bytes()     {
        val d = ByteArray(256) { it.toByte() }; roundTrip(d) }
    @Test fun roundTrip_binaryData()      {
        roundTrip(byteArrayOf(0, 1, 2, 3, 255.toByte(), 128.toByte(), 64)) }
    @Test fun roundTrip_longInput()       { roundTrip(repeat(
        "the quick brown fox jumps over the lazy dog ".toByteArray(), 200)) }
    @Test fun roundTrip_singleByte()      { roundTrip(byteArrayOf('A'.code.toByte())) }
    @Test fun roundTrip_twoBytes()        { roundTrip(byteArrayOf('A'.code.toByte(), 'B'.code.toByte())) }
    @Test fun roundTrip_singleRepeated()  { roundTrip(ByteArray(100) { 'A'.code.toByte() }) }
    @Test fun roundTrip_newlineHeavy()    { roundTrip(repeat("line\n".toByteArray(), 200)) }
    @Test fun roundTrip_nullBytes()       { roundTrip(ByteArray(100)) }
    @Test fun roundTrip_highBytes()       {
        val d = ByteArray(128) { (128 + it).toByte() }; roundTrip(d) }

    // =========================================================================
    // 2. Token stream correctness
    // =========================================================================

    @Test fun encode_ABABABAB_hasMatch() {
        val data   = "ABABABAB".toByteArray()
        val tokens = LZSS.encode(data)
        assertTrue(tokens.any { !it.isLiteral },
            "Repeating AB pattern should produce at least one Match token")
        assertContentEquals(data, LZSS.decode(tokens, -1))
    }

    @Test fun encode_shortNonRepeat_allLiterals() {
        val data   = byteArrayOf(1, 2, 3)
        val tokens = LZSS.encode(data)
        assertTrue(tokens.all { it.isLiteral },
            "3-byte non-repeating input should produce only literals")
    }

    @Test fun encode_AAAAAAA_producesMatch() {
        val data   = "AAAAAAA".toByteArray()
        val tokens = LZSS.encode(data)
        assertTrue(tokens.any { it is Match })
        assertContentEquals(data, LZSS.decode(tokens, -1))
    }

    @Test fun literal_properties() {
        val lit = Literal(65)
        assertTrue(lit.isLiteral)
        assertEquals(65, lit.value)
    }

    @Test fun match_properties() {
        val m = Match(10, 5)
        assertFalse(m.isLiteral)
        assertEquals(10, m.offset)
        assertEquals(5,  m.length)
    }

    // =========================================================================
    // 3. Wire format
    // =========================================================================

    @Test fun wireFormat_emptyInput() {
        val result = LZSS.compress(ByteArray(0))
        assertEquals(8, result.size)
        val buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN)
        assertEquals(0, buf.getInt()) // original_length
        assertEquals(0, buf.getInt()) // block_count
    }

    @Test fun wireFormat_originalLengthStored() {
        for (len in intArrayOf(1, 5, 100)) {
            val data       = ByteArray(len) { 'A'.code.toByte() }
            val compressed = LZSS.compress(data)
            val stored     = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt()
            assertEquals(len, stored)
        }
    }

    @Test fun wireFormat_blockCountField() {
        // 1 byte → 1 token → 1 block
        val compressed = LZSS.compress("A".toByteArray())
        val buf = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN)
        buf.getInt() // skip original_length
        val blockCount = buf.getInt()
        assertEquals(1, blockCount)
    }

    @Test fun wireFormat_flagByteIsFirstByteOfBlock() {
        // "A" → 1 Literal token → flag_byte = 0 (bit 0 = 0 = Literal)
        val compressed = LZSS.compress("A".toByteArray())
        val flagByte   = compressed[8].toInt() and 0xFF
        assertEquals(0, flagByte, "Single literal → flag_byte should be 0")
    }

    // =========================================================================
    // 4. Edge cases
    // =========================================================================

    @Test fun compress_null()       { assertContentEquals(ByteArray(0), LZSS.decompress(LZSS.compress(null))) }
    @Test fun compress_empty()      { assertContentEquals(ByteArray(0), LZSS.decompress(LZSS.compress(ByteArray(0)))) }
    @Test fun decompress_null()     { assertContentEquals(ByteArray(0), LZSS.decompress(null)) }
    @Test fun decompress_tooShort() { assertContentEquals(ByteArray(0), LZSS.decompress(ByteArray(4))) }

    @ParameterizedTest
    @ValueSource(ints = [0, 65, 127, 255])
    fun roundTrip_singleSymbol(sym: Int) { roundTrip(byteArrayOf(sym.toByte())) }

    // =========================================================================
    // 5. Overlapping matches
    // =========================================================================

    @Test fun decode_overlappingMatch_runs() {
        // literal('A') → [A]; match(offset=1, length=6) → [A,A,A,A,A,A,A]
        val tokens = listOf(
            Literal(65),
            Match(1, 6)
        )
        val result = LZSS.decode(tokens, -1)
        assertContentEquals("AAAAAAA".toByteArray(), result)
    }

    @Test fun decode_abOverlap() {
        // [A,B] then match(2,4) → [A,B,A,B,A,B]
        val tokens = listOf(
            Literal(65),
            Literal(66),
            Match(2, 4)
        )
        val result = LZSS.decode(tokens, -1)
        assertContentEquals("ABABAB".toByteArray(), result)
    }

    // =========================================================================
    // 6. Compression effectiveness
    // =========================================================================

    @Test fun effectiveness_repeatedPattern_shrinks() {
        val data = repeat("ABCABCABC".toByteArray(), 100)
        assertTrue(LZSS.compress(data).size < data.size)
    }

    @Test fun effectiveness_repeatedByte_shrinks() {
        val data = ByteArray(1000) { 'X'.code.toByte() }
        assertTrue(LZSS.compress(data).size < data.size)
    }

    @Test fun effectiveness_vsLz77_noBloat() {
        // LZSS should not produce more output than raw input for all-unique data
        val data       = ByteArray(256) { it.toByte() }
        val compressed = LZSS.compress(data)
        // Each byte → 1 Literal token (1 byte payload) + flag byte per 8 = 288+8 header
        assertTrue(compressed.size < data.size * 3,
            "LZSS should not expand incompressible data by more than 3x")
    }

    // =========================================================================
    // 7. Security / robustness
    // =========================================================================

    @Test fun decode_craftedOffset_throwsOnOOB() {
        // A Match token whose offset exceeds the current output buffer size
        // must throw IllegalArgumentException, not ArrayIndexOutOfBoundsException.
        val tokens = listOf(Match(65535, 3))
        assertThrows<IllegalArgumentException> { LZSS.decode(tokens, -1) }
    }

    // =========================================================================
    // 8. Determinism
    // =========================================================================

    @Test fun deterministicCompression() {
        val data = "the quick brown fox jumps over the lazy dog".toByteArray()
        assertContentEquals(LZSS.compress(data), LZSS.compress(data))
    }

    @Test fun deterministicDecompression() {
        val compressed = LZSS.compress("hello world hello world".toByteArray())
        assertContentEquals(LZSS.decompress(compressed), LZSS.decompress(compressed))
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun roundTrip(data: ByteArray) {
        assertContentEquals(data, LZSS.decompress(LZSS.compress(data)),
            "Round-trip failed for ${data.size} bytes")
    }

    private fun repeat(src: ByteArray, times: Int): ByteArray {
        val out = ByteArray(src.size * times)
        for (i in 0 until times) System.arraycopy(src, 0, out, i * src.size, src.size)
        return out
    }
}
