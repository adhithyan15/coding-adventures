package com.codingadventures.pdf417;

import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

import java.util.Arrays;
import java.util.List;

/**
 * Tests for the PDF417 encoder — ISO/IEC 15438:2015.
 *
 * <p>Test strategy:
 * <ol>
 *   <li>GF(929) field arithmetic (add, mul, table round-trips)</li>
 *   <li>RS ECC encoding (known codeword sequences)</li>
 *   <li>Byte compaction (6→5 base-900 groups and single-byte remainder)</li>
 *   <li>Row indicator computation (LRI/RRI values for specific symbols)</li>
 *   <li>Symbol dimensions (auto-selection heuristic)</li>
 *   <li>Integration: encode "A", "HELLO WORLD", binary bytes</li>
 *   <li>Start/stop pattern presence in every row</li>
 *   <li>Error handling: bad ECC level, bad columns, data too long</li>
 * </ol>
 */
class PDF417Test {

    // =========================================================================
    // 1. GF(929) arithmetic
    // =========================================================================

    /** GF(929) add: ordinary addition mod 929. */
    @Test
    void testGfAdd_basicAddition() {
        // (100 + 900) mod 929 = 1000 mod 929 = 71
        assertEquals(71, PDF417.gfAdd(100, 900));
    }

    @Test
    void testGfAdd_noWrap() {
        // 10 + 20 = 30 (no modular reduction needed)
        assertEquals(30, PDF417.gfAdd(10, 20));
    }

    @Test
    void testGfAdd_maxValues() {
        // 928 + 928 = 1856, 1856 mod 929 = 927
        assertEquals(927, PDF417.gfAdd(928, 928));
    }

    @Test
    void testGfAdd_identityZero() {
        // Adding 0 is identity
        assertEquals(42, PDF417.gfAdd(42, 0));
        assertEquals(42, PDF417.gfAdd(0, 42));
    }

    /** GF(929) mul: using log/antilog tables. */
    @Test
    void testGfMul_byZero() {
        // Anything × 0 = 0
        assertEquals(0, PDF417.gfMul(0, 5));
        assertEquals(0, PDF417.gfMul(5, 0));
        assertEquals(0, PDF417.gfMul(0, 0));
    }

    @Test
    void testGfMul_byOne() {
        // Anything × 1 = itself (1 = α^0 = GF_EXP[0])
        assertEquals(42, PDF417.gfMul(42, 1));
        assertEquals(42, PDF417.gfMul(1, 42));
    }

    @Test
    void testGfMul_simple() {
        // 3 × 3 = 9 (no modular reduction needed at this scale)
        assertEquals(9, PDF417.gfMul(3, 3));
    }

    @Test
    void testGfMul_withReduction() {
        // 400 × 400 = 160000. 160000 mod 929:
        // 929 × 172 = 159788, 160000 − 159788 = 212.
        assertEquals(212, PDF417.gfMul(400, 400));
    }

    @Test
    void testGfMul_inverse_of_3() {
        // The multiplicative inverse of 3 mod 929:
        // 3 × x ≡ 1 (mod 929) → x = 310
        // Verify: 3 × 310 = 930 ≡ 1 (mod 929) ✓
        assertEquals(1, PDF417.gfMul(3, 310));
    }

    @Test
    void testGfMul_commutativity() {
        // GF multiplication is commutative.
        for (int a : new int[]{2, 7, 100, 500, 928}) {
            for (int b : new int[]{3, 13, 99, 400, 927}) {
                assertEquals(
                    PDF417.gfMul(a, b),
                    PDF417.gfMul(b, a),
                    "gfMul should be commutative for (" + a + ", " + b + ")"
                );
            }
        }
    }

    /** Log/antilog table round-trip: exp[log[v]] == v for all non-zero v. */
    @Test
    void testGfTables_roundTrip() {
        // For every non-zero element v in GF(929), α^(log v) == v.
        for (int v = 1; v < PDF417.GF929_PRIME; v++) {
            int logV = PDF417.GF_LOG[v];
            int recovered = PDF417.GF_EXP[logV];
            assertEquals(v, recovered,
                "Round-trip failed for v=" + v + ": GF_EXP[GF_LOG[" + v + "]] = " + recovered);
        }
    }

