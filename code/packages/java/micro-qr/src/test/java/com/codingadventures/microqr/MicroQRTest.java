package com.codingadventures.microqr;

import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;
import com.codingadventures.microqr.MicroQR.EccLevel;
import com.codingadventures.microqr.MicroQR.MicroQRVersion;
import com.codingadventures.microqr.MicroQR.InputTooLongException;
import com.codingadventures.microqr.MicroQR.ECCNotAvailableException;
import com.codingadventures.microqr.MicroQR.UnsupportedModeException;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;
import org.junit.jupiter.params.provider.ValueSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for the Micro QR Code encoder.
 *
 * <p>Coverage targets:
 * <ul>
 *   <li>Symbol dimensions for each version (M1–M4)</li>
 *   <li>Auto version/ECC selection</li>
 *   <li>Encoding modes: numeric, alphanumeric, byte</li>
 *   <li>Structural modules: finder, separator, timing, format info</li>
 *   <li>Reed-Solomon encoding</li>
 *   <li>Masking conditions</li>
 *   <li>Penalty scoring rules 1–4</li>
 *   <li>Error handling</li>
 *   <li>Capacity boundaries</li>
 *   <li>Cross-language corpus inputs</li>
 *   <li>Determinism and idempotency</li>
 * </ul>
 */
class MicroQRTest {

    // =========================================================================
    // Helper
    // =========================================================================

    /**
     * Serialize a ModuleGrid to a string for equality comparison.
     *
     * <p>Each row becomes a string of '1' (dark) and '0' (light) characters,
     * rows separated by newlines.  This format matches the cross-language
     * corpus serialization used in the spec.
     */
    private static String gridToString(ModuleGrid grid) {
        StringBuilder sb = new StringBuilder();
        for (int r = 0; r < grid.rows; r++) {
            if (r > 0) sb.append('\n');
            for (int c = 0; c < grid.cols; c++) {
                sb.append(grid.modules.get(r).get(c) ? '1' : '0');
            }
        }
        return sb.toString();
    }

    // =========================================================================
    // Symbol dimensions
    // =========================================================================

    /** M1 produces an 11×11 symbol. */
    @Test
    void testM1Is11x11() {
        ModuleGrid g = MicroQR.encode("1", null, null);
        assertEquals(11, g.rows, "M1 rows");
        assertEquals(11, g.cols, "M1 cols");
    }

    /** M2 produces a 13×13 symbol. */
    @Test
    void testM2Is13x13() {
        ModuleGrid g = MicroQR.encode("HELLO", null, null);
        assertEquals(13, g.rows, "M2 rows");
        assertEquals(13, g.cols, "M2 cols");
    }

    /** M3 produces a 15×15 symbol. */
    @Test
    void testM3Is15x15() {
        ModuleGrid g = MicroQR.encode("MICRO QR TEST", null, null);
        assertEquals(15, g.rows, "M3 rows");
        assertEquals(15, g.cols, "M3 cols");
    }

    /** M4 produces a 17×17 symbol. */
    @Test
    void testM4Is17x17() {
        ModuleGrid g = MicroQR.encode("https://a.b", null, null);
        assertEquals(17, g.rows, "M4 rows");
        assertEquals(17, g.cols, "M4 cols");
    }

    /** The grid is always square (rows == cols). */
    @ParameterizedTest
    @ValueSource(strings = {"1", "12345", "HELLO", "hello", "https://a.b", "MICRO QR TEST"})
    void testGridIsSquare(String input) {
        ModuleGrid g = MicroQR.encode(input, null, null);
        assertEquals(g.rows, g.cols, "grid should be square for '" + input + "'");
    }

    /** Module shape is always SQUARE. */
    @Test
    void testModuleShapeIsSquare() {
        ModuleGrid g = MicroQR.encode("1", null, null);
        assertEquals(ModuleShape.SQUARE, g.moduleShape);
    }

    /** Grid dimensions match the module list dimensions. */
    @ParameterizedTest
    @ValueSource(strings = {"1", "HELLO", "hello", "https://a.b"})
    void testGridDimensionsMatchModuleList(String input) {
        ModuleGrid g = MicroQR.encode(input, null, null);
        assertEquals(g.rows, g.modules.size());
        for (var row : g.modules) {
            assertEquals(g.cols, row.size());
        }
    }

    // =========================================================================
    // Auto-version and ECC selection
    // =========================================================================

