/**
 * AztecCodeTest.kt — JUnit 5 test suite for the Aztec Code Kotlin encoder.
 *
 * Coverage targets:
 * - VERSION constant
 * - AztecError sealed class hierarchy
 * - GF(16) arithmetic (mul, generator, RS encode)
 * - GF(256)/0x12D arithmetic (mul, RS encode)
 * - Bit stuffing correctness
 * - Binary-Shift bit encoding
 * - Mode message encoding (compact and full)
 * - Symbol size selection
 * - Full encode pipeline (smoke, determinism, cross-language corpus)
 * - Bullseye pattern validation
 * - Grid dimensions vs. layer count
 * - AztecError.InputTooLong thrown for oversized input
 */
package com.codingadventures.azteccode

import com.codingadventures.barcode2d.ModuleGrid
import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.CsvSource
import org.junit.jupiter.params.provider.ValueSource

// ---------------------------------------------------------------------------
// Helper: serialize a ModuleGrid to a compact string for determinism checks
// ---------------------------------------------------------------------------

private fun gridToString(g: ModuleGrid): String = buildString {
    for (r in 0 until g.rows) {
        if (r > 0) append('\n')
        for (c in 0 until g.cols) {
            append(if (g.modules[r][c]) '1' else '0')
        }
    }
}

class AztecCodeTest {

    // =========================================================================
    // 1. VERSION constant
    // =========================================================================

    @Nested
    inner class VersionTest {
        @Test
        fun `VERSION is 0_1_0`() {
            assertEquals("0.1.0", VERSION)
        }
    }

    // =========================================================================
    // 2. AztecError sealed class hierarchy
    // =========================================================================

    @Nested
    inner class ErrorHierarchyTest {
        @Test
        fun `InputTooLong is an AztecError`() {
            val err = AztecError.InputTooLong("too long")
            assertInstanceOf(AztecError::class.java, err)
        }

        @Test
        fun `InputTooLong is an Exception`() {
            val err = AztecError.InputTooLong("too long")
            assertInstanceOf(Exception::class.java, err)
        }

        @Test
        fun `InputTooLong has the correct message`() {
            val err = AztecError.InputTooLong("test msg")
            assertEquals("test msg", err.message)
        }
    }

    // =========================================================================
    // 3. GF(16) arithmetic
    // =========================================================================

    @Nested
    inner class Gf16ArithmeticTest {

        @Test
        fun `gf16Mul returns 0 if either operand is 0`() {
            assertEquals(0, gf16Mul(0, 7))
            assertEquals(0, gf16Mul(7, 0))
            assertEquals(0, gf16Mul(0, 0))
        }

        @Test
        fun `gf16Mul 1 times x equals x (multiplicative identity)`() {
            for (x in 1..15) {
                assertEquals(x, gf16Mul(1, x), "1 * $x should be $x")
            }
        }

        @Test
        fun `gf16Mul commutative`() {
            for (a in 1..15) {
                for (b in 1..15) {
                    assertEquals(gf16Mul(a, b), gf16Mul(b, a), "gf16Mul($a,$b) should equal gf16Mul($b,$a)")
                }
            }
        }

        @Test
        fun `alpha^15 equals 1 (period 15 confirms table correctness)`() {
            // alpha^1 = 2 in GF(16), so alpha^15 = 2^15 mod p(x)
            // This verifies the primitive element has the expected order.
            var x = 1
            for (i in 0 until 15) {
                x = gf16Mul(x, 2)
            }
            assertEquals(1, x, "alpha^15 should be 1 (period = 15)")
        }

        @Test
        fun `buildGf16Generator degree equals n`() {
            val g = buildGf16Generator(5)
            assertEquals(6, g.size, "degree-5 generator has 6 coefficients")
            assertEquals(1, g[5], "leading coefficient is 1 (monic)")
        }

        @Test
        fun `gf16RsEncode compact mode message produces 5 nibbles`() {
            val dataNibbles = intArrayOf(3, 0)  // m = (0<<6)|(4) = 4, nibbles = [4,0]
            val ecc = gf16RsEncode(dataNibbles, 5)
            assertEquals(5, ecc.size)
            // All values must be in 0..15 (valid nibbles)
            for (n in ecc) assertTrue(n in 0..15, "ECC nibble $n not in 0..15")
        }

        @Test
        fun `gf16RsEncode full mode message produces 6 nibbles`() {
            val dataNibbles = intArrayOf(1, 0, 0, 0)
            val ecc = gf16RsEncode(dataNibbles, 6)
            assertEquals(6, ecc.size)
        }

        @Test
        fun `gf16RsEncode all-zero data produces deterministic ECC`() {
            val zeros = intArrayOf(0, 0)
            val ecc1 = gf16RsEncode(zeros, 5)
            val ecc2 = gf16RsEncode(zeros, 5)
            assertArrayEquals(ecc1, ecc2)
        }
    }