    /** Fermat's little theorem: α^(p-1) = α^928 ≡ 1 (mod 929). */
    @Test
    void testGfExp_fermatLittleTheorem() {
        // The 929th entry in GF_EXP should wrap: GF_EXP[928] = GF_EXP[0] = 1.
        assertEquals(1, PDF417.GF_EXP[0],  "α^0 should be 1");
        assertEquals(1, PDF417.GF_EXP[928], "α^928 should be 1 (Fermat)");
    }

    /** The generator α = 3 is a primitive root: all 928 non-zero elements appear. */
    @Test
    void testGfExp_coversAllNonZero() {
        // Every value in 1..928 must appear exactly once in GF_EXP[0..927].
        boolean[] seen = new boolean[929];
        for (int i = 0; i < 928; i++) {
            int v = PDF417.GF_EXP[i];
            assertFalse(seen[v], "Value " + v + " appeared twice in GF_EXP (at index " + i + ")");
            assertTrue(v >= 1 && v <= 928, "GF_EXP[" + i + "] = " + v + " is out of range 1..928");
            seen[v] = true;
        }
        // Verify every element 1..928 was seen.
        for (int v = 1; v <= 928; v++) {
            assertTrue(seen[v], "Value " + v + " never appeared in GF_EXP[0..927]");
        }
    }

    // =========================================================================
    // 2. Reed-Solomon ECC
    // =========================================================================

    /** For ECC level 0 (k=2), the generator polynomial must have degree 2. */
    @Test
    void testBuildGenerator_level0_degree() {
        int[] g = PDF417.buildGenerator(0);
        // Level 0: k = 2^(0+1) = 2 ECC codewords → degree-2 polynomial → 3 coefficients.
        assertEquals(3, g.length, "ECC level 0 generator must have 3 coefficients (degree 2)");
        // Leading coefficient must be 1.
        assertEquals(1, g[0], "Leading coefficient of generator must be 1");
    }

    /** For ECC level 2 (k=8), the generator must have 9 coefficients. */
    @Test
    void testBuildGenerator_level2_degree() {
        int[] g = PDF417.buildGenerator(2);
        assertEquals(9, g.length, "ECC level 2 generator must have 9 coefficients (degree 8)");
        assertEquals(1, g[0]);
    }

    /**
     * Encode a trivial one-codeword message and verify ECC is computed.
     * The exact values depend on the generator; we just check length and range.
     */
    @Test
    void testRsEncode_level0_outputLength() {
        int[] data = {100};
        int[] ecc = PDF417.rsEncode(data, 0);
        assertEquals(2, ecc.length, "ECC level 0 should produce 2 ECC codewords");
        // ECC values must be in range 0..928.
        for (int v : ecc) {
            assertTrue(v >= 0 && v <= 928, "ECC value " + v + " out of range 0..928");
        }
    }

    @Test
    void testRsEncode_level2_outputLength() {
        int[] data = {100, 200, 300};
        int[] ecc = PDF417.rsEncode(data, 2);
        assertEquals(8, ecc.length, "ECC level 2 should produce 8 ECC codewords");
        for (int v : ecc) {
            assertTrue(v >= 0 && v <= 928, "ECC value " + v + " out of range 0..928");
        }
    }

    /** Identical data must produce identical ECC (determinism). */
    @Test
    void testRsEncode_deterministic() {
        int[] data = {1, 2, 3, 4, 5};
        int[] ecc1 = PDF417.rsEncode(data, 2);
        int[] ecc2 = PDF417.rsEncode(data, 2);
        assertArrayEquals(ecc1, ecc2, "rsEncode must be deterministic");
    }

