// ============================================================================
// HuffmanCompressionTest.java — CMP04: Huffman Lossless Compression Tests
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
// Wire format recap (CMP04):
//   Bytes 0–3:    original_length  (big-endian uint32)
//   Bytes 4–7:    symbol_count     (big-endian uint32)
//   Bytes 8–8+2N: code-lengths table — N × 2 bytes:
//                   [0] symbol value (uint8)
//                   [1] code length  (uint8)
//                 Sorted by (code_length, symbol_value) ascending.
//   Bytes 8+2N+:  bit stream, LSB-first, zero-padded to byte boundary.
//
// ============================================================================

package com.codingadventures.huffmancompression;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Arrays;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for {@link HuffmanCompression}.
 */
class HuffmanCompressionTest {

    // =========================================================================
    // 1. Round-trip tests
    // =========================================================================

    /** compress(data) → decompress → original for a variety of inputs. */

    @Test
    void roundTrip_simpleAAABBC() {
        byte[] data = "AAABBC".getBytes();
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_helloWorld() {
        byte[] data = "hello world".getBytes();
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_all256ByteValues() {
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_all256BytesRepeated() {
        byte[] base = new byte[256];
        for (int i = 0; i < 256; i++) base[i] = (byte) i;
        byte[] data = repeatBytes(base, 10);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_singleByte() {
        byte[] data = new byte[]{(byte) 'X'};
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_twoBytes() {
        byte[] data = new byte[]{(byte) 'A', (byte) 'B'};
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_loremIpsum() {
        byte[] data = (
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. " +
            "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
        ).getBytes();
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_binaryData() {
        byte[] data = new byte[]{0, 1, 2, 3, (byte)255, (byte)254, (byte)253, (byte)128, 64, 32};
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_repeatedPattern() {
        byte[] pattern = "ABCABC".getBytes();
        byte[] data = repeatBytes(pattern, 100);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_singleRepeatedByte() {
        byte[] data = new byte[100];
        Arrays.fill(data, (byte) 'A');
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_twoSymbolInput() {
        byte[] data = repeatBytes(new byte[]{'A', 'B'}, 50);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_newlineHeavyText() {
        byte[] data = repeatBytes("line\n".getBytes(), 200);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void roundTrip_longInput() {
        byte[] data = repeatBytes("the quick brown fox jumps over the lazy dog ".getBytes(), 500);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    // =========================================================================
    // 2. Wire format verification
    // =========================================================================

    /** Verify the exact byte layout of the CMP04 wire format. */

    @Test
    void wireFormat_emptyInput() {
        // compress(b'') must produce exactly an 8-byte header.
        byte[] result = HuffmanCompression.compress(new byte[0]);
        assertEquals(8, result.length);
        ByteBuffer buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN);
        assertEquals(0, buf.getInt(), "original_length must be 0");
        assertEquals(0, buf.getInt(), "symbol_count must be 0");
    }

    @Test
    void wireFormat_nullInput() {
        // compress(null) must behave same as empty.
        byte[] result = HuffmanCompression.compress(null);
        assertEquals(8, result.length);
        ByteBuffer buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN);
        assertEquals(0, buf.getInt(), "original_length must be 0");
        assertEquals(0, buf.getInt(), "symbol_count must be 0");
    }

    /**
     * Verify the exact wire-format bytes for b"AAABBC".
     *
     * <pre>
     * Frequencies: A=3, B=2, C=1
     * Canonical table: A→"0" (len=1), B→"10" (len=2), C→"11" (len=2)
     * Lengths sorted by (length, symbol): [(65,1), (66,2), (67,2)]
     *
     * Header:
     *   00 00 00 06   original_length = 6
     *   00 00 00 03   symbol_count = 3
     * Code-lengths table:
     *   41 01         symbol='A'(65), length=1
     *   42 02         symbol='B'(66), length=2
     *   43 02         symbol='C'(67), length=2
     * Bit stream:
     *   A→"0", A→"0", A→"0", B→"10", B→"10", C→"11"
     *   Concatenated: "000101011" (9 bits)
     *   Packed LSB-first:
     *     Byte 0: bits 0..7 → 0b10101000 = 0xA8
     *     Byte 1: bit  8    → 0b00000001 = 0x01
     * Total: 4+4+6+2 = 16 bytes
     * </pre>
     */
    @Test
    void wireFormat_aaabbc_exactBytes() {
        byte[] result = HuffmanCompression.compress("AAABBC".getBytes());

        // Header
        ByteBuffer buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN);
        assertEquals(6, buf.getInt(), "original_length");
        assertEquals(3, buf.getInt(), "symbol_count");

        // Code-lengths table
        assertEquals(65, result[8]  & 0xFF, "symbol 'A'");
        assertEquals(1,  result[9]  & 0xFF, "length 1");
        assertEquals(66, result[10] & 0xFF, "symbol 'B'");
        assertEquals(2,  result[11] & 0xFF, "length 2");
        assertEquals(67, result[12] & 0xFF, "symbol 'C'");
        assertEquals(2,  result[13] & 0xFF, "length 2");

        // Bit stream
        assertEquals(0xA8, result[14] & 0xFF, "bit-stream byte 0");
        assertEquals(0x01, result[15] & 0xFF, "bit-stream byte 1");

        assertEquals(16, result.length, "total compressed length");
    }

    @ParameterizedTest
    @ValueSource(ints = {1, 5, 100, 1000})
    void wireFormat_originalLengthField(int length) {
        byte[] data = new byte[length];
        Arrays.fill(data, (byte) 'A');
        byte[] compressed = HuffmanCompression.compress(data);
        ByteBuffer buf = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN);
        assertEquals(length, buf.getInt(), "original_length field");
    }

    @Test
    void wireFormat_symbolCountField() {
        // 1 distinct byte
        assertEquals(1, symbolCountOf(HuffmanCompression.compress("A".getBytes())));
        // 2 distinct bytes
        assertEquals(2, symbolCountOf(HuffmanCompression.compress("AB".getBytes())));
        // 3 distinct bytes
        assertEquals(3, symbolCountOf(HuffmanCompression.compress("ABC".getBytes())));
        // All 256 distinct bytes
        byte[] all256 = new byte[256];
        for (int i = 0; i < 256; i++) all256[i] = (byte) i;
        assertEquals(256, symbolCountOf(HuffmanCompression.compress(all256)));
    }

    @Test
    void wireFormat_codeLengthsTableSorted() {
        // Wire format entries must be sorted by (code_length, symbol).
        byte[] result = HuffmanCompression.compress("AAABBC".getBytes());
        int symbolCount = symbolCountOf(result);
        int prev_len = 0, prev_sym = -1;
        for (int i = 0; i < symbolCount; i++) {
            int sym = result[8 + 2 * i] & 0xFF;
            int len = result[8 + 2 * i + 1] & 0xFF;
            assertTrue(len > prev_len || (len == prev_len && sym > prev_sym),
                "Code-lengths table not sorted at entry " + i);
            prev_len = len;
            prev_sym = sym;
        }
    }

    @Test
    void wireFormat_bitStreamStartsAfterTable() {
        // Bit stream must begin at offset 8 + 2*symbol_count.
        byte[] result = HuffmanCompression.compress("AAABBC".getBytes());
        int symbolCount = symbolCountOf(result);
        int bitsOffset = 8 + 2 * symbolCount;
        assertTrue(result.length > bitsOffset, "Bit stream has no content");
    }

    @Test
    void wireFormat_singleByteInput() {
        // Single byte input: symbol='A'(65), length=1, bit stream = [0x00].
        byte[] result = HuffmanCompression.compress("A".getBytes());
        ByteBuffer buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN);
        assertEquals(1, buf.getInt(), "original_length");
        assertEquals(1, buf.getInt(), "symbol_count");
        // Code-lengths table: (65, 1)
        assertEquals(65, result[8] & 0xFF, "symbol 'A'");
        assertEquals(1,  result[9] & 0xFF, "code length 1");
        // Bit stream: "0" packed → 0x00
        assertEquals(0x00, result[10] & 0xFF, "bit-stream byte");
        assertEquals(11, result.length, "total length");
    }

    // =========================================================================
    // 3. Edge cases
    // =========================================================================

    @Test
    void edgeCase_emptyCompress() {
        byte[] expected = ByteBuffer.allocate(8).order(ByteOrder.BIG_ENDIAN)
            .putInt(0).putInt(0).array();
        assertArrayEquals(expected, HuffmanCompression.compress(new byte[0]));
    }

    @Test
    void edgeCase_emptyDecompress() {
        assertArrayEquals(new byte[0],
            HuffmanCompression.decompress(HuffmanCompression.compress(new byte[0])));
    }

    @Test
    void edgeCase_decompressEmptyBytes() {
        // Decompressing raw b'' should return b'' gracefully.
        assertArrayEquals(new byte[0], HuffmanCompression.decompress(new byte[0]));
    }

    @Test
    void edgeCase_decompressShortHeader() {
        // Headers shorter than 8 bytes return b''.
        assertArrayEquals(new byte[0],
            HuffmanCompression.decompress(new byte[]{0, 0, 0, 0}));
    }

    @Test
    void edgeCase_decompressNullReturnsEmpty() {
        assertArrayEquals(new byte[0], HuffmanCompression.decompress(null));
    }

    @ParameterizedTest
    @ValueSource(ints = {0, 65, 127, 255})
    void edgeCase_singleSymbolRoundTrip(int sym) {
        byte[] data = new byte[50];
        Arrays.fill(data, (byte) sym);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void edgeCase_singleSymbolEncodesToOneBit() {
        // With one distinct symbol, each occurrence uses exactly 1 bit.
        byte[] data = new byte[8];
        Arrays.fill(data, (byte) 'A');
        byte[] result = HuffmanCompression.compress(data);
        int symbolCount = symbolCountOf(result);
        assertEquals(1, symbolCount, "symbol_count");
        // 8 bits → 1 byte bit stream
        int bitsOffset = 8 + 2 * symbolCount;
        assertEquals(bitsOffset + 1, result.length, "bit-stream should be exactly 1 byte");
    }

    @Test
    void edgeCase_nullBytes() {
        byte[] data = new byte[100]; // all zeros
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void edgeCase_twoSymbolsEqualFrequency() {
        byte[] data = repeatBytes(new byte[]{'A', 'B'}, 100);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void edgeCase_allByteValuesSingleOccurrence() {
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    @Test
    void edgeCase_highByteValues() {
        // Bytes 128–255 (signed in Java) must round-trip correctly.
        byte[] data = new byte[128];
        for (int i = 0; i < 128; i++) data[i] = (byte) (128 + i);
        assertArrayEquals(data, HuffmanCompression.decompress(HuffmanCompression.compress(data)));
    }

    // =========================================================================
    // 4. Compression effectiveness
    // =========================================================================

    @Test
    void effectiveness_compressibleInputShrinks() {
        // 'A' × 900 + 'B' × 100 should compress to fewer bytes than the original.
        byte[] data = new byte[1000];
        Arrays.fill(data, 0, 900, (byte) 'A');
        Arrays.fill(data, 900, 1000, (byte) 'B');
        byte[] compressed = HuffmanCompression.compress(data);
        assertTrue(compressed.length < data.length,
            "Highly skewed distribution should compress below raw size");
    }

    @Test
    void effectiveness_repeatedByte_compressesWell() {
        byte[] data = new byte[1000];
        Arrays.fill(data, (byte) 'X');
        byte[] compressed = HuffmanCompression.compress(data);
        // 1000 bits = 125 bytes of bit stream + ~11 bytes overhead ≪ 1000 raw bytes.
        assertTrue(compressed.length < data.length,
            "Repeated byte should compress well below raw size");
    }

    @Test
    void effectiveness_uniformDistributionLargerThanOriginal() {
        // All 256 symbols once — overhead dominates small inputs.
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        byte[] compressed = HuffmanCompression.compress(data);
        assertTrue(compressed.length > data.length,
            "Uniform 256-symbol input should be larger when compressed");
    }

    // =========================================================================
    // 5. Idempotency / determinism
    // =========================================================================

    @Test
    void idempotent_deterministicCompression() {
        byte[] data = "the quick brown fox jumps over the lazy dog".getBytes();
        assertArrayEquals(
            HuffmanCompression.compress(data),
            HuffmanCompression.compress(data),
            "compress must be deterministic"
        );
    }

    @Test
    void idempotent_deterministicDecompression() {
        byte[] data = "hello world".getBytes();
        byte[] compressed = HuffmanCompression.compress(data);
        assertArrayEquals(
            HuffmanCompression.decompress(compressed),
            HuffmanCompression.decompress(compressed),
            "decompress must be deterministic"
        );
    }

    @Test
    void idempotent_compressTwiceSameResult() {
        byte[] data = repeatBytes("ABCABCABC".getBytes(), 10);
        assertArrayEquals(
            HuffmanCompression.compress(data),
            HuffmanCompression.compress(data)
        );
    }

    // =========================================================================
    // 6. Error handling
    // =========================================================================

    @Test
    void error_bitStreamExhausted_throwsIllegalArgument() {
        // Craft a header claiming originalLength=100 but supply only 1 symbol + 1 bit byte.
        // Decompress must throw rather than silently corrupt.
        ByteBuffer buf = ByteBuffer.allocate(11).order(ByteOrder.BIG_ENDIAN);
        buf.putInt(100);  // original_length = 100
        buf.putInt(1);    // symbol_count = 1
        buf.put((byte) 65);  // symbol = 'A'
        buf.put((byte) 1);   // code_length = 1
        buf.put((byte) 0);   // 1 byte of bit stream (8 bits → 8 symbols, need 100)
        byte[] truncated = buf.array();

        assertThrows(IllegalArgumentException.class,
            () -> HuffmanCompression.decompress(truncated),
            "Should throw when bit stream is exhausted before all symbols decoded");
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /** Extract the symbol_count field (bytes 4–7, big-endian) from a CMP04 blob. */
    private static int symbolCountOf(byte[] compressed) {
        return ByteBuffer.wrap(compressed, 4, 4).order(ByteOrder.BIG_ENDIAN).getInt();
    }

    /** Repeat {@code src} {@code times} times into a new array. */
    private static byte[] repeatBytes(byte[] src, int times) {
        byte[] out = new byte[src.length * times];
        for (int i = 0; i < times; i++) System.arraycopy(src, 0, out, i * src.length, src.length);
        return out;
    }
}
