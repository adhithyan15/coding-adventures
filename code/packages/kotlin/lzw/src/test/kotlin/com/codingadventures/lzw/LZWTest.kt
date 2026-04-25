// ============================================================================
// LZWTest.kt — CMP03: LZW Compression Tests
// ============================================================================

package com.codingadventures.lzw

import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import org.junit.jupiter.api.assertThrows

class LZWTest {

    // =========================================================================
    // 1. Round-trip
    // =========================================================================

    @Test fun roundTrip_helloWorld()      { roundTrip("hello world".toByteArray()) }
    @Test fun roundTrip_helloHello()      { roundTrip("hello hello hello".toByteArray()) }
    @Test fun roundTrip_aaabbc()          { roundTrip("AAABBC".toByteArray()) }
    @Test fun roundTrip_repeatedPattern() { roundTrip(repeat("ABCABC".toByteArray(), 100)) }
    @Test fun roundTrip_loremIpsum()      {
        roundTrip("Lorem ipsum dolor sit amet, consectetur adipiscing elit.".toByteArray()) }
    @Test fun roundTrip_all256Bytes()     {
        roundTrip(ByteArray(256) { it.toByte() }) }
    @Test fun roundTrip_binaryData()      {
        roundTrip(byteArrayOf(0, 1, 2, 3, 255.toByte(), 128.toByte(), 64)) }
    @Test fun roundTrip_longInput()       {
        roundTrip(repeat("the quick brown fox jumps over the lazy dog ".toByteArray(), 200)) }
    @Test fun roundTrip_singleByte()      { roundTrip(byteArrayOf('A'.code.toByte())) }
    @Test fun roundTrip_twoBytes()        { roundTrip(byteArrayOf('A'.code.toByte(), 'B'.code.toByte())) }
    @Test fun roundTrip_singleRepeated()  { roundTrip(ByteArray(100) { 'A'.code.toByte() }) }
    @Test fun roundTrip_newlineHeavy()    { roundTrip(repeat("line\n".toByteArray(), 200)) }
    @Test fun roundTrip_nullBytes()       { roundTrip(ByteArray(100)) }
    @Test fun roundTrip_highBytes()       {
        roundTrip(ByteArray(128) { (128 + it).toByte() }) }

    @ParameterizedTest
    @ValueSource(ints = [0, 65, 127, 255])
    fun roundTrip_singleSymbol(sym: Int)  { roundTrip(byteArrayOf(sym.toByte())) }

    // =========================================================================
    // 2. Code stream structure
    // =========================================================================

    @Test fun codes_startWithClearCode() {
        val codes = LZW.encodeCodes("hello".toByteArray())
        assertEquals(LZW.CLEAR_CODE, codes.first(),
            "First code should always be CLEAR_CODE")
    }

    @Test fun codes_endWithStopCode() {
        val codes = LZW.encodeCodes("hello".toByteArray())
        assertEquals(LZW.STOP_CODE, codes.last(),
            "Last code should always be STOP_CODE")
    }

    @Test fun codes_repeatedInput_hasCodesAbove257() {
        // A repeated pattern must produce at least one multi-byte dictionary entry.
        val codes = LZW.encodeCodes("ABABABABAB".toByteArray())
        assertTrue(codes.any { it >= LZW.INITIAL_NEXT_CODE },
            "Repeated input should produce codes above 257")
    }

    @Test fun codes_singleByte_roundTrips() {
        val codes  = LZW.encodeCodes(byteArrayOf('X'.code.toByte()))
        val result = LZW.decodeCodes(codes)
        assertContentEquals(byteArrayOf('X'.code.toByte()), result)
    }

    // =========================================================================
    // 3. Wire format
    // =========================================================================

    @Test fun wireFormat_originalLengthStoredInHeader() {
        for (len in intArrayOf(1, 5, 100)) {
            val data       = ByteArray(len) { 'A'.code.toByte() }
            val compressed = LZW.compress(data)
            val stored     = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt()
            assertEquals(len, stored)
        }
    }

    @Test fun wireFormat_headerIs4Bytes() {
        val compressed = LZW.compress("A".toByteArray())
        assertTrue(compressed.size > 4, "Compressed output must exceed the 4-byte header")
    }

    @Test fun wireFormat_emptyInput_producesHeaderOnly() {
        val compressed = LZW.compress(ByteArray(0))
        assertTrue(compressed.size >= 4, "Empty input should at least have a header")
        val stored = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt()
        assertEquals(0, stored, "original_length should be 0 for empty input")
    }

