package com.codingadventures.aztec;

import com.codingadventures.barcode2d.ModuleGrid;
import org.junit.jupiter.api.*;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Tests for the Java Aztec Code encoder (ISO/IEC 24778:2008).
 *
 * <p>Coverage goals:
 * <ul>
 *   <li>VERSION constant</li>
 *   <li>Error hierarchy: AztecException, InputTooLongException</li>
 *   <li>Compact symbol sizes (15×15 for single byte)</li>
 *   <li>Symbol grows with data</li>
 *   <li>Bullseye center module</li>
 *   <li>All encode() overloads</li>
 *   <li>AztecOptions with custom ECC percentage</li>
 *   <li>Determinism and uniqueness</li>
 * </ul>
 */
class AztecCodeTest {

    // =========================================================================
    // Version
    // =========================================================================

    @Test
    void versionIs010() {
        assertEquals("0.1.0", AztecCode.VERSION);
    }

    // =========================================================================
    // Error hierarchy
    // =========================================================================

    @Nested
    class ErrorHierarchy {
        @Test
        void aztecExceptionIsRuntimeException() {
            AztecCode.AztecException e = new AztecCode.AztecException("test");
            assertInstanceOf(RuntimeException.class, e);
            assertEquals("test", e.getMessage());
        }

        @Test
        void inputTooLongExtendsAztecException() {
            AztecCode.InputTooLongException e = new AztecCode.InputTooLongException("big");
            assertInstanceOf(AztecCode.AztecException.class, e);
        }

        @Test
        void hugeInputThrowsInputTooLong() {
            assertThrows(AztecCode.InputTooLongException.class,
                () -> AztecCode.encode("x".repeat(3000)));
        }
    }

    // =========================================================================
    // Symbol size selection
    // =========================================================================

    @Nested
    class SymbolSizes {
        @Test
        void singleByteIsCompact1_15x15() {
            ModuleGrid grid = AztecCode.encode("A");
            assertEquals(15, grid.rows);
            assertEquals(15, grid.cols);
        }

        @Test
        void symbolIsAlwaysSquare() {
            for (String data : new String[]{"X", "Hello", "Hello, World!", "A".repeat(100)}) {
                ModuleGrid grid = AztecCode.encode(data);
                assertEquals(grid.rows, grid.cols,
                    "Not square for input length " + data.length());
            }
        }

        @Test
        void largerInputGrowsSymbol() {
            ModuleGrid small = AztecCode.encode("A");
            ModuleGrid large = AztecCode.encode("A".repeat(200));
            assertTrue(large.rows > small.rows,
                "Larger input should produce bigger symbol");
        }
    }

    // =========================================================================
    // Bullseye structure
    // =========================================================================

    @Nested
    class BullseyeStructure {
        @Test
        void centerModuleIsDark() {
            ModuleGrid grid = AztecCode.encode("A");
            int cx = grid.rows / 2;
            int cy = grid.cols / 2;
            assertTrue(grid.modules.get(cx).get(cy),
                "Center module must be dark (compact-1 bullseye)");
        }

        @Test
        void compact1Has15x15Modules() {
            ModuleGrid grid = AztecCode.encode("A");
            assertEquals(15, grid.modules.size(), "row count");
            assertEquals(15, grid.modules.get(0).size(), "col count");
        }
    }

    // =========================================================================
    // encode() overloads
    // =========================================================================

    @Nested
    class EncodeOverloads {
        @Test
        void encodeStringDefault() {
            ModuleGrid grid = AztecCode.encode("Hello");
            assertNotNull(grid);
            assertTrue(grid.rows >= 15);
        }

        @Test
        void encodeStringWithOptions() {
            AztecCode.AztecOptions opts = new AztecCode.AztecOptions(33);
            ModuleGrid grid = AztecCode.encode("Hello", opts);
            assertNotNull(grid);
        }

        @Test
        void encodeBytesDefault() {
            ModuleGrid grid = AztecCode.encode(new byte[]{65, 66, 67}); // "ABC"
            assertNotNull(grid);
            assertTrue(grid.rows >= 15);
        }

        @Test
        void encodeBytesWithOptions() {
            AztecCode.AztecOptions opts = new AztecCode.AztecOptions();
            ModuleGrid grid = AztecCode.encode(new byte[]{72, 101, 108, 108, 111}, opts);
            assertNotNull(grid);
        }

        @Test
        void nullStringEncodesEmpty() {
            ModuleGrid grid = AztecCode.encode((String) null);
            assertNotNull(grid);
            assertTrue(grid.rows >= 15);
        }

        @Test
        void nullBytesEncodesEmpty() {
            ModuleGrid grid = AztecCode.encode((byte[]) null);
            assertNotNull(grid);
        }
    }

    // =========================================================================
    // AztecOptions — ECC control
    // =========================================================================

    @Nested
    class AztecOptionsTest {
        @Test
        void higherECCMayProduceLargerSymbol() {
            ModuleGrid low = AztecCode.encode("A");
            AztecCode.AztecOptions opts = new AztecCode.AztecOptions(80);
            ModuleGrid high = AztecCode.encode("A", opts);
            // Higher ECC forces a larger symbol or equal (never smaller).
            assertTrue(high.rows >= low.rows);
        }
    }

    // =========================================================================
    // Determinism and uniqueness
    // =========================================================================

    @Nested
    class Determinism {
        @Test
        void sameInputSameOutput() {
            ModuleGrid g1 = AztecCode.encode("Hello!");
            ModuleGrid g2 = AztecCode.encode("Hello!");
            assertEquals(g1.rows, g2.rows);
            assertEquals(g1.cols, g2.cols);
            for (int r = 0; r < g1.rows; r++) {
                assertEquals(g1.modules.get(r), g2.modules.get(r),
                    "Row " + r + " differs");
            }
        }

        @Test
        void differentInputsDifferentOutput() {
            ModuleGrid g1 = AztecCode.encode("ABC");
            ModuleGrid g2 = AztecCode.encode("XYZ");
            boolean same = g1.rows == g2.rows && g1.cols == g2.cols
                && g1.modules.equals(g2.modules);
            assertFalse(same, "Different inputs must produce different grids");
        }
    }
}