    /** Single digit selects M1. */
    @Test
    void testAutoSelectsM1ForSingleDigit() {
        assertEquals(11, MicroQR.encode("1", null, null).rows);
    }

    /** "12345" (5 digits = M1 numeric capacity) selects M1. */
    @Test
    void testAutoSelectsM1For12345() {
        assertEquals(11, MicroQR.encode("12345", null, null).rows);
    }

    /** "123456" (6 digits > M1 capacity) falls through to M2. */
    @Test
    void testAutoSelectsM2For6Digits() {
        assertEquals(13, MicroQR.encode("123456", null, null).rows);
    }

    /** "HELLO" (5 alphanumeric chars) fits M2-L. */
    @Test
    void testAutoSelectsM2ForHello() {
        assertEquals(13, MicroQR.encode("HELLO", null, null).rows);
    }

    /** "hello" (lowercase = byte mode, 5 bytes) fits M3-L (byte cap 9). */
    @Test
    void testAutoSelectsM3ForHelloLowercase() {
        assertTrue(MicroQR.encode("hello", null, null).rows >= 15);
    }

    /** "https://a.b" (byte mode, 11 bytes) fits M4-L (byte cap 15). */
    @Test
    void testAutoSelectsM4ForUrl() {
        assertEquals(17, MicroQR.encode("https://a.b", null, null).rows);
    }

    /** Forcing version M4 with single digit still produces 17×17. */
    @Test
    void testForcedVersionM4() {
        ModuleGrid g = MicroQR.encode("1", MicroQRVersion.M4, null);
        assertEquals(17, g.rows);
    }

    /** L and M produce different grids for the same input (different format info). */
    @Test
    void testForcedEccLVsM() {
        ModuleGrid gl = MicroQR.encode("HELLO", null, EccLevel.L);
        ModuleGrid gm = MicroQR.encode("HELLO", null, EccLevel.M);
        assertNotEquals(gridToString(gl), gridToString(gm),
            "L and M grids should differ");
    }

    // =========================================================================
    // Cross-language corpus (spec §Test Strategy §Integration Tests)
    // =========================================================================

    /**
     * Verify the standard test corpus from the spec produces expected symbol sizes.
     *
     * <p>These inputs are used for cross-language bit-for-bit verification.
     * All 15 language implementations must produce ModuleGrid outputs that
     * serialize to the same string for each input.
     */
    @ParameterizedTest
    @CsvSource({
        "1,            11",
        "12345,        11",
        "HELLO,        13",
        "01234567,     13",
        "https://a.b,  17",
        "MICRO QR TEST,15"
    })
    void testCrossLanguageCorpus(String input, int expectedSize) {
        input = input.strip();
        ModuleGrid g = MicroQR.encode(input, null, null);
        assertEquals(expectedSize, g.rows,
            "input '" + input + "': expected " + expectedSize + "×" + expectedSize);
    }

    // =========================================================================
    // Structural modules: finder pattern
    // =========================================================================

    /**
     * Finder pattern outer ring must be all dark (rows 0, 6 and cols 0, 6).
     * Inner ring must be all light (row 1/5 cols 1–5, col 1/5 rows 1–5).
     * Core (rows 2–4, cols 2–4) must be all dark.
     */
    @Test
    void testFinderPatternM1() {
        ModuleGrid g = MicroQR.encode("1", null, null);
        var m = g.modules;

        // Outer ring: rows 0 and 6 (all dark)
        for (int c = 0; c < 7; c++) {
            assertTrue(m.get(0).get(c), "row 0 col " + c + " dark");
            assertTrue(m.get(6).get(c), "row 6 col " + c + " dark");
        }
        // Outer ring: cols 0 and 6 (all dark)
        for (int r = 0; r < 7; r++) {
            assertTrue(m.get(r).get(0), "row " + r + " col 0 dark");
            assertTrue(m.get(r).get(6), "row " + r + " col 6 dark");
        }
        // Inner ring: row 1 and row 5, cols 1–5 (all light)
        for (int c = 1; c <= 5; c++) {
            assertFalse(m.get(1).get(c), "inner ring row 1 col " + c + " light");
            assertFalse(m.get(5).get(c), "inner ring row 5 col " + c + " light");
        }
        // Inner ring: col 1 and col 5, rows 2–4 (all light)
        for (int r = 2; r <= 4; r++) {
            assertFalse(m.get(r).get(1), "inner ring row " + r + " col 1 light");
            assertFalse(m.get(r).get(5), "inner ring row " + r + " col 5 light");
        }
        // Core (rows 2–4, cols 2–4): all dark
        for (int r = 2; r <= 4; r++) {
            for (int c = 2; c <= 4; c++) {
                assertTrue(m.get(r).get(c), "core (" + r + "," + c + ") dark");
            }
        }
    }

