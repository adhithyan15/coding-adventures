package com.codingadventures.zstd;

// Unit tests for the Java ZStd (CMP07) implementation.
//
// Strategy: every test verifies a round-trip (compress then decompress yields
// the original bytes), plus specific assertions about output size where the
// algorithm promises a benefit (RLE, prose, repeat-offset patterns).
//
// The 12 tests mirror the Rust/C# reference suites so that behaviour is
// consistent across language ports.

import org.junit.jupiter.api.Test;

import java.io.IOException;
import java.util.Arrays;
import java.util.List;

import static com.codingadventures.zstd.Zstd.*;
import static org.junit.jupiter.api.Assertions.*;

class ZstdTest {

    // ─── Helper ───────────────────────────────────────────────────────────────

    /**
     * Compress then decompress; asserts that the result matches the input.
     *
     * @param data the input to round-trip
     * @return the decompressed bytes (equal to input)
     */
    private static byte[] rt(byte[] data) throws IOException {
        byte[] compressed = compress(data);
        byte[] restored = decompress(compressed);
        assertArrayEquals(data, restored, "round-trip mismatch");
        return restored;
    }

    // ─── TC-1: empty input ────────────────────────────────────────────────────

    /**
     * An empty input must produce a valid ZStd frame and decompress back to
     * empty bytes without panic or error.
     */
    @Test
    void tc1Empty() throws IOException {
        assertArrayEquals(new byte[0], rt(new byte[0]));
    }

    // ─── TC-2: single literal byte ────────────────────────────────────────────

    /**
     * A single literal byte 0x42 round-trips without issue.
     *
     * <p>This is the smallest non-trivial case: one literal byte, no match
     * possible, falls through to a raw block.</p>
     */
    @Test
    void tc2Literal() throws IOException {
        assertArrayEquals(new byte[]{0x42}, rt(new byte[]{0x42}));
    }

    // ─── TC-3: all 256 byte values ────────────────────────────────────────────

    /**
     * Every possible byte value 0x00–0xFF in order.
     *
     * <p>This exercises literal encoding of non-ASCII and zero bytes. No
     * significant compression is expected; all 256 bytes are distinct.</p>
     */
    @Test
    void tc3AllBytes() throws IOException {
        byte[] input = new byte[256];
        for (int i = 0; i < 256; i++) input[i] = (byte) i;
        rt(input);
    }

    // ─── TC-4: RLE block ──────────────────────────────────────────────────────

    /**
     * 1024 identical bytes should be detected as an RLE block.
     *
     * <p>Expected compressed size:</p>
     * <pre>
     *   4 (magic) + 1 (FHD) + 8 (FCS) + 3 (block header) + 1 (RLE byte) = 17
     * </pre>
     */
    @Test
    void tc4RleBlock() throws IOException {
        byte[] input = new byte[1024];
        Arrays.fill(input, (byte) 'A');
        byte[] compressed = compress(input);
        assertArrayEquals(input, decompress(compressed));
        assertTrue(compressed.length < 30,
                "RLE of 1024 bytes compressed to " + compressed.length +
                        " (expected < 30)");
    }

    // ─── TC-5: English prose ──────────────────────────────────────────────────

    /**
     * Repeated English text has strong LZ77 matches.
     *
     * <p>Must achieve ≥ 20% compression (output ≤ 80% of input size).</p>
     */
    @Test
    void tc5Prose() throws IOException {
        String text = "the quick brown fox jumps over the lazy dog ".repeat(25);
        byte[] input = text.getBytes();
        byte[] compressed = compress(input);
        assertArrayEquals(input, decompress(compressed));
        int threshold = input.length * 80 / 100;
        assertTrue(compressed.length < threshold,
                "prose: compressed " + compressed.length + " bytes (input " +
                        input.length + "), expected < " + threshold + " (80%)");
    }

    // ─── TC-6: pseudo-random data ─────────────────────────────────────────────

    /**
     * LCG pseudo-random bytes.
     *
     * <p>No significant compression expected, but round-trip must be exact
     * regardless of block type chosen.</p>
     */
    @Test
    void tc6Random() throws IOException {
        int seed = 42;
        byte[] input = new byte[512];
        for (int i = 0; i < 512; i++) {
            seed = seed * 1664525 + 1013904223;
            input[i] = (byte) (seed & 0xFF);
        }
        rt(input);
    }

    // ─── TC-7: 300 KB — forces multiple blocks ────────────────────────────────

    /**
     * 300 KB &gt; MAX_BLOCK_SIZE (128 KB), so this requires at least 3 blocks.
     *
     * <p>Uses a repeating byte to guarantee RLE blocks. Verifies that the
     * multi-block frame is correctly assembled and decoded.</p>
     */
    @Test
    void tc7Multiblock() throws IOException {
        byte[] input = new byte[300 * 1024];
        Arrays.fill(input, (byte) 'x');
        assertArrayEquals(input, rt(input));
    }

    // ─── TC-8: repeat-offset pattern ─────────────────────────────────────────

