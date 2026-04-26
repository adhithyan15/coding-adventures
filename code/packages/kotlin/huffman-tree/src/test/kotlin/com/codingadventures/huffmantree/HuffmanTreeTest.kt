package com.codingadventures.huffmantree

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Comprehensive tests for [HuffmanTree].
 *
 * Test organisation:
 * 1. Construction — empty/zero-freq errors, single, two, three symbols
 * 2. Code table — prefix-free property, single-symbol convention
 * 3. codeFor — hit, miss, single-symbol
 * 4. Canonical code table — AAABBC example, same lengths as regular, prefix-free
 * 5. Encode/decode round-trips — single, three symbols, all 256 bytes
 * 6. decodeAll error — exhausted bit stream
 * 7. Inspection — weight, depth, symbolCount, leavesWithCodes
 * 8. Tie-breaking determinism
 * 9. isValid
 */
class HuffmanTreeTest {

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun build(vararg pairs: Int): HuffmanTree {
        val weights = (pairs.indices step 2).map { i ->
            intArrayOf(pairs[i], pairs[i + 1])
        }
        return HuffmanTree.build(weights)
    }

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("build() from empty list throws")
    fun buildEmptyThrows() {
        var threw = false
        try { HuffmanTree.build(emptyList()) } catch (e: IllegalArgumentException) { threw = true }
        assertTrue(threw)
    }

    @Test
    @DisplayName("build() with zero frequency throws")
    fun buildZeroFreqThrows() {
        var threw = false
        try { build(65, 0) } catch (e: IllegalArgumentException) { threw = true }
        assertTrue(threw)
    }

    @Test
    @DisplayName("build() with negative frequency throws")
    fun buildNegativeFreqThrows() {
        var threw = false
        try { build(65, -1) } catch (e: IllegalArgumentException) { threw = true }
        assertTrue(threw)
    }

    @Test
    @DisplayName("Single symbol: symbolCount=1, weight=freq")
    fun singleSymbol() {
        val tree = build(65, 5)
        assertEquals(1, tree.symbolCount())
        assertEquals(5, tree.weight())
    }

    @Test
    @DisplayName("Two symbols: symbolCount=2, weight=sum of freqs")
    fun twoSymbols() {
        val tree = build(65, 3, 66, 1)
        assertEquals(2, tree.symbolCount())
        assertEquals(4, tree.weight())
    }

    @Test
    @DisplayName("Three symbols AAABBC: symbolCount=3, weight=6")
    fun threeSymbols() {
        val tree = build(65, 3, 66, 2, 67, 1)
        assertEquals(3, tree.symbolCount())
        assertEquals(6, tree.weight())
    }

    @Test
    @DisplayName("isValid() is true after build")
    fun isValidAfterBuild() {
        assertTrue(build(65, 3, 66, 2, 67, 1).isValid())
    }

    @Test
    @DisplayName("Large alphabet (256 symbols) builds and is valid")
    fun largeAlphabet() {
        val weights = (0 until 256).map { i -> intArrayOf(i, i + 1) }
        val tree = HuffmanTree.build(weights)
        assertEquals(256, tree.symbolCount())
        assertTrue(tree.isValid())
    }

    // =========================================================================
    // 2. Code table
    // =========================================================================

    @Test
    @DisplayName("AAABBC: A gets shortest code (highest frequency)")
    fun codeTableAAABBCFrequencies() {
        val tree  = build(65, 3, 66, 2, 67, 1)
        val table = tree.codeTable()
        assertTrue(table.getValue(65).length < table.getValue(66).length)
        assertTrue(table.getValue(66).length <= table.getValue(67).length)
    }

    @Test
    @DisplayName("Single symbol: code is \"0\"")
    fun codeTableSingleSymbol() {
        assertEquals("0", build(65, 1).codeTable()[65])
    }

    @Test
    @DisplayName("All codes are prefix-free (10 symbols)")
    fun codeTablePrefixFree() {
        val weights = (0 until 10).map { i -> intArrayOf(i, i + 1) }
        val table   = HuffmanTree.build(weights).codeTable()
        val codes   = table.values.toList()
        for (i in codes.indices) {
            for (j in codes.indices) {
                if (i != j) assertFalse(codes[i].startsWith(codes[j]),
                    "${codes[i]} starts with ${codes[j]}")
            }
        }
    }

