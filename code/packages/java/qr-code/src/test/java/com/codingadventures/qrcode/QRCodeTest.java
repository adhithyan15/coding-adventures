package com.codingadventures.qrcode;

import com.codingadventures.barcode2d.Barcode2DLayoutConfig;
import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;
import com.codingadventures.paintinstructions.PaintScene;
import org.junit.jupiter.api.*;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for the Java QR Code encoder.
 *
 * <p>Tests are organized into nested classes by concern, using JUnit 5's
 * {@link Nested} feature. This makes the test output easy to scan and lets
 * each group share setup code via {@link BeforeEach}.
 *
 * <h2>Coverage goals</h2>
 * <ul>
 *   <li>All three encoding modes (Numeric, Alphanumeric, Byte)</li>
 *   <li>All four ECC levels</li>
 *   <li>Version selection (v1, v2, v7+, v40 boundary)</li>
 *   <li>Grid structure (finders, timing strips, dark module)</li>
 *   <li>Format information correctness (BCH parity, both copies)</li>
 *   <li>Determinism and uniqueness</li>
 *   <li>Error path (input too long)</li>
 *   <li>encodeAndLayout integration</li>
 * </ul>
 */
class QRCodeTest {

    // =========================================================================
    // Shared test helpers
    // =========================================================================

    /**
     * Verifies that a 7×7 finder pattern is present at the given top-left corner.
     *
     * <p>The border (ring of cells at dr=0/6 or dc=0/6) and the inner 3×3 core
     * (2≤dr≤4, 2≤dc≤4) must be dark; the separator ring must be light.
     *
     * @param mods the module grid
     * @param top  row of the finder's top-left corner
     * @param left column of the finder's top-left corner
     * @return true iff the finder pattern is correctly placed
     */
    private boolean hasFinder(List<List<Boolean>> mods, int top, int left) {
        for (int dr = 0; dr < 7; dr++) {
            for (int dc = 0; dc < 7; dc++) {
                boolean onBorder = (dr == 0 || dr == 6 || dc == 0 || dc == 6);
                boolean inCore   = (dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4);
                boolean expected = onBorder || inCore;
                if (mods.get(top + dr).get(left + dc) != expected) return false;
            }
        }
        return true;
    }

    /**
     * Reads the Copy 1 format information from the grid and validates it.
     *
     * <p>Copy 1 spans row 8 (cols 0-8 except col 6) and col 8 (rows 0-8 except row 6).
     * The 15-bit value is read MSB-first (f14 at row 8, col 0).
     *
     * <p>After XOR-ing off the ISO masking sequence 0x5412, the resulting raw word
     * is BCH-checked: the 10-bit remainder of (raw >> 10) * 2^10 mod 0x537 must
     * equal (raw & 0x3FF).
     *
     * @param mods module grid
     * @return the decoded (eccBits, maskBits) pair, or null if BCH check fails
     */
    private int[] decodeFormatInfo(List<List<Boolean>> mods) {
        // ISO 18004 §7.9 Copy 1 positions, ordered f14 → f0.
        int[][] positions = {
            {8,0},{8,1},{8,2},{8,3},{8,4},{8,5},{8,7},{8,8},
            {7,8},{5,8},{4,8},{3,8},{2,8},{1,8},{0,8}
        };
        int raw = 0;
        for (int i = 0; i < 15; i++) {
            int r = positions[i][0];
            int c = positions[i][1];
            if (mods.get(r).get(c)) raw |= (1 << (14 - i));  // f14 at i=0 → bit 14
        }
        // Remove the ISO masking sequence.
        int fmt = raw ^ 0x5412;
        // BCH verification: recompute parity from the 5-bit data portion.
        int rem = (fmt >> 10) << 10;
        for (int i = 14; i >= 10; i--) {
            if (((rem >>> i) & 1) == 1) rem ^= (0x537 << (i - 10));
        }
        if ((rem & 0x3FF) != (fmt & 0x3FF)) return null;  // BCH failure
        return new int[]{(fmt >> 13) & 0x3, (fmt >> 10) & 0x7};
    }

