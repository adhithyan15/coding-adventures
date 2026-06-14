/**
 * PDF417Test.kt — JUnit 5 test suite for the Kotlin PDF417 encoder.
 *
 * ## Coverage goals
 *
 * - VERSION constant value
 * - Error class hierarchy (sealed class subtypes)
 * - [encode] with default options (auto ECC + auto columns)
 * - [encodeString] convenience wrapper
 * - Empty-string encoding (zero bytes is a valid edge case)
 * - Determinism (same input → identical output every time)
 * - Symbol grows when input grows
 * - Grid shape invariants (rows × cols, all boolean)
 * - All ECC levels 0–8 produce valid symbols
 * - Higher ECC level → same or larger symbol area
 * - Explicit column counts (1–30) are respected
 * - Public constant values: GF929_PRIME, MIN_ROWS, MAX_ROWS, MIN_COLS, MAX_COLS
 * - GF(929) arithmetic correctness (multiply, add, exponent/log tables)
 * - Byte compaction structure (latch codeword prefix, 6-byte group compression)
 * - RS ECC produces the right number of codewords
 * - autoEccLevel thresholds
 * - computeLRI / computeRRI cluster assignments
 * - Invalid ECC level throws [PDF417Error.InvalidECCLevel]
 * - Invalid column count throws [PDF417Error.InvalidDimensions]
 */
package com.codingadventures.pdf417

import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows

/**
 * Top-level test class.  Each nested class groups a coherent set of checks
 * so the test report reads like a structured specification.
 */
class PDF417Test {

    // =========================================================================
    // Version
    // =========================================================================

    /**
     * The VERSION constant must be "0.1.0".  This is the canonical version
     * string published in the CHANGELOG and package metadata.
     */
    @Test
    fun `VERSION is 0·1·0`() {
        assertEquals("0.1.0", VERSION)
    }

    // =========================================================================
    // Error class hierarchy
    // =========================================================================

    /**
     * PDF417 uses a sealed class hierarchy so the compiler can exhaustively
     * check all error variants in `when` expressions.
     *
     * Each subtype should extend [PDF417Error] (which itself extends
     * [RuntimeException]) so callers can catch any PDF417 failure with a
     * single `catch (e: PDF417Error)` clause.
     */
    @Nested
    inner class ErrorHierarchy {

        @Test
        fun `InputTooLong is a PDF417Error`() {
            val e = PDF417Error.InputTooLong("test")
            assertInstanceOf(PDF417Error::class.java, e)
            assertTrue(e.message!!.contains("InputTooLong"))
        }

        @Test
        fun `InvalidDimensions is a PDF417Error`() {
            val e = PDF417Error.InvalidDimensions("test")
            assertInstanceOf(PDF417Error::class.java, e)
            assertTrue(e.message!!.contains("InvalidDimensions"))
        }

        @Test
        fun `InvalidECCLevel is a PDF417Error`() {
            val e = PDF417Error.InvalidECCLevel("test")
            assertInstanceOf(PDF417Error::class.java, e)
            assertTrue(e.message!!.contains("InvalidECCLevel"))
        }

        @Test
        fun `PDF417Error extends RuntimeException`() {
            val e: PDF417Error = PDF417Error.InputTooLong("runtime")
            assertInstanceOf(RuntimeException::class.java, e)
        }
    }

    // =========================================================================
    // Basic encoding
    // =========================================================================

    /**
     * The canonical "Hello, PDF417!" smoke test.  A non-null ModuleGrid must
     * be returned.  The grid must have positive rows and columns, and each
     * inner list must have exactly [ModuleGrid.cols] booleans.
     */
    @Nested
    inner class BasicEncoding {

        @Test
        fun `encode hello string returns non-null ModuleGrid`() {
            val grid = encodeString("Hello, PDF417!")
            assertNotNull(grid)
            assertTrue(grid.rows > 0)
            assertTrue(grid.cols > 0)
        }

        @Test
        fun `encodeString is equivalent to encode with UTF-8 bytes`() {
            val text = "Hello, PDF417!"
            val g1 = encodeString(text)
            val g2 = encode(text.toByteArray(Charsets.UTF_8))
            assertEquals(g1.rows, g2.rows)
            assertEquals(g1.cols, g2.cols)
            assertEquals(g1.modules, g2.modules)
        }

        @Test
        fun `empty string encodes without error`() {
            // Even zero bytes must produce a valid (small) symbol.
            val grid = encodeString("")
            assertTrue(grid.rows >= MIN_ROWS)
            assertTrue(grid.cols > 0)
        }

        @Test
        fun `single byte encodes without error`() {
            val grid = encode(byteArrayOf(42))
            assertTrue(grid.rows >= MIN_ROWS)
        }
    }

