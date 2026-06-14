/**
 * MicroQRTest.kt — JUnit 5 test suite for the Micro QR Code Kotlin encoder.
 *
 * Coverage targets:
 * - VERSION constant
 * - Error hierarchy (MicroQRError subtypes are Exception subclasses)
 * - Basic encoding smoke tests (encode returns a ModuleGrid)
 * - Grid shape (always square)
 * - Module value types (booleans)
 * - Determinism (same input → identical output)
 * - Auto-version selection (larger input → larger symbol)
 * - All four symbol versions (M1–M4)
 * - Exact module counts per version (11², 13², 15², 17²)
 * - Finder pattern structure
 * - ECC levels L, M, Q
 * - Invalid ECC level throws MicroQRError.InvalidECCLevel
 * - Input too long throws MicroQRError.InputTooLong
 * - Structural modules (separator, timing, format info)
 * - RS encoder internals
 * - Mask condition logic
 * - Penalty scorer
 * - Capacity boundaries
 * - Mask pattern forced via options
 * - ModuleGrid immutability
 */
package com.codingadventures.microqr

import com.codingadventures.barcode2d.ModuleGrid
import com.codingadventures.barcode2d.ModuleShape
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.CsvSource
import org.junit.jupiter.params.provider.ValueSource

/**
 * Serialize a [ModuleGrid] to a compact string for determinism comparisons.
 *
 * Each row becomes a string of '1' (dark) and '0' (light), rows separated
 * by newlines.  This matches the cross-language corpus format used in the spec.
 */
private fun gridToString(g: ModuleGrid): String = buildString {
    for (r in 0 until g.rows) {
        if (r > 0) append('\n')
        for (c in 0 until g.cols) {
            append(if (g.modules[r][c]) '1' else '0')
        }
    }
}

class MicroQRTest {

    // =========================================================================
    // 1. VERSION constant
    // =========================================================================

    @Nested
    inner class VersionTest {
        @Test
        fun `VERSION is 0-1-0`() {
            assertEquals("0.1.0", VERSION)
        }
    }

    // =========================================================================
    // 2. Error hierarchy
    // =========================================================================

    @Nested
    inner class ErrorHierarchyTest {

        @Test
        fun `InputTooLong is a subtype of MicroQRError`() {
            val e = MicroQRError.InputTooLong("test")
            assertTrue(e is MicroQRError)
        }

        @Test
        fun `InputTooLong is a subtype of Exception`() {
            val e = MicroQRError.InputTooLong("test")
            assertTrue(e is Exception)
        }

        @Test
        fun `InvalidECCLevel is a subtype of MicroQRError`() {
            val e = MicroQRError.InvalidECCLevel("test")
            assertTrue(e is MicroQRError)
        }

        @Test
        fun `InvalidECCLevel is a subtype of Exception`() {
            val e = MicroQRError.InvalidECCLevel("test")
            assertTrue(e is Exception)
        }

        @Test
        fun `InvalidOptions is a subtype of MicroQRError`() {
            val e = MicroQRError.InvalidOptions("test")
            assertTrue(e is MicroQRError)
        }

        @Test
        fun `InvalidOptions is a subtype of Exception`() {
            val e = MicroQRError.InvalidOptions("test")
            assertTrue(e is Exception)
        }

        @Test
        fun `error message is preserved`() {
            val msg = "something went wrong"
            assertEquals(msg, MicroQRError.InputTooLong(msg).message)
            assertEquals(msg, MicroQRError.InvalidECCLevel(msg).message)
            assertEquals(msg, MicroQRError.InvalidOptions(msg).message)
        }
    }

    // =========================================================================
    // 3. Basic smoke test — encode("A") returns a non-null ModuleGrid
    // =========================================================================