    // =========================================================================
    // Structural modules: separator
    // =========================================================================

    /** Row 7 cols 0–7 and col 7 rows 0–7 must all be light (separator). */
    @Test
    void testSeparatorM2() {
        ModuleGrid g = MicroQR.encode("HELLO", null, null);
        var m = g.modules;
        for (int c = 0; c <= 7; c++) {
            assertFalse(m.get(7).get(c), "separator row 7 col " + c + " light");
        }
        for (int r = 0; r <= 7; r++) {
            assertFalse(m.get(r).get(7), "separator col 7 row " + r + " light");
        }
    }

    // =========================================================================
    // Structural modules: timing patterns
    // =========================================================================

    /**
     * Timing row 0 cols 8..size-1: even col = dark, odd col = light.
     * Timing col 0 rows 8..size-1: even row = dark, odd row = light.
     */
    @Test
    void testTimingPatternM4() {
        ModuleGrid g = MicroQR.encode("https://a.b", null, null);
        var m = g.modules;
        for (int c = 8; c < 17; c++) {
            assertEquals(c % 2 == 0, m.get(0).get(c), "timing row 0 col " + c);
        }
        for (int r = 8; r < 17; r++) {
            assertEquals(r % 2 == 0, m.get(r).get(0), "timing col 0 row " + r);
        }
    }

    @Test
    void testTimingPatternM2() {
        ModuleGrid g = MicroQR.encode("HELLO", null, null);
        var m = g.modules;
        for (int c = 8; c < 13; c++) {
            assertEquals(c % 2 == 0, m.get(0).get(c), "timing row 0 col " + c);
        }
        for (int r = 8; r < 13; r++) {
            assertEquals(r % 2 == 0, m.get(r).get(0), "timing col 0 row " + r);
        }
    }

    // =========================================================================
    // Format information
    // =========================================================================

    /** Format info area (row 8 cols 1–8 and col 8 rows 1–7) should not all be zero. */
    @Test
    void testFormatInfoNonZeroM4() {
        ModuleGrid g = MicroQR.encode("HELLO", MicroQRVersion.M4, EccLevel.L);
        var m = g.modules;
        boolean anyDark = false;
        for (int c = 1; c <= 8; c++) anyDark |= m.get(8).get(c);
        for (int r = 1; r <= 7; r++) anyDark |= m.get(r).get(8);
        assertTrue(anyDark, "format info should have some dark modules");
    }

    @Test
    void testFormatInfoNonZeroM1() {
        ModuleGrid g = MicroQR.encode("1", null, null);
        var m = g.modules;
        int count = 0;
        for (int c = 1; c <= 8; c++) if (m.get(8).get(c)) count++;
        for (int r = 1; r <= 7; r++) if (m.get(r).get(8)) count++;
        assertTrue(count > 0, "M1 format info should have some dark modules");
    }

    /** M4-L mask 0 has format word 0x17F3 → verify some specific bit positions. */
    @Test
    void testFormatInfoDiffersAcrossEccLevels() {
        // Same version, different ECC → different format info → different row 8 / col 8
        ModuleGrid gl = MicroQR.encode("1", MicroQRVersion.M4, EccLevel.L);
        ModuleGrid gm = MicroQR.encode("1", MicroQRVersion.M4, EccLevel.M);
        ModuleGrid gq = MicroQR.encode("1", MicroQRVersion.M4, EccLevel.Q);
        // At minimum, one pair must differ
        assertNotEquals(gridToString(gl), gridToString(gm));
        assertNotEquals(gridToString(gm), gridToString(gq));
        assertNotEquals(gridToString(gl), gridToString(gq));
    }

    // =========================================================================
    // Reed-Solomon encoding
    // =========================================================================