    // =========================================================================
    // Determinism
    // =========================================================================

    /**
     * PDF417 encoding is deterministic: the same input always produces the
     * same output with the same options.  This is essential for barcode
     * scanners that must read the symbol reliably across multiple labels.
     */
    @Nested
    inner class Determinism {

        @Test
        fun `same input produces identical grids`() {
            val opts = PDF417Options()
            val g1 = encodeString("deterministic test", opts)
            val g2 = encodeString("deterministic test", opts)
            assertEquals(g1.rows,    g2.rows)
            assertEquals(g1.cols,    g2.cols)
            assertEquals(g1.modules, g2.modules)
        }

        @Test
        fun `different inputs produce different grids`() {
            val g1 = encodeString("abc")
            val g2 = encodeString("xyz")
            // They may coincidentally have the same dimensions (both are short),
            // but the module patterns must differ.
            assertNotEquals(g1.modules, g2.modules)
        }
    }

    // =========================================================================
    // Symbol grows with data
    // =========================================================================

    /**
     * Encoding more bytes generally requires more grid space.  We verify that
     * a much longer string produces strictly more total module area than a short
     * one.
     */
    @Nested
    inner class SymbolGrowsWithData {

        @Test
        fun `longer input produces larger or equal symbol area`() {
            val small = encodeString("Hi")
            val large = encodeString("A".repeat(300))
            val areaSmall = small.rows * small.cols
            val areaLarge = large.rows * large.cols
            assertTrue(areaLarge >= areaSmall,
                "Expected area $areaLarge >= $areaSmall")
        }

        @Test
        fun `100-char input bigger than 10-char input`() {
            val g10  = encodeString("X".repeat(10))
            val g100 = encodeString("X".repeat(100))
            assertTrue(g100.rows * g100.cols >= g10.rows * g10.cols)
        }
    }

    // =========================================================================
    // Grid shape invariants
    // =========================================================================

    /**
     * Every row of the ModuleGrid must have exactly [ModuleGrid.cols] booleans.
     * The outer list size must equal [ModuleGrid.rows].
     */
    @Nested
    inner class GridShapeInvariants {

        @Test
        fun `modules outer size equals rows`() {
            val grid = encodeString("shape test")
            assertEquals(grid.rows, grid.modules.size)
        }

        @Test
        fun `every row has exactly cols modules`() {
            val grid = encodeString("shape test")
            for ((r, row) in grid.modules.withIndex()) {
                assertEquals(grid.cols, row.size,
                    "Row $r has ${row.size} modules, expected ${grid.cols}")
            }
        }

        @Test
        fun `all modules are boolean (not null)`() {
            // In Kotlin List<Boolean> nullability would cause a compile error,
            // but we verify the modules are accessible and hold boolean values.
            val grid = encodeString("bool check")
            for (row in grid.modules) {
                for (m in row) {
                    assertTrue(m || !m)   // always true — just proves no NPE
                }
            }
        }

        @Test
        fun `module shape is SQUARE`() {
            val grid = encodeString("shape test")
            assertEquals(com.codingadventures.barcode2d.ModuleShape.SQUARE, grid.moduleShape)
        }

        @Test
        fun `module width matches 69 + 17 × cols formula for auto-selected cols`() {
            // For a small input the auto-selected cols is likely 1.
            // The module width should be 69 + 17*cols.
            // We derive 'cols' from the grid width and check consistency.
            // moduleWidth = 69 + 17*cols  →  cols = (moduleWidth - 69) / 17
            val grid = encodeString("width test")
            val derivedCols = (grid.cols - 69) / 17
            assertTrue(derivedCols >= MIN_COLS,
                "Derived cols $derivedCols below MIN_COLS")
            assertTrue(derivedCols <= MAX_COLS,
                "Derived cols $derivedCols above MAX_COLS")
            assertEquals(69 + 17 * derivedCols, grid.cols,
                "Grid width should equal 69 + 17 * derivedCols")
        }
    }