    /**
     * Alternating pattern with long runs of 'X' and repeated "ABCDEFGH".
     *
     * <p>Both the 'X' runs and the repeated string give strong LZ77 matches.
     * Expects &gt; 30% compression (output ≤ 70% of input size).</p>
     */
    @Test
    void tc8RepeatOffset() throws IOException {
        byte[] pattern = "ABCDEFGH".getBytes();
        List<Byte> buf = new java.util.ArrayList<>();
        for (byte b : pattern) buf.add(b);
        for (int i = 0; i < 10; i++) {
            for (int j = 0; j < 128; j++) buf.add((byte) 'X');
            for (byte b : pattern) buf.add(b);
        }
        byte[] input = new byte[buf.size()];
        for (int i = 0; i < input.length; i++) input[i] = buf.get(i);

        byte[] compressed = compress(input);
        assertArrayEquals(input, decompress(compressed));
        int threshold = input.length * 70 / 100;
        assertTrue(compressed.length < threshold,
                "repeat-offset: compressed " + compressed.length +
                        " (input " + input.length + "), expected < " + threshold + " (70%)");
    }

    // ─── TC-9: deterministic output ───────────────────────────────────────────

    /**
     * Compressing the same data twice must produce identical bytes.
     *
     * <p>This is required for reproducible builds and cache invalidation.</p>
     */
    @Test
    void tc9Deterministic() {
        String text = "hello, ZStd world! ".repeat(50);
        byte[] data = text.getBytes();
        assertArrayEquals(compress(data), compress(data));
    }

    // ─── RT: repeated pattern ─────────────────────────────────────────────────

    /**
     * Cyclic byte pattern "ABCDEF" repeated across 3000 bytes.
     *
     * <p>LZ77 should find strong long-distance back-references and achieve
     * significant compression.</p>
     */
    @Test
    void rtRepeatedPattern() throws IOException {
        byte[] src = "ABCDEF".getBytes();
        byte[] input = new byte[3000];
        for (int i = 0; i < input.length; i++) input[i] = src[i % src.length];
        rt(input);
    }

    // ─── RT: binary data ─────────────────────────────────────────────────────

    /**
     * Binary data with a repeating 0–255 cycle across 300 bytes.
     *
     * <p>Tests handling of all byte values including 0x00 and 0xFF.</p>
     */
    @Test
    void rtBinaryData() throws IOException {
        byte[] input = new byte[300];
        for (int i = 0; i < input.length; i++) input[i] = (byte) (i % 256);
        rt(input);
    }

    // ─── Unit: RevBitWriter / RevBitReader round-trip ─────────────────────────

    /**
     * Tests the backward bit-stream codec in isolation.
     *
     * <p>The backward stream stores bits so the LAST-written bits are read
     * FIRST by the decoder. This mirrors how ZStd's sequence codec writes the
     * initial FSE states last (so the decoder reads them first).</p>
     *
     * <pre>
     * Write order:  A=0b101 (3 bits), B=0b11001100 (8 bits), C=0b1 (1 bit)
     * Read order:   C first, then B, then A  (reversed)
     * </pre>
     */
    @Test
    void testRevBitWriterRevBitReaderRoundtrip() throws IOException {
        RevBitWriter bw = new RevBitWriter();
        bw.addBits(0b101L, 3);       // A — written first → read last
        bw.addBits(0b11001100L, 8);  // B
        bw.addBits(0b1L, 1);         // C — written last → read first
        bw.flush();
        byte[] buf = bw.finish();

        RevBitReader br = new RevBitReader(buf);
        assertEquals(0b1L,        br.readBits(1), "C: last written, first read");
        assertEquals(0b11001100L, br.readBits(8), "B");
        assertEquals(0b101L,      br.readBits(3), "A: first written, last read");
    }

    // ─── Unit: llToCode / mlToCode ───────────────────────────────────────────

    /**
     * For literal lengths 0–15 the LL code is the identity (code == value).
     */
    @Test
    void testLlToCodeSmall() {
        for (int i = 0; i < 16; i++) {
            assertEquals(i, llToCode(i), "LL code for " + i);
        }
    }

    /**
     * For match lengths 3–34 the ML code is (value - 3).
     */
    @Test
    void testMlToCodeSmall() {
        for (int i = 3; i < 35; i++) {
            assertEquals(i - 3, mlToCode(i), "ML code for " + i);
        }
    }

    // ─── Unit: encodeSeqCount / decodeSeqCount ────────────────────────────────

    /**
     * The sequence count encoding is a variable-length integer. Verifies
     * round-trip for values across all three encoding ranges.
     */
    @Test
    void testSeqCountRoundtrip() throws IOException {
        int[] values = {0, 1, 50, 127, 128, 1000, 0x7FFE};
        for (int n : values) {
            byte[] enc = encodeSeqCount(n);
            int[] dec = decodeSeqCount(enc, 0);
            assertEquals(n, dec[0], "seq count round-trip for " + n);
        }
    }

    // ─── Unit: FSE decode table coverage ─────────────────────────────────────