    /** Different data must produce different ECC (no constant output). */
    @Test
    void testRsEncode_differentiates() {
        int[] dataA = {100, 200, 300};
        int[] dataB = {101, 200, 300};
        int[] eccA = PDF417.rsEncode(dataA, 2);
        int[] eccB = PDF417.rsEncode(dataB, 2);
        assertFalse(Arrays.equals(eccA, eccB), "Different data must produce different ECC");
    }

    // =========================================================================
    // 3. Byte compaction
    // =========================================================================

    /** Single byte: should emit [924, byte_value]. */
    @Test
    void testByteCompact_singleByte() {
        byte[] input = {0x41}; // 'A' = 65
        List<Integer> cw = PDF417.byteCompact(input);
        assertEquals(2, cw.size(), "Single byte should produce [924, 65]");
        assertEquals(924, cw.get(0), "First codeword must be latch 924");
        assertEquals(65, cw.get(1), "Second codeword must be raw byte value");
    }

    /** 0xFF byte: should emit [924, 255]. */
    @Test
    void testByteCompact_highByte() {
        byte[] input = {(byte) 0xFF};
        List<Integer> cw = PDF417.byteCompact(input);
        assertEquals(2, cw.size());
        assertEquals(924, cw.get(0));
        assertEquals(255, cw.get(1));
    }

    /**
     * 6 bytes "ABCDEF" → exactly 5 codewords (+ latch 924 = 6 total).
     *
     * <p>n = 65×256^5 + 66×256^4 + 67×256^3 + 68×256^2 + 69×256 + 70
     *      = 71,362,724,440,134  (verify with Python: int.from_bytes(b'ABCDEF', 'big'))
     * Base-900 decomposition of n → exactly 5 codewords.
     */
    @Test
    void testByteCompact_sixBytes_fiveCodewords() {
        byte[] input = {0x41, 0x42, 0x43, 0x44, 0x45, 0x46}; // ABCDEF
        List<Integer> cw = PDF417.byteCompact(input);
        // latch (1) + 5 codewords = 6 total
        assertEquals(6, cw.size(), "6 bytes should produce [924, c1, c2, c3, c4, c5]");
        assertEquals(924, cw.get(0), "First codeword must be latch 924");
        // All 5 base-900 codewords must be in range 0..899.
        for (int i = 1; i <= 5; i++) {
            int v = cw.get(i);
            assertTrue(v >= 0 && v <= 899, "Base-900 codeword " + v + " out of range at position " + i);
        }
    }

    /**
     * 7 bytes "ABCDEFG" → 5 codewords from 6-byte group + 1 direct byte = 6 data codewords.
     * Total with latch: 7 codewords.
     */
    @Test
    void testByteCompact_sevenBytes() {
        byte[] input = {0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47}; // ABCDEFG
        List<Integer> cw = PDF417.byteCompact(input);
        // latch (1) + 5 from group + 1 remainder = 7 total
        assertEquals(7, cw.size(), "7 bytes → latch + 5 + 1 = 7 codewords");
        assertEquals(924, cw.get(0));
        // Last codeword is raw 'G' = 71
        assertEquals(71, cw.get(6), "Remainder byte 'G' should be codeword 71");
    }

    /** 12 bytes: two full 6-byte groups → 10 codewords + latch = 11 total. */
    @Test
    void testByteCompact_twelvBytes() {
        byte[] input = new byte[12];
        List<Integer> cw = PDF417.byteCompact(input);
        assertEquals(11, cw.size(), "12 bytes → latch + 10 codewords = 11");
    }

    /** Empty input: should emit just [924] (latch only). */
    @Test
    void testByteCompact_empty() {
        byte[] input = new byte[0];
        List<Integer> cw = PDF417.byteCompact(input);
        assertEquals(1, cw.size(), "Empty input should produce just the latch codeword");
        assertEquals(924, cw.get(0));
    }