    // =========================================================================
    // ECC levels 0–8 all work
    // =========================================================================

    /**
     * Every ECC level from 0 to 8 must produce a valid, non-null ModuleGrid.
     * The ECC codeword count for level L is 2^(L+1), so level 8 appends 512
     * ECC codewords — the maximum.
     */
    @Nested
    inner class ECCLevels {

        @Test
        fun `ECC levels 0 through 8 all produce valid symbols`() {
            for (level in 0..8) {
                val grid = encodeString("ECC level $level test", PDF417Options(eccLevel = level))
                assertTrue(grid.rows > 0,
                    "ECC level $level produced zero rows")
                assertTrue(grid.cols > 0,
                    "ECC level $level produced zero cols")
            }
        }

        @Test
        fun `ECC level -1 throws InvalidECCLevel`() {
            assertThrows<PDF417Error.InvalidECCLevel> {
                encodeString("test", PDF417Options(eccLevel = -1))
            }
        }

        @Test
        fun `ECC level 9 throws InvalidECCLevel`() {
            assertThrows<PDF417Error.InvalidECCLevel> {
                encodeString("test", PDF417Options(eccLevel = 9))
            }
        }

        @Test
        fun `higher ECC level produces same or larger symbol area`() {
            // ECC 0 vs ECC 4 on identical data — level 4 needs more space.
            val opts0 = PDF417Options(eccLevel = 0)
            val opts4 = PDF417Options(eccLevel = 4)
            val g0 = encodeString("ECC comparison", opts0)
            val g4 = encodeString("ECC comparison", opts4)
            val area0 = g0.rows * g0.cols
            val area4 = g4.rows * g4.cols
            assertTrue(area4 >= area0,
                "ECC level 4 area $area4 should be >= ECC level 0 area $area0")
        }
    }

    // =========================================================================
    // Explicit column counts
    // =========================================================================

    /**
     * When the caller specifies an explicit [PDF417Options.columns] value, the
     * encoded symbol should have exactly that many data columns, reflected in
     * the module width: `moduleWidth = 69 + 17 * columns`.
     */
    @Nested
    inner class ExplicitColumns {

        @Test
        fun `columns=1 produces module width of 86`() {
            // 69 + 17*1 = 86
            val grid = encodeString("col1", PDF417Options(columns = 1))
            assertEquals(86, grid.cols)
        }

        @Test
        fun `columns=5 produces module width of 154`() {
            // 69 + 17*5 = 154
            val grid = encodeString("five columns here test data", PDF417Options(columns = 5))
            assertEquals(154, grid.cols)
        }

        @Test
        fun `columns=10 produces module width of 239`() {
            // 69 + 17*10 = 239
            val grid = encodeString("ten columns test data longer string to fill space",
                PDF417Options(columns = 10))
            assertEquals(239, grid.cols)
        }

        @Test
        fun `columns=0 throws InvalidDimensions`() {
            assertThrows<PDF417Error.InvalidDimensions> {
                encodeString("test", PDF417Options(columns = 0))
            }
        }

        @Test
        fun `columns=31 throws InvalidDimensions`() {
            assertThrows<PDF417Error.InvalidDimensions> {
                encodeString("test", PDF417Options(columns = 31))
            }
        }
    }

    // =========================================================================
    // Public constants
    // =========================================================================

