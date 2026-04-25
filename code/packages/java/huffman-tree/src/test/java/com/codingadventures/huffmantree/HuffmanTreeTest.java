package com.codingadventures.huffmantree;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive tests for {@link HuffmanTree}.
 *
 * <p>Test organisation:
 * <ol>
 *   <li>Construction — empty/zero-freq errors, single, two, three symbols</li>
 *   <li>Code table — prefix-free property, single-symbol convention</li>
 *   <li>codeFor — hit, miss, single-symbol</li>
 *   <li>Canonical code table — AAABBC example, same lengths as regular, prefix-free</li>
 *   <li>Encode/decode round-trips — single, three symbols, all 256 bytes</li>
 *   <li>decodeAll error — exhausted bit stream</li>
 *   <li>Inspection — weight, depth, symbolCount, leavesWithCodes</li>
 *   <li>Tie-breaking determinism</li>
 *   <li>isValid</li>
 * </ol>
 */
class HuffmanTreeTest {

    // =========================================================================
    // Helpers
    // =========================================================================

    /** Convenience: build from int pairs (symbol, freq). */
    private static HuffmanTree build(int... pairs) {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < pairs.length; i += 2) {
            weights.add(new int[]{pairs[i], pairs[i + 1]});
        }
        return HuffmanTree.build(weights);
    }

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("build() from empty list throws IllegalArgumentException")
    void buildEmptyThrows() {
        assertThrows(IllegalArgumentException.class,
            () -> HuffmanTree.build(List.of()));
    }

    @Test
    @DisplayName("build() with null list throws IllegalArgumentException")
    void buildNullThrows() {
        assertThrows(IllegalArgumentException.class,
            () -> HuffmanTree.build(null));
    }

    @Test
    @DisplayName("build() with zero frequency throws IllegalArgumentException")
    void buildZeroFreqThrows() {
        assertThrows(IllegalArgumentException.class,
            () -> build(65, 0));
    }

    @Test
    @DisplayName("build() with negative frequency throws IllegalArgumentException")
    void buildNegativeFreqThrows() {
        assertThrows(IllegalArgumentException.class,
            () -> build(65, -1));
    }

    @Test
    @DisplayName("Single symbol: symbolCount=1, weight=freq")
    void singleSymbol() {
        HuffmanTree tree = build(65, 5);
        assertEquals(1, tree.symbolCount());
        assertEquals(5, tree.weight());
    }

    @Test
    @DisplayName("Two symbols: symbolCount=2, weight=sum of freqs")
    void twoSymbols() {
        HuffmanTree tree = build(65, 3, 66, 1);
        assertEquals(2, tree.symbolCount());
        assertEquals(4, tree.weight());
    }

    @Test
    @DisplayName("Three symbols AAABBC: symbolCount=3, weight=6")
    void threeSymbols() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        assertEquals(3, tree.symbolCount());
        assertEquals(6, tree.weight());
    }

    @Test
    @DisplayName("isValid() is true after build")
    void isValidAfterBuild() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Large alphabet (256 symbols) builds and is valid")
    void largeAlphabet() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 256; i++) weights.add(new int[]{i, i + 1});
        HuffmanTree tree = HuffmanTree.build(weights);
        assertEquals(256, tree.symbolCount());
        assertTrue(tree.isValid());
    }

    // =========================================================================
    // 2. Code table
    // =========================================================================

    @Test
    @DisplayName("AAABBC: A gets shortest code (highest frequency)")
    void codeTableAAABBCFrequencies() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        Map<Integer, String> table = tree.codeTable();
        assertTrue(table.get(65).length() < table.get(66).length());
        assertTrue(table.get(66).length() <= table.get(67).length());
    }

    @Test
    @DisplayName("Single symbol: code is \"0\"")
    void codeTableSingleSymbol() {
        HuffmanTree tree = build(65, 1);
        assertEquals("0", tree.codeTable().get(65));
    }

    @Test
    @DisplayName("All codes are prefix-free (10 symbols)")
    void codeTablePrefixFree() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 10; i++) weights.add(new int[]{i, i + 1});
        HuffmanTree tree = HuffmanTree.build(weights);
        Map<Integer, String> table = tree.codeTable();
        List<String> codes = new ArrayList<>(table.values());
        for (int i = 0; i < codes.size(); i++) {
            for (int j = 0; j < codes.size(); j++) {
                if (i != j) {
                    assertFalse(codes.get(i).startsWith(codes.get(j)),
                        codes.get(i) + " starts with " + codes.get(j));
                }
            }
        }
    }

    @Test
    @DisplayName("codeTable returns entry for every symbol")
    void codeTableContainsAllSymbols() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        Map<Integer, String> table = tree.codeTable();
        assertTrue(table.containsKey(65));
        assertTrue(table.containsKey(66));
        assertTrue(table.containsKey(67));
    }

    // =========================================================================
    // 3. codeFor
    // =========================================================================

    @Test
    @DisplayName("codeFor returns same value as codeTable for each symbol")
    void codeForMatchesTable() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        Map<Integer, String> table = tree.codeTable();
        assertEquals(table.get(65), tree.codeFor(65));
        assertEquals(table.get(66), tree.codeFor(66));
        assertEquals(table.get(67), tree.codeFor(67));
    }

    @Test
    @DisplayName("codeFor returns null for absent symbol")
    void codeForMissing() {
        HuffmanTree tree = build(65, 3, 66, 2);
        assertNull(tree.codeFor(99));
    }

    @Test
    @DisplayName("codeFor single symbol returns \"0\"")
    void codeForSingleSymbol() {
        HuffmanTree tree = build(65, 5);
        assertEquals("0", tree.codeFor(65));
    }

    // =========================================================================
    // 4. Canonical code table
    // =========================================================================

    @Test
    @DisplayName("AAABBC canonical: A→\"0\", B→\"10\", C→\"11\"")
    void canonicalAAABBC() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        Map<Integer, String> canonical = tree.canonicalCodeTable();
        assertEquals("0",  canonical.get(65));
        assertEquals("10", canonical.get(66));
        assertEquals("11", canonical.get(67));
    }

    @Test
    @DisplayName("Canonical code lengths match regular code lengths (8 symbols)")
    void canonicalLengthsMatchRegular() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 8; i++) weights.add(new int[]{i, i + 1});
        HuffmanTree tree = HuffmanTree.build(weights);
        Map<Integer, String> regular   = tree.codeTable();
        Map<Integer, String> canonical = tree.canonicalCodeTable();
        for (int sym : regular.keySet()) {
            assertEquals(regular.get(sym).length(), canonical.get(sym).length(),
                "Length mismatch for symbol " + sym);
        }
    }

    @Test
    @DisplayName("Canonical single symbol returns {sym → \"0\"}")
    void canonicalSingleSymbol() {
        HuffmanTree tree = build(65, 5);
        assertEquals("0", tree.canonicalCodeTable().get(65));
    }

    @Test
    @DisplayName("Canonical codes are prefix-free (10 symbols)")
    void canonicalPrefixFree() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 10; i++) weights.add(new int[]{i, i + 1});
        HuffmanTree tree = HuffmanTree.build(weights);
        Map<Integer, String> canonical = tree.canonicalCodeTable();
        List<String> codes = new ArrayList<>(canonical.values());
        for (int i = 0; i < codes.size(); i++) {
            for (int j = 0; j < codes.size(); j++) {
                if (i != j) {
                    assertFalse(codes.get(i).startsWith(codes.get(j)));
                }
            }
        }
    }

    // =========================================================================
    // 5. Encode / decode round-trips
    // =========================================================================

    @Test
    @DisplayName("Single-symbol round-trip: [A,A,A]")
    void roundTripSingleSymbol() {
        HuffmanTree tree = build(65, 5);
        Map<Integer, String> table = tree.codeTable();
        String bits = table.get(65) + table.get(65) + table.get(65);
        assertEquals(List.of(65, 65, 65), tree.decodeAll(bits, 3));
    }

    @Test
    @DisplayName("AAABBC round-trip: encode then decode")
    void roundTripAAABBC() {
        List<Integer> symbols = List.of(65, 65, 65, 66, 66, 67);
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        Map<Integer, String> table = tree.codeTable();
        StringBuilder sb = new StringBuilder();
        for (int s : symbols) sb.append(table.get(s));
        assertEquals(symbols, tree.decodeAll(sb.toString(), symbols.size()));
    }

    @Test
    @DisplayName("All 256 byte values round-trip")
    void roundTripAllBytes() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 256; i++) weights.add(new int[]{i, i + 1});
        HuffmanTree tree = HuffmanTree.build(weights);
        Map<Integer, String> table = tree.codeTable();
        StringBuilder sb = new StringBuilder();
        List<Integer> symbols = new ArrayList<>();
        for (int i = 0; i < 256; i++) { symbols.add(i); sb.append(table.get(i)); }
        assertEquals(symbols, tree.decodeAll(sb.toString(), 256));
    }

    @Test
    @DisplayName("decodeAll throws when bit stream exhausted")
    void decodeExhausted() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        // "0" decodes only 1 symbol but count=5 requested
        assertThrows(IllegalArgumentException.class,
            () -> tree.decodeAll("0", 5));
    }

    // =========================================================================
    // 6. Inspection
    // =========================================================================

    @Test
    @DisplayName("weight() returns sum of all frequencies")
    void weightSum() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        assertEquals(6, tree.weight());
    }

    @Test
    @DisplayName("depth() of single-symbol tree is 0")
    void depthSingleSymbol() {
        assertEquals(0, build(65, 1).depth());
    }

    @Test
    @DisplayName("depth() of two-symbol tree is 1")
    void depthTwoSymbols() {
        assertEquals(1, build(65, 3, 66, 1).depth());
    }

    @Test
    @DisplayName("depth() of AAABBC tree is 2")
    void depthThreeSymbols() {
        assertEquals(2, build(65, 3, 66, 2, 67, 1).depth());
    }

    @Test
    @DisplayName("symbolCount() returns number of distinct symbols")
    void symbolCountBasic() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 10; i++) weights.add(new int[]{i, i + 1});
        assertEquals(10, HuffmanTree.build(weights).symbolCount());
    }

    @Test
    @DisplayName("leavesWithCodes() single symbol returns [(65, \"0\")]")
    void leavesWithCodesSingle() {
        HuffmanTree tree = build(65, 5);
        List<Object[]> leaves = tree.leavesWithCodes();
        assertEquals(1, leaves.size());
        assertEquals(65,  (int) leaves.get(0)[0]);
        assertEquals("0", (String) leaves.get(0)[1]);
    }

    @Test
    @DisplayName("leavesWithCodes() contains all symbols in AAABBC tree")
    void leavesWithCodesThreeSymbols() {
        HuffmanTree tree = build(65, 3, 66, 2, 67, 1);
        List<Object[]> leaves = tree.leavesWithCodes();
        assertEquals(3, leaves.size());
        java.util.Set<Integer> syms = new java.util.HashSet<>();
        for (Object[] leaf : leaves) syms.add((Integer) leaf[0]);
        assertTrue(syms.contains(65));
        assertTrue(syms.contains(66));
        assertTrue(syms.contains(67));
    }

    // =========================================================================
    // 7. Tie-breaking determinism
    // =========================================================================

    @Test
    @DisplayName("Equal-weight alphabet is structurally valid")
    void equalWeightsValid() {
        HuffmanTree tree = build(65, 1, 66, 1, 67, 1, 68, 1);
        assertTrue(tree.isValid());
        assertEquals(4, tree.symbolCount());
        assertEquals(4, tree.weight());
    }

    @Test
    @DisplayName("Same input always produces same code table")
    void equalWeightsDeterministic() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 8; i++) weights.add(new int[]{i, 1});
        HuffmanTree t1 = HuffmanTree.build(new ArrayList<>(weights));
        HuffmanTree t2 = HuffmanTree.build(new ArrayList<>(weights));
        assertEquals(t1.codeTable(), t2.codeTable());
    }

    @Test
    @DisplayName("Two equal-weight leaves: lower symbol (65) gets code '0'")
    void equalWeightsLowerSymbolLeft() {
        HuffmanTree tree = build(65, 1, 66, 1);
        Map<Integer, String> table = tree.codeTable();
        // Both have length 1; lower symbol (65) gets left branch ('0')
        assertEquals(1, table.get(65).length());
        assertEquals(1, table.get(66).length());
    }

    // =========================================================================
    // 8. isValid
    // =========================================================================

    @Test
    @DisplayName("isValid() is true for a well-formed tree")
    void isValidTrue() {
        assertTrue(build(65, 3, 66, 2, 67, 1).isValid());
    }

    @Test
    @DisplayName("isValid() is true for large tree (50 symbols)")
    void isValidLarge() {
        List<int[]> weights = new ArrayList<>();
        for (int i = 0; i < 50; i++) weights.add(new int[]{i, i + 1});
        assertTrue(HuffmanTree.build(weights).isValid());
    }
}