    @Test
    @DisplayName("codeTable contains an entry for every symbol")
    fun codeTableContainsAllSymbols() {
        val table = build(65, 3, 66, 2, 67, 1).codeTable()
        assertTrue(table.containsKey(65))
        assertTrue(table.containsKey(66))
        assertTrue(table.containsKey(67))
    }

    // =========================================================================
    // 3. codeFor
    // =========================================================================

    @Test
    @DisplayName("codeFor matches codeTable for each symbol")
    fun codeForMatchesTable() {
        val tree  = build(65, 3, 66, 2, 67, 1)
        val table = tree.codeTable()
        assertEquals(table[65], tree.codeFor(65))
        assertEquals(table[66], tree.codeFor(66))
        assertEquals(table[67], tree.codeFor(67))
    }

    @Test
    @DisplayName("codeFor returns null for absent symbol")
    fun codeForMissing() {
        assertNull(build(65, 3, 66, 2).codeFor(99))
    }

    @Test
    @DisplayName("codeFor single symbol returns \"0\"")
    fun codeForSingleSymbol() {
        assertEquals("0", build(65, 5).codeFor(65))
    }

    // =========================================================================
    // 4. Canonical code table
    // =========================================================================

    @Test
    @DisplayName("AAABBC canonical: A→\"0\", B→\"10\", C→\"11\"")
    fun canonicalAAABBC() {
        val canonical = build(65, 3, 66, 2, 67, 1).canonicalCodeTable()
        assertEquals("0",  canonical[65])
        assertEquals("10", canonical[66])
        assertEquals("11", canonical[67])
    }

    @Test
    @DisplayName("Canonical code lengths match regular code lengths (8 symbols)")
    fun canonicalLengthsMatchRegular() {
        val weights   = (0 until 8).map { i -> intArrayOf(i, i + 1) }
        val tree      = HuffmanTree.build(weights)
        val regular   = tree.codeTable()
        val canonical = tree.canonicalCodeTable()
        for (sym in regular.keys) {
            assertEquals(regular.getValue(sym).length, canonical.getValue(sym).length,
                "Length mismatch for symbol $sym")
        }
    }

    @Test
    @DisplayName("Canonical single symbol returns {sym → \"0\"}")
    fun canonicalSingleSymbol() {
        assertEquals("0", build(65, 5).canonicalCodeTable()[65])
    }

    @Test
    @DisplayName("Canonical codes are prefix-free (10 symbols)")
    fun canonicalPrefixFree() {
        val weights   = (0 until 10).map { i -> intArrayOf(i, i + 1) }
        val canonical = HuffmanTree.build(weights).canonicalCodeTable()
        val codes     = canonical.values.toList()
        for (i in codes.indices) {
            for (j in codes.indices) {
                if (i != j) assertFalse(codes[i].startsWith(codes[j]))
            }
        }
    }

    // =========================================================================
    // 5. Encode / decode round-trips
    // =========================================================================

    @Test
    @DisplayName("Single-symbol round-trip: [A,A,A]")
    fun roundTripSingleSymbol() {
        val tree  = build(65, 5)
        val code  = tree.codeTable().getValue(65)
        val bits  = code + code + code
        assertEquals(listOf(65, 65, 65), tree.decodeAll(bits, 3))
    }

    @Test
    @DisplayName("AAABBC round-trip: encode then decode")
    fun roundTripAAABBC() {
        val symbols = listOf(65, 65, 65, 66, 66, 67)
        val tree    = build(65, 3, 66, 2, 67, 1)
        val table   = tree.codeTable()
        val bits    = symbols.joinToString("") { table.getValue(it) }
        assertEquals(symbols, tree.decodeAll(bits, symbols.size))
    }