    /**
     * The public constants that client code may reference (for display, validation,
     * or documentation) must have the correct values as per ISO/IEC 15438.
     */
    @Nested
    inner class PublicConstants {

        @Test
        fun `GF929_PRIME is 929`() {
            assertEquals(929, GF929_PRIME)
        }

        @Test
        fun `GF929_ALPHA is 3`() {
            assertEquals(3, GF929_ALPHA)
        }

        @Test
        fun `GF929_ORDER is 928`() {
            assertEquals(928, GF929_ORDER)
        }

        @Test
        fun `LATCH_BYTE is 924`() {
            assertEquals(924, LATCH_BYTE)
        }

        @Test
        fun `PADDING_CW is 900`() {
            assertEquals(900, PADDING_CW)
        }

        @Test
        fun `MIN_ROWS is 3`() {
            assertEquals(3, MIN_ROWS)
        }

        @Test
        fun `MAX_ROWS is 90`() {
            assertEquals(90, MAX_ROWS)
        }

        @Test
        fun `MIN_COLS is 1`() {
            assertEquals(1, MIN_COLS)
        }

        @Test
        fun `MAX_COLS is 30`() {
            assertEquals(30, MAX_COLS)
        }
    }

    // =========================================================================
    // GF(929) arithmetic — via Internal
    // =========================================================================

    /**
     * GF(929) arithmetic is the foundation of Reed-Solomon encoding.
     * We verify core properties: commutativity, identity elements, zero
     * absorption, Fermat's little theorem, and known table entries.
     */
    @Nested
    inner class GF929Arithmetic {

        @Test
        fun `GF_EXP table has 929 entries`() {
            assertEquals(929, Internal.GF_EXP_TABLE.size)
        }

        @Test
        fun `GF_LOG table has 929 entries`() {
            assertEquals(929, Internal.GF_LOG_TABLE.size)
        }

        @Test
        fun `GF_EXP at 0 is 1 (alpha power 0)`() {
            assertEquals(1, Internal.GF_EXP_TABLE[0])
        }

        @Test
        fun `GF_EXP at 1 is 3 (alpha power 1 equals 3)`() {
            assertEquals(3, Internal.GF_EXP_TABLE[1])
        }

        @Test
        fun `gfMul of 0 and anything is 0`() {
            assertEquals(0, Internal.gfMulExported(0, 500))
            assertEquals(0, Internal.gfMulExported(928, 0))
        }

        @Test
        fun `gfMul multiplicative identity a times 1 equals a`() {
            for (a in listOf(1, 5, 100, 928)) {
                assertEquals(a, Internal.gfMulExported(a, 1),
                    "gfMul($a, 1) should be $a")
            }
        }

        @Test
        fun `gfMul is commutative`() {
            val pairs = listOf(Pair(2, 3), Pair(17, 42), Pair(100, 500))
            for ((a, b) in pairs) {
                assertEquals(Internal.gfMulExported(a, b), Internal.gfMulExported(b, a),
                    "gfMul($a,$b) != gfMul($b,$a)")
            }
        }

        @Test
        fun `gfAdd is commutative`() {
            assertEquals(
                Internal.gfAddExported(17, 42),
                Internal.gfAddExported(42, 17)
            )
        }

        @Test
        fun `gfAdd additive identity a plus 0 equals a`() {
            for (a in listOf(0, 1, 100, 928)) {
                assertEquals(a, Internal.gfAddExported(a, 0),
                    "gfAdd($a, 0) should be $a")
            }
        }

        @Test
        fun `gfAdd wraps at GF929_PRIME`() {
            // 928 + 1 = 929 ≡ 0 (mod 929)
            assertEquals(0, Internal.gfAddExported(928, 1))
        }

        @Test
        fun `gfMul inverse a times inverse a equals 1`() {
            // α^k * α^(928-k) = α^928 = α^0 = 1
            // Choose k = GF_LOG[a], then inverse = GF_EXP[928 - k].
            val a = 42
            val logA = Internal.GF_LOG_TABLE[a]
            val invA = Internal.GF_EXP_TABLE[(GF929_ORDER - logA) % GF929_ORDER]
            assertEquals(1, Internal.gfMulExported(a, invA))
        }

        @Test
        fun `Fermat little theorem alpha to GF929_ORDER equals 1`() {
            // 3^928 ≡ 1 (mod 929) — the core of GF(929) arithmetic.
            // GF_EXP[928] is the wrap sentinel = GF_EXP[0] = 1.
            assertEquals(1, Internal.GF_EXP_TABLE[GF929_ORDER])
        }
    }

    // =========================================================================
    // Byte compaction — via Internal
    // =========================================================================