    // =========================================================================
    // 4. GF(256)/0x12D arithmetic
    // =========================================================================

    @Nested
    inner class Gf256ArithmeticTest {

        @Test
        fun `gf256Mul returns 0 for zero inputs`() {
            assertEquals(0, gf256Mul(0, 7))
            assertEquals(0, gf256Mul(7, 0))
            assertEquals(0, gf256Mul(0, 0))
        }

        @Test
        fun `gf256Mul 1 times x equals x`() {
            for (x in 1..255) {
                assertEquals(x, gf256Mul(1, x), "1 * $x should be $x")
            }
        }

        @Test
        fun `gf256Mul is commutative`() {
            // Spot-check 20 pairs
            val pairs = listOf(2 to 3, 5 to 7, 17 to 31, 127 to 200, 255 to 128)
            for ((a, b) in pairs) {
                assertEquals(gf256Mul(a, b), gf256Mul(b, a), "gf256Mul($a,$b) commutative")
            }
        }

        @Test
        fun `gf256Mul alpha^255 = 1 (period 255)`() {
            var x = 1
            for (i in 0 until 255) x = gf256Mul(x, 2)
            assertEquals(1, x, "2^255 should be 1 in GF(256)/0x12D")
        }

        @Test
        fun `gf256RsEncode output length equals nCheck`() {
            val data = IntArray(10) { it + 1 }
            val ecc = gf256RsEncode(data, 4)
            assertEquals(4, ecc.size)
        }

        @Test
        fun `gf256RsEncode is deterministic`() {
            val data = IntArray(5) { 42 - it }
            val ecc1 = gf256RsEncode(data, 6)
            val ecc2 = gf256RsEncode(data, 6)
            assertArrayEquals(ecc1, ecc2)
        }

        @Test
        fun `gf256RsEncode different data produces different ECC`() {
            val data1 = intArrayOf(1, 2, 3, 4, 5)
            val data2 = intArrayOf(1, 2, 3, 4, 6)
            assertFalse(gf256RsEncode(data1, 4).contentEquals(gf256RsEncode(data2, 4)))
        }
    }

    // =========================================================================
    // 5. Bit stuffing
    // =========================================================================