    // =========================================================================
    // 4. Edge cases
    // =========================================================================

    @Test fun compress_null()       { assertContentEquals(ByteArray(0), LZW.decompress(LZW.compress(null))) }
    @Test fun compress_empty()      { assertContentEquals(ByteArray(0), LZW.decompress(LZW.compress(ByteArray(0)))) }
    @Test fun decompress_null()     { assertContentEquals(ByteArray(0), LZW.decompress(null)) }
    @Test fun decompress_tooShort() { assertContentEquals(ByteArray(0), LZW.decompress(ByteArray(2))) }

    // =========================================================================
    // 5. Tricky token (code == next_code)
    // =========================================================================

    @Test fun trickyToken_aaaa() {
        // "AAAA" triggers the tricky token scenario in LZW decoding.
        roundTrip("AAAA".toByteArray())
    }

    @Test fun trickyToken_longRun() {
        val data = ByteArray(200) { 'Z'.code.toByte() }
        roundTrip(data)
    }

    @Test fun trickyToken_ababab() {
        roundTrip(repeat("AB".toByteArray(), 50))
    }

    // =========================================================================
    // 6. Bit I/O round-trip
    // =========================================================================

    @Test fun bitWriter_bitReader_roundTrip() {
        val w = LZW.BitWriter()
        w.write(421,  9)   // 9-bit code
        w.write(258,  9)   // another 9-bit code
        w.write(1023, 10)  // 10-bit code
        w.flush()
        val packed = w.toByteArray()

        val r = LZW.BitReader(packed, 0)
        assertEquals(421,  r.read(9))
        assertEquals(258,  r.read(9))
        assertEquals(1023, r.read(10))
    }

    @Test fun bitWriter_singleBit() {
        val w = LZW.BitWriter()
        w.write(1, 1)
        w.flush()
        val packed = w.toByteArray()
        assertEquals(1, packed[0].toInt() and 0xFF)
    }

    @Test fun bitWriter_exactlyOneByte() {
        val w = LZW.BitWriter()
        w.write(0b10110101, 8)
        w.flush()
        val packed = w.toByteArray()
        assertEquals(1, packed.size)
        assertEquals(0b10110101, packed[0].toInt() and 0xFF)
    }

    // =========================================================================
    // 7. Compression effectiveness
    // =========================================================================

    @Test fun effectiveness_repeatedPattern_shrinks() {
        val data = repeat("ABCABCABC".toByteArray(), 100)
        assertTrue(LZW.compress(data).size < data.size)
    }

    @Test fun effectiveness_repeatedByte_shrinks() {
        val data = ByteArray(1000) { 'X'.code.toByte() }
        assertTrue(LZW.compress(data).size < data.size)
    }

    // =========================================================================
    // 8. Security / robustness
    // =========================================================================

    @Test fun decodeCodes_outputLimit_throwsWhenExceeded() {
        // Encode a large repeated pattern, then try to decode with a tiny limit.
        val data  = repeat("ABCDEFGH".toByteArray(), 100)
        val codes = LZW.encodeCodes(data)
        // 1-byte limit should immediately trigger the guard.
        assertThrows<IllegalArgumentException> { LZW.decodeCodes(codes, 1) }
    }

    @Test fun decodeCodes_invalidCode_throwsIllegalArgument() {
        // A code far beyond next_code should throw IllegalArgumentException.
        val codes = listOf(LZW.CLEAR_CODE, 65, 9999)
        assertThrows<IllegalArgumentException> { LZW.decodeCodes(codes) }
    }

    // =========================================================================
    // 9. Determinism
    // =========================================================================

    @Test fun deterministicCompression() {
        val data = "the quick brown fox jumps over the lazy dog".toByteArray()
        assertContentEquals(LZW.compress(data), LZW.compress(data))
    }

    @Test fun deterministicDecompression() {
        val compressed = LZW.compress("hello world hello world".toByteArray())
        assertContentEquals(LZW.decompress(compressed), LZW.decompress(compressed))
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun roundTrip(data: ByteArray) {
        assertContentEquals(data, LZW.decompress(LZW.compress(data)),
            "Round-trip failed for ${data.size} bytes")
    }

    private fun repeat(src: ByteArray, times: Int): ByteArray {
        val out = ByteArray(src.size * times)
        for (i in 0 until times) System.arraycopy(src, 0, out, i * src.size, src.size)
        return out
    }
}