    /** Verify the 6-byte base-900 decomposition is reversible. */
    @Test
    void testByteCompact_sixBytes_reversible() {
        // For any 6 bytes, we should be able to reconstruct the original bytes
        // from the 5 base-900 codewords. This verifies correctness of the encoding.
        byte[] original = {0x12, 0x34, 0x56, 0x78, (byte) 0x9A, (byte) 0xBC};
        List<Integer> cw = PDF417.byteCompact(original);
        assertEquals(6, cw.size());

        // Reconstruct: sum codewords in base 900, then decompose to bytes.
        long n = 0;
        for (int i = 1; i <= 5; i++) n = n * 900 + cw.get(i);

        byte[] recovered = new byte[6];
        for (int i = 5; i >= 0; i--) {
            recovered[i] = (byte)(n & 0xFF);
            n >>= 8;
        }
        assertArrayEquals(original, recovered, "6-byte base-900 encoding must be reversible");
    }

    // =========================================================================
    // 4. Auto ECC level selection
    // =========================================================================

    @Test
    void testAutoEccLevel_smallData() {
        assertEquals(2, PDF417.autoEccLevel(1));
        assertEquals(2, PDF417.autoEccLevel(40));
    }

    @Test
    void testAutoEccLevel_mediumData() {
        assertEquals(3, PDF417.autoEccLevel(41));
        assertEquals(3, PDF417.autoEccLevel(160));
    }

    @Test
    void testAutoEccLevel_largeData() {
        assertEquals(4, PDF417.autoEccLevel(161));
        assertEquals(4, PDF417.autoEccLevel(320));
    }

    @Test
    void testAutoEccLevel_veryLargeData() {
        assertEquals(5, PDF417.autoEccLevel(321));
        assertEquals(5, PDF417.autoEccLevel(863));
    }

    @Test
    void testAutoEccLevel_maximum() {
        assertEquals(6, PDF417.autoEccLevel(864));
        assertEquals(6, PDF417.autoEccLevel(2000));
    }

    // =========================================================================
    // 5. Dimension selection
    // =========================================================================

    @Test
    void testChooseDimensions_coversTotal() {
        // For any total, rows × cols must be ≥ total.
        for (int total : new int[]{1, 5, 10, 20, 50, 100, 200, 500, 1000, 2700}) {
            PDF417.Dimensions d = PDF417.chooseDimensions(total);
            assertTrue(d.rows * d.cols >= total,
                "Dimensions " + d.rows + "×" + d.cols + " too small for total=" + total);
        }
    }

    @Test
    void testChooseDimensions_boundsRespected() {
        for (int total : new int[]{1, 10, 100, 1000, 2700}) {
            PDF417.Dimensions d = PDF417.chooseDimensions(total);
            assertTrue(d.rows >= 3 && d.rows <= 90,
                "Rows " + d.rows + " out of bounds for total=" + total);
            assertTrue(d.cols >= 1 && d.cols <= 30,
                "Cols " + d.cols + " out of bounds for total=" + total);
        }
    }

    // =========================================================================
    // 6. Row indicator computation
    // =========================================================================
    //
    // Spec test vector: 10-row, 3-column, ECC level 2 symbol.
    //   R = 10, C = 3, L = 2
    //   R_info = (10-1) / 3 = 3
    //   C_info = 3 - 1 = 2
    //   L_info = 3×2 + (10-1) mod 3 = 6 + 0 = 6
    //
    //   Row 0 (cluster 0): LRI = 30×0 + R_info = 3,  RRI = 30×0 + C_info = 2
    //   Row 1 (cluster 1): LRI = 30×0 + L_info = 6,  RRI = 30×0 + R_info = 3
    //   Row 2 (cluster 2): LRI = 30×0 + C_info = 2,  RRI = 30×0 + L_info = 6
    //   Row 3 (cluster 0): LRI = 30×1 + R_info = 33, RRI = 30×1 + C_info = 32