    @Nested
    inner class BitStuffingTest {

        @Test
        fun `alternating bits produce no stuffing`() {
            val input = intArrayOf(1, 0, 1, 0, 1, 0, 1, 0)
            val result = stuffBits(input)
            assertArrayEquals(input, result)
        }

        @Test
        fun `four identical bits then a stuff bit`() {
            // Four 1s → insert one 0
            val input = intArrayOf(1, 1, 1, 1)
            val result = stuffBits(input)
            assertEquals(5, result.size)
            assertArrayEquals(intArrayOf(1, 1, 1, 1, 0), result)
        }

        @Test
        fun `four zeros then a stuff bit`() {
            val input = intArrayOf(0, 0, 0, 0)
            val result = stuffBits(input)
            assertEquals(5, result.size)
            assertArrayEquals(intArrayOf(0, 0, 0, 0, 1), result)
        }

        @Test
        fun `eight identical bits produce two stuff bits`() {
            // 1 1 1 1 [0] 1 1 1 [0] is NOT right: the stuff resets the counter.
            // Actually after stuffing bit=0, runVal=0, runLen=1.
            // Then next three 1s: runLen=1,2,3 (not 4), so the 8th 1 gives runLen=4
            // but the stuff was after position 4 (value 0), so we won't stuff again.
            // Let's verify with 8 ones:
            // positions 1-4: 1111 → stuff 0, runLen=1 with val=0
            // positions 5-8: four 1s, but runLen restarted from 1 so at pos 8 runLen=4? no
            // After stuff, runVal=0, runLen=1. Position 5 is 1: runVal=1,runLen=1.
            // Position 6 is 1: runLen=2. Position 7 is 1: runLen=3. Position 8 is 1: runLen=4 → stuff.
            val input = IntArray(8) { 1 }
            val result = stuffBits(input)
            // 8 ones = 2 stuffed 0s inserted
            assertEquals(10, result.size)
            assertEquals(0, result[4])  // first stuff bit
            assertEquals(0, result[9])  // second stuff bit
        }

        @Test
        fun `mixed run 1111 0000 produces two stuff bits`() {
            // Input: [1,1,1,1, 0,0,0,0]
            //
            // Trace:
            //   bits 0-3: four 1s → runLen=4 → stuff 0 inserted.  runVal=0, runLen=1
            //   bit 4 (orig 0): runLen=2, push 0
            //   bit 5 (orig 0): runLen=3, push 0
            //   bit 6 (orig 0): runLen=4, push 0 → stuff 1 inserted.  runVal=1, runLen=1
            //   bit 7 (orig 0): bit≠runVal → runVal=0, runLen=1, push 0
            //
            // Output: [1,1,1,1, 0, 0,0,0, 1, 0]  → size 10
            val input = intArrayOf(1, 1, 1, 1, 0, 0, 0, 0)
            val result = stuffBits(input)
            assertEquals(10, result.size)
            assertEquals(0, result[4])  // first stuff bit (after 4x1)
            assertEquals(1, result[8])  // second stuff bit (after 4x0 run that includes the first stuff)
            assertEquals(0, result[9])  // 4th original zero (starts a new run)
        }

        @Test
        fun `output is strictly longer for run of 5 identical bits`() {
            val input = IntArray(5) { 1 }
            val result = stuffBits(input)
            assertTrue(result.size > input.size)
        }
    }

    // =========================================================================
    // 6. Binary-Shift encoding
    // =========================================================================

    @Nested
    inner class BinaryShiftTest {

        @Test
        fun `encodeBytesAsBits starts with 11111 (binary-shift escape)`() {
            val result = encodeBytesAsBits(byteArrayOf(65)) // 'A'
            // First 5 bits should be 1,1,1,1,1
            assertArrayEquals(intArrayOf(1, 1, 1, 1, 1), result.take(5).toIntArray())
        }

        @Test
        fun `encodeBytesAsBits for single byte has correct length`() {
            // 5 (escape) + 5 (len) + 8 (byte) = 18 bits
            val result = encodeBytesAsBits(byteArrayOf(65))
            assertEquals(18, result.size)
        }

        @Test
        fun `encodeBytesAsBits for empty input`() {
            // 5 (escape) + 5 (len=0) = 10 bits, no data bytes
            val result = encodeBytesAsBits(byteArrayOf())
            assertEquals(10, result.size)
            assertArrayEquals(intArrayOf(1, 1, 1, 1, 1), result.take(5).toIntArray())
            // length nibble should be 0
            assertArrayEquals(intArrayOf(0, 0, 0, 0, 0), result.drop(5).toIntArray())
        }

        @Test
        fun `encodeBytesAsBits for 32 bytes uses 11-bit length`() {
            // len > 31 → 5 escape + 5 (zero prefix) + 11 (length) + 32*8 bits
            val data = ByteArray(32) { it.toByte() }
            val result = encodeBytesAsBits(data)
            // 5 + 5 + 11 + 256 = 277
            assertEquals(277, result.size)
            // The 6th..10th bits should be 00000 (zero-length prefix)
            assertArrayEquals(intArrayOf(0, 0, 0, 0, 0), result.slice(5..9).toIntArray())
        }

        @Test
        fun `encodeBytesAsBits correctly encodes byte value`() {
            // 'A' = 0x41 = 0100 0001
            val result = encodeBytesAsBits(byteArrayOf(0x41))
            val bytePos = 10  // after 5-bit escape + 5-bit length
            val encoded = result.drop(bytePos).toIntArray()
            assertArrayEquals(intArrayOf(0, 1, 0, 0, 0, 0, 0, 1), encoded)
        }
    }