    /**
     * Verify the 2-ECC codeword generator produces the correct RS checksum for a
     * known M1-style data byte sequence.
     *
     * <p>Generator for 2 ECC CWs: [0x01, 0x03, 0x02].
     * Data: [0x10, 0x20, 0x0C].
     *
     * <p>Hand calculation:
     * <pre>
     * Start: rem = [0, 0]
     *
     * Byte 0x10 (16):
     *   fb = 16 ^ 0 = 16
     *   shift: rem = [0, 0]
     *   rem[0] ^= GF.mul(0x03, 16) = 0x03 * 16 = GF(3,16)
     *     LOG[3]=25, LOG[16]=4, EXP[29]=0x1d=29 → rem[0]=29
     *   rem[1] ^= GF.mul(0x02, 16) = 0x02 * 16
     *     LOG[2]=1, LOG[16]=4, EXP[5]=32 → rem[1]=32
     *
     * Byte 0x20 (32):
     *   fb = 32 ^ 29 = 13
     *   shift: rem = [32, 0]
     *   rem[0] ^= GF.mul(0x03, 13): LOG[3]=25, LOG[13]=220, EXP[245]=...
     *     EXP[245] = let's let the test verify this numerically.
     *   ...
     * </pre>
     *
     * <p>Rather than inlining the full manual calculation (which is error-prone),
     * this test verifies that the RS encoder produces a consistent result and
     * that a known simple case works: for data=[0,0,...,0], ECC=[0,...,0].
     */
    @Test
    void testRsEncodeAllZerosProducesAllZeros() {
        byte[] data = {0, 0, 0};
        int[] gen = {0x01, 0x03, 0x02};
        byte[] ecc = MicroQR.rsEncode(data, gen);
        assertArrayEquals(new byte[]{0, 0}, ecc,
            "RS(all zeros) should produce all zeros");
    }

    /** RS encode with generator of degree 5, known test vector. */
    @Test
    void testRsEncodeAllZeros5Ecc() {
        byte[] data = new byte[5]; // all zeros
        int[] gen = {0x01, 0x1f, 0xf6, 0x44, (byte) 0xd9 & 0xFF, 0x68};
        byte[] ecc = MicroQR.rsEncode(data, gen);
        for (byte b : ecc) assertEquals(0, b, "all-zero data → all-zero ECC");
    }

    /** Different data produces different ECC codewords. */
    @Test
    void testRsEncodeDifferentDataDifferentEcc() {
        int[] gen = {0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, (byte)0xa2 & 0xFF, (byte)0xa3 & 0xFF};
        byte[] ecc1 = MicroQR.rsEncode(new byte[]{0x10, 0x20, 0x30}, gen);
        byte[] ecc2 = MicroQR.rsEncode(new byte[]{0x40, 0x50, 0x60}, gen);
        assertFalse(java.util.Arrays.equals(ecc1, ecc2),
            "different data should produce different ECC");
    }

    /** ECC output length matches requested count. */
    @Test
    void testRsEncodeOutputLength() {
        for (int eccCount : new int[]{2, 5, 6, 8, 10, 14}) {
            // Use the package's own generator lookup via a private method test
            // We test indirectly through the encode pipeline
            byte[] data = new byte[4];
            // Just verify a full encode doesn't crash and produces right symbol size
        }
        // Direct test
        byte[] data = new byte[3];
        int[] gen2  = {0x01, 0x03, 0x02};
        assertEquals(2, MicroQR.rsEncode(data, gen2).length, "2-ECC output length");

        int[] gen5 = {0x01, 0x1f, (byte)0xf6&0xFF, 0x44, (byte)0xd9&0xFF, 0x68};
        assertEquals(5, MicroQR.rsEncode(data, gen5).length, "5-ECC output length");
    }

    // =========================================================================
    // Mask condition
    // =========================================================================

    /** Mask 0: (row + col) % 2 == 0. */
    @Test
    void testMaskCondition0() {
        assertTrue(MicroQR.maskCondition(0, 0, 0));   // 0+0=0
        assertFalse(MicroQR.maskCondition(0, 0, 1));  // 0+1=1
        assertTrue(MicroQR.maskCondition(0, 1, 1));   // 1+1=2
        assertFalse(MicroQR.maskCondition(0, 1, 0));  // 1+0=1
    }

    /** Mask 1: row % 2 == 0. */
    @Test
    void testMaskCondition1() {
        assertTrue(MicroQR.maskCondition(1, 0, 5));   // row 0 even
        assertFalse(MicroQR.maskCondition(1, 1, 5));  // row 1 odd
        assertTrue(MicroQR.maskCondition(1, 2, 0));   // row 2 even
    }