    @Nested
    inner class BasicEncodeTest {

        @Test
        fun `encode returns a ModuleGrid`() {
            val g = encode("A")
            assertNotNull(g)
        }

        @Test
        fun `encode with default options works`() {
            val g = encode("1", MicroQROptions())
            assertNotNull(g)
        }

        @Test
        fun `encode with all-null opts works`() {
            val g = encode("HELLO")
            assertEquals(13, g.rows)
        }
    }

    // =========================================================================
    // 4. Grid shape — rows == cols (square)
    // =========================================================================

    @Nested
    inner class GridShapeTest {

        @ParameterizedTest
        @ValueSource(strings = ["1", "12345", "HELLO", "hello", "https://a.b", "MICRO QR TEST"])
        fun `grid is always square`(input: String) {
            val g = encode(input)
            assertEquals(g.rows, g.cols, "grid should be square for '$input'")
        }

        @Test
        fun `module shape is SQUARE`() {
            val g = encode("1")
            assertEquals(ModuleShape.SQUARE, g.moduleShape)
        }

        @ParameterizedTest
        @ValueSource(strings = ["1", "HELLO", "hello", "https://a.b"])
        fun `grid dimensions match module list`(input: String) {
            val g = encode(input)
            assertEquals(g.rows, g.modules.size)
            for (row in g.modules) assertEquals(g.cols, row.size)
        }
    }

    // =========================================================================
    // 5. All modules are booleans (Kotlin List<List<Boolean>>)
    // =========================================================================

    @Nested
    inner class ModuleTypesTest {

        @Test
        fun `all modules are Boolean values`() {
            val g = encode("HELLO")
            for (r in 0 until g.rows) {
                for (c in 0 until g.cols) {
                    // If the runtime type is Boolean, this simply completes without error.
                    val m: Boolean = g.modules[r][c]
                    assertTrue(m || !m)  // tautology — verifies it IS a Boolean
                }
            }
        }
    }

    // =========================================================================
    // 6. Determinism — same input → identical grid
    // =========================================================================

    @Nested
    inner class DeterminismTest {

        @ParameterizedTest
        @ValueSource(strings = ["1", "12345", "HELLO", "A1B2C3", "hello", "https://a.b"])
        fun `encoding is deterministic`(input: String) {
            val g1 = encode(input)
            val g2 = encode(input)
            assertEquals(gridToString(g1), gridToString(g2),
                "encoding should be deterministic for '$input'")
        }

        @Test
        fun `different inputs produce different grids`() {
            assertNotEquals(gridToString(encode("1")), gridToString(encode("2")))
        }

        @Test
        fun `each call returns a distinct ModuleGrid object`() {
            val g1 = encode("1")
            val g2 = encode("1")
            assertNotSame(g1, g2)
            assertEquals(g1, g2)  // same content
        }
    }

    // =========================================================================
    // 7. Larger input needs bigger symbol (auto-version selection)
    // =========================================================================

    @Nested
    inner class AutoVersionSelectionTest {

        @Test
        fun `single digit selects M1 (11x11)`() {
            assertEquals(11, encode("1").rows)
        }

        @Test
        fun `5 digits selects M1 (max numeric cap)`() {
            assertEquals(11, encode("12345").rows)
        }

        @Test
        fun `6 digits overflows M1 and selects M2`() {
            assertEquals(13, encode("123456").rows)
        }

        @Test
        fun `HELLO (5 alpha chars) selects M2`() {
            assertEquals(13, encode("HELLO").rows)
        }

        @Test
        fun `hello (5 byte chars) selects M3 or larger`() {
            assertTrue(encode("hello").rows >= 15)
        }

        @Test
        fun `https URL selects M4`() {
            assertEquals(17, encode("https://a.b").rows)
        }

        @Test
        fun `MICRO QR TEST selects M3`() {
            assertEquals(15, encode("MICRO QR TEST").rows)
        }

        @Test
        fun `forced M4 with single digit still gives 17x17`() {
            assertEquals(17, encode("1", MicroQROptions(symbol = "M4")).rows)
        }

        @Test
        fun `lowercase symbol string is normalised`() {
            // "m4" should be treated the same as "M4"
            assertEquals(17, encode("1", MicroQROptions(symbol = "m4")).rows)
        }
    }