    // =========================================================================
    // 7. Symbol size selection
    // =========================================================================

    @Nested
    inner class SymbolSizeTest {

        @Test
        fun `single byte selects compact 1-layer`() {
            // 18 raw bits -> compact layer 1 should fit
            val spec = selectSymbol(18, 23)
            assertTrue(spec.compact)
            assertEquals(1, spec.layers)
        }

        @Test
        fun `large input uses full symbol`() {
            // Force a large bit count
            val spec = selectSymbol(1000, 23)
            assertFalse(spec.compact)
        }

        @Test
        fun `extremely large input throws InputTooLong`() {
            // 11496 * 8 bits = max capacity at 100%, so requesting 0% ECC doesn't
            // but with 23% ECC and dataBits >> totalBits it will fail
            assertThrows(AztecError.InputTooLong::class.java) {
                selectSymbol(999_999, 23)
            }
        }

        @Test
        fun `dataCwCount plus eccCwCount equals total codewords`() {
            val spec = selectSymbol(100, 23)
            // Look up capacity for the selected symbol
            val totalCw = if (spec.compact) {
                when (spec.layers) {
                    1 -> 9; 2 -> 25; 3 -> 49; 4 -> 81; else -> 0
                }
            } else {
                // Full capacity maxBytes8 values
                val fullMax = listOf(0, 11, 27, 45, 65, 87, 111, 137, 165, 195, 227,
                    261, 297, 335, 375, 417, 461, 507, 555, 605, 657, 711,
                    767, 825, 885, 947, 1011, 1077, 1145, 1215, 1287, 1361, 1437)
                fullMax[spec.layers]
            }
            assertEquals(totalCw, spec.dataCwCount + spec.eccCwCount,
                "dataCwCount + eccCwCount should equal total for ${spec.layers} layers compact=${spec.compact}")
        }
    }

    // =========================================================================
    // 8. Mode message encoding
    // =========================================================================

    @Nested
    inner class ModeMessageTest {

        @Test
        fun `compact mode message is 28 bits`() {
            val bits = encodeModeMessage(compact = true, layers = 1, dataCwCount = 5)
            assertEquals(28, bits.size)
        }

        @Test
        fun `full mode message is 40 bits`() {
            val bits = encodeModeMessage(compact = false, layers = 2, dataCwCount = 12)
            assertEquals(40, bits.size)
        }

        @Test
        fun `mode message bits are all 0 or 1`() {
            for (compact in listOf(true, false)) {
                val bits = encodeModeMessage(compact, layers = 1, dataCwCount = 1)
                for (b in bits) assertTrue(b == 0 || b == 1, "bit $b not in {0,1}")
            }
        }

        @Test
        fun `different layer counts produce different mode messages`() {
            val msg1 = encodeModeMessage(compact = true, layers = 1, dataCwCount = 5)
            val msg2 = encodeModeMessage(compact = true, layers = 2, dataCwCount = 5)
            assertFalse(msg1.contentEquals(msg2))
        }

        @Test
        fun `different codeword counts produce different mode messages`() {
            val msg1 = encodeModeMessage(compact = true, layers = 1, dataCwCount = 5)
            val msg2 = encodeModeMessage(compact = true, layers = 1, dataCwCount = 6)
            assertFalse(msg1.contentEquals(msg2))
        }

        @Test
        fun `mode message is deterministic`() {
            val m1 = encodeModeMessage(compact = false, layers = 5, dataCwCount = 40)
            val m2 = encodeModeMessage(compact = false, layers = 5, dataCwCount = 40)
            assertArrayEquals(m1, m2)
        }
    }