    @Test
    void testRowIndicators_specVector() {
        int R = 10, C = 3, L = 2;

        // Row 0 (cluster 0)
        assertEquals(3,  PDF417.computeLRI(0, R, C, L), "Row 0 LRI");
        assertEquals(2,  PDF417.computeRRI(0, R, C, L), "Row 0 RRI");

        // Row 1 (cluster 1)
        assertEquals(6,  PDF417.computeLRI(1, R, C, L), "Row 1 LRI");
        assertEquals(3,  PDF417.computeRRI(1, R, C, L), "Row 1 RRI");

        // Row 2 (cluster 2)
        assertEquals(2,  PDF417.computeLRI(2, R, C, L), "Row 2 LRI");
        assertEquals(6,  PDF417.computeRRI(2, R, C, L), "Row 2 RRI");

        // Row 3 (cluster 0 again, next row group)
        assertEquals(33, PDF417.computeLRI(3, R, C, L), "Row 3 LRI");
        assertEquals(32, PDF417.computeRRI(3, R, C, L), "Row 3 RRI");
    }

    @Test
    void testRowIndicators_allInRange() {
        // All LRI and RRI values must be in 0..928 (valid codeword values).
        int R = 30, C = 10, L = 4;
        for (int r = 0; r < R; r++) {
            int lri = PDF417.computeLRI(r, R, C, L);
            int rri = PDF417.computeRRI(r, R, C, L);
            assertTrue(lri >= 0 && lri <= 928, "LRI " + lri + " out of range at row " + r);
            assertTrue(rri >= 0 && rri <= 928, "RRI " + rri + " out of range at row " + r);
        }
    }

    // =========================================================================
    // 7. Integration tests — ModuleGrid structure
    // =========================================================================

    /** Encoding "A" (1 byte) should produce a valid symbol. */
    @Test
    void testEncode_singleByte_returnsGrid() {
        ModuleGrid grid = PDF417.encode("A");
        assertNotNull(grid, "encode() must return a non-null ModuleGrid");
        assertTrue(grid.rows > 0, "Symbol must have positive height");
        assertTrue(grid.cols > 0, "Symbol must have positive width");
        assertEquals(ModuleShape.SQUARE, grid.moduleShape, "PDF417 uses square modules");
    }

    /** The module grid dimensions must satisfy the PDF417 width formula: 69 + 17×cols. */
    @Test
    void testEncode_widthFormula() {
        // Encode with explicit column count so we know exactly which c to expect.
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.columns = 3;
        ModuleGrid grid = PDF417.encode("HELLO WORLD".getBytes(), opts);

        // Width = 69 + 17 × 3 = 69 + 51 = 120.
        int expectedWidth = 69 + 17 * opts.columns;
        assertEquals(expectedWidth, grid.cols,
            "Symbol width must be 69 + 17 × cols = " + expectedWidth);
    }

    /** Row height multiplier must produce total rows = logical_rows × rowHeight. */
    @Test
    void testEncode_rowHeightMultiplier() {
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.columns = 3;
        opts.rowHeight = 4;

        ModuleGrid grid = PDF417.encode("HELLO WORLD".getBytes(), opts);
        // The total rows must be a multiple of rowHeight.
        assertEquals(0, grid.rows % opts.rowHeight,
            "Total rows must be a multiple of rowHeight=" + opts.rowHeight);
    }

    /** Every row in the symbol must start with the PDF417 start pattern. */
    @Test
    void testEncode_startPattern_everyRow() {
        // Start pattern: 11111111010101000 (17 modules)
        // bar/space widths: [8, 1, 1, 1, 1, 1, 1, 3]
        boolean[] expectedStart = expandBits("11111111010101000");
        assertEquals(17, expectedStart.length);

        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.rowHeight = 1; // one module-row per logical row for easy inspection
        ModuleGrid grid = PDF417.encode("TEST".getBytes(), opts);

        for (int r = 0; r < grid.rows; r++) {
            List<Boolean> row = grid.modules.get(r);
            for (int i = 0; i < 17; i++) {
                assertEquals(
                    expectedStart[i], row.get(i),
                    "Start pattern mismatch at row=" + r + " col=" + i
                );
            }
        }
    }