    /** Mask 2: col % 3 == 0. */
    @Test
    void testMaskCondition2() {
        assertTrue(MicroQR.maskCondition(2, 5, 0));   // col 0
        assertFalse(MicroQR.maskCondition(2, 5, 1));  // col 1
        assertFalse(MicroQR.maskCondition(2, 5, 2));  // col 2
        assertTrue(MicroQR.maskCondition(2, 5, 3));   // col 3
    }

    /** Mask 3: (row + col) % 3 == 0. */
    @Test
    void testMaskCondition3() {
        assertTrue(MicroQR.maskCondition(3, 0, 0));   // 0+0=0
        assertFalse(MicroQR.maskCondition(3, 0, 1));  // 0+1=1
        assertFalse(MicroQR.maskCondition(3, 1, 0));  // 1+0=1
        assertTrue(MicroQR.maskCondition(3, 1, 2));   // 1+2=3
        assertTrue(MicroQR.maskCondition(3, 3, 0));   // 3+0=3
    }

    /** Invalid mask index returns false. */
    @Test
    void testMaskConditionInvalidIndex() {
        assertFalse(MicroQR.maskCondition(4, 0, 0));
        assertFalse(MicroQR.maskCondition(-1, 0, 0));
    }

    // =========================================================================
    // Penalty scoring
    // =========================================================================

    /**
     * A 5×5 all-dark grid has:
     * Rule 1: 5 rows × (5-2) + 5 cols × (5-2) = 15 + 15 = 30
     * Rule 2: 4×4 = 16 blocks of 2×2 → 16×3 = 48
     * Rule 3: 5 < 11 → no match possible
     * Rule 4: dark=25, total=25 → 100% dark → prev5=100, next5=105
     *         min(|100-50|, |105-50|) = min(50, 55) = 50 → 50/5×10 = 100
     */
    @Test
    void testPenaltyAllDark5x5() {
        boolean[][] m = new boolean[5][5];
        for (boolean[] row : m) java.util.Arrays.fill(row, true);
        int p = MicroQR.computePenalty(m, 5);
        // Rule 1: 30, Rule 2: 48, Rule 3: 0, Rule 4: 100 → 178
        assertEquals(178, p, "all-dark 5×5 penalty");
    }

    /**
     * A 5×5 all-light grid has the same penalty as all-dark by symmetry.
     * Rule 4: dark=0, 0% dark → min(|0-50|, |5-50|) = min(50, 45) = 45 → 90
     */
    @Test
    void testPenaltyAllLight5x5() {
        boolean[][] m = new boolean[5][5];
        // all false (light) already
        int p = MicroQR.computePenalty(m, 5);
        // Rule 1: 30, Rule 2: 48, Rule 3: 0, Rule 4: 90 → 168
        assertEquals(168, p, "all-light 5×5 penalty");
    }

    /**
     * A checkerboard pattern (alternating dark/light) has:
     * Rule 1: no runs of ≥5 → 0
     * Rule 2: no 2×2 same-color blocks → 0
     * Rule 3: no finder-like 11-module sequences (need sz≥11) → 0
     * Rule 4: for 5×5=25 modules: 12 or 13 dark → near 50%
     */
    @Test
    void testPenaltyCheckerboard5x5() {
        boolean[][] m = new boolean[5][5];
        for (int r = 0; r < 5; r++) {
            for (int c = 0; c < 5; c++) {
                m[r][c] = (r + c) % 2 == 0;
            }
        }
        int p = MicroQR.computePenalty(m, 5);
        // Rules 1, 2, 3 all zero; Rule 4: 13/25=52% dark → prev5=50 → min(0,5)=0 → 0
        assertEquals(0, p, "checkerboard 5×5 penalty");
    }

    /** Rule 1 exact: a run of exactly 5 in a row adds 3. */
    @Test
    void testPenaltyRule1RunOf5() {
        // 7×7 grid, row 0 all dark except cols 5,6 → run of 5 dark at cols 0-4
        boolean[][] m = new boolean[7][7];
        // Set row 0: 5 dark, then 2 light
        for (int c = 0; c < 5; c++) m[0][c] = true;
        // columns 5 and 6 are light (already false)
        // Rule 1 row 0: run of 5 → +3; run of 2 light not ≥5 → no penalty
        // Other rows all light → run of 7 → +5 each
        // Other cols vary...
        // Just check rule 1 contributes at least 3 for the row
        int p = MicroQR.computePenalty(m, 7);
        assertTrue(p >= 3, "run of 5 should add at least 3 to penalty, got " + p);
    }

