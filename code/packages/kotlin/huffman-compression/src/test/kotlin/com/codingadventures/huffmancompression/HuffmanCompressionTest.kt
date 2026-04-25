// ============================================================================
// HuffmanCompressionTest.kt — CMP04: Huffman Lossless Compression Tests
// ============================================================================
//
// Test organisation
// -----------------
//  1. Round-trip tests (compress → decompress = original)
//  2. Wire format verification (exact byte layout per the CMP04 spec)
//  3. Edge cases (empty, single byte, single repeated symbol, null bytes)
//  4. Compression effectiveness (compressible data shrinks)
//  5. Idempotency (same input → same output every time)
//  6. Error handling (malformed / truncated input)
//
// ============================================================================

package com.codingadventures.huffmancompression

import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import java.nio.ByteBuffer
import java.nio.ByteOrder
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class HuffmanCompressionTest {

    // =========================================================================
    // 1. Round-trip tests
    // =========================================================================

    @Test fun roundTrip_simpleAAABBC() {
        val data = "AAABBC".toByteArray()
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_helloWorld() {
        val data = "hello world".toByteArray()
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_all256ByteValues() {
        val data = ByteArray(256) { it.toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_all256BytesRepeated() {
        val data = ByteArray(256 * 10) { (it % 256).toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_singleByte() {
        val data = byteArrayOf('X'.code.toByte())
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_twoBytes() {
        val data = byteArrayOf('A'.code.toByte(), 'B'.code.toByte())
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_loremIpsum() {
        val data = ("Lorem ipsum dolor sit amet, consectetur adipiscing elit. " +
                    "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").toByteArray()
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_binaryData() {
        val data = byteArrayOf(0, 1, 2, 3, 255.toByte(), 254.toByte(), 253.toByte(), 128.toByte(), 64, 32)
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_repeatedPattern() {
        val data = "ABCABC".toByteArray().let { p -> ByteArray(p.size * 100) { i -> p[i % p.size] } }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_singleRepeatedByte() {
        val data = ByteArray(100) { 'A'.code.toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_twoSymbolInput() {
        val data = ByteArray(100) { if (it % 2 == 0) 'A'.code.toByte() else 'B'.code.toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_newlineHeavyText() {
        val data = "line\n".toByteArray().let { p -> ByteArray(p.size * 200) { i -> p[i % p.size] } }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun roundTrip_longInput() {
        val pat = "the quick brown fox jumps over the lazy dog ".toByteArray()
        val data = ByteArray(pat.size * 500) { i -> pat[i % pat.size] }
        assertContentEquals(data, decompress(compress(data)))
    }

    // =========================================================================
    // 2. Wire format verification
    // =========================================================================

    @Test fun wireFormat_emptyInput() {
        val result = compress(ByteArray(0))
        assertEquals(8, result.size)
        val buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN)
        assertEquals(0, buf.getInt(), "original_length must be 0")
        assertEquals(0, buf.getInt(), "symbol_count must be 0")
    }

    @Test fun wireFormat_nullInput() {
        val result = compress(null)
        assertEquals(8, result.size)
        val buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN)
        assertEquals(0, buf.getInt(), "original_length")
        assertEquals(0, buf.getInt(), "symbol_count")
    }

    /**
     * Verify the exact wire-format bytes for "AAABBC".
     *
     * Frequencies: A=3, B=2, C=1
     * Canonical codes: A→"0" (len 1), B→"10" (len 2), C→"11" (len 2)
     *
     * Header:      00 00 00 06  00 00 00 03
     * Code table:  41 01  42 02  43 02
     * Bit stream:  "000101011" → 0xA8 0x01
     * Total:       16 bytes
     */
    @Test fun wireFormat_aaabbc_exactBytes() {
        val result = compress("AAABBC".toByteArray())

        val buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN)
        assertEquals(6, buf.getInt(), "original_length")
        assertEquals(3, buf.getInt(), "symbol_count")

        assertEquals(65,   result[8].toInt()  and 0xFF, "symbol 'A'")
        assertEquals(1,    result[9].toInt()  and 0xFF, "length 1")
        assertEquals(66,   result[10].toInt() and 0xFF, "symbol 'B'")
        assertEquals(2,    result[11].toInt() and 0xFF, "length 2")
        assertEquals(67,   result[12].toInt() and 0xFF, "symbol 'C'")
        assertEquals(2,    result[13].toInt() and 0xFF, "length 2")

        assertEquals(0xA8, result[14].toInt() and 0xFF, "bit-stream byte 0")
        assertEquals(0x01, result[15].toInt() and 0xFF, "bit-stream byte 1")

        assertEquals(16, result.size, "total compressed length")
    }

    @ParameterizedTest
    @ValueSource(ints = [1, 5, 100, 1000])
    fun wireFormat_originalLengthField(length: Int) {
        val data = ByteArray(length) { 'A'.code.toByte() }
        val compressed = compress(data)
        val stored = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt()
        assertEquals(length, stored, "original_length field")
    }

    @Test fun wireFormat_symbolCountField() {
        assertEquals(1, symbolCountOf(compress("A".toByteArray())))
        assertEquals(2, symbolCountOf(compress("AB".toByteArray())))
        assertEquals(3, symbolCountOf(compress("ABC".toByteArray())))
        val all256 = ByteArray(256) { it.toByte() }
        assertEquals(256, symbolCountOf(compress(all256)))
    }

    @Test fun wireFormat_codeLengthsTableSorted() {
        val result = compress("AAABBC".toByteArray())
        val n = symbolCountOf(result)
        var prevLen = 0; var prevSym = -1
        for (i in 0 until n) {
            val sym = result[8 + 2 * i].toInt() and 0xFF
            val len = result[8 + 2 * i + 1].toInt() and 0xFF
            assertTrue(len > prevLen || (len == prevLen && sym > prevSym),
                "Code-lengths not sorted at entry $i")
            prevLen = len; prevSym = sym
        }
    }

    @Test fun wireFormat_bitStreamStartsAfterTable() {
        val result = compress("AAABBC".toByteArray())
        val n = symbolCountOf(result)
        val bitsOffset = 8 + 2 * n
        assertTrue(result.size > bitsOffset)
    }

    @Test fun wireFormat_singleByteInput() {
        val result = compress("A".toByteArray())
        val buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN)
        assertEquals(1, buf.getInt(), "original_length")
        assertEquals(1, buf.getInt(), "symbol_count")
        assertEquals(65, result[8].toInt() and 0xFF, "symbol 'A'")
        assertEquals(1,  result[9].toInt() and 0xFF, "code length 1")
        assertEquals(0x00, result[10].toInt() and 0xFF, "bit-stream byte")
        assertEquals(11, result.size, "total length")
    }

    // =========================================================================
    // 3. Edge cases
    // =========================================================================

    @Test fun edgeCase_emptyCompress() {
        val expected = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN)
            .putInt(0).putInt(0).array()
        assertContentEquals(expected, compress(ByteArray(0)))
    }

    @Test fun edgeCase_emptyDecompress() {
        assertContentEquals(ByteArray(0), decompress(compress(ByteArray(0))))
    }

    @Test fun edgeCase_decompressEmptyBytes() {
        assertContentEquals(ByteArray(0), decompress(ByteArray(0)))
    }

    @Test fun edgeCase_decompressShortHeader() {
        assertContentEquals(ByteArray(0), decompress(byteArrayOf(0, 0, 0, 0)))
    }

    @Test fun edgeCase_decompressNullReturnsEmpty() {
        assertContentEquals(ByteArray(0), decompress(null))
    }

    @ParameterizedTest
    @ValueSource(ints = [0, 65, 127, 255])
    fun edgeCase_singleSymbolRoundTrip(sym: Int) {
        val data = ByteArray(50) { sym.toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun edgeCase_singleSymbolEncodesToOneBit() {
        val data = ByteArray(8) { 'A'.code.toByte() }
        val result = compress(data)
        val n = symbolCountOf(result)
        assertEquals(1, n, "symbol_count")
        // 8 bits → 1 byte bit stream
        val bitsOffset = 8 + 2 * n
        assertEquals(bitsOffset + 1, result.size, "bit-stream should be exactly 1 byte")
    }

    @Test fun edgeCase_nullBytes() {
        val data = ByteArray(100) // all zeros
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun edgeCase_twoSymbolsEqualFrequency() {
        val data = ByteArray(200) { if (it % 2 == 0) 'A'.code.toByte() else 'B'.code.toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun edgeCase_allByteValuesSingleOccurrence() {
        val data = ByteArray(256) { it.toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    @Test fun edgeCase_highByteValues() {
        val data = ByteArray(128) { (128 + it).toByte() }
        assertContentEquals(data, decompress(compress(data)))
    }

    // =========================================================================
    // 4. Compression effectiveness
    // =========================================================================

    @Test fun effectiveness_compressibleInputShrinks() {
        val data = ByteArray(1000).also { d ->
            for (i in 0 until 900) d[i] = 'A'.code.toByte()
            for (i in 900 until 1000) d[i] = 'B'.code.toByte()
        }
        val compressed = compress(data)
        assertTrue(compressed.size < data.size)
    }

    @Test fun effectiveness_repeatedByte_compressesWell() {
        val data = ByteArray(1000) { 'X'.code.toByte() }
        val compressed = compress(data)
        assertTrue(compressed.size < data.size)
    }

    @Test fun effectiveness_uniformDistributionLargerThanOriginal() {
        val data = ByteArray(256) { it.toByte() }
        val compressed = compress(data)
        assertTrue(compressed.size > data.size)
    }

    // =========================================================================
    // 5. Idempotency / determinism
    // =========================================================================

    @Test fun idempotent_deterministicCompression() {
        val data = "the quick brown fox jumps over the lazy dog".toByteArray()
        assertContentEquals(compress(data), compress(data))
    }

    @Test fun idempotent_deterministicDecompression() {
        val data = "hello world".toByteArray()
        val compressed = compress(data)
        assertContentEquals(decompress(compressed), decompress(compressed))
    }

    @Test fun idempotent_compressTwiceSameResult() {
        val pat = "ABCABCABC".toByteArray()
        val data = ByteArray(pat.size * 10) { i -> pat[i % pat.size] }
        assertContentEquals(compress(data), compress(data))
    }

    // =========================================================================
    // 6. Error handling
    // =========================================================================

    @Test fun error_bitStreamExhausted_throwsIllegalArgument() {
        // Craft a header claiming originalLength=100 but supply only 1 bit byte.
        val buf = ByteBuffer.allocate(11).order(ByteOrder.BIG_ENDIAN)
        buf.putInt(100)      // original_length = 100
        buf.putInt(1)        // symbol_count = 1
        buf.put(65.toByte()) // symbol = 'A'
        buf.put(1.toByte())  // code_length = 1
        buf.put(0.toByte())  // 1 byte bit stream (only 8 bits → 8 symbols, need 100)
        val truncated = buf.array()

        assertThrows<IllegalArgumentException> {
            decompress(truncated)
        }
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    private fun compress(data: ByteArray?) = HuffmanCompression.compress(data)
    private fun decompress(data: ByteArray?) = HuffmanCompression.decompress(data)

    private fun symbolCountOf(compressed: ByteArray): Int =
        ByteBuffer.wrap(compressed, 4, 4).order(ByteOrder.BIG_ENDIAN).getInt()
}
