// ============================================================================
// LZSSTest.java — CMP02: LZSS Compression Tests
// ============================================================================

package com.codingadventures.lzss;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class LZSSTest {

    // =========================================================================
    // 1. Round-trip
    // =========================================================================

    @Test void roundTrip_helloWorld()      { roundTrip("hello world".getBytes()); }
    @Test void roundTrip_aaabbc()          { roundTrip("AAABBC".getBytes()); }
    @Test void roundTrip_repeatedPattern() { roundTrip(repeat("ABCABC".getBytes(), 100)); }
    @Test void roundTrip_loremIpsum()      { roundTrip(("Lorem ipsum dolor sit amet, consectetur " +
        "adipiscing elit.").getBytes()); }
    @Test void roundTrip_all256Bytes()     {
        byte[] d = new byte[256]; for (int i=0;i<256;i++) d[i]=(byte)i; roundTrip(d); }
    @Test void roundTrip_binaryData()      { roundTrip(new byte[]{0,1,2,3,(byte)255,(byte)128,64}); }
    @Test void roundTrip_longInput()       { roundTrip(repeat(
        "the quick brown fox jumps over the lazy dog ".getBytes(), 200)); }
    @Test void roundTrip_singleByte()      { roundTrip(new byte[]{(byte)'A'}); }
    @Test void roundTrip_twoBytes()        { roundTrip(new byte[]{(byte)'A',(byte)'B'}); }
    @Test void roundTrip_singleRepeated()  { byte[] d=new byte[100]; Arrays.fill(d,(byte)'A'); roundTrip(d); }
    @Test void roundTrip_newlineHeavy()    { roundTrip(repeat("line\n".getBytes(), 200)); }
    @Test void roundTrip_nullBytes()       { roundTrip(new byte[100]); }
    @Test void roundTrip_highBytes()       {
        byte[] d=new byte[128]; for(int i=0;i<128;i++) d[i]=(byte)(128+i); roundTrip(d); }

    // =========================================================================
    // 2. Token stream correctness
    // =========================================================================

    @Test void encode_ABABABAB_hasMatch() {
        byte[] data = "ABABABAB".getBytes();
        List<LZSS.Token> tokens = LZSS.encode(data);
        assertTrue(tokens.stream().anyMatch(t -> !t.isLiteral()),
            "Repeating AB pattern should produce at least one Match token");
        assertArrayEquals(data, LZSS.decode(tokens, -1));
    }

    @Test void encode_shortNonRepeat_allLiterals() {
        byte[] data = new byte[]{1, 2, 3};
        List<LZSS.Token> tokens = LZSS.encode(data);
        assertTrue(tokens.stream().allMatch(LZSS.Token::isLiteral),
            "3-byte non-repeating input should produce only literals");
    }

    @Test void encode_AAAAAAA_producesMatch() {
        byte[] data = "AAAAAAA".getBytes();
        List<LZSS.Token> tokens = LZSS.encode(data);
        assertTrue(tokens.stream().anyMatch(t -> t instanceof LZSS.Match));
        assertArrayEquals(data, LZSS.decode(tokens, -1));
    }

    @Test void literal_properties() {
        LZSS.Literal lit = new LZSS.Literal(65);
        assertTrue(lit.isLiteral());
        assertEquals(65, lit.value());
    }

    @Test void match_properties() {
        LZSS.Match m = new LZSS.Match(10, 5);
        assertFalse(m.isLiteral());
        assertEquals(10, m.offset());
        assertEquals(5,  m.length());
    }

    // =========================================================================
    // 3. Wire format
    // =========================================================================

    @Test void wireFormat_emptyInput() {
        byte[] result = LZSS.compress(new byte[0]);
        assertEquals(8, result.length);
        ByteBuffer buf = ByteBuffer.wrap(result).order(ByteOrder.BIG_ENDIAN);
        assertEquals(0, buf.getInt()); // original_length
        assertEquals(0, buf.getInt()); // block_count
    }

    @Test void wireFormat_originalLengthStored() {
        for (int len : new int[]{1, 5, 100}) {
            byte[] data = new byte[len]; Arrays.fill(data, (byte)'A');
            byte[] compressed = LZSS.compress(data);
            int stored = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt();
            assertEquals(len, stored);
        }
    }

    @Test void wireFormat_blockCountField() {
        // 1 byte → 1 token → 1 block
        byte[] compressed = LZSS.compress("A".getBytes());
        ByteBuffer buf = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN);
        buf.getInt(); // skip original_length
        int blockCount = buf.getInt();
        assertEquals(1, blockCount);
    }

    @Test void wireFormat_flagByteIsFirstByteOfBlock() {
        // "A" → 1 Literal token → flag_byte = 0 (bit 0 = 0 = Literal)
        byte[] compressed = LZSS.compress("A".getBytes());
        int flagByte = compressed[8] & 0xFF;
        assertEquals(0, flagByte, "Single literal → flag_byte should be 0");
    }

    // =========================================================================
    // 4. Edge cases
    // =========================================================================

    @Test void compress_null()        { assertArrayEquals(new byte[0], LZSS.decompress(LZSS.compress(null))); }
    @Test void compress_empty()       { assertArrayEquals(new byte[0], LZSS.decompress(LZSS.compress(new byte[0]))); }
    @Test void decompress_null()      { assertArrayEquals(new byte[0], LZSS.decompress(null)); }
    @Test void decompress_tooShort()  { assertArrayEquals(new byte[0], LZSS.decompress(new byte[4])); }

    @ParameterizedTest
    @ValueSource(ints = {0, 65, 127, 255})
    void roundTrip_singleSymbol(int sym) { roundTrip(new byte[]{(byte) sym}); }

    // =========================================================================
    // 5. Overlapping matches
    // =========================================================================

    @Test void decode_overlappingMatch_runs() {
        // literal('A') → [A]; match(offset=1, length=6) → [A,A,A,A,A,A,A]
        List<LZSS.Token> tokens = List.of(
            new LZSS.Literal(65),
            new LZSS.Match(1, 6)
        );
        byte[] result = LZSS.decode(tokens, -1);
        assertArrayEquals("AAAAAAA".getBytes(), result);
    }

    @Test void decode_abOverlap() {
        // [A,B] then match(2,4) → [A,B,A,B,A,B]
        List<LZSS.Token> tokens = List.of(
            new LZSS.Literal(65),
            new LZSS.Literal(66),
            new LZSS.Match(2, 4)
        );
        byte[] result = LZSS.decode(tokens, -1);
        assertArrayEquals("ABABAB".getBytes(), result);
    }

    // =========================================================================
    // 6. Compression effectiveness
    // =========================================================================

    @Test void effectiveness_repeatedPattern_shrinks() {
        byte[] data = repeat("ABCABCABC".getBytes(), 100);
        assertTrue(LZSS.compress(data).length < data.length);
    }

    @Test void effectiveness_repeatedByte_shrinks() {
        byte[] data = new byte[1000]; Arrays.fill(data, (byte)'X');
        assertTrue(LZSS.compress(data).length < data.length);
    }

    @Test void effectiveness_vsLz77_noBloat() {
        // LZSS should not produce more output than raw input for all-unique data
        byte[] data = new byte[256]; for (int i=0;i<256;i++) data[i]=(byte)i;
        byte[] compressed = LZSS.compress(data);
        // Each byte → 1 Literal token (1 byte payload) + flag byte per 8 = 288+8 header
        assertTrue(compressed.length < data.length * 3,
            "LZSS should not expand incompressible data by more than 3x");
    }

    // =========================================================================
    // 7. Security / robustness
    // =========================================================================

    @Test void decode_craftedOffset_throwsOnOOB() {
        // A Match token whose offset exceeds the current output buffer size
        // must throw IllegalArgumentException, not ArrayIndexOutOfBoundsException.
        List<LZSS.Token> tokens = List.of(new LZSS.Match(65535, 3));
        assertThrows(IllegalArgumentException.class, () -> LZSS.decode(tokens, -1));
    }

    // =========================================================================
    // 8. Determinism
    // =========================================================================

    @Test void deterministicCompression() {
        byte[] data = "the quick brown fox jumps over the lazy dog".getBytes();
        assertArrayEquals(LZSS.compress(data), LZSS.compress(data));
    }

    @Test void deterministicDecompression() {
        byte[] compressed = LZSS.compress("hello world hello world".getBytes());
        assertArrayEquals(LZSS.decompress(compressed), LZSS.decompress(compressed));
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private static void roundTrip(byte[] data) {
        assertArrayEquals(data, LZSS.decompress(LZSS.compress(data)),
            "Round-trip failed for " + data.length + " bytes");
    }

    private static byte[] repeat(byte[] src, int times) {
        byte[] out = new byte[src.length * times];
        for (int i = 0; i < times; i++) System.arraycopy(src, 0, out, i*src.length, src.length);
        return out;
    }
}