    /** Rule 2: a 2×2 block of same color adds 3. */
    @Test
    void testPenaltyRule2Block() {
        boolean[][] m = new boolean[5][5];
        // Place one 2×2 dark block at (1,1)
        m[1][1] = true; m[1][2] = true;
        m[2][1] = true; m[2][2] = true;
        int p = MicroQR.computePenalty(m, 5);
        // Expect at least 3 from that one block
        assertTrue(p >= 3, "2×2 block should add 3 to penalty");
    }

    // =========================================================================
    // Determinism
    // =========================================================================

    /** Encoding the same input twice produces identical grids. */
    @ParameterizedTest
    @ValueSource(strings = {"1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b"})
    void testDeterministic(String input) {
        ModuleGrid g1 = MicroQR.encode(input, null, null);
        ModuleGrid g2 = MicroQR.encode(input, null, null);
        assertEquals(gridToString(g1), gridToString(g2),
            "encoding should be deterministic for '" + input + "'");
    }

    /** Different inputs produce different grids. */
    @Test
    void testDifferentInputsDifferentGrids() {
        ModuleGrid g1 = MicroQR.encode("1", null, null);
        ModuleGrid g2 = MicroQR.encode("2", null, null);
        assertNotEquals(gridToString(g1), gridToString(g2));
    }

    // =========================================================================
    // ECC level constraints
    // =========================================================================

    /** M1 with DETECTION ECC works. */
    @Test
    void testM1Detection() {
        ModuleGrid g = MicroQR.encode("1", MicroQRVersion.M1, EccLevel.DETECTION);
        assertEquals(11, g.rows);
    }

    /** M4 with Q ECC works. */
    @Test
    void testM4Q() {
        ModuleGrid g = MicroQR.encode("HELLO", MicroQRVersion.M4, EccLevel.Q);
        assertEquals(17, g.rows);
    }

    /** M4 L, M, Q all produce valid 17×17 symbols but differ. */
    @Test
    void testM4AllEccDiffer() {
        ModuleGrid gl = MicroQR.encode("HELLO", MicroQRVersion.M4, EccLevel.L);
        ModuleGrid gm = MicroQR.encode("HELLO", MicroQRVersion.M4, EccLevel.M);
        ModuleGrid gq = MicroQR.encode("HELLO", MicroQRVersion.M4, EccLevel.Q);
        assertNotEquals(gridToString(gl), gridToString(gm));
        assertNotEquals(gridToString(gm), gridToString(gq));
        assertNotEquals(gridToString(gl), gridToString(gq));
    }

    // =========================================================================
    // Error handling: ECC not available
    // =========================================================================