    // =========================================================================
    // 9. Symbol geometry helpers
    // =========================================================================

    @Nested
    inner class GeometryTest {

        @ParameterizedTest
        @CsvSource("1,15", "2,19", "3,23", "4,27")
        fun `compact symbol size equals 11 + 4 * layers`(layers: Int, expected: Int) {
            assertEquals(expected, symbolSize(compact = true, layers = layers))
        }

        @ParameterizedTest
        @CsvSource("1,19", "2,23", "3,27", "4,31", "10,55", "32,143")
        fun `full symbol size equals 15 + 4 * layers`(layers: Int, expected: Int) {
            assertEquals(expected, symbolSize(compact = false, layers = layers))
        }

        @Test
        fun `compact bullseye radius is 5`() {
            assertEquals(5, bullseyeRadius(true))
        }

        @Test
        fun `full bullseye radius is 7`() {
            assertEquals(7, bullseyeRadius(false))
        }
    }

    // =========================================================================
    // 10. Full encode — smoke tests
    // =========================================================================

    @Nested
    inner class EncodeSmoke {

        @Test
        fun `encode single character returns non-null ModuleGrid`() {
            val grid = encode("A")
            assertNotNull(grid)
        }

        @Test
        fun `encode returns square grid`() {
            val grid = encode("Hello")
            assertEquals(grid.rows, grid.cols)
        }

        @Test
        fun `encode returns correct module dimensions`() {
            val grid = encode("A")
            assertEquals(grid.rows, grid.modules.size)
            for (row in grid.modules) {
                assertEquals(grid.cols, row.size)
            }
        }

        @Test
        fun `encode is deterministic`() {
            val g1 = encode("Hello World")
            val g2 = encode("Hello World")
            assertEquals(gridToString(g1), gridToString(g2))
        }

        @Test
        fun `encode accepts ByteArray overload`() {
            val byteGrid = encode("A".toByteArray(Charsets.UTF_8))
            val strGrid = encode("A")
            assertEquals(gridToString(strGrid), gridToString(byteGrid))
        }

        @Test
        fun `encode AztecOptions default equals no-options overload`() {
            val g1 = encode("test")
            val g2 = encode("test", AztecOptions())
            assertEquals(gridToString(g1), gridToString(g2))
        }
    }

    // =========================================================================
    // 11. Grid size by input length
    // =========================================================================

    @Nested
    inner class GridSizeTest {

        @Test
        fun `single character gives compact 1-layer 15x15 symbol`() {
            val grid = encode("A")
            assertEquals(15, grid.rows)
            assertEquals(15, grid.cols)
        }

        @Test
        fun `longer input gives a larger symbol`() {
            val small = encode("A")
            val large = encode("A".repeat(200))
            assertTrue(large.rows > small.rows, "More data → larger symbol")
        }

        @ParameterizedTest
        @CsvSource(
            "A,15",              // compact 1-layer (15x15)
            "Hello World,19",    // compact 2-layer (19x19) — 11 bytes needs more capacity
        )
        fun `expected symbol size for inputs`(input: String, expectedSize: Int) {
            val grid = encode(input)
            assertEquals(expectedSize, grid.rows, "Input='$input' should give ${expectedSize}x${expectedSize}")
        }
    }

    // =========================================================================
    // 12. Bullseye pattern validation
    // =========================================================================