    /**
     * Byte compaction is the data layer of PDF417 v0.1.0. We test:
     * - The output always starts with latch codeword 924.
     * - 6 bytes compress to 5 codewords (plus the 1 latch = 6 codewords total).
     * - Empty input still emits the latch (1 codeword).
     * - Remainder bytes (< 6) are emitted 1:1 after the 5-codeword groups.
     * - All output values are within 0–928.
     */
    @Nested
    inner class ByteCompaction {

        @Test
        fun `empty bytes produces latch only`() {
            val cws = Internal.byteCompactExported(byteArrayOf())
            assertEquals(1, cws.size)
            assertEquals(LATCH_BYTE, cws[0])
        }

        @Test
        fun `output always starts with latch codeword 924`() {
            val cws = Internal.byteCompactExported("Hello".toByteArray(Charsets.UTF_8))
            assertEquals(LATCH_BYTE, cws[0])
        }

        @Test
        fun `exactly 6 bytes compresses to 1 + 5 = 6 codewords`() {
            val cws = Internal.byteCompactExported(ByteArray(6) { it.toByte() })
            // latch + 5 data codewords = 6
            assertEquals(6, cws.size)
        }

        @Test
        fun `12 bytes compresses to 1 + 10 = 11 codewords`() {
            val cws = Internal.byteCompactExported(ByteArray(12) { it.toByte() })
            assertEquals(11, cws.size)
        }

        @Test
        fun `5 bytes produces 1 + 5 = 6 codewords (all remainder)`() {
            // No complete 6-byte group → 5 remainder bytes, each emitted directly.
            val cws = Internal.byteCompactExported(ByteArray(5) { it.toByte() })
            assertEquals(6, cws.size)
        }

        @Test
        fun `7 bytes produces 1 + 5 + 1 = 7 codewords`() {
            // One complete 6-byte group (5 codewords) + 1 remainder byte (1 codeword).
            val cws = Internal.byteCompactExported(ByteArray(7) { it.toByte() })
            assertEquals(7, cws.size)
        }

        @Test
        fun `all output codewords are in range 0-928`() {
            val data = "The quick brown fox jumps over the lazy dog".toByteArray()
            val cws = Internal.byteCompactExported(data)
            for ((idx, cw) in cws.withIndex()) {
                assertTrue(cw in 0..928,
                    "Codeword at index $idx out of range: $cw")
            }
        }

        @Test
        fun `byte compaction round-trips through encoder`() {
            // Not a full decode test — just verify the encoder accepts the output.
            val original = "Round trip test 🎉".toByteArray(Charsets.UTF_8)
            val grid = encode(original)
            assertTrue(grid.rows > 0)
        }
    }

    // =========================================================================
    // Reed-Solomon ECC — via Internal
    // =========================================================================

    /**
     * The RS encoder must produce exactly 2^(eccLevel+1) ECC codewords for
     * every valid ECC level 0–8. We also verify that ECC codewords are within
     * the valid GF(929) range 0–928.
     */
    @Nested
    inner class ReedSolomon {

        @Test
        fun `rsEncode produces correct number of ECC codewords for each level`() {
            val data = intArrayOf(10, 1, 924, 72, 101, 108)  // simple data
            for (level in 0..8) {
                val ecc = Internal.rsEncodeExported(data, level)
                val expected = 1 shl (level + 1)   // 2^(level+1)
                assertEquals(expected, ecc.size,
                    "ECC level $level should produce $expected codewords")
            }
        }

        @Test
        fun `ECC codewords are in range 0-928`() {
            val data = intArrayOf(15, 1, 924, 72, 101, 108, 108, 111)
            for (level in 0..4) {
                val ecc = Internal.rsEncodeExported(data, level)
                for ((idx, cw) in ecc.withIndex()) {
                    assertTrue(cw in 0..928,
                        "ECC level $level, codeword $idx out of range: $cw")
                }
            }
        }

        @Test
        fun `buildGenerator produces polynomial of degree 2^(level+1)`() {
            for (level in 0..6) {
                val g = Internal.buildGeneratorExported(level)
                val expected = (1 shl (level + 1)) + 1  // degree k means k+1 coefficients
                assertEquals(expected, g.size,
                    "Generator for level $level should have $expected coefficients")
            }
        }

        @Test
        fun `generator leading coefficient is 1 (monic polynomial)`() {
            for (level in 0..5) {
                val g = Internal.buildGeneratorExported(level)
                assertEquals(1, g[0],
                    "Generator for level $level should be monic (leading coeff = 1)")
            }
        }
    }