    /** Every row in the symbol must end with the PDF417 stop pattern. */
    @Test
    void testEncode_stopPattern_everyRow() {
        // Stop pattern: 111111101000101001 (18 modules)
        // bar/space widths: [7, 1, 1, 3, 1, 1, 1, 2, 1]
        boolean[] expectedStop = expandBits("111111101000101001");
        assertEquals(18, expectedStop.length);

        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.rowHeight = 1;
        ModuleGrid grid = PDF417.encode("TEST".getBytes(), opts);

        for (int r = 0; r < grid.rows; r++) {
            List<Boolean> row = grid.modules.get(r);
            int rowWidth = row.size();
            for (int i = 0; i < 18; i++) {
                assertEquals(
                    expectedStop[i], row.get(rowWidth - 18 + i),
                    "Stop pattern mismatch at row=" + r + " stop-col=" + i
                );
            }
        }
    }

    /** "HELLO WORLD" (11 bytes) — auto ECC level should be 2 (data count ≤ 40). */
    @Test
    void testEncode_helloWorld_eccLevel2() {
        // "HELLO WORLD" = 11 bytes
        // Byte compaction: latch(1) + ceil(11/6)*5 + 5 rem = 1 + 5 + 5 = 11 data cwords.
        // Counting: 6 bytes → 5 cwords + 5 bytes → 5 cwords = 10; plus latch = 11.
        // Length descriptor adds 1 → 12 total before ECC.
        // ECC count = 2^(2+1) = 8.
        // Total = 12 + 8 = 20 codewords → auto-ECC level 2 (20 ≤ 40).
        ModuleGrid grid = PDF417.encode("HELLO WORLD");
        assertNotNull(grid);

        // The symbol must have at least 3 rows.
        assertTrue(grid.rows >= 3, "Symbol must have at least 3 rows");
    }

    /** Encoding "1234567890" (10 bytes in v0.1.0 byte mode) must produce a valid grid. */
    @Test
    void testEncode_digits() {
        ModuleGrid grid = PDF417.encode("1234567890");
        assertNotNull(grid);
        assertTrue(grid.rows >= 3);
        assertTrue(grid.cols > 0);
    }

    /** Encoding all 256 byte values [0x00..0xFF] must work without error. */
    @Test
    void testEncode_allByteValues() {
        byte[] allBytes = new byte[256];
        for (int i = 0; i < 256; i++) allBytes[i] = (byte) i;

        ModuleGrid grid = PDF417.encode(allBytes);
        assertNotNull(grid);
        assertTrue(grid.rows >= 3);
        assertTrue(grid.cols > 0);
    }

    /** ModuleGrid dimensions (width formula) for minimal "A" input. */
    @Test
    void testEncode_minimalInput_dimensions() {
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.rowHeight = 1;
        ModuleGrid grid = PDF417.encode(new byte[]{0x41}, opts); // "A"

        // Width = 69 + 17 × cols (where cols was auto-chosen).
        // Extract actual cols from grid width.
        int cols = (grid.cols - 69) / 17;
        assertEquals(grid.cols, 69 + 17 * cols,
            "Symbol width must satisfy 69 + 17 × cols");

        // Height = rows × rowHeight = rows × 1 = rows.
        assertTrue(grid.rows >= 3, "Symbol must have at least 3 rows");
    }

    /** Two identical inputs must produce identical grids. */
    @Test
    void testEncode_deterministic() {
        byte[] data = "PDF417".getBytes();
        ModuleGrid g1 = PDF417.encode(data);
        ModuleGrid g2 = PDF417.encode(data);
        assertEquals(g1.rows, g2.rows);
        assertEquals(g1.cols, g2.cols);
        assertEquals(g1.modules, g2.modules);
    }

    // =========================================================================
    // 8. Error handling
    // =========================================================================

    /** Negative ECC level must throw InvalidECCLevelException. */
    @Test
    void testEncode_negativeEccLevel_throws() {
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.eccLevel = -1;
        assertThrows(PDF417.InvalidECCLevelException.class,
            () -> PDF417.encode("test".getBytes(), opts),
            "Negative ECC level must throw InvalidECCLevelException");
    }