    @Nested
    inner class BullseyeTest {

        /**
         * For a compact 1-layer 15×15 symbol, center = (7,7).
         * Bullseye radius = 5 → rows/cols 2..12, center 7..7.
         */
        @Test
        fun `center module is dark (d=0)`() {
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            assertTrue(grid.modules[cy][cx], "Center module (d=0) must be dark")
        }

        @Test
        fun `inner 3x3 core (d at most 1) is fully dark`() {
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            for (row in cy - 1..cy + 1) {
                for (col in cx - 1..cx + 1) {
                    assertTrue(grid.modules[row][col], "Inner core (row=$row,col=$col) must be dark")
                }
            }
        }

        @Test
        fun `ring at d=2 is light (gap ring)`() {
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            // Sample the four cardinal points at d=2
            assertFalse(grid.modules[cy - 2][cx], "d=2 top module must be light")
            assertFalse(grid.modules[cy + 2][cx], "d=2 bottom module must be light")
            assertFalse(grid.modules[cy][cx - 2], "d=2 left module must be light")
            assertFalse(grid.modules[cy][cx + 2], "d=2 right module must be light")
        }

        @Test
        fun `ring at d=3 is dark`() {
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            // d=3 ring is DARK
            assertTrue(grid.modules[cy - 3][cx], "d=3 top module must be dark")
            assertTrue(grid.modules[cy + 3][cx], "d=3 bottom module must be dark")
            assertTrue(grid.modules[cy][cx - 3], "d=3 left module must be dark")
            assertTrue(grid.modules[cy][cx + 3], "d=3 right module must be dark")
        }

        @Test
        fun `ring at d=4 is light`() {
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            assertFalse(grid.modules[cy - 4][cx], "d=4 top module must be light")
            assertFalse(grid.modules[cy + 4][cx], "d=4 bottom module must be light")
        }

        @Test
        fun `ring at d=5 (outermost bullseye) is dark`() {
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            assertTrue(grid.modules[cy - 5][cx], "d=5 top must be dark")
            assertTrue(grid.modules[cy + 5][cx], "d=5 bottom must be dark")
            assertTrue(grid.modules[cy][cx - 5], "d=5 left must be dark")
            assertTrue(grid.modules[cy][cx + 5], "d=5 right must be dark")
        }
    }

    // =========================================================================
    // 13. Orientation marks
    // =========================================================================

    @Nested
    inner class OrientationMarkTest {

        @Test
        fun `all four orientation mark corners are dark in compact 1-layer`() {
            // Compact: bullseye radius = 5, mode ring radius = 6
            val grid = encode("A")
            val cx = grid.cols / 2
            val cy = grid.rows / 2
            val r = 6  // bullseyeRadius(compact=true) + 1
            assertTrue(grid.modules[cy - r][cx - r], "TL corner orientation mark")
            assertTrue(grid.modules[cy - r][cx + r], "TR corner orientation mark")
            assertTrue(grid.modules[cy + r][cx + r], "BR corner orientation mark")
            assertTrue(grid.modules[cy + r][cx - r], "BL corner orientation mark")
        }
    }

    // =========================================================================
    // 14. Error handling
    // =========================================================================

    @Nested
    inner class ErrorHandlingTest {

        @Test
        fun `oversized input throws InputTooLong`() {
            // 2000 bytes well exceeds any symbol capacity
            val bigInput = "X".repeat(2000)
            assertThrows(AztecError.InputTooLong::class.java) {
                encode(bigInput)
            }
        }

        @Test
        fun `empty string encodes successfully`() {
            val grid = encode("")
            assertNotNull(grid)
            assertTrue(grid.rows >= 15, "Empty string should still produce a valid symbol")
        }
    }

    // =========================================================================
    // 15. ModuleGrid immutability
    // =========================================================================

    @Nested
    inner class ImmutabilityTest {

        @Test
        fun `outer module list is unmodifiable`() {
            val grid = encode("A")
            val mutableList = grid.modules as? MutableList<*>
            if (mutableList != null) {
                assertThrows(UnsupportedOperationException::class.java) {
                    @Suppress("UNCHECKED_CAST")
                    (grid.modules as MutableList<Any>).removeAt(0)
                }
            }
            // If not cast-able to MutableList, it's already immutable — pass.
        }

        @Test
        fun `inner row lists are unmodifiable`() {
            val grid = encode("A")
            val firstRow = grid.modules[0]
            val mutableRow = firstRow as? MutableList<*>
            if (mutableRow != null) {
                assertThrows(UnsupportedOperationException::class.java) {
                    @Suppress("UNCHECKED_CAST")
                    (firstRow as MutableList<Boolean>)[0] = !firstRow[0]
                }
            }
        }
    }