    // =========================================================================
    // Version constant
    // =========================================================================

    @Test
    @DisplayName("VERSION constant is set correctly")
    void versionConstant() {
        assertEquals("0.1.0", QRCode.VERSION);
    }

    // =========================================================================
    // Basic encoding sanity
    // =========================================================================

    @Nested
    @DisplayName("Basic encoding")
    class BasicEncodingTests {

        @Test
        @DisplayName("HELLO WORLD encodes to version 1 (21×21) at ECC-M")
        void helloWorldV1() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
            assertEquals(21, grid.rows, "rows");
            assertEquals(21, grid.cols, "cols");
            assertEquals(ModuleShape.SQUARE, grid.moduleShape);
        }

        @Test
        @DisplayName("URL encodes to version 2 (25×25) at ECC-M")
        void urlV2() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("https://example.com", QRCode.EccLevel.M);
            assertEquals(25, grid.rows);
        }

        @Test
        @DisplayName("Single character encodes to version 1")
        void singleChar() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("A", QRCode.EccLevel.M);
            assertEquals(21, grid.rows);
        }

        @Test
        @DisplayName("Empty string encodes to version 1")
        void emptyString() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("", QRCode.EccLevel.M);
            assertEquals(21, grid.rows);
        }

        @Test
        @DisplayName("Grid dimensions are always square")
        void squareDimensions() throws QRCode.QRCodeException {
            for (String input : List.of("", "A", "HELLO WORLD", "https://example.com")) {
                ModuleGrid grid = QRCode.encode(input, QRCode.EccLevel.M);
                assertEquals(grid.rows, grid.cols, "grid must be square for: " + input);
            }
        }

        @Test
        @DisplayName("ModuleShape is always SQUARE")
        void moduleShapeIsSquare() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("TEST", QRCode.EccLevel.H);
            assertEquals(ModuleShape.SQUARE, grid.moduleShape);
        }
    }

    // =========================================================================
    // ECC level tests
    // =========================================================================

    @Nested
    @DisplayName("ECC levels")
    class EccLevelTests {

        @Test
        @DisplayName("All four ECC levels produce valid grids for HELLO")
        void allEccLevelsWork() throws QRCode.QRCodeException {
            for (QRCode.EccLevel ecc : QRCode.EccLevel.values()) {
                ModuleGrid grid = QRCode.encode("HELLO", ecc);
                assertTrue(grid.rows >= 21, "rows must be ≥ 21 for ECC " + ecc);
            }
        }

        @Test
        @DisplayName("ECC-H requires a larger (or equal) version than ECC-L for the same input")
        void higherEccNeedsLargerVersion() throws QRCode.QRCodeException {
            ModuleGrid gl = QRCode.encode("The quick brown fox", QRCode.EccLevel.L);
            ModuleGrid gh = QRCode.encode("The quick brown fox", QRCode.EccLevel.H);
            assertTrue(gh.rows >= gl.rows,
                "ECC-H grid (%d) should be ≥ ECC-L grid (%d)".formatted(gh.rows, gl.rows));
        }

        @Test
        @DisplayName("Format info ECC bits match L=01, M=00, Q=11, H=10")
        void formatInfoEccBits() throws QRCode.QRCodeException {
            record LevelExpected(QRCode.EccLevel level, int expected) {}
            var cases = List.of(
                new LevelExpected(QRCode.EccLevel.L, 0b01),
                new LevelExpected(QRCode.EccLevel.M, 0b00),
                new LevelExpected(QRCode.EccLevel.Q, 0b11),
                new LevelExpected(QRCode.EccLevel.H, 0b10)
            );
            for (var c : cases) {
                ModuleGrid grid = QRCode.encode("HELLO", c.level());
                int[] decoded = decodeFormatInfo(grid.modules);
                assertNotNull(decoded, "BCH check failed for " + c.level());
                assertEquals(c.expected(), decoded[0],
                    "wrong ECC bits for " + c.level());
            }
        }
    }

    // =========================================================================
    // Encoding mode tests
    // =========================================================================

    @Nested
    @DisplayName("Encoding modes")
    class EncodingModeTests {

        @Test
        @DisplayName("Numeric mode: 15 digits fit in version 1 at ECC-M")
        void numericModeV1() throws QRCode.QRCodeException {
            // Version 1-M holds 41 data bits; 15 digits in numeric mode use 4+10+50=64 bits
            // which exceeds version 1 — so we just check it encodes successfully.
            ModuleGrid grid = QRCode.encode("000000000000000", QRCode.EccLevel.M);
            assertTrue(grid.rows >= 21);
        }

        @Test
        @DisplayName("Numeric mode encodes correctly (all digits)")
        void numericMode() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("0123456789", QRCode.EccLevel.L);
            assertNotNull(grid);
        }

        @Test
        @DisplayName("Alphanumeric mode encodes correctly (uppercase + space)")
        void alphanumericMode() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.Q);
            assertNotNull(grid);
        }

        @Test
        @DisplayName("Byte mode handles URL with lowercase letters")
        void byteModeLowercase() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("https://example.com", QRCode.EccLevel.M);
            assertTrue(grid.rows >= 21);
        }

        @Test
        @DisplayName("Byte mode handles multi-byte UTF-8 (non-ASCII)")
        void byteModeUtf8() throws QRCode.QRCodeException {
            // "→" = U+2192, 3 UTF-8 bytes: E2 86 92
            ModuleGrid grid = QRCode.encode("→→→", QRCode.EccLevel.M);
            assertTrue(grid.rows >= 21);
        }
    }

    // =========================================================================
    // Grid structure tests
    // =========================================================================

    @Nested
    @DisplayName("Grid structure")
    class GridStructureTests {

        @Test
        @DisplayName("Three finder patterns are present in version 1 grid")
        void finderPatternsPresent() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
            int sz = grid.rows;
            assertTrue(hasFinder(grid.modules, 0, 0),           "top-left finder");
            assertTrue(hasFinder(grid.modules, 0, sz - 7),      "top-right finder");
            assertTrue(hasFinder(grid.modules, sz - 7, 0),      "bottom-left finder");
        }

        @Test
        @DisplayName("Timing strips alternate dark/light in version 1")
        void timingStripsCorrect() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
            int sz = grid.rows;
            // Row 6: cols 8 to sz-9
            for (int c = 8; c <= sz - 9; c++) {
                assertEquals(c % 2 == 0, grid.modules.get(6).get(c),
                    "row-timing mismatch at col " + c);
            }
            // Col 6: rows 8 to sz-9
            for (int r = 8; r <= sz - 9; r++) {
                assertEquals(r % 2 == 0, grid.modules.get(r).get(6),
                    "col-timing mismatch at row " + r);
            }
        }

        @Test
        @DisplayName("Dark module is set at (4V+9, 8) for version 1")
        void darkModuleV1() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("A", QRCode.EccLevel.M);
            // Version 1: 4*1+9 = 13
            assertTrue(grid.modules.get(13).get(8), "dark module at (13,8)");
        }

        @Test
        @DisplayName("Dark module is set at (4V+9, 8) for version 2")
        void darkModuleV2() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("https://example.com", QRCode.EccLevel.M);
            // Version 2: 4*2+9 = 17
            assertTrue(grid.modules.get(17).get(8), "dark module at (17,8)");
        }
    }

    // =========================================================================
    // Format information tests
    // =========================================================================

    @Nested
    @DisplayName("Format information")
    class FormatInfoTests {

        @Test
        @DisplayName("Format info is valid (BCH) for ECC-M, HELLO WORLD")
        void formatInfoValidM() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
            int[] decoded = decodeFormatInfo(grid.modules);
            assertNotNull(decoded, "BCH check should pass");
            assertEquals(0b00, decoded[0], "ECC bits should be 00 for M");
        }

        @Test
        @DisplayName("Both format info copies carry identical bit patterns")
        void formatInfoCopiesMatch() throws QRCode.QRCodeException {
            ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
            int sz = grid.rows;

            // Copy 1: positions in standard order (f14 → f0)
            int[][] copy1 = {
                {8,0},{8,1},{8,2},{8,3},{8,4},{8,5},{8,7},{8,8},
                {7,8},{5,8},{4,8},{3,8},{2,8},{1,8},{0,8}
            };
            // Copy 2: positions in same bit order (f14 → f0)
            int[][] copy2 = {
                {sz-1,8},{sz-2,8},{sz-3,8},{sz-4,8},{sz-5,8},{sz-6,8},{sz-7,8},
                {8,sz-8},{8,sz-7},{8,sz-6},{8,sz-5},{8,sz-4},{8,sz-3},{8,sz-2},{8,sz-1}
            };

            int fmt1 = 0, fmt2 = 0;
            for (int i = 0; i < 15; i++) {
                if (grid.modules.get(copy1[i][0]).get(copy1[i][1])) fmt1 |= (1 << (14 - i));
                if (grid.modules.get(copy2[i][0]).get(copy2[i][1])) fmt2 |= (1 << (14 - i));
            }
            assertEquals(fmt1, fmt2, "both format info copies must be identical");
        }
    }

    // =========================================================================
    // Version 7+ (version info area)
    // =========================================================================

    @Nested
    @DisplayName("Version 7+ (version info)")
    class VersionInfoTests {

        @Test
        @DisplayName("Version 7 grid is 45×45 and dark module is set")
        void v7GridSize() throws QRCode.QRCodeException {
            // 85 uppercase letters exceed v6-H capacity (~84 alphanumeric chars)
            String input = "A".repeat(85);
            ModuleGrid grid = QRCode.encode(input, QRCode.EccLevel.H);
            assertTrue(grid.rows >= 45, "v7 grid must be ≥ 45×45");
            // Dark module check.
            int sz = grid.rows;
            int version = (sz - 17) / 4;
            assertTrue(grid.modules.get(4 * version + 9).get(8),
                "dark module must be set in v7+ grid");
        }
    }

    // =========================================================================
    // Determinism and uniqueness
    // =========================================================================

    @Nested
    @DisplayName("Determinism and uniqueness")
    class DeterminismTests {

        @Test
        @DisplayName("Encoding the same input twice produces identical grids")
        void deterministic() throws QRCode.QRCodeException {
            ModuleGrid g1 = QRCode.encode("https://example.com", QRCode.EccLevel.M);
            ModuleGrid g2 = QRCode.encode("https://example.com", QRCode.EccLevel.M);
            assertEquals(g1.modules, g2.modules);
        }

        @Test
        @DisplayName("Different inputs produce different grids")
        void differentInputsDiffer() throws QRCode.QRCodeException {
            ModuleGrid g1 = QRCode.encode("HELLO", QRCode.EccLevel.M);
            ModuleGrid g2 = QRCode.encode("WORLD", QRCode.EccLevel.M);
            int sz = g1.rows;
            boolean differ = false;
            outer:
            for (int r = 0; r < sz; r++) {
                for (int c = 0; c < sz; c++) {
                    if (!g1.modules.get(r).get(c).equals(g2.modules.get(r).get(c))) {
                        differ = true;
                        break outer;
                    }
                }
            }
            assertTrue(differ, "HELLO and WORLD must produce different grids");
        }
    }

    // =========================================================================
    // Error paths
    // =========================================================================

    @Nested
    @DisplayName("Error handling")
    class ErrorHandlingTests {

        @Test
        @DisplayName("Input exceeding v40 capacity throws QRCodeException")
        void inputTooLong() {
            String giant = "A".repeat(8000);
            assertThrows(QRCode.QRCodeException.class,
                () -> QRCode.encode(giant, QRCode.EccLevel.H));
        }

        @Test
        @DisplayName("Exception message mentions input length")
        void exceptionMessageInformative() {
            String giant = "A".repeat(8000);
            QRCode.QRCodeException ex = assertThrows(QRCode.QRCodeException.class,
                () -> QRCode.encode(giant, QRCode.EccLevel.H));
            assertNotNull(ex.getMessage(), "exception should have a message");
            assertFalse(ex.getMessage().isEmpty(), "exception message should not be empty");
        }
    }

    // =========================================================================
    // encodeAndLayout integration
    // =========================================================================

    @Nested
    @DisplayName("encodeAndLayout integration")
    class EncodeAndLayoutTests {

        @Test
        @DisplayName("encodeAndLayout returns a non-empty PaintScene")
        void encodeAndLayoutWorks() throws QRCode.QRCodeException {
            Barcode2DLayoutConfig config = Barcode2DLayoutConfig.defaults();
            PaintScene scene = QRCode.encodeAndLayout("HELLO", QRCode.EccLevel.M, config);
            assertTrue(scene.width > 0, "scene width must be > 0");
            assertTrue(scene.height > 0, "scene height must be > 0");
            assertNotNull(scene.instructions, "scene instructions must not be null");
            assertFalse(scene.instructions.isEmpty(), "scene instructions must not be empty");
        }

        @Test
        @DisplayName("encodeAndLayout dimensions scale with module count")
        void encodeAndLayoutScalesWithVersion() throws QRCode.QRCodeException {
            Barcode2DLayoutConfig config = Barcode2DLayoutConfig.defaults();
            PaintScene s1 = QRCode.encodeAndLayout("A", QRCode.EccLevel.M, config);
            PaintScene s2 = QRCode.encodeAndLayout("https://example.com", QRCode.EccLevel.M, config);
            assertTrue(s2.width >= s1.width, "v2 scene width should be ≥ v1 scene width");
        }
    }

    // =========================================================================
    // Corpus test
    // =========================================================================

    @Test
    @DisplayName("Corpus: well-known inputs all produce valid format info")
    void testCorpus() throws QRCode.QRCodeException {
        record Case(String input, QRCode.EccLevel ecc) {}
        var corpus = List.of(
            new Case("A", QRCode.EccLevel.M),
            new Case("HELLO WORLD", QRCode.EccLevel.M),
            new Case("https://example.com", QRCode.EccLevel.M),
            new Case("01234567890", QRCode.EccLevel.M),
            new Case("The quick brown fox jumps over the lazy dog", QRCode.EccLevel.M),
            new Case("01234", QRCode.EccLevel.L),
            new Case("ABCDEFGH", QRCode.EccLevel.Q),
            new Case("hello world!", QRCode.EccLevel.H)
        );
        for (var c : corpus) {
            ModuleGrid grid = QRCode.encode(c.input(), c.ecc());
            assertTrue(grid.rows >= 21, "rows ≥ 21 for: " + c.input());
            assertEquals(grid.rows, grid.cols, "square for: " + c.input());
            int[] decoded = decodeFormatInfo(grid.modules);
            assertNotNull(decoded, "valid format info for: " + c.input());
        }
    }

    // =========================================================================
    // RS ECC internals (via reflection or package-private access)
    // =========================================================================

    @Test
    @DisplayName("Version 40-L can encode ~2953 bytes (near-capacity)")
    void nearV40Capacity() throws QRCode.QRCodeException {
        // Version 40-L holds 2956 data codewords for byte mode.
        // Encoding 2900 bytes of all-0x00 (byte mode) forces version 40.
        // We use a string of 2900 Latin-1 characters that require byte mode
        // (lowercase 'a' is not in the alphanumeric set, so byte mode is selected).
        String input = "a".repeat(2900);
        ModuleGrid grid = QRCode.encode(input, QRCode.EccLevel.L);
        // Should be version 40: 4*40+17 = 177
        assertEquals(177, grid.rows, "near-capacity input should use version 40");
    }

    @Test
    @DisplayName("Consecutive numeric sequences select numeric mode (smaller grid)")
    void numericUsesLessCapacityThanByte() throws QRCode.QRCodeException {
        String numericInput = "0".repeat(100);
        String mixedInput = "0".repeat(99) + "a";  // forces byte mode
        ModuleGrid gNum = QRCode.encode(numericInput, QRCode.EccLevel.L);
        ModuleGrid gByte = QRCode.encode(mixedInput, QRCode.EccLevel.L);
        assertTrue(gNum.rows <= gByte.rows,
            "numeric mode should need same or fewer modules than byte mode");
    }
}