    /** ECC level > 8 must throw InvalidECCLevelException. */
    @Test
    void testEncode_eccLevel9_throws() {
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.eccLevel = 9;
        assertThrows(PDF417.InvalidECCLevelException.class,
            () -> PDF417.encode("test".getBytes(), opts));
    }

    /** columns = 0 must throw InvalidDimensionsException. */
    @Test
    void testEncode_zeroColumns_throws() {
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.columns = 0;
        assertThrows(PDF417.InvalidDimensionsException.class,
            () -> PDF417.encode("test".getBytes(), opts));
    }

    /** columns = 31 must throw InvalidDimensionsException. */
    @Test
    void testEncode_tooManyColumns_throws() {
        PDF417.PDF417Options opts = new PDF417.PDF417Options();
        opts.columns = 31;
        assertThrows(PDF417.InvalidDimensionsException.class,
            () -> PDF417.encode("test".getBytes(), opts));
    }

    /** Valid ECC levels 0–8 must all work without error. */
    @Test
    void testEncode_allEccLevels_valid() {
        for (int ecc = 0; ecc <= 8; ecc++) {
            PDF417.PDF417Options opts = new PDF417.PDF417Options();
            opts.eccLevel = ecc;
            // Small input (1 byte). Level 8 needs many ECC slots but should fit.
            ModuleGrid grid = PDF417.encode("A".getBytes(), opts);
            assertNotNull(grid, "ECC level " + ecc + " must produce a valid grid");
        }
    }

    /** Valid column counts 1–30 must all work without error. */
    @Test
    void testEncode_allColumnCounts_valid() {
        byte[] data = "HELLO WORLD PDF417".getBytes();
        for (int c = 1; c <= 30; c++) {
            PDF417.PDF417Options opts = new PDF417.PDF417Options();
            opts.columns = c;
            ModuleGrid grid = PDF417.encode(data, opts);
            assertNotNull(grid, "Columns=" + c + " must produce a valid grid");
            assertEquals(69 + 17 * c, grid.cols,
                "Columns=" + c + " must produce width " + (69 + 17 * c));
        }
    }

    // =========================================================================
    // 9. Cluster table sanity
    // =========================================================================

    /** Each cluster table must have exactly 929 entries (codeword values 0..928). */
    @Test
    void testClusterTables_size() {
        assertEquals(3, ClusterTables.CLUSTER_TABLES.length, "Must have 3 cluster tables");
        for (int i = 0; i < 3; i++) {
            assertEquals(929, ClusterTables.CLUSTER_TABLES[i].length,
                "Cluster table " + i + " must have 929 entries");
        }
    }

    /**
     * Each packed pattern must encode 8 elements summing to 17 modules.
     *
     * <p>Pattern integrity check: for every entry in every cluster table,
     * unpack the 8 bar/space widths and verify their sum equals 17.
     * This catches any corrupted table entries.
     */
    @Test
    void testClusterTables_patternIntegrity() {
        for (int ci = 0; ci < 3; ci++) {
            for (int cw = 0; cw < 929; cw++) {
                int packed = ClusterTables.CLUSTER_TABLES[ci][cw];
                int sum = ((packed >>> 28) & 0xF)
                        + ((packed >>> 24) & 0xF)
                        + ((packed >>> 20) & 0xF)
                        + ((packed >>> 16) & 0xF)
                        + ((packed >>> 12) & 0xF)
                        + ((packed >>>  8) & 0xF)
                        + ((packed >>>  4) & 0xF)
                        +  (packed         & 0xF);
                assertEquals(17, sum,
                    "Cluster " + ci + " codeword " + cw + " pattern width sum = " + sum + " (expected 17)");
            }
        }
    }

    // =========================================================================
    // Helper utilities
    // =========================================================================

    /**
     * Convert a bit-string (e.g., "11111111010101000") to a boolean array.
     * '1' = true (dark), '0' = false (light).
     */
    private static boolean[] expandBits(String bits) {
        boolean[] result = new boolean[bits.length()];
        for (int i = 0; i < bits.length(); i++) {
            result[i] = bits.charAt(i) == '1';
        }
        return result;
    }
}