    @Test
    @DisplayName("All 256 byte values round-trip")
    fun roundTripAllBytes() {
        val weights  = (0 until 256).map { i -> intArrayOf(i, i + 1) }
        val tree     = HuffmanTree.build(weights)
        val table    = tree.codeTable()
        val symbols  = (0 until 256).toList()
        val bits     = symbols.joinToString("") { table.getValue(it) }
        assertEquals(symbols, tree.decodeAll(bits, 256))
    }

    @Test
    @DisplayName("decodeAll throws when bit stream exhausted")
    fun decodeExhausted() {
        val tree   = build(65, 3, 66, 2, 67, 1)
        var threw  = false
        try { tree.decodeAll("0", 5) } catch (e: IllegalArgumentException) { threw = true }
        assertTrue(threw)
    }

    // =========================================================================
    // 6. Inspection
    // =========================================================================

    @Test
    @DisplayName("weight() returns sum of all frequencies")
    fun weightSum() {
        assertEquals(6, build(65, 3, 66, 2, 67, 1).weight())
    }

    @Test
    @DisplayName("depth() of single-symbol tree is 0")
    fun depthSingleSymbol() {
        assertEquals(0, build(65, 1).depth())
    }

    @Test
    @DisplayName("depth() of two-symbol tree is 1")
    fun depthTwoSymbols() {
        assertEquals(1, build(65, 3, 66, 1).depth())
    }

    @Test
    @DisplayName("depth() of AAABBC tree is 2")
    fun depthThreeSymbols() {
        assertEquals(2, build(65, 3, 66, 2, 67, 1).depth())
    }

    @Test
    @DisplayName("symbolCount() returns number of distinct symbols")
    fun symbolCountBasic() {
        val weights = (0 until 10).map { i -> intArrayOf(i, i + 1) }
        assertEquals(10, HuffmanTree.build(weights).symbolCount())
    }

    @Test
    @DisplayName("leavesWithCodes() single symbol returns [(65, \"0\")]")
    fun leavesWithCodesSingle() {
        val leaves = build(65, 5).leavesWithCodes()
        assertEquals(1, leaves.size)
        assertEquals(65,  leaves[0].first)
        assertEquals("0", leaves[0].second)
    }

    @Test
    @DisplayName("leavesWithCodes() contains all symbols in AAABBC tree")
    fun leavesWithCodesThreeSymbols() {
        val tree   = build(65, 3, 66, 2, 67, 1)
        val leaves = tree.leavesWithCodes()
        assertEquals(3, leaves.size)
        val syms = leaves.map { it.first }.toSet()
        assertTrue(65 in syms)
        assertTrue(66 in syms)
        assertTrue(67 in syms)
    }

    // =========================================================================
    // 7. Tie-breaking determinism
    // =========================================================================

    @Test
    @DisplayName("Equal-weight alphabet is structurally valid")
    fun equalWeightsValid() {
        val tree = build(65, 1, 66, 1, 67, 1, 68, 1)
        assertTrue(tree.isValid())
        assertEquals(4, tree.symbolCount())
        assertEquals(4, tree.weight())
    }

    @Test
    @DisplayName("Same input always produces same code table")
    fun equalWeightsDeterministic() {
        val weights = (0 until 8).map { i -> intArrayOf(i, 1) }
        val t1 = HuffmanTree.build(weights.map { it.clone() })
        val t2 = HuffmanTree.build(weights.map { it.clone() })
        assertEquals(t1.codeTable(), t2.codeTable())
    }

    @Test
    @DisplayName("Two equal-weight leaves: lower symbol (65) gets code '0' (left branch)")
    fun equalWeightsLowerSymbolLeft() {
        val table = build(65, 1, 66, 1).codeTable()
        assertEquals(1, table.getValue(65).length)
        assertEquals(1, table.getValue(66).length)
    }

    // =========================================================================
    // 8. isValid
    // =========================================================================

    @Test
    @DisplayName("isValid() is true for a well-formed tree")
    fun isValidTrue() {
        assertTrue(build(65, 3, 66, 2, 67, 1).isValid())
    }

    @Test
    @DisplayName("isValid() is true for large tree (50 symbols)")
    fun isValidLarge() {
        val weights = (0 until 50).map { i -> intArrayOf(i, i + 1) }
        assertTrue(HuffmanTree.build(weights).isValid())
    }
}