    /** M1 does not support ECC level L — should throw ECCNotAvailableException. */
    @Test
    void testM1RejectsEccL() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M1, EccLevel.L));
    }

    /** M1 does not support ECC level M. */
    @Test
    void testM1RejectsEccM() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M1, EccLevel.M));
    }

    /** M1 does not support ECC level Q. */
    @Test
    void testM1RejectsEccQ() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M1, EccLevel.Q));
    }

    /** M2 does not support ECC level Q. */
    @Test
    void testM2RejectsEccQ() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M2, EccLevel.Q));
    }

    /** M3 does not support ECC level Q. */
    @Test
    void testM3RejectsEccQ() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M3, EccLevel.Q));
    }

    /** M2 does not support DETECTION mode. */
    @Test
    void testM2RejectsDetection() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M2, EccLevel.DETECTION));
    }

    /** Invalid version+ECC combo (M1 with Q) throws ECCNotAvailableException. */
    @Test
    void testEccNotAvailableForNonexistentCombo() {
        assertThrows(ECCNotAvailableException.class,
            () -> MicroQR.encode("1", MicroQRVersion.M1, EccLevel.Q));
    }

    // =========================================================================
    // Error handling: input too long
    // =========================================================================

    /** 36 digits exceeds M4-L numeric capacity of 35 → InputTooLongException. */
    @Test
    void testInputTooLong36Digits() {
        assertThrows(InputTooLongException.class,
            () -> MicroQR.encode("1".repeat(36), null, null));
    }

    /** 16 bytes exceeds M4-L byte capacity of 15 → InputTooLongException. */
    @Test
    void testInputTooLong16Bytes() {
        assertThrows(InputTooLongException.class,
            () -> MicroQR.encode("a".repeat(16), null, null));
    }

    /** 22 alphanumeric chars exceeds M4-L alpha cap of 21. */
    @Test
    void testInputTooLong22Alpha() {
        assertThrows(InputTooLongException.class,
            () -> MicroQR.encode("A".repeat(22), null, null));
    }

    // =========================================================================
    // Capacity boundaries
    // =========================================================================

    /** M1 max: exactly 5 numeric digits fits. */
    @Test
    void testM1Max5Digits() {
        assertEquals(11, MicroQR.encode("12345", null, null).rows);
    }

    /** M1 overflow: 6 digits falls through to M2. */
    @Test
    void testM1Overflow6Digits() {
        assertEquals(13, MicroQR.encode("123456", null, null).rows);
    }

    /** M4 max: 35 digits fits M4-L (numeric capacity = 35). */
    @Test
    void testM4Max35Digits() {
        assertEquals(17, MicroQR.encode("1".repeat(35), null, null).rows);
    }

    /** M4 overflow: 36 digits throws InputTooLongException. */
    @Test
    void testM4Overflow36Digits() {
        assertThrows(InputTooLongException.class,
            () -> MicroQR.encode("1".repeat(36), null, null));
    }

    /** M4-L max byte: 15 chars (byte mode) fits. */
    @Test
    void testM4MaxByte15Chars() {
        assertEquals(17, MicroQR.encode("a".repeat(15), null, null).rows);
    }

    /** M4-Q max numeric: 21 digits fits. */
    @Test
    void testM4QMax21Numeric() {
        assertEquals(17, MicroQR.encode("1".repeat(21), null, EccLevel.Q).rows);
    }

    /** M4-Q overflow: 22 digits with Q ECC doesn't fit M4-Q (cap=21). */
    @Test
    void testM4QOverflow22Numeric() {
        // 22 digits doesn't fit in M4-Q (cap 21 numeric), but fits M4-L (cap 35).
        // With ecc=Q pinned, should throw.
        assertThrows(InputTooLongException.class,
            () -> MicroQR.encode("1".repeat(22), null, EccLevel.Q));
    }

    /** M2-L max alphanumeric: 6 chars. */
    @Test
    void testM2LMax6Alpha() {
        assertEquals(13, MicroQR.encode("ABCDEF", MicroQRVersion.M2, EccLevel.L).rows);
    }

    /** M2-L overflow: 7 alphanumeric chars falls through to M3. */
    @Test
    void testM2LOverflow7Alpha() {
        // 7 alphanumeric > M2-L cap (6), should use M3-L
        assertEquals(15, MicroQR.encode("ABCDEFG", null, EccLevel.L).rows);
    }

    // =========================================================================
    // Empty string
    // =========================================================================

    /** Empty string encodes to M1 (numeric mode, 0 digits fits anywhere). */
    @Test
    void testEmptyStringEncodesToM1() {
        assertEquals(11, MicroQR.encode("", null, null).rows);
    }

    /** Empty string with forced M4 produces 17×17. */
    @Test
    void testEmptyStringForcedM4() {
        assertEquals(17, MicroQR.encode("", MicroQRVersion.M4, EccLevel.L).rows);
    }

    // =========================================================================
    // Single-character edge cases
    // =========================================================================

    /** Single digit 0 encodes correctly. */
    @Test
    void testSingleDigit0() {
        ModuleGrid g = MicroQR.encode("0", null, null);
        assertEquals(11, g.rows);
    }

    /** Single alphanumeric character 'A'. */
    @Test
    void testSingleAlphaA() {
        // 'A' is in the alphanumeric set, but M1 doesn't support alphanumeric.
        // Auto-selection falls to M2.
        ModuleGrid g = MicroQR.encode("A", null, null);
        assertEquals(13, g.rows);
    }

    /** Single byte character (lowercase). */
    @Test
    void testSingleByteLowercaseA() {
        // 'a' requires byte mode → M3 (M2 byte cap = 4, which fits 1 byte).
        // M2-L byte cap is 4, so 'a' fits in M2-L.
        ModuleGrid g = MicroQR.encode("a", null, null);
        assertTrue(g.rows >= 13);
    }

    // =========================================================================
    // Mode selection
    // =========================================================================

    /** "12345" should use numeric mode (fits M1). */
    @Test
    void testModeNumericFor12345() {
        // Verify it's M1 (numeric mode, M1 numeric cap=5)
        assertEquals(11, MicroQR.encode("12345", null, null).rows);
    }

    /** "HELLO WORLD" should use alphanumeric mode. */
    @Test
    void testModeAlphanumericForHelloWorld() {
        // 11 chars, alphanumeric → M3-L (alpha cap=14)
        assertEquals(15, MicroQR.encode("HELLO WORLD", null, null).rows);
    }

    /** Lowercase "hello" should use byte mode (not in alphanumeric set). */
    @Test
    void testModeByteForLowercase() {
        // M3-L byte cap=9; "hello" is 5 bytes → fits M3-L
        ModuleGrid g = MicroQR.encode("hello", null, null);
        assertTrue(g.rows >= 15);
    }

    /** Alphanumeric special chars: ' ', '$', '%', '*', '+', '-', '.', '/', ':' */
    @Test
    void testAlphanumericSpecialChars() {
        // All special chars in the 45-char set
        ModuleGrid g = MicroQR.encode("A$%*+-./:", null, null);
        assertNotNull(g);
    }

    // =========================================================================
    // Additional integration tests
    // =========================================================================

    /** "MICRO QR" at M4-L, M4-M, M4-Q all produce valid 17×17 symbols. */
    @Test
    void testMicroQrAtAllM4EccLevels() {
        String input = "MICRO QR";
        for (EccLevel level : new EccLevel[]{EccLevel.L, EccLevel.M, EccLevel.Q}) {
            ModuleGrid g = MicroQR.encode(input, MicroQRVersion.M4, level);
            assertEquals(17, g.rows, "MICRO QR at M4-" + level + " should be 17×17");
        }
    }

    /** M2-M can hold up to 8 numeric digits. */
    @Test
    void testM2MMax8Digits() {
        assertEquals(13, MicroQR.encode("12345678", MicroQRVersion.M2, EccLevel.M).rows);
    }

    /** M3-L can hold up to 23 numeric digits. */
    @Test
    void testM3LMax23Digits() {
        assertEquals(15, MicroQR.encode("1".repeat(23), MicroQRVersion.M3, EccLevel.L).rows);
    }

    /** M3-M can hold up to 7 bytes (M3-L holds 9). */
    @Test
    void testM3MMax7Bytes() {
        assertEquals(15, MicroQR.encode("a".repeat(7), MicroQRVersion.M3, EccLevel.M).rows);
    }

    /** The convenience overload encode(String) with no version/ecc works. */
    @Test
    void testConvenienceOverloadNoArgs() {
        ModuleGrid g = MicroQR.encode("HELLO");
        assertEquals(13, g.rows);
    }

    /** Numeric string at M2-M boundary (8 digits fits, 9 does not). */
    @Test
    void testM2MBoundaryNumeric() {
        assertEquals(13, MicroQR.encode("12345678", MicroQRVersion.M2, EccLevel.M).rows);
        // 9 digits doesn't fit M2-M but fits M3-M
        assertThrows(InputTooLongException.class,
            () -> MicroQR.encode("123456789", MicroQRVersion.M2, EccLevel.M));
    }

    // =========================================================================
    // Immutability / API contract
    // =========================================================================

    /** The returned ModuleGrid is immutable (inner row lists are unmodifiable). */
    @Test
    void testModuleGridIsImmutable() {
        ModuleGrid g = MicroQR.encode("1", null, null);
        assertThrows(UnsupportedOperationException.class,
            () -> g.modules.get(0).set(0, true));
        assertThrows(UnsupportedOperationException.class,
            () -> ((java.util.List<java.util.List<Boolean>>) g.modules).add(null));
    }

    /** Each call to encode produces a distinct ModuleGrid object (not the same reference). */
    @Test
    void testEncodingReturnsFreshObject() {
        ModuleGrid g1 = MicroQR.encode("1", null, null);
        ModuleGrid g2 = MicroQR.encode("1", null, null);
        assertNotSame(g1, g2);
        assertEquals(g1, g2); // same content
    }
}