    /**
     * Every slot in the LL decode table must map to a valid symbol.
     *
     * <p>This catches off-by-one errors in the spreading algorithm that would
     * leave a cell with a sym index beyond the norm array length.</p>
     */
    @Test
    void testFseDecodeTableCoverage() {
        FseDe[] dt = buildDecodeTable(LL_NORM, LL_ACC_LOG);
        assertEquals(1 << LL_ACC_LOG, dt.length);
        for (FseDe cell : dt) {
            assertTrue((cell.sym & 0xFF) < LL_NORM.length,
                    "sym " + (cell.sym & 0xFF) + " out of range");
        }
    }

    // ─── Unit: FSE two-sequence encode/decode ────────────────────────────────

    /**
     * Encode two sequences then decode them, verifying (ll, ml, off) match.
     *
     * <p>This isolates the FSE codec from the full compression pipeline.</p>
     */
    @Test
    @SuppressWarnings("unchecked")
    void testFseTwoSequenceRoundtrip() throws IOException {
        // Two sequences to encode/decode
        // We use package-private access via the same package.
        // Build a minimal bitstream manually using the internal helpers.

        // Build encode tables
        Object[] resLl = buildEncodeTable(LL_NORM, LL_ACC_LOG);
        Object[] resMl = buildEncodeTable(ML_NORM, ML_ACC_LOG);
        Object[] resOf = buildEncodeTable(OF_NORM, OF_ACC_LOG);
        FseEe[] eeLl = (FseEe[]) resLl[0]; int[] stLl = (int[]) resLl[1];
        FseEe[] eeMl = (FseEe[]) resMl[0]; int[] stMl = (int[]) resMl[1];
        FseEe[] eeOf = (FseEe[]) resOf[0]; int[] stOf = (int[]) resOf[1];

        long szLl = 1L << LL_ACC_LOG;
        long szMl = 1L << ML_ACC_LOG;
        long szOf = 1L << OF_ACC_LOG;

        // Sequences: (ll=2, ml=4, off=1), (ll=0, ml=3, off=2)
        int[][] seqData = {{2, 4, 1}, {0, 3, 2}};

        long[] stateLl = {szLl};
        long[] stateMl = {szMl};
        long[] stateOf = {szOf};
        RevBitWriter bw = new RevBitWriter();

        // Encode in reverse order
        for (int si = seqData.length - 1; si >= 0; si--) {
            int ll = seqData[si][0], ml = seqData[si][1], off = seqData[si][2];
            int llCode = llToCode(ll);
            int mlCode = mlToCode(ml);
            int rawOff = off + 3;
            int ofCode = (rawOff <= 1) ? 0 : (31 - Integer.numberOfLeadingZeros(rawOff));
            int ofExtra = rawOff - (1 << ofCode);

            bw.addBits(ofExtra, ofCode);
            bw.addBits(ml - ML_CODES[mlCode][0], ML_CODES[mlCode][1]);
            bw.addBits(ll - LL_CODES[llCode][0], LL_CODES[llCode][1]);

            fseEncodeSym(stateMl, mlCode, eeMl, stMl, bw);
            fseEncodeSym(stateOf, ofCode, eeOf, stOf, bw);
            fseEncodeSym(stateLl, llCode, eeLl, stLl, bw);
        }
        bw.addBits(stateOf[0] - szOf, OF_ACC_LOG);
        bw.addBits(stateMl[0] - szMl, ML_ACC_LOG);
        bw.addBits(stateLl[0] - szLl, LL_ACC_LOG);
        bw.flush();
        byte[] bitstream = bw.finish();

        // Decode
        FseDe[] dtLl = buildDecodeTable(LL_NORM, LL_ACC_LOG);
        FseDe[] dtMl = buildDecodeTable(ML_NORM, ML_ACC_LOG);
        FseDe[] dtOf = buildDecodeTable(OF_NORM, OF_ACC_LOG);

        RevBitReader br = new RevBitReader(bitstream);
        int[] dStateLl = {(int) br.readBits(LL_ACC_LOG)};
        int[] dStateMl = {(int) br.readBits(ML_ACC_LOG)};
        int[] dStateOf = {(int) br.readBits(OF_ACC_LOG)};

        for (int i = 0; i < seqData.length; i++) {
            int llCode = fseDecodeSym(dStateLl, dtLl, br);
            int ofCode = fseDecodeSym(dStateOf, dtOf, br);
            int mlCode = fseDecodeSym(dStateMl, dtMl, br);

            int ll = LL_CODES[llCode][0] + (int) br.readBits(LL_CODES[llCode][1]);
            int ml = ML_CODES[mlCode][0] + (int) br.readBits(ML_CODES[mlCode][1]);
            int ofRaw = (1 << ofCode) | (int) br.readBits(ofCode);
            int off = ofRaw - 3;

            assertEquals(seqData[i][0], ll,  "seq " + i + " LL");
            assertEquals(seqData[i][1], ml,  "seq " + i + " ML");
            assertEquals(seqData[i][2], off, "seq " + i + " OFF");
        }
    }
}