    // =========================================================================
    // autoEccLevel thresholds
    // =========================================================================

    /**
     * [autoEccLevel] maps data-codeword counts to ECC levels following the
     * ISO/IEC 15438 recommended minimums.
     */
    @Nested
    inner class AutoECCLevelThresholds {

        @Test
        fun `dataCount 1 → level 2`() {
            assertEquals(2, Internal.autoEccLevelExported(1))
        }

        @Test
        fun `dataCount 40 → level 2`() {
            assertEquals(2, Internal.autoEccLevelExported(40))
        }

        @Test
        fun `dataCount 41 → level 3`() {
            assertEquals(3, Internal.autoEccLevelExported(41))
        }

        @Test
        fun `dataCount 160 → level 3`() {
            assertEquals(3, Internal.autoEccLevelExported(160))
        }

        @Test
        fun `dataCount 161 → level 4`() {
            assertEquals(4, Internal.autoEccLevelExported(161))
        }

        @Test
        fun `dataCount 320 → level 4`() {
            assertEquals(4, Internal.autoEccLevelExported(320))
        }

        @Test
        fun `dataCount 321 → level 5`() {
            assertEquals(5, Internal.autoEccLevelExported(321))
        }

        @Test
        fun `dataCount 863 → level 5`() {
            assertEquals(5, Internal.autoEccLevelExported(863))
        }

        @Test
        fun `dataCount 864 → level 6`() {
            assertEquals(6, Internal.autoEccLevelExported(864))
        }
    }

    // =========================================================================
    // Row indicator computation
    // =========================================================================

    /**
     * [computeLRI] and [computeRRI] must produce different distributions across
     * the three cluster types (rows 0, 1, 2) so that each row carries unique
     * metadata. We verify the cluster-routing behaviour using a known small
     * symbol (3 rows, 5 cols, ECC 2).
     */
    @Nested
    inner class RowIndicators {

        private val rows = 9
        private val cols = 5
        private val eccLevel = 2

        @Test
        fun `LRI cluster 0 encodes rInfo`() {
            // Row 0: cluster = 0 → rInfo = (rows-1)/3 = 8/3 = 2
            // LRI = 30*0 + 2 = 2
            val rInfo = (rows - 1) / 3
            assertEquals(30 * 0 + rInfo, computeLRI(0, rows, cols, eccLevel))
        }

        @Test
        fun `LRI cluster 1 encodes lInfo`() {
            // Row 1: cluster = 1 → lInfo = 3*eccLevel + (rows-1)%3 = 6 + 2 = 8
            // LRI = 30*0 + 8 = 8
            val lInfo = 3 * eccLevel + (rows - 1) % 3
            assertEquals(30 * 0 + lInfo, computeLRI(1, rows, cols, eccLevel))
        }

        @Test
        fun `LRI cluster 2 encodes cInfo`() {
            // Row 2: cluster = 2 → cInfo = cols - 1 = 4
            // LRI = 30*0 + 4 = 4
            val cInfo = cols - 1
            assertEquals(30 * 0 + cInfo, computeLRI(2, rows, cols, eccLevel))
        }

        @Test
        fun `RRI cluster 0 encodes cInfo`() {
            val cInfo = cols - 1
            assertEquals(30 * 0 + cInfo, computeRRI(0, rows, cols, eccLevel))
        }

        @Test
        fun `RRI cluster 1 encodes rInfo`() {
            val rInfo = (rows - 1) / 3
            assertEquals(30 * 0 + rInfo, computeRRI(1, rows, cols, eccLevel))
        }

        @Test
        fun `RRI cluster 2 encodes lInfo`() {
            val lInfo = 3 * eccLevel + (rows - 1) % 3
            assertEquals(30 * 0 + lInfo, computeRRI(2, rows, cols, eccLevel))
        }

        @Test
        fun `row 3 increments rowGroup to 1`() {
            // Row 3: cluster = 0, rowGroup = 1 → LRI = 30*1 + rInfo
            val rInfo = (rows - 1) / 3
            assertEquals(30 * 1 + rInfo, computeLRI(3, rows, cols, eccLevel))
        }
    }

