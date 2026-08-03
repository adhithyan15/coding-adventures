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
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;

import static com.codingadventures.zstd.Zstd.*;
import static org.junit.jupiter.api.Assertions.*;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

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

    // ─── TC-9: CLI interoperability ───────────────────────────────────────────

    /**
     * Cross-implementation round-trip against the real {@code zstd} CLI binary.
     *
     * <p>Both directions must work:</p>
     * <ol>
     *   <li>Compress with {@link Zstd#compress}, decompress with {@code zstd -d}.</li>
     *   <li>Compress with {@code zstd}, decompress with {@link Zstd#decompress}.</li>
     * </ol>
     *
     * <p>This is the test that actually proves the wire format is real RFC 8878,
     * not just a self-consistent internal format — a codec whose encoder and
     * decoder always agree with each other can still be silently wrong (see
     * lessons.md Lesson 250 / Lesson 95 for two bugs of exactly this shape that
     * this test line is what would have caught). Skipped (not failed) when the
     * {@code zstd} binary isn't on {@code PATH}, since CI/dev environments vary.</p>
     */
    @Test
    void tc9CliInterop() throws IOException, InterruptedException {
        assumeTrue(isZstdCliAvailable(), "zstd CLI not found on PATH — skipping interop test");

        String text = "the quick brown fox jumps over the lazy dog ".repeat(25);
        byte[] original = text.getBytes();

        // ── Direction 1: compress with ours, decompress with `zstd -d` ────────
        byte[] ourCompressed = compress(original);
        Path oursZst = Files.createTempFile("zstd-java-tc9-ours-", ".zst");
        Path oursOut;
        try {
            Files.write(oursZst, ourCompressed);
            oursOut = runZstd(List.of("-d", "-q", "-c", oursZst.toString()));
            byte[] decodedByCli = Files.readAllBytes(oursOut);
            assertArrayEquals(original, decodedByCli,
                    "real `zstd -d` failed to decode our compressed output");
        } finally {
            Files.deleteIfExists(oursZst);
        }

        // ── Direction 2: compress with `zstd`, decompress with ours ───────────
        Path theirsInput = Files.createTempFile("zstd-java-tc9-theirs-", ".txt");
        Path theirsZst;
        try {
            Files.write(theirsInput, original);
            theirsZst = runZstd(List.of("-q", "-c", theirsInput.toString()));
            byte[] theirCompressed = Files.readAllBytes(theirsZst);
            byte[] decodedByUs = decompress(theirCompressed);
            assertArrayEquals(original, decodedByUs,
                    "our decompress() failed to decode real `zstd`'s compressed output");
        } finally {
            Files.deleteIfExists(theirsInput);
        }
    }

    // ─── RT: CLI interop with a high sequence count ──────────────────────────

    /**
     * Real {@code zstd} CLI interop on an input large enough to push our
     * compressor's single-block sequence count past 128 — the exact boundary
     * where the sequence-count wire encoding switches from its 1-byte form to
     * its 2-byte form (RFC 8878 §3.1.1.3.1). A byte-order bug in that 2-byte
     * form (see lessons.md Lesson 250 / Lesson 95 — this port had exactly this
     * bug) round-trips fine against ITSELF but silently produces a
     * non-conformant frame, so only a real cross-implementation check like
     * this one can catch it. Not one of the spec's 10 mandatory TCs; extra
     * regression coverage for the fix.
     */
    @Test
    void rtCliInteropHighSequenceCount() throws IOException, InterruptedException {
        assumeTrue(isZstdCliAvailable(), "zstd CLI not found on PATH — skipping interop test");

        // A repeating 6-byte cycle across ~9 KB gives LZSS plenty of short,
        // distinct matches — comfortably more than 128 sequences in one block,
        // while staying well under the 128 KB block cap.
        byte[] src = "ABCDEF".getBytes();
        byte[] original = new byte[9000];
        for (int i = 0; i < original.length; i++) original[i] = src[i % src.length];

        byte[] ourCompressed = compress(original);
        Path oursZst = Files.createTempFile("zstd-java-rt-highseq-", ".zst");
        try {
            Files.write(oursZst, ourCompressed);
            Path oursOut = runZstd(List.of("-d", "-q", "-c", oursZst.toString()));
            byte[] decodedByCli = Files.readAllBytes(oursOut);
            assertArrayEquals(original, decodedByCli,
                    "real `zstd -d` failed to decode our high-sequence-count output " +
                            "(likely a sequence-count wire-format regression)");
        } finally {
            Files.deleteIfExists(oursZst);
        }
    }

    /**
     * Checks whether the {@code zstd} CLI binary is reachable on {@code PATH}.
     *
     * @return true if {@code zstd --version} exits successfully
     */
    private static boolean isZstdCliAvailable() {
        try {
            Process p = new ProcessBuilder("zstd", "--version")
                    .redirectErrorStream(true)
                    .start();
            boolean finished = p.waitFor(10, java.util.concurrent.TimeUnit.SECONDS);
            return finished && p.exitValue() == 0;
        } catch (IOException | InterruptedException e) {
            return false;
        }
    }

    /**
     * Runs {@code zstd} with the given arguments, capturing stdout to a temp file.
     *
     * @param args arguments to pass to the {@code zstd} binary
     * @return path to a temp file containing the captured stdout bytes
     */
    private static Path runZstd(List<String> args) throws IOException, InterruptedException {
        List<String> cmd = new java.util.ArrayList<>();
        cmd.add("zstd");
        cmd.addAll(args);

        Path stdout = Files.createTempFile("zstd-java-tc9-stdout-", ".bin");
        ProcessBuilder pb = new ProcessBuilder(cmd)
                .redirectOutput(stdout.toFile())
                .redirectError(ProcessBuilder.Redirect.DISCARD);
        Process p = pb.start();
        boolean finished = p.waitFor(30, java.util.concurrent.TimeUnit.SECONDS);
        if (!finished) {
            p.destroyForcibly();
            throw new IOException("zstd CLI timed out: " + cmd);
        }
        if (p.exitValue() != 0) {
            throw new IOException("zstd CLI failed (exit " + p.exitValue() + "): " + cmd);
        }
        return stdout;
    }

    // ─── TC-10: hand-built minimal wire-format frame ──────────────────────────

    /**
     * Manually constructs a minimal ZStd frame byte-by-byte and verifies
     * {@link Zstd#decompress} reads it correctly — independent of our own
     * encoder, so this exercises the decoder's wire-format parsing in
     * isolation.
     *
     * <p>Frame layout:</p>
     * <pre>
     *   [0..3]  Magic = 0xFD2FB528 LE = 28 B5 2F FD
     *   [4]     FHD = 0x20:
     *             bits [7:6] = 00 -&gt; FCS_flag = 0
     *             bit  [5]   = 1  -&gt; Single_Segment = 1
     *             (with Single_Segment=1 and FCS_flag=00, FCS is 1 byte)
     *   [5]     FCS = 0x05 (content size = 5)
     *   [6..8]  Block header: Last=1, Type=Raw(00), Size=5
     *             = (5 &lt;&lt; 3) | (0 &lt;&lt; 1) | 1 = 41 = 0x29
     *             = [0x29, 0x00, 0x00]
     *   [9..13] b"hello"
     * </pre>
     */
    @Test
    void tc10WireFormat() throws IOException {
        byte[] frame = {
                0x28, (byte) 0xB5, 0x2F, (byte) 0xFD, // magic
                0x20,                                  // FHD: Single_Segment=1, FCS=1 byte
                0x05,                                  // FCS = 5
                0x29, 0x00, 0x00,                       // block header: last=1, raw, size=5
                'h', 'e', 'l', 'l', 'o',
        };
        assertArrayEquals("hello".getBytes(), decompress(frame));
    }

    // ─── Extra: deterministic output ──────────────────────────────────────────

    /**
     * Compressing the same data twice must produce identical bytes.
     *
     * <p>This is required for reproducible builds and cache invalidation. Not
     * one of the spec's 10 mandatory test cases, but cheap extra coverage.</p>
     */
    @Test
    void rtDeterministicOutput() {
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
        // Covers the 1-byte/2-byte/3-byte form boundaries: 127|128 and
        // 0x7EFF|0x7F00 (2-byte form is [128, 0x7F00) per RFC 8878 §3.1.1.3.1).
        int[] values = {0, 1, 50, 127, 128, 1000, 0x7EFF, 0x7F00, 0x7F01, 90000};
        for (int n : values) {
            byte[] enc = encodeSeqCount(n);
            int[] dec = decodeSeqCount(enc, 0);
            assertEquals(n, dec[0], "seq count round-trip for " + n);
            assertEquals(enc.length, dec[1], "bytes-consumed mismatch for " + n);
        }
    }

    /**
     * Exact wire-byte assertions for the 2-byte sequence-count form.
     *
     * <p>A round-trip test alone cannot catch a byte-order bug: if the encoder
     * and decoder are internally self-consistent (paired with each other),
     * swapping which byte carries the 0x80 marker still round-trips correctly
     * within this codec — it just no longer matches RFC 8878 §3.1.1.3.1 or any
     * other implementation. This bit the Java port once already (see
     * lessons.md Lesson 250 / Lesson 95): the marker/high byte MUST be
     * transmitted FIRST, low byte second. Deliberately includes values whose
     * LOW byte is &lt; 128 (300 → low byte 0x2C, 768 → low byte 0x00) — those
     * are exactly the values a reversed-byte-order bug would corrupt silently,
     * because a low byte &lt; 0x80 could be mistaken for a 1-byte form marker.</p>
     */
    @Test
    void testSeqCountWireBytesExact() {
        // count=200: hi = 128 + (200>>8) = 128 = 0x80, lo = 200 & 0xFF = 0xC8
        assertArrayEquals(new byte[]{(byte) 0x80, (byte) 0xC8}, encodeSeqCount(200));
        // count=300: hi = 128 + (300>>8) = 129 = 0x81, lo = 300 & 0xFF = 0x2C (< 0x80)
        assertArrayEquals(new byte[]{(byte) 0x81, (byte) 0x2C}, encodeSeqCount(300));
        // count=515: hi = 128 + (515>>8) = 130 = 0x82, lo = 515 & 0xFF = 0x03 (< 0x80)
        assertArrayEquals(new byte[]{(byte) 0x82, (byte) 0x03}, encodeSeqCount(515));
        // count=768: hi = 128 + (768>>8) = 131 = 0x83, lo = 768 & 0xFF = 0x00 (< 0x80)
        assertArrayEquals(new byte[]{(byte) 0x83, (byte) 0x00}, encodeSeqCount(768));

        // And the decode side reads the marker byte FIRST.
        try {
            assertEquals(300, decodeSeqCount(new byte[]{(byte) 0x81, (byte) 0x2C}, 0)[0]);
            assertEquals(768, decodeSeqCount(new byte[]{(byte) 0x83, (byte) 0x00}, 0)[0]);
        } catch (IOException e) {
            fail(e);
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

        // Sequences: (ll=2, ml=4, off=1), (ll=0, ml=3, off=2)
        int[][] seqData = {{2, 4, 1}, {0, 3, 2}};

        long[] stateLl = new long[1];
        long[] stateMl = new long[1];
        long[] stateOf = new long[1];
        RevBitWriter bw = new RevBitWriter();

        // Encode in reverse order, mirroring encodeSequencesSection() exactly
        // (RFC 8878 §3.1.1.3.2.1.2, verified against the reference C source
        // and the real `zstd` CLI — see Lesson 95): per sequence, extras are
        // written LL,ML,OF; state transitions (all but the last-processed
        // sequence) are written OF,ML,LL; the last-processed sequence (= the
        // semantically LAST real sequence) gets its state INITIALISED via
        // fseInitState() instead of a transition, since a real decoder never
        // reads an "update" after the last sequence.
        boolean first = true;
        for (int si = seqData.length - 1; si >= 0; si--) {
            int ll = seqData[si][0], ml = seqData[si][1], off = seqData[si][2];
            int llCode = llToCode(ll);
            int mlCode = mlToCode(ml);
            int rawOff = off + 3;
            int ofCode = (rawOff <= 1) ? 0 : (31 - Integer.numberOfLeadingZeros(rawOff));
            int ofExtra = rawOff - (1 << ofCode);

            if (!first) {
                fseEncodeSym(stateOf, ofCode, eeOf, stOf, bw);
                fseEncodeSym(stateMl, mlCode, eeMl, stMl, bw);
                fseEncodeSym(stateLl, llCode, eeLl, stLl, bw);
            } else {
                stateOf[0] = fseInitState(ofCode, eeOf, stOf);
                stateMl[0] = fseInitState(mlCode, eeMl, stMl);
                stateLl[0] = fseInitState(llCode, eeLl, stLl);
                first = false;
            }

            bw.addBits(ll - LL_CODES[llCode][0], LL_CODES[llCode][1]);
            bw.addBits(ml - ML_CODES[mlCode][0], ML_CODES[mlCode][1]);
            bw.addBits(ofExtra, ofCode);
        }
        // Initial-state flush order: decode order is LL,OF,ML, so write ML,OF,LL.
        long szLl = 1L << LL_ACC_LOG;
        long szMl = 1L << ML_ACC_LOG;
        long szOf = 1L << OF_ACC_LOG;
        bw.addBits(stateMl[0] - szMl, ML_ACC_LOG);
        bw.addBits(stateOf[0] - szOf, OF_ACC_LOG);
        bw.addBits(stateLl[0] - szLl, LL_ACC_LOG);
        bw.flush();
        byte[] bitstream = bw.finish();

        // Decode, mirroring decompressBlock() exactly: peek symbols (no bits
        // consumed), read extras OF,ML,LL, then — unless this is the last
        // sequence — update states LL,ML,OF.
        FseDe[] dtLl = buildDecodeTable(LL_NORM, LL_ACC_LOG);
        FseDe[] dtMl = buildDecodeTable(ML_NORM, ML_ACC_LOG);
        FseDe[] dtOf = buildDecodeTable(OF_NORM, OF_ACC_LOG);

        RevBitReader br = new RevBitReader(bitstream);
        int[] dStateLl = {(int) br.readBits(LL_ACC_LOG)};
        int[] dStateOf = {(int) br.readBits(OF_ACC_LOG)};
        int[] dStateMl = {(int) br.readBits(ML_ACC_LOG)};

        for (int i = 0; i < seqData.length; i++) {
            FseDe llEntry = dtLl[dStateLl[0]];
            FseDe mlEntry = dtMl[dStateMl[0]];
            FseDe ofEntry = dtOf[dStateOf[0]];
            int llCode = llEntry.sym & 0xFF;
            int mlCode = mlEntry.sym & 0xFF;
            int ofCode = ofEntry.sym & 0xFF;

            int ofRaw = (1 << ofCode) | (int) br.readBits(ofCode);
            int ml = ML_CODES[mlCode][0] + (int) br.readBits(ML_CODES[mlCode][1]);
            int ll = LL_CODES[llCode][0] + (int) br.readBits(LL_CODES[llCode][1]);
            int off = ofRaw - 3;

            if (i != seqData.length - 1) {
                dStateLl[0] = llEntry.base + (int) br.readBits(llEntry.nb & 0xFF);
                dStateMl[0] = mlEntry.base + (int) br.readBits(mlEntry.nb & 0xFF);
                dStateOf[0] = ofEntry.base + (int) br.readBits(ofEntry.nb & 0xFF);
            }

            assertEquals(seqData[i][0], ll,  "seq " + i + " LL");
            assertEquals(seqData[i][1], ml,  "seq " + i + " ML");
            assertEquals(seqData[i][2], off, "seq " + i + " OFF");
        }
    }
}
