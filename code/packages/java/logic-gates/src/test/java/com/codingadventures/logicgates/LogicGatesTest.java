// ============================================================================
// LogicGatesTest.java — Unit Tests for LogicGates
// ============================================================================
//
// Tests verify every row of every truth table for all seven gates, plus:
//   - NAND-derived gates (functional completeness)
//   - Multi-input AND_N, OR_N, XOR_N
//   - Input validation (illegal values throw IllegalArgumentException)
// ============================================================================

package com.codingadventures.logicgates;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class LogicGatesTest {

    // =========================================================================
    // 1. NOT — truth table
    // =========================================================================

    @Test
    void not0Returns1() {
        assertEquals(1, LogicGates.NOT(0));
    }

    @Test
    void not1Returns0() {
        assertEquals(0, LogicGates.NOT(1));
    }

    // =========================================================================
    // 2. AND — truth table
    // =========================================================================

    @Test
    void and00() { assertEquals(0, LogicGates.AND(0, 0)); }
    @Test
    void and01() { assertEquals(0, LogicGates.AND(0, 1)); }
    @Test
    void and10() { assertEquals(0, LogicGates.AND(1, 0)); }
    @Test
    void and11() { assertEquals(1, LogicGates.AND(1, 1)); }

    // =========================================================================
    // 3. OR — truth table
    // =========================================================================

    @Test
    void or00() { assertEquals(0, LogicGates.OR(0, 0)); }
    @Test
    void or01() { assertEquals(1, LogicGates.OR(0, 1)); }
    @Test
    void or10() { assertEquals(1, LogicGates.OR(1, 0)); }
    @Test
    void or11() { assertEquals(1, LogicGates.OR(1, 1)); }

    // =========================================================================
    // 4. XOR — truth table
    // =========================================================================

    @Test
    void xor00() { assertEquals(0, LogicGates.XOR(0, 0)); }
    @Test
    void xor01() { assertEquals(1, LogicGates.XOR(0, 1)); }
    @Test
    void xor10() { assertEquals(1, LogicGates.XOR(1, 0)); }
    @Test
    void xor11() { assertEquals(0, LogicGates.XOR(1, 1)); }

    // =========================================================================
    // 5. NAND — truth table
    // =========================================================================

    @Test
    void nand00() { assertEquals(1, LogicGates.NAND(0, 0)); }
    @Test
    void nand01() { assertEquals(1, LogicGates.NAND(0, 1)); }
    @Test
    void nand10() { assertEquals(1, LogicGates.NAND(1, 0)); }
    @Test
    void nand11() { assertEquals(0, LogicGates.NAND(1, 1)); }

    // =========================================================================
    // 6. NOR — truth table
    // =========================================================================

    @Test
    void nor00() { assertEquals(1, LogicGates.NOR(0, 0)); }
    @Test
    void nor01() { assertEquals(0, LogicGates.NOR(0, 1)); }
    @Test
    void nor10() { assertEquals(0, LogicGates.NOR(1, 0)); }
    @Test
    void nor11() { assertEquals(0, LogicGates.NOR(1, 1)); }

    // =========================================================================
    // 7. XNOR — truth table
    // =========================================================================

    @Test
    void xnor00() { assertEquals(1, LogicGates.XNOR(0, 0)); }
    @Test
    void xnor01() { assertEquals(0, LogicGates.XNOR(0, 1)); }
    @Test
    void xnor10() { assertEquals(0, LogicGates.XNOR(1, 0)); }
    @Test
    void xnor11() { assertEquals(1, LogicGates.XNOR(1, 1)); }

    // =========================================================================
    // 8. NAND-derived gates match the originals (functional completeness)
    // =========================================================================

    @Test
    void nandNotMatchesNot() {
        assertEquals(LogicGates.NOT(0), LogicGates.nandNOT(0));
        assertEquals(LogicGates.NOT(1), LogicGates.nandNOT(1));
    }

    @Test
    void nandAndMatchesAnd() {
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                assertEquals(LogicGates.AND(a, b), LogicGates.nandAND(a, b),
                    "nandAND(" + a + "," + b + ") should match AND");
            }
        }
    }

    @Test
    void nandOrMatchesOr() {
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                assertEquals(LogicGates.OR(a, b), LogicGates.nandOR(a, b),
                    "nandOR(" + a + "," + b + ") should match OR");
            }
        }
    }

    @Test
    void nandXorMatchesXor() {
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                assertEquals(LogicGates.XOR(a, b), LogicGates.nandXOR(a, b),
                    "nandXOR(" + a + "," + b + ") should match XOR");
            }
        }
    }

    // =========================================================================
    // 9. AND_N
    // =========================================================================

    @Test
    void andNAllOnes() {
        assertEquals(1, LogicGates.AND_N(1, 1, 1, 1));
    }

    @Test
    void andNWithOneZero() {
        assertEquals(0, LogicGates.AND_N(1, 1, 0, 1));
    }

    @Test
    void andN2Inputs() {
        assertEquals(1, LogicGates.AND_N(1, 1));
        assertEquals(0, LogicGates.AND_N(1, 0));
    }

    @Test
    void andNRequiresAtLeast2Inputs() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.AND_N(1));
        assertThrows(IllegalArgumentException.class, () -> LogicGates.AND_N());
    }

    // =========================================================================
    // 10. OR_N
    // =========================================================================

    @Test
    void orNAllZeros() {
        assertEquals(0, LogicGates.OR_N(0, 0, 0, 0));
    }

    @Test
    void orNWithOneOne() {
        assertEquals(1, LogicGates.OR_N(0, 0, 1, 0));
    }

    @Test
    void orN2Inputs() {
        assertEquals(0, LogicGates.OR_N(0, 0));
        assertEquals(1, LogicGates.OR_N(0, 1));
    }

    @Test
    void orNRequiresAtLeast2Inputs() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.OR_N(0));
        assertThrows(IllegalArgumentException.class, () -> LogicGates.OR_N());
    }

    // =========================================================================
    // 11. XOR_N (parity)
    // =========================================================================

    @Test
    void xorNEmpty() {
        assertEquals(0, LogicGates.XOR_N());  // zero inputs → even parity = 0
    }

    @Test
    void xorNSingleBit0() {
        assertEquals(0, LogicGates.XOR_N(0));
    }

    @Test
    void xorNSingleBit1() {
        assertEquals(1, LogicGates.XOR_N(1));
    }

    @Test
    void xorNEvenNumberOfOnes() {
        assertEquals(0, LogicGates.XOR_N(1, 1));            // 2 ones → even
        assertEquals(0, LogicGates.XOR_N(1, 1, 1, 1));      // 4 ones → even
        assertEquals(0, LogicGates.XOR_N(0, 0, 0, 0, 0, 0, 1, 1)); // 2 ones → even
    }

    @Test
    void xorNOddNumberOfOnes() {
        assertEquals(1, LogicGates.XOR_N(1, 0));            // 1 one → odd
        assertEquals(1, LogicGates.XOR_N(1, 1, 1));         // 3 ones → odd
        assertEquals(1, LogicGates.XOR_N(0, 0, 0, 0, 0, 0, 0, 1)); // 1 one → odd
    }

    // =========================================================================
    // 12. Input validation — invalid values
    // =========================================================================

    @Test
    void notRejectsNegative() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.NOT(-1));
    }

    @Test
    void notRejects2() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.NOT(2));
    }

    @Test
    void andRejectsInvalidA() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.AND(2, 1));
    }

    @Test
    void andRejectsInvalidB() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.AND(1, -1));
    }

    @Test
    void orRejectsInvalid() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.OR(5, 0));
    }

    @Test
    void xorRejectsInvalid() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.XOR(0, 2));
    }

    @Test
    void nandRejectsInvalid() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.NAND(3, 1));
    }

    @Test
    void norRejectsInvalid() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.NOR(0, -1));
    }

    @Test
    void xnorRejectsInvalid() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.XNOR(2, 0));
    }

    @Test
    void andNRejectsInvalidInput() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.AND_N(1, 2));
    }

    @Test
    void orNRejectsInvalidInput() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.OR_N(1, -1));
    }

    @Test
    void xorNRejectsInvalidInput() {
        assertThrows(IllegalArgumentException.class, () -> LogicGates.XOR_N(1, 2));
    }

    // =========================================================================
    // 13. Cross-consistency checks
    // =========================================================================

    @Test
    void nandIsNotAnd() {
        // NAND(a,b) = NOT(AND(a,b)) for all inputs
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                assertEquals(LogicGates.NOT(LogicGates.AND(a, b)), LogicGates.NAND(a, b));
            }
        }
    }

    @Test
    void norIsNotOr() {
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                assertEquals(LogicGates.NOT(LogicGates.OR(a, b)), LogicGates.NOR(a, b));
            }
        }
    }

    @Test
    void xnorIsNotXor() {
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                assertEquals(LogicGates.NOT(LogicGates.XOR(a, b)), LogicGates.XNOR(a, b));
            }
        }
    }

    @Test
    void deMorganAndToNand() {
        // De Morgan: NOT(A AND B) = NOT(A) OR NOT(B)
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                int lhs = LogicGates.NAND(a, b);
                int rhs = LogicGates.OR(LogicGates.NOT(a), LogicGates.NOT(b));
                assertEquals(lhs, rhs, "De Morgan: NAND(" + a + "," + b + ")");
            }
        }
    }

    @Test
    void deMorganOrToNor() {
        // De Morgan: NOT(A OR B) = NOT(A) AND NOT(B)
        for (int a : new int[]{0, 1}) {
            for (int b : new int[]{0, 1}) {
                int lhs = LogicGates.NOR(a, b);
                int rhs = LogicGates.AND(LogicGates.NOT(a), LogicGates.NOT(b));
                assertEquals(lhs, rhs, "De Morgan: NOR(" + a + "," + b + ")");
            }
        }
    }
}
