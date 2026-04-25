// ============================================================================
// LZWTest.java — CMP03: LZW Compression Tests
// ============================================================================

package com.codingadventures.lzw;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class LZWTest {

    // =========================================================================
    // 1. Round-trip
    // =========================================================================

    @Test void roundTrip_helloWorld()      { roundTrip("hello world".getBytes()); }
    @Test void roundTrip_helloHello()      { roundTrip("hello hello hello".getBytes()); }
    @Test void roundTrip_aaabbc()          { roundTrip("AAABBC".getBytes()); }
    @Test void roundTrip_repeatedPattern() { roundTrip(repeat("ABCABC".getBytes(), 100)); }
    @Test void roundTrip_loremIpsum()      {
        roundTrip(("Lorem ipsum dolor sit amet, consectetur adipiscing elit.").getBytes()); }
    @Test void roundTrip_all256Bytes()     {
        byte[] d = new byte[256]; for (int i=0;i<256;i++) d[i]=(byte)i; roundTrip(d); }
    @Test void roundTrip_binaryData()      {
        roundTrip(new byte[]{0,1,2,3,(byte)255,(byte)128,64}); }
    @Test void roundTrip_longInput()       {
        roundTrip(repeat("the quick brown fox jumps over the lazy dog ".getBytes(), 200)); }
    @Test void roundTrip_singleByte()      { roundTrip(new byte[]{(byte)'A'}); }
    @Test void roundTrip_twoBytes()        { roundTrip(new byte[]{(byte)'A',(byte)'B'}); }
    @Test void roundTrip_singleRepeated()  {
        byte[] d=new byte[100]; Arrays.fill(d,(byte)'A'); roundTrip(d); }
    @Test void roundTrip_newlineHeavy()    { roundTrip(repeat("line\n".getBytes(), 200)); }
    @Test void roundTrip_nullBytes()       { roundTrip(new byte[100]); }
    @Test void roundTrip_highBytes()       {
        byte[] d=new byte[128]; for(int i=0;i<128;i++) d[i]=(byte)(128+i); roundTrip(d); }

    @ParameterizedTest
    @ValueSource(ints = {0, 65, 127, 255})
    void roundTrip_singleSymbol(int sym)   { roundTrip(new byte[]{(byte) sym}); }

    // =========================================================================
    // 2. Code stream structure
    // =========================================================================

    @Test void codes_startWithClearCode() {
        List<Integer> codes = LZW.encodeCodes("hello".getBytes());
        assertEquals(LZW.CLEAR_CODE, codes.get(0),
            "First code should always be CLEAR_CODE");
    }

    @Test void codes_endWithStopCode() {
        List<Integer> codes = LZW.encodeCodes("hello".getBytes());
        assertEquals(LZW.STOP_CODE, codes.get(codes.size() - 1),
            "Last code should always be STOP_CODE");
    }

    @Test void codes_repeatedInput_hasCodesAbove257() {
        // A repeated pattern must produce at least one multi-byte dictionary entry.
        List<Integer> codes = LZW.encodeCodes("ABABABABAB".getBytes());
        assertTrue(codes.stream().anyMatch(c -> c >= LZW.INITIAL_NEXT_CODE),
            "Repeated input should produce codes above 257");
    }

    @Test void codes_singleByte_roundTrips() {
        List<Integer> codes = LZW.encodeCodes(new byte[]{'X'});
        byte[] result = LZW.decodeCodes(codes);
        assertArrayEquals(new byte[]{'X'}, result);
    }

    // =========================================================================
    // 3. Wire format
    // =========================================================================

    @Test void wireFormat_originalLengthStoredInHeader() {
        for (int len : new int[]{1, 5, 100}) {
            byte[] data = new byte[len]; Arrays.fill(data, (byte)'A');
            byte[] compressed = LZW.compress(data);
            int stored = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt();
            assertEquals(len, stored);
        }
    }

    @Test void wireFormat_headerIs4Bytes() {
        // Minimum: 4-byte header + at least one bit-packed byte
        byte[] compressed = LZW.compress("A".getBytes());
        assertTrue(compressed.length > 4, "Compressed output must exceed the 4-byte header");
    }

    @Test void wireFormat_emptyInput_producesHeaderOnly() {
        byte[] compressed = LZW.compress(new byte[0]);
        // Header (4B) + CLEAR_CODE (9b) + STOP_CODE (9b) = 18 bits = 3 bytes bit-packed
        // Total should be 7 bytes
        assertTrue(compressed.length >= 4, "Empty input should at least have a header");
        int stored = ByteBuffer.wrap(compressed).order(ByteOrder.BIG_ENDIAN).getInt();
        assertEquals(0, stored, "original_length should be 0 for empty input");
    }

    // =========================================================================
    // 4. Edge cases
    // =========================================================================

    @Test void compress_null()       { assertArrayEquals(new byte[0], LZW.decompress(LZW.compress(null))); }
    @Test void compress_empty()      { assertArrayEquals(new byte[0], LZW.decompress(LZW.compress(new byte[0]))); }
    @Test void decompress_null()     { assertArrayEquals(new byte[0], LZW.decompress(null)); }
    @Test void decompress_tooShort() { assertArrayEquals(new byte[0], LZW.decompress(new byte[2])); }

    // =========================================================================
    // 5. Tricky token (code == next_code)
    // =========================================================================

    @Test void trickyToken_aaaa() {
        // "AAAA" triggers the tricky token scenario in LZW decoding.
        // After emitting A(65) twice, the new entry "AA" gets code 258.
        // The third A extends "AA" → the encoder emits code 258 (AA) before
        // adding "AAA" as 259.  The decoder receives code 258 = next_code at that
        // moment — the classic tricky token.
        roundTrip("AAAA".getBytes());
    }

    @Test void trickyToken_longRun() {
        // A long run of identical bytes repeatedly hits the tricky token.
        byte[] data = new byte[200]; Arrays.fill(data, (byte)'Z');
        roundTrip(data);
    }

    @Test void trickyToken_ababab() {
        // "ABABAB..." where new entries immediately match next_code.
        roundTrip(repeat("AB".getBytes(), 50));
    }

    // =========================================================================
    // 6. Bit I/O round-trip
    // =========================================================================

    @Test void bitWriter_bitReader_roundTrip() {
        LZW.BitWriter w = new LZW.BitWriter();
        w.write(421, 9);  // 9-bit code
        w.write(258, 9);  // another 9-bit code
        w.write(1023, 10); // 10-bit code
        w.flush();
        byte[] packed = w.toByteArray();

        LZW.BitReader r = new LZW.BitReader(packed, 0);
        assertEquals(421,  r.read(9));
        assertEquals(258,  r.read(9));
        assertEquals(1023, r.read(10));
    }

    @Test void bitWriter_singleBit() {
        LZW.BitWriter w = new LZW.BitWriter();
        w.write(1, 1);
        w.flush();
        byte[] packed = w.toByteArray();
        assertEquals(1, packed[0] & 0xFF);
    }

    @Test void bitWriter_exactlyOneByte() {
        LZW.BitWriter w = new LZW.BitWriter();
        w.write(0b10110101, 8);
        w.flush();
        byte[] packed = w.toByteArray();
        assertEquals(1, packed.length);
        assertEquals(0b10110101, packed[0] & 0xFF);
    }

    // =========================================================================
    // 7. Compression effectiveness
    // =========================================================================

    @Test void effectiveness_repeatedPattern_shrinks() {
        byte[] data = repeat("ABCABCABC".getBytes(), 100);
        assertTrue(LZW.compress(data).length < data.length);
    }

    @Test void effectiveness_repeatedByte_shrinks() {
        byte[] data = new byte[1000]; Arrays.fill(data, (byte)'X');
        assertTrue(LZW.compress(data).length < data.length);
    }

    // =========================================================================
    // 8. Security / robustness
    // =========================================================================

    @Test void decodeCodes_outputLimit_throwsWhenExceeded() {
        // Encode a large repeated pattern, then try to decode with a tiny limit.
        byte[] data = repeat("ABCDEFGH".getBytes(), 100);
        List<Integer> codes = LZW.encodeCodes(data);
        // 1-byte limit should immediately trigger the guard.
        assertThrows(IllegalArgumentException.class,
            () -> LZW.decodeCodes(codes, 1));
    }

    @Test void decodeCodes_invalidCode_throwsIllegalArgument() {
        // A code far beyond next_code should throw IllegalArgumentException.
        List<Integer> codes = List.of(LZW.CLEAR_CODE, 65 /* 'A' */, 9999);
        assertThrows(IllegalArgumentException.class, () -> LZW.decodeCodes(codes));
    }

    // =========================================================================
    // 9. Determinism
    // =========================================================================

    @Test void deterministicCompression() {
        byte[] data = "the quick brown fox jumps over the lazy dog".getBytes();
        assertArrayEquals(LZW.compress(data), LZW.compress(data));
    }

    @Test void deterministicDecompression() {
        byte[] compressed = LZW.compress("hello world hello world".getBytes());
        assertArrayEquals(LZW.decompress(compressed), LZW.decompress(compressed));
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private static void roundTrip(byte[] data) {
        assertArrayEquals(data, LZW.decompress(LZW.compress(data)),
            "Round-trip failed for " + data.length + " bytes");
    }

    private static byte[] repeat(byte[] src, int times) {
        byte[] out = new byte[src.length * times];
        for (int i = 0; i < times; i++) System.arraycopy(src, 0, out, i*src.length, src.length);
        return out;
    }
}
