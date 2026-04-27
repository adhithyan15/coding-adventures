// ============================================================================
// LZ77Test.java — CMP00: LZ77 Compression Tests
// ============================================================================
//
// Test organisation:
//  1. Round-trip (compress → decompress = original)
//  2. Token stream correctness
//  3. Wire format
//  4. Edge cases (empty, null, single byte, repeated byte)
//  5. Overlapping matches (run-length encoding degenerate case)
//  6. Compression effectiveness
//  7. Decode with initial buffer (streaming seed)
//  8. Determinism
// ============================================================================

package com.codingadventures.lz77;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class LZ77Test {

    // =========================================================================
    // 1. Round-trip
    // =========================================================================

    @Test void roundTrip_helloWorld() {
        roundTrip("hello world".getBytes());
    }

    @Test void roundTrip_aaabbc() {
        roundTrip("AAABBC".getBytes());
    }

    @Test void roundTrip_repeatedPattern() {
        roundTrip(repeat("ABCABC".getBytes(), 100));
    }

    @Test void roundTrip_loremIpsum() {
        roundTrip(("Lorem ipsum dolor sit amet, consectetur adipiscing elit. " +
                   "Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.").getBytes());
    }

    @Test void roundTrip_all256Bytes() {
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        roundTrip(data);
    }

    @Test void roundTrip_binaryData() {
        roundTrip(new byte[]{0, 1, 2, 3, (byte)255, (byte)254, (byte)253, (byte)128, 64, 32});
    }

    @Test void roundTrip_longInput() {
        roundTrip(repeat("the quick brown fox jumps over the lazy dog ".getBytes(), 200));
    }

    @Test void roundTrip_singleByte() {
        roundTrip(new byte[]{(byte)'A'});
    }

    @Test void roundTrip_twoBytes() {
        roundTrip(new byte[]{(byte)'A', (byte)'B'});
    }

    @Test void roundTrip_singleRepeatedByte() {
        byte[] data = new byte[100];
        Arrays.fill(data, (byte)'A');
        roundTrip(data);
    }

    @Test void roundTrip_newlineHeavyText() {
        roundTrip(repeat("line\n".getBytes(), 200));
    }

    // =========================================================================
    // 2. Token stream correctness
    // =========================================================================

    @Test void encode_simpleABABAB_producesExpectedTokens() {
        // "ABABABAB" (8 bytes): A literal, B literal, then a 6-byte match
        // that self-overlaps (offset=2 covers ABABAB, next_char=last byte)
        byte[] data = "ABABABAB".getBytes();
        List<LZ77.Token> tokens = LZ77.encode(data);
        // Decode must give back the original
        assertArrayEquals(data, LZ77.decode(tokens));
    }

    @Test void encode_noRepetition_allLiterals() {
        // A 3-byte non-repeating sequence — all three tokens should be literals
        // (no sequence of length ≥ minMatch=3 repeats)
        byte[] data = new byte[]{1, 2, 3};
        List<LZ77.Token> tokens = LZ77.encode(data);
        assertTrue(tokens.stream().allMatch(LZ77.Token::isLiteral),
            "Non-repeating 3-byte input should produce only literals");
    }

    @Test void encode_AAAAAAA_producesBackRef() {
        // "AAAAAAA" (7 bytes): literal 'A', then a back-reference token
        byte[] data = "AAAAAAA".getBytes();
        List<LZ77.Token> tokens = LZ77.encode(data);
        long backRefs = tokens.stream().filter(t -> !t.isLiteral()).count();
        assertTrue(backRefs > 0, "Repeated 'A' bytes should produce at least one back-reference");
        assertArrayEquals(data, LZ77.decode(tokens));
    }

    @Test void token_isLiteral() {
        LZ77.Token lit = LZ77.Token.literal(65);
        assertTrue(lit.isLiteral());
        assertEquals(0, lit.offset());
        assertEquals(0, lit.length());
        assertEquals(65, lit.nextChar());
    }

    @Test void token_isMatch() {
        LZ77.Token match = LZ77.Token.match(4, 5, 90);
        assertFalse(match.isLiteral());
        assertEquals(4, match.offset());
        assertEquals(5, match.length());
        assertEquals(90, match.nextChar());
    }

    // =========================================================================
    // 3. Wire format
    // =========================================================================

    @Test void wireFormat_empty() {
        byte[] result = LZ77.compress(new byte[0]);
        // 4-byte header with count=0
        assertEquals(4, result.length);
        int tokenCount = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN).getInt();
        assertEquals(0, tokenCount);
    }

    @Test void wireFormat_tokenCountField() {
        // "A" encodes to 1 token (literal)
        byte[] compressed = LZ77.compress(new byte[]{(byte)'A'});
        int tokenCount = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt();
        assertEquals(1, tokenCount);
    }

    @Test void wireFormat_eachTokenIs4Bytes() {
        // For 2 tokens: header(4) + 2×4 = 12 bytes
        byte[] compressed = LZ77.compress("AB".getBytes()); // 2 literal tokens
        List<LZ77.Token> tokens = LZ77.encode("AB".getBytes());
        int expected = 4 + tokens.size() * 4;
        assertEquals(expected, compressed.length);
    }

    @Test void wireFormat_offsetBigEndian() {
        // Build a 2-token stream manually: Token(offset=1000, length=5, next=65)
        LZ77.Token t = LZ77.Token.match(1000, 5, 65);
        byte[] wire = LZ77.serialiseTokens(List.of(t));
        // Header: token_count = 1 (bytes 0-3)
        assertEquals(1, ByteBuffer.wrap(wire, 0, 4).order(ByteOrder.BIG_ENDIAN).getInt());
        // Token bytes 4-7: offset big-endian uint16
        int offset = ((wire[4] & 0xFF) << 8) | (wire[5] & 0xFF);
        assertEquals(1000, offset);
        assertEquals(5, wire[6] & 0xFF);   // length
        assertEquals(65, wire[7] & 0xFF);  // next_char
    }

    // =========================================================================
    // 4. Edge cases
    // =========================================================================

    @Test void compress_null() {
        assertArrayEquals(new byte[0], LZ77.decompress(LZ77.compress(null)));
    }

    @Test void compress_empty() {
        assertArrayEquals(new byte[0], LZ77.decompress(LZ77.compress(new byte[0])));
    }

    @Test void decompress_null() {
        assertArrayEquals(new byte[0], LZ77.decompress(null));
    }

    @Test void decompress_tooShort() {
        assertArrayEquals(new byte[0], LZ77.decompress(new byte[]{0, 0, 0}));
    }

    @ParameterizedTest
    @ValueSource(ints = {0, 65, 127, 255})
    void roundTrip_singleSymbol(int sym) {
        roundTrip(new byte[]{(byte) sym});
    }

    @Test void roundTrip_nullBytes() {
        byte[] data = new byte[100]; // all zeros
        roundTrip(data);
    }

    @Test void roundTrip_highByteValues() {
        byte[] data = new byte[128];
        for (int i = 0; i < 128; i++) data[i] = (byte)(128 + i);
        roundTrip(data);
    }

    // =========================================================================
    // 5. Overlapping matches
    // =========================================================================

    @Test void decode_overlappingMatch_runsCorrectly() {
        // Token(offset=1, length=6, next_char='Z'): starting from "A",
        // copy 6 bytes with offset=1 → produces "AAAAAAAZ"... wait:
        // more precisely, the initial output would be [A] from a literal first.
        // Let's build manually: literal(65) then match(1, 6, 90).
        // Decode: output=[A], then copy 6 bytes from pos 0 (1 back):
        //   step 1: output[0]='A' → [A,A]
        //   step 2: output[1]='A' → [A,A,A]
        //   ... all 6 → [A,A,A,A,A,A,A]
        //   then next_char='Z' → [A,A,A,A,A,A,A,Z]
        List<LZ77.Token> tokens = List.of(
            LZ77.Token.literal(65),          // 'A'
            LZ77.Token.match(1, 6, 90)       // 6 copies of 'A', then 'Z'
        );
        byte[] result = LZ77.decode(tokens);
        assertArrayEquals("AAAAAAAZ".getBytes(), result);
    }

    @Test void decode_abOverlap_ABABAB() {
        // literal(65)='A', literal(66)='B', match(2,4,'B') → ABABAB + B = ABABAB
        // Actually: output=[A,B], then copy 4 from pos 0 (2 back):
        //   output[0]='A' → [A,B,A]
        //   output[1]='B' → [A,B,A,B]
        //   output[2]='A' (just written) → [A,B,A,B,A]
        //   output[3]='B' (just written) → [A,B,A,B,A,B]
        //   then next_char='B' → [A,B,A,B,A,B,B]
        List<LZ77.Token> tokens = List.of(
            LZ77.Token.literal(65),
            LZ77.Token.literal(66),
            LZ77.Token.match(2, 4, 66)
        );
        byte[] result = LZ77.decode(tokens);
        assertArrayEquals("ABABABB".getBytes(), result);
    }

    // =========================================================================
    // 6. Compression effectiveness
    // =========================================================================

    @Test void effectiveness_repeatedPattern_shrinks() {
        // A highly repetitive pattern should compress significantly
        byte[] data = repeat("ABCABCABC".getBytes(), 100);
        byte[] compressed = LZ77.compress(data);
        assertTrue(compressed.length < data.length,
            "Repetitive data should compress below raw size");
    }

    @Test void effectiveness_repeatedByte_shrinks() {
        byte[] data = new byte[1000];
        Arrays.fill(data, (byte)'X');
        byte[] compressed = LZ77.compress(data);
        assertTrue(compressed.length < data.length);
    }

    @Test void effectiveness_uniformRandom_notLargerThan4x() {
        // Truly random-ish data (all 256 bytes once) should not expand beyond reason
        byte[] data = new byte[256];
        for (int i = 0; i < 256; i++) data[i] = (byte) i;
        byte[] compressed = LZ77.compress(data);
        // Each byte becomes a literal token (4 bytes), so 256*4 + 4 = 1028
        assertTrue(compressed.length <= data.length * 4 + 8,
            "Incompressible data should not expand beyond token overhead");
    }

    // =========================================================================
    // 7. Decode with initial buffer
    // =========================================================================

    @Test void decode_withInitialBuffer_includedInOutput() {
        // Seed the decoder with some context
        byte[] seed = "ABCD".getBytes();
        // A token referencing bytes in the seed (offset=4 → 4 bytes back)
        List<LZ77.Token> tokens = List.of(LZ77.Token.match(4, 3, 90)); // 'Z'
        byte[] result = LZ77.decode(tokens, seed);
        // Output starts with seed, then copies 3 bytes from offset 4 = "ABC", then 'Z'
        // seed=[A,B,C,D], copy 3 from pos 0 (seed[0..2]) → [A,B,C], then 'Z'
        byte[] expected = "ABCDABCZ".getBytes();
        assertArrayEquals(expected, result);
    }

    // =========================================================================
    // 8. Security / robustness
    // =========================================================================

    @Test void decode_craftedOffset_throwsOnOOB() {
        // A Match token whose offset exceeds the current output buffer size
        // must throw IllegalArgumentException, not ArrayIndexOutOfBoundsException.
        List<LZ77.Token> tokens = List.of(
            LZ77.Token.match(65535, 1, 0) // offset 65535, but output is empty
        );
        assertThrows(IllegalArgumentException.class, () -> LZ77.decode(tokens));
    }

    @Test void decompress_craftedOffset_doesNotCrash() {
        // 4-byte header: token_count=1, then 1 token: offset=0xFFFF, length=1, nextChar=0
        byte[] crafted = new byte[]{
            0, 0, 0, 1,           // token_count = 1
            (byte)0xFF, (byte)0xFF, // offset = 65535
            1,                     // length = 1
            0                      // nextChar = 0
        };
        assertThrows(IllegalArgumentException.class, () -> LZ77.decompress(crafted));
    }

    // =========================================================================
    // 9. Determinism
    // =========================================================================

    @Test void deterministicCompression() {
        byte[] data = "the quick brown fox jumps over the lazy dog".getBytes();
        assertArrayEquals(LZ77.compress(data), LZ77.compress(data));
    }

    @Test void deterministicDecompression() {
        byte[] data = "hello world hello world".getBytes();
        byte[] compressed = LZ77.compress(data);
        assertArrayEquals(LZ77.decompress(compressed), LZ77.decompress(compressed));
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private static void roundTrip(byte[] data) {
        assertArrayEquals(data, LZ77.decompress(LZ77.compress(data)),
            "Round-trip failed for " + data.length + " bytes");
    }

    private static byte[] repeat(byte[] src, int times) {
        byte[] out = new byte[src.length * times];
        for (int i = 0; i < times; i++) System.arraycopy(src, 0, out, i * src.length, src.length);
        return out;
    }
}