    // =========================================================================
    // 8. All four symbols (M1–M4) produce grids
    // =========================================================================

    @Nested
    inner class AllSymbolsTest {

        @Test
        fun `M1 produces a 11x11 grid`() {
            val g = encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.DETECTION))
            assertEquals(11, g.rows)
            assertEquals(11, g.cols)
        }

        @Test
        fun `M2 produces a 13x13 grid`() {
            val g = encode("HELLO", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.L))
            assertEquals(13, g.rows)
            assertEquals(13, g.cols)
        }

        @Test
        fun `M3 produces a 15x15 grid`() {
            val g = encode("MICRO QR TEST", MicroQROptions(symbol = "M3", eccLevel = ECCLevel.L))
            assertEquals(15, g.rows)
            assertEquals(15, g.cols)
        }

        @Test
        fun `M4 produces a 17x17 grid`() {
            val g = encode("https://a.b", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L))
            assertEquals(17, g.rows)
            assertEquals(17, g.cols)
        }
    }

    // =========================================================================
    // 9. Module counts: M1=121, M2=169, M3=225, M4=289
    // =========================================================================

    @Nested
    inner class ModuleCountTest {

        private fun countModules(g: ModuleGrid): Int = g.rows * g.cols

        @Test
        fun `M1 has 121 modules (11 squared)`() {
            assertEquals(121, countModules(encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.DETECTION))))
        }

        @Test
        fun `M2 has 169 modules (13 squared)`() {
            assertEquals(169, countModules(encode("HELLO", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.L))))
        }

        @Test
        fun `M3 has 225 modules (15 squared)`() {
            assertEquals(225, countModules(encode("MICRO QR TEST", MicroQROptions(symbol = "M3", eccLevel = ECCLevel.L))))
        }

        @Test
        fun `M4 has 289 modules (17 squared)`() {
            assertEquals(289, countModules(encode("https://a.b", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L))))
        }
    }

    // =========================================================================
    // 10. Top-left 7×7 is the finder pattern
    // =========================================================================

    @Nested
    inner class FinderPatternTest {

        /**
         * The finder pattern structure:
         * ```
         * Outer ring (rows 0 & 6, cols 0 & 6): all dark
         * Inner ring (rows 1 & 5, cols 1–5; rows 2–4, cols 1 & 5): all light
         * Core (rows 2–4, cols 2–4): all dark
         * ```
         */
        @Test
        fun `M1 finder outer ring is all dark`() {
            val m = encode("1").modules
            // Row 0 all dark
            for (c in 0..6) assertTrue(m[0][c], "row 0 col $c should be dark")
            // Row 6 all dark
            for (c in 0..6) assertTrue(m[6][c], "row 6 col $c should be dark")
            // Col 0 all dark
            for (r in 0..6) assertTrue(m[r][0], "col 0 row $r should be dark")
            // Col 6 all dark
            for (r in 0..6) assertTrue(m[r][6], "col 6 row $r should be dark")
        }

        @Test
        fun `M1 finder inner ring is all light`() {
            val m = encode("1").modules
            // Row 1, cols 1–5: light
            for (c in 1..5) assertFalse(m[1][c], "inner ring row 1 col $c should be light")
            // Row 5, cols 1–5: light
            for (c in 1..5) assertFalse(m[5][c], "inner ring row 5 col $c should be light")
            // Col 1, rows 2–4: light
            for (r in 2..4) assertFalse(m[r][1], "inner ring row $r col 1 should be light")
            // Col 5, rows 2–4: light
            for (r in 2..4) assertFalse(m[r][5], "inner ring row $r col 5 should be light")
        }

        @Test
        fun `M1 finder core (rows 2-4, cols 2-4) is all dark`() {
            val m = encode("1").modules
            for (r in 2..4) for (c in 2..4) assertTrue(m[r][c], "core ($r,$c) should be dark")
        }

        @Test
        fun `M4 finder pattern matches M1 (same 7x7)`() {
            val m1 = encode("1").modules
            val m4 = encode("https://a.b").modules
            // Top-left 7×7 must match between all symbol sizes
            for (r in 0..6) {
                for (c in 0..6) {
                    assertEquals(m1[r][c], m4[r][c], "finder [$r][$c] should match")
                }
            }
        }
    }

    // =========================================================================
    // 11. ECC levels L, M, Q work for appropriate symbols
    // =========================================================================

    @Nested
    inner class ECCLevelTest {

        @Test
        fun `M1 with DETECTION produces 11x11`() {
            val g = encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.DETECTION))
            assertEquals(11, g.rows)
        }

        @Test
        fun `M2-L works`() {
            assertEquals(13, encode("HELLO", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.L)).rows)
        }

        @Test
        fun `M2-M works`() {
            assertEquals(13, encode("HELLO", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.M)).rows)
        }

        @Test
        fun `M3-L works`() {
            assertEquals(15, encode("MICRO QR TEST", MicroQROptions(symbol = "M3", eccLevel = ECCLevel.L)).rows)
        }

        @Test
        fun `M3-M works`() {
            // M3-M alpha cap = 11; "HELLO WORLD" = 11 chars, fits exactly.
            assertEquals(15, encode("HELLO WORLD", MicroQROptions(symbol = "M3", eccLevel = ECCLevel.M)).rows)
        }

        @Test
        fun `M4-L works`() {
            assertEquals(17, encode("https://a.b", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L)).rows)
        }

        @Test
        fun `M4-M works`() {
            assertEquals(17, encode("https://a.b", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.M)).rows)
        }

        @Test
        fun `M4-Q works`() {
            assertEquals(17, encode("HELLO", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.Q)).rows)
        }

        @Test
        fun `L and M produce different grids for same input`() {
            val gL = encode("HELLO", MicroQROptions(eccLevel = ECCLevel.L))
            val gM = encode("HELLO", MicroQROptions(eccLevel = ECCLevel.M))
            assertNotEquals(gridToString(gL), gridToString(gM),
                "L and M grids should differ (different format info)")
        }

        @Test
        fun `M4 L, M, Q all produce 17x17 but differ`() {
            val gL = encode("HELLO", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L))
            val gM = encode("HELLO", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.M))
            val gQ = encode("HELLO", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.Q))
            assertNotEquals(gridToString(gL), gridToString(gM))
            assertNotEquals(gridToString(gM), gridToString(gQ))
            assertNotEquals(gridToString(gL), gridToString(gQ))
        }
    }

    // =========================================================================
    // 12. Invalid ECC level throws MicroQRError.InvalidECCLevel
    // =========================================================================

    @Nested
    inner class InvalidECCLevelTest {

        @Test
        fun `M1 with L throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.L))
            }
        }

        @Test
        fun `M1 with M throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.M))
            }
        }

        @Test
        fun `M1 with Q throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M1", eccLevel = ECCLevel.Q))
            }
        }

        @Test
        fun `M2 with DETECTION throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.DETECTION))
            }
        }

        @Test
        fun `M2 with Q throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.Q))
            }
        }

        @Test
        fun `M3 with Q throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M3", eccLevel = ECCLevel.Q))
            }
        }

        @Test
        fun `M3 with DETECTION throws InvalidECCLevel`() {
            assertThrows(MicroQRError.InvalidECCLevel::class.java) {
                encode("1", MicroQROptions(symbol = "M3", eccLevel = ECCLevel.DETECTION))
            }
        }

        @Test
        fun `invalid symbol string throws InvalidOptions`() {
            assertThrows(MicroQRError.InvalidOptions::class.java) {
                encode("1", MicroQROptions(symbol = "M5"))
            }
        }

        @Test
        fun `mask pattern out of range throws InvalidOptions`() {
            assertThrows(MicroQRError.InvalidOptions::class.java) {
                encode("1", MicroQROptions(maskPattern = 7))
            }
        }

        @Test
        fun `negative mask pattern throws InvalidOptions`() {
            assertThrows(MicroQRError.InvalidOptions::class.java) {
                encode("1", MicroQROptions(maskPattern = -1))
            }
        }
    }

    // =========================================================================
    // 13. Input too long throws MicroQRError.InputTooLong
    // =========================================================================

    @Nested
    inner class InputTooLongTest {

        @Test
        fun `36 digits exceeds M4-L numeric capacity (35)`() {
            assertThrows(MicroQRError.InputTooLong::class.java) {
                encode("1".repeat(36))
            }
        }

        @Test
        fun `16 byte-mode bytes exceeds M4-L byte capacity (15)`() {
            assertThrows(MicroQRError.InputTooLong::class.java) {
                encode("a".repeat(16))
            }
        }

        @Test
        fun `22 alphanumeric chars exceeds M4-L alpha capacity (21)`() {
            assertThrows(MicroQRError.InputTooLong::class.java) {
                encode("A".repeat(22))
            }
        }

        @Test
        fun `9 digits pinned to M2-M throws (M2-M numeric cap=8)`() {
            assertThrows(MicroQRError.InputTooLong::class.java) {
                encode("123456789", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.M))
            }
        }

        @Test
        fun `22 digits pinned to Q throws (M4-Q numeric cap=21)`() {
            assertThrows(MicroQRError.InputTooLong::class.java) {
                encode("1".repeat(22), MicroQROptions(eccLevel = ECCLevel.Q))
            }
        }

        @Test
        fun `35 digits fits M4-L (boundary at maximum)`() {
            val g = encode("1".repeat(35))
            assertEquals(17, g.rows)
        }

        @Test
        fun `15 byte-mode chars fits M4-L (boundary at maximum)`() {
            val g = encode("a".repeat(15))
            assertEquals(17, g.rows)
        }
    }

    // =========================================================================
    // Separator and timing structural modules
    // =========================================================================

    @Nested
    inner class StructuralModulesTest {

        @Test
        fun `separator row 7 cols 0-7 is all light`() {
            val m = encode("HELLO").modules
            for (c in 0..7) assertFalse(m[7][c], "separator row 7 col $c should be light")
        }

        @Test
        fun `separator col 7 rows 0-7 is all light`() {
            val m = encode("HELLO").modules
            for (r in 0..7) assertFalse(m[r][7], "separator col 7 row $r should be light")
        }

        @Test
        fun `timing row 0 cols 8 to size-1 alternates dark-light`() {
            val g = encode("https://a.b")  // M4, size=17
            val m = g.modules
            for (c in 8 until 17) {
                assertEquals(c % 2 == 0, m[0][c], "timing row 0 col $c")
            }
        }

        @Test
        fun `timing col 0 rows 8 to size-1 alternates dark-light`() {
            val g = encode("https://a.b")  // M4, size=17
            val m = g.modules
            for (r in 8 until 17) {
                assertEquals(r % 2 == 0, m[r][0], "timing col 0 row $r")
            }
        }

        @Test
        fun `timing row 0 on M2 (cols 8-12)`() {
            val m = encode("HELLO").modules
            for (c in 8 until 13) {
                assertEquals(c % 2 == 0, m[0][c], "timing row 0 col $c")
            }
        }

        @Test
        fun `format info area has at least some dark modules`() {
            val m = encode("1").modules
            var anyDark = false
            for (c in 1..8) anyDark = anyDark || m[8][c]
            for (r in 1..7) anyDark = anyDark || m[r][8]
            assertTrue(anyDark, "format info should have some dark modules")
        }

        @Test
        fun `format info differs across ECC levels on same version`() {
            val gL = encode("1", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L))
            val gM = encode("1", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.M))
            val gQ = encode("1", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.Q))
            assertNotEquals(gridToString(gL), gridToString(gM))
            assertNotEquals(gridToString(gM), gridToString(gQ))
            assertNotEquals(gridToString(gL), gridToString(gQ))
        }
    }

    // =========================================================================
    // Reed-Solomon encoder internals
    // =========================================================================

    @Nested
    inner class RSEncoderTest {

        @Test
        fun `all-zero data produces all-zero ECC (gen degree 2)`() {
            val gen = intArrayOf(0x01, 0x03, 0x02)
            val ecc = rsEncode(ByteArray(3), gen)
            assertArrayEquals(byteArrayOf(0, 0), ecc,
                "RS(all zeros) should produce all zeros")
        }

        @Test
        fun `all-zero data produces all-zero ECC (gen degree 5)`() {
            val gen = intArrayOf(0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68)
            val ecc = rsEncode(ByteArray(5), gen)
            for (b in ecc) assertEquals(0.toByte(), b, "all-zero data → all-zero ECC")
        }

        @Test
        fun `ECC output length equals generator degree`() {
            val gen2  = intArrayOf(0x01, 0x03, 0x02)
            val gen5  = intArrayOf(0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68)
            assertEquals(2, rsEncode(ByteArray(3), gen2).size)
            assertEquals(5, rsEncode(ByteArray(3), gen5).size)
        }

        @Test
        fun `different data produces different ECC`() {
            val gen = intArrayOf(0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3)
            val e1 = rsEncode(byteArrayOf(0x10, 0x20, 0x30), gen)
            val e2 = rsEncode(byteArrayOf(0x40, 0x50, 0x60), gen)
            assertFalse(e1.contentEquals(e2), "different data should produce different ECC")
        }

        @Test
        fun `RS is idempotent (same data → same ECC)`() {
            val gen = intArrayOf(0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37)
            val data = byteArrayOf(0x12, 0x34, 0x56, 0x78, 0x9a.toByte(), 0xbc.toByte())
            val e1 = rsEncode(data, gen)
            val e2 = rsEncode(data, gen)
            assertArrayEquals(e1, e2)
        }
    }

    // =========================================================================
    // Mask condition logic
    // =========================================================================

    @Nested
    inner class MaskConditionTest {

        @Test
        fun `mask 0 - (row+col) mod 2 == 0`() {
            assertTrue(maskCondition(0, 0, 0))   // 0+0=0 → true
            assertFalse(maskCondition(0, 0, 1))  // 0+1=1 → false
            assertTrue(maskCondition(0, 1, 1))   // 1+1=2 → true
            assertFalse(maskCondition(0, 1, 0))  // 1+0=1 → false
        }

        @Test
        fun `mask 1 - row mod 2 == 0`() {
            assertTrue(maskCondition(1, 0, 5))   // row 0 even
            assertFalse(maskCondition(1, 1, 5))  // row 1 odd
            assertTrue(maskCondition(1, 2, 0))   // row 2 even
            assertFalse(maskCondition(1, 3, 3))  // row 3 odd
        }

        @Test
        fun `mask 2 - col mod 3 == 0`() {
            assertTrue(maskCondition(2, 5, 0))   // col 0 → 0 mod 3 = 0
            assertFalse(maskCondition(2, 5, 1))  // col 1 → 1 mod 3 = 1
            assertFalse(maskCondition(2, 5, 2))  // col 2 → 2 mod 3 = 2
            assertTrue(maskCondition(2, 5, 3))   // col 3 → 3 mod 3 = 0
            assertTrue(maskCondition(2, 5, 6))   // col 6 → 6 mod 3 = 0
        }

        @Test
        fun `mask 3 - (row+col) mod 3 == 0`() {
            assertTrue(maskCondition(3, 0, 0))   // 0+0=0 → 0 mod 3 = 0
            assertFalse(maskCondition(3, 0, 1))  // 0+1=1
            assertFalse(maskCondition(3, 1, 0))  // 1+0=1
            assertTrue(maskCondition(3, 1, 2))   // 1+2=3 → 3 mod 3 = 0
            assertTrue(maskCondition(3, 3, 0))   // 3+0=3 → 0
        }

        @Test
        fun `out-of-range mask index returns false`() {
            assertFalse(maskCondition(4, 0, 0))
            assertFalse(maskCondition(-1, 0, 0))
            assertFalse(maskCondition(99, 5, 5))
        }
    }

    // =========================================================================
    // Penalty scorer
    // =========================================================================

    @Nested
    inner class PenaltyScorerTest {

        /**
         * All-dark 5×5 grid:
         * Rule 1: 5 rows × (5−2) + 5 cols × (5−2) = 30
         * Rule 2: 4×4 = 16 blocks → 16×3 = 48
         * Rule 3: 5 < 11 → 0
         * Rule 4: 100% dark → prev5=100, next5=105
         *         min(|100−50|, |105−50|) = min(50,55) = 50 → 50/5×10 = 100
         * Total: 178
         */
        @Test
        fun `all-dark 5x5 has penalty 178`() {
            val m = Array(5) { BooleanArray(5) { true } }
            assertEquals(178, computePenalty(m, 5))
        }

        /**
         * All-light 5×5 grid:
         * Rule 1: same as all-dark → 30
         * Rule 2: 48
         * Rule 3: 0
         * Rule 4: 0% dark → prev5=0, next5=5
         *         min(|0−50|, |5−50|) = min(50, 45) = 45 → 45/5×10 = 90
         * Total: 168
         */
        @Test
        fun `all-light 5x5 has penalty 168`() {
            val m = Array(5) { BooleanArray(5) { false } }
            assertEquals(168, computePenalty(m, 5))
        }

        /**
         * Checkerboard 5×5 (alternating dark/light):
         * Rule 1: no runs of ≥5 → 0
         * Rule 2: no 2×2 same-color blocks → 0
         * Rule 3: sz=5 < 11 → 0
         * Rule 4: 13 dark / 25 total = 52% → prev5=50 → min(0,5) = 0 → 0
         * Total: 0
         */
        @Test
        fun `checkerboard 5x5 has penalty 0`() {
            val m = Array(5) { r -> BooleanArray(5) { c -> (r + c) % 2 == 0 } }
            assertEquals(0, computePenalty(m, 5))
        }

        @Test
        fun `run of exactly 5 contributes at least 3 to penalty`() {
            // 7×7 grid, row 0: 5 dark then 2 light
            val m = Array(7) { BooleanArray(7) }
            for (c in 0..4) m[0][c] = true  // 5 dark at row 0 cols 0–4
            val p = computePenalty(m, 7)
            assertTrue(p >= 3, "run of 5 should add at least 3 to penalty, got $p")
        }

        @Test
        fun `2x2 same-color block contributes at least 3 to penalty`() {
            val m = Array(5) { BooleanArray(5) }
            m[1][1] = true; m[1][2] = true; m[2][1] = true; m[2][2] = true
            val p = computePenalty(m, 5)
            assertTrue(p >= 3, "2×2 block should add 3 to penalty, got $p")
        }
    }

    // =========================================================================
    // Forced mask pattern via options
    // =========================================================================

    @Nested
    inner class ForcedMaskTest {

        @Test
        fun `all 4 forced masks produce valid grids of the right size`() {
            for (mask in 0..3) {
                val g = encode("HELLO", MicroQROptions(maskPattern = mask))
                assertEquals(13, g.rows, "mask $mask should produce 13×13")
            }
        }

        @Test
        fun `different forced masks produce different grids`() {
            val g0 = gridToString(encode("HELLO", MicroQROptions(maskPattern = 0)))
            val g1 = gridToString(encode("HELLO", MicroQROptions(maskPattern = 1)))
            val g2 = gridToString(encode("HELLO", MicroQROptions(maskPattern = 2)))
            val g3 = gridToString(encode("HELLO", MicroQROptions(maskPattern = 3)))
            // All four should differ (at least finder-pattern mask XOR changes format info)
            val grids = setOf(g0, g1, g2, g3)
            assertTrue(grids.size > 1, "different masks should produce different grids")
        }
    }

    // =========================================================================
    // Capacity boundary tests
    // =========================================================================

    @Nested
    inner class CapacityBoundaryTest {

        @Test
        fun `M1 max 5 numeric digits fits (M1 numeric cap 5)`() {
            assertEquals(11, encode("12345").rows)
        }

        @Test
        fun `M1 overflow 6 digits falls through to M2`() {
            assertEquals(13, encode("123456").rows)
        }

        @Test
        fun `M2-L max 6 alphanumeric chars fits`() {
            assertEquals(13, encode("ABCDEF", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.L)).rows)
        }

        @Test
        fun `M2-L overflow 7 alpha chars falls to M3`() {
            assertEquals(15, encode("ABCDEFG", MicroQROptions(eccLevel = ECCLevel.L)).rows)
        }

        @Test
        fun `M2-M max 8 numeric digits fits`() {
            assertEquals(13, encode("12345678", MicroQROptions(symbol = "M2", eccLevel = ECCLevel.M)).rows)
        }

        @Test
        fun `M4 max 35 numeric digits fits`() {
            assertEquals(17, encode("1".repeat(35)).rows)
        }

        @Test
        fun `M4-Q max 21 numeric digits fits`() {
            assertEquals(17, encode("1".repeat(21), MicroQROptions(eccLevel = ECCLevel.Q)).rows)
        }

        @Test
        fun `empty string encodes to M1`() {
            assertEquals(11, encode("").rows)
        }

        @Test
        fun `empty string with forced M4-L produces 17x17`() {
            assertEquals(17, encode("", MicroQROptions(symbol = "M4", eccLevel = ECCLevel.L)).rows)
        }
    }

    // =========================================================================
    // Single-character edge cases
    // =========================================================================

    @Nested
    inner class SingleCharTest {

        @Test
        fun `single digit 0 encodes to M1`() {
            assertEquals(11, encode("0").rows)
        }

        @Test
        fun `single alphanumeric A falls through to M2`() {
            // 'A' is alphanumeric, but M1 doesn't support alphanumeric mode.
            assertEquals(13, encode("A").rows)
        }

        @Test
        fun `single byte char (lowercase a) encodes to M2 or larger`() {
            // 'a' requires byte mode; M2-L byte cap=4, so it fits M2.
            assertTrue(encode("a").rows >= 13)
        }
    }

    // =========================================================================
    // Cross-language corpus — from the spec integration test table
    // =========================================================================

    @Nested
    inner class CrossLanguageCorpusTest {

        @ParameterizedTest
        @CsvSource(
            "1,            11",
            "12345,        11",
            "HELLO,        13",
            "01234567,     13",
            "https://a.b,  17",
            "MICRO QR TEST,15",
        )
        fun `corpus inputs produce expected symbol sizes`(input: String, expectedSize: Int) {
            val trimmed = input.trim()
            val g = encode(trimmed)
            assertEquals(expectedSize, g.rows,
                "input '$trimmed' should produce ${expectedSize}×${expectedSize}")
        }
    }

    // =========================================================================
    // ModuleGrid immutability
    // =========================================================================

    @Nested
    inner class ImmutabilityTest {

        @Test
        fun `inner row list is unmodifiable`() {
            val g = encode("1")
            assertThrows(UnsupportedOperationException::class.java) {
                @Suppress("UNCHECKED_CAST")
                (g.modules[0] as MutableList<Boolean>).set(0, true)
            }
        }

        @Test
        fun `outer modules list is unmodifiable`() {
            val g = encode("1")
            assertThrows(UnsupportedOperationException::class.java) {
                @Suppress("UNCHECKED_CAST")
                (g.modules as MutableList<List<Boolean>>).add(emptyList())
            }
        }
    }
}