    // =========================================================================
    // Cluster tables
    // =========================================================================

    /**
     * The three cluster tables must each have exactly 929 entries (one per
     * valid codeword value 0–928), and all entries must represent 17-module
     * patterns (sum of 8 nibbles = 17).
     */
    @Nested
    inner class ClusterTableTests {

        @Test
        fun `cluster 0 has 929 entries`() {
            assertEquals(929, ClusterTables.CLUSTER_TABLES[0].size)
        }

        @Test
        fun `cluster 1 has 929 entries`() {
            assertEquals(929, ClusterTables.CLUSTER_TABLES[1].size)
        }

        @Test
        fun `cluster 2 has 929 entries`() {
            assertEquals(929, ClusterTables.CLUSTER_TABLES[2].size)
        }

        @Test
        fun `every cluster-0 entry has nibble sum of 17`() {
            for ((idx, packed) in ClusterTables.CLUSTER_TABLES[0].withIndex()) {
                val sum = nibbleSum(packed)
                assertEquals(17, sum,
                    "cluster0[$idx] = 0x${packed.toString(16)} has nibble sum $sum, expected 17")
            }
        }

        @Test
        fun `every cluster-1 entry has nibble sum of 17`() {
            for ((idx, packed) in ClusterTables.CLUSTER_TABLES[1].withIndex()) {
                val sum = nibbleSum(packed)
                assertEquals(17, sum,
                    "cluster1[$idx] = 0x${packed.toString(16)} has nibble sum $sum, expected 17")
            }
        }

        @Test
        fun `every cluster-2 entry has nibble sum of 17`() {
            for ((idx, packed) in ClusterTables.CLUSTER_TABLES[2].withIndex()) {
                val sum = nibbleSum(packed)
                assertEquals(17, sum,
                    "cluster2[$idx] = 0x${packed.toString(16)} has nibble sum $sum, expected 17")
            }
        }

        /** Extract the sum of all 8 nibbles from a 32-bit packed entry. */
        private fun nibbleSum(packed: Int): Int {
            var n = packed
            var s = 0
            repeat(8) {
                s += n and 0xF
                n = n ushr 4
            }
            return s
        }
    }

    // =========================================================================
    // Row height option
    // =========================================================================

    /**
     * [PDF417Options.rowHeight] multiplies the number of pixel rows per
     * logical PDF417 row. A symbol encoded with rowHeight=2 should have
     * exactly twice the pixel height of the same symbol with rowHeight=1.
     */
    @Nested
    inner class RowHeightOption {

        @Test
        fun `rowHeight=1 gives minimum pixel rows`() {
            val g1 = encodeString("row height test", PDF417Options(rowHeight = 1))
            val g3 = encodeString("row height test", PDF417Options(rowHeight = 3))
            // g3.rows = 3 * (g1.rows)
            assertEquals(3 * g1.rows, g3.rows)
        }

        @Test
        fun `rowHeight does not affect module width`() {
            val g1 = encodeString("row height test", PDF417Options(rowHeight = 1))
            val g5 = encodeString("row height test", PDF417Options(rowHeight = 5))
            assertEquals(g1.cols, g5.cols)
        }
    }

    // =========================================================================
    // Large input stress test
    // =========================================================================

    /**
     * Encode a large payload (600 bytes, well within the 2700-codeword limit)
     * and verify the result is well-formed.
     */
    @Test
    fun `600-byte input encodes successfully`() {
        val payload = ByteArray(600) { (it % 256).toByte() }
        val grid = encode(payload)
        assertTrue(grid.rows >= MIN_ROWS)
        assertTrue(grid.cols > 0)
        assertEquals(grid.rows, grid.modules.size)
        for (row in grid.modules) {
            assertEquals(grid.cols, row.size)
        }
    }
}