    // =========================================================================
    // 16. Cross-language corpus (spec-defined test vectors)
    // =========================================================================
    //
    // These inputs are the canonical cross-language corpus from the spec:
    //   code/specs/aztec-code.md — Test Strategy section.
    //
    // All language implementations must produce identical symbol sizes.

    @Nested
    inner class CrossLanguageCorpus {

        @ParameterizedTest
        @CsvSource(
            // Sizes match both the TypeScript reference implementation and the
            // Kotlin encoder — verified by running encode() and checking grid.rows.
            // All these inputs use the Binary-Shift path (byte mode only, v0.1.0).
            "A,15",
            "Hello World,19",
            "https://example.com,23",
            "01234567890123456789,23",
        )
        fun `corpus inputs produce expected symbol sizes`(input: String, expectedSize: Int) {
            val grid = encode(input)
            assertEquals(expectedSize, grid.rows,
                "Corpus input='$input' expected ${expectedSize}×${expectedSize} grid")
        }

        @Test
        fun `corpus input Hello World is deterministic across calls`() {
            val g1 = encode("Hello World")
            val g2 = encode("Hello World")
            assertEquals(gridToString(g1), gridToString(g2))
        }

        @Test
        fun `corpus URL input encodes successfully`() {
            val grid = encode("https://example.com")
            assertNotNull(grid)
            assertTrue(grid.rows >= 15)
        }

        @Test
        fun `corpus digit-heavy string encodes successfully`() {
            val grid = encode("01234567890123456789")
            assertNotNull(grid)
            assertTrue(grid.rows >= 15)
        }
    }

    // =========================================================================
    // 17. Compact vs. full symbol selection
    // =========================================================================

    @Nested
    inner class VariantSelectionTest {

        @Test
        fun `short string uses compact symbol`() {
            // A single character should fit in compact 1-layer
            val spec = selectSymbol(encodeBytesAsBits("A".toByteArray()).size, 23)
            assertTrue(spec.compact, "Short input should use compact Aztec")
        }

        @Test
        fun `medium string may use compact or full`() {
            // Just verify it encodes without error
            val grid = encode("The quick brown fox jumps over the lazy dog")
            assertNotNull(grid)
            assertTrue(grid.rows >= 15)
        }

        @Test
        fun `very long string uses full symbol`() {
            // 100 bytes
            val input = "X".repeat(100)
            val grid = encode(input)
            // Full symbol minimum size is 19×19
            assertTrue(grid.rows >= 19, "Long input must use full symbol (≥19×19)")
        }
    }

    // =========================================================================
    // 18. Padding
    // =========================================================================

    @Nested
    inner class PaddingTest {

        @Test
        fun `padToBytes produces exactly targetBytes * 8 bits`() {
            val bits = intArrayOf(1, 0, 1)
            val padded = padToBytes(bits, 3)
            assertEquals(24, padded.size)
        }

        @Test
        fun `padToBytes preserves leading bits`() {
            val bits = intArrayOf(1, 0, 1)
            val padded = padToBytes(bits, 1)
            assertEquals(1, padded[0])
            assertEquals(0, padded[1])
            assertEquals(1, padded[2])
        }

        @Test
        fun `padToBytes pads with zeros`() {
            val bits = intArrayOf(1)
            val padded = padToBytes(bits, 1)
            assertEquals(8, padded.size)
            assertEquals(1, padded[0])
            for (i in 1..7) assertEquals(0, padded[i])
        }

        @Test
        fun `padToBytes truncates if bits already longer`() {
            val bits = IntArray(100) { 1 }
            val padded = padToBytes(bits, 8)
            assertEquals(64, padded.size)
        }
    }
}
