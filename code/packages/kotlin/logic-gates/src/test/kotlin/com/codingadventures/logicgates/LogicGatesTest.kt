// ============================================================================
// LogicGatesTest.kt — Unit Tests for LogicGates
// ============================================================================
//
// Tests verify every row of every truth table for all seven gates, plus:
//   - NAND-derived gates (functional completeness)
//   - Multi-input AND_N, OR_N, XOR_N
//   - Input validation (illegal values throw IllegalArgumentException)
// ============================================================================

package com.codingadventures.logicgates

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals

class LogicGatesTest {

    // =========================================================================
    // 1. NOT — truth table
    // =========================================================================

    @Test fun not0Returns1() = assertEquals(1, LogicGates.NOT(0))
    @Test fun not1Returns0() = assertEquals(0, LogicGates.NOT(1))

    // =========================================================================
    // 2. AND — truth table
    // =========================================================================

    @Test fun and00() = assertEquals(0, LogicGates.AND(0, 0))
    @Test fun and01() = assertEquals(0, LogicGates.AND(0, 1))
    @Test fun and10() = assertEquals(0, LogicGates.AND(1, 0))
    @Test fun and11() = assertEquals(1, LogicGates.AND(1, 1))

    // =========================================================================
    // 3. OR — truth table
    // =========================================================================

    @Test fun or00() = assertEquals(0, LogicGates.OR(0, 0))
    @Test fun or01() = assertEquals(1, LogicGates.OR(0, 1))
    @Test fun or10() = assertEquals(1, LogicGates.OR(1, 0))
    @Test fun or11() = assertEquals(1, LogicGates.OR(1, 1))

    // =========================================================================
    // 4. XOR — truth table
    // =========================================================================

    @Test fun xor00() = assertEquals(0, LogicGates.XOR(0, 0))
    @Test fun xor01() = assertEquals(1, LogicGates.XOR(0, 1))
    @Test fun xor10() = assertEquals(1, LogicGates.XOR(1, 0))
    @Test fun xor11() = assertEquals(0, LogicGates.XOR(1, 1))

    // =========================================================================
    // 5. NAND — truth table
    // =========================================================================

    @Test fun nand00() = assertEquals(1, LogicGates.NAND(0, 0))
    @Test fun nand01() = assertEquals(1, LogicGates.NAND(0, 1))
    @Test fun nand10() = assertEquals(1, LogicGates.NAND(1, 0))
    @Test fun nand11() = assertEquals(0, LogicGates.NAND(1, 1))

    // =========================================================================
    // 6. NOR — truth table
    // =========================================================================

    @Test fun nor00() = assertEquals(1, LogicGates.NOR(0, 0))
    @Test fun nor01() = assertEquals(0, LogicGates.NOR(0, 1))
    @Test fun nor10() = assertEquals(0, LogicGates.NOR(1, 0))
    @Test fun nor11() = assertEquals(0, LogicGates.NOR(1, 1))

    // =========================================================================
    // 7. XNOR — truth table
    // =========================================================================

    @Test fun xnor00() = assertEquals(1, LogicGates.XNOR(0, 0))
    @Test fun xnor01() = assertEquals(0, LogicGates.XNOR(0, 1))
    @Test fun xnor10() = assertEquals(0, LogicGates.XNOR(1, 0))
    @Test fun xnor11() = assertEquals(1, LogicGates.XNOR(1, 1))

    // =========================================================================
    // 8. NAND-derived gates match the originals (functional completeness)
    // =========================================================================

    @Test
    fun nandNotMatchesNot() {
        assertEquals(LogicGates.NOT(0), LogicGates.nandNOT(0))
        assertEquals(LogicGates.NOT(1), LogicGates.nandNOT(1))
    }

    @Test
    fun nandAndMatchesAnd() {
        for (a in 0..1) for (b in 0..1) {
            assertEquals(
                LogicGates.AND(a, b),
                LogicGates.nandAND(a, b),
                "nandAND($a,$b) should match AND"
            )
        }
    }

    @Test
    fun nandOrMatchesOr() {
        for (a in 0..1) for (b in 0..1) {
            assertEquals(
                LogicGates.OR(a, b),
                LogicGates.nandOR(a, b),
                "nandOR($a,$b) should match OR"
            )
        }
    }

    @Test
    fun nandXorMatchesXor() {
        for (a in 0..1) for (b in 0..1) {
            assertEquals(
                LogicGates.XOR(a, b),
                LogicGates.nandXOR(a, b),
                "nandXOR($a,$b) should match XOR"
            )
        }
    }

    // =========================================================================
    // 9. AND_N
    // =========================================================================

    @Test fun andNAllOnes()       = assertEquals(1, LogicGates.AND_N(1, 1, 1, 1))
    @Test fun andNWithOneZero()   = assertEquals(0, LogicGates.AND_N(1, 1, 0, 1))

    @Test
    fun andN2Inputs() {
        assertEquals(1, LogicGates.AND_N(1, 1))
        assertEquals(0, LogicGates.AND_N(1, 0))
    }

    @Test
    fun andNRequiresAtLeast2Inputs() {
        assertThrows<IllegalArgumentException> { LogicGates.AND_N(1) }
        assertThrows<IllegalArgumentException> { LogicGates.AND_N() }
    }

    // =========================================================================
    // 10. OR_N
    // =========================================================================

    @Test fun orNAllZeros()     = assertEquals(0, LogicGates.OR_N(0, 0, 0, 0))
    @Test fun orNWithOneOne()   = assertEquals(1, LogicGates.OR_N(0, 0, 1, 0))

    @Test
    fun orN2Inputs() {
        assertEquals(0, LogicGates.OR_N(0, 0))
        assertEquals(1, LogicGates.OR_N(0, 1))
    }

    @Test
    fun orNRequiresAtLeast2Inputs() {
        assertThrows<IllegalArgumentException> { LogicGates.OR_N(0) }
        assertThrows<IllegalArgumentException> { LogicGates.OR_N() }
    }

    // =========================================================================
    // 11. XOR_N (parity)
    // =========================================================================

    @Test fun xorNEmpty()      = assertEquals(0, LogicGates.XOR_N())       // even parity
    @Test fun xorNSingleBit0() = assertEquals(0, LogicGates.XOR_N(0))
    @Test fun xorNSingleBit1() = assertEquals(1, LogicGates.XOR_N(1))

    @Test
    fun xorNEvenNumberOfOnes() {
        assertEquals(0, LogicGates.XOR_N(1, 1))                            // 2 ones
        assertEquals(0, LogicGates.XOR_N(1, 1, 1, 1))                     // 4 ones
        assertEquals(0, LogicGates.XOR_N(0, 0, 0, 0, 0, 0, 1, 1))        // 2 ones in 8
    }

    @Test
    fun xorNOddNumberOfOnes() {
        assertEquals(1, LogicGates.XOR_N(1, 0))                            // 1 one
        assertEquals(1, LogicGates.XOR_N(1, 1, 1))                        // 3 ones
        assertEquals(1, LogicGates.XOR_N(0, 0, 0, 0, 0, 0, 0, 1))        // 1 one in 8
    }

    // =========================================================================
    // 12. Input validation — invalid values
    // =========================================================================

    @Test fun notRejectsNegative()  = assertThrows<IllegalArgumentException> { LogicGates.NOT(-1) }
    @Test fun notRejects2()         = assertThrows<IllegalArgumentException> { LogicGates.NOT(2) }
    @Test fun andRejectsInvalidA()  = assertThrows<IllegalArgumentException> { LogicGates.AND(2, 1) }
    @Test fun andRejectsInvalidB()  = assertThrows<IllegalArgumentException> { LogicGates.AND(1, -1) }
    @Test fun orRejectsInvalid()    = assertThrows<IllegalArgumentException> { LogicGates.OR(5, 0) }
    @Test fun xorRejectsInvalid()   = assertThrows<IllegalArgumentException> { LogicGates.XOR(0, 2) }
    @Test fun nandRejectsInvalid()  = assertThrows<IllegalArgumentException> { LogicGates.NAND(3, 1) }
    @Test fun norRejectsInvalid()   = assertThrows<IllegalArgumentException> { LogicGates.NOR(0, -1) }
    @Test fun xnorRejectsInvalid()  = assertThrows<IllegalArgumentException> { LogicGates.XNOR(2, 0) }

    @Test fun andNRejectsInvalidInput() =
        assertThrows<IllegalArgumentException> { LogicGates.AND_N(1, 2) }

    @Test fun orNRejectsInvalidInput() =
        assertThrows<IllegalArgumentException> { LogicGates.OR_N(1, -1) }

    @Test fun xorNRejectsInvalidInput() =
        assertThrows<IllegalArgumentException> { LogicGates.XOR_N(1, 2) }

    // =========================================================================
    // 13. Cross-consistency checks
    // =========================================================================

    @Test
    fun nandIsNotAnd() {
        // NAND(a,b) = NOT(AND(a,b)) for all inputs
        for (a in 0..1) for (b in 0..1) {
            assertEquals(LogicGates.NOT(LogicGates.AND(a, b)), LogicGates.NAND(a, b))
        }
    }

    @Test
    fun norIsNotOr() {
        for (a in 0..1) for (b in 0..1) {
            assertEquals(LogicGates.NOT(LogicGates.OR(a, b)), LogicGates.NOR(a, b))
        }
    }

    @Test
    fun xnorIsNotXor() {
        for (a in 0..1) for (b in 0..1) {
            assertEquals(LogicGates.NOT(LogicGates.XOR(a, b)), LogicGates.XNOR(a, b))
        }
    }

    @Test
    fun deMorganAndToNand() {
        // De Morgan: NOT(A AND B) = NOT(A) OR NOT(B)
        for (a in 0..1) for (b in 0..1) {
            val lhs = LogicGates.NAND(a, b)
            val rhs = LogicGates.OR(LogicGates.NOT(a), LogicGates.NOT(b))
            assertEquals(lhs, rhs, "De Morgan: NAND($a,$b)")
        }
    }

    @Test
    fun deMorganOrToNor() {
        // De Morgan: NOT(A OR B) = NOT(A) AND NOT(B)
        for (a in 0..1) for (b in 0..1) {
            val lhs = LogicGates.NOR(a, b)
            val rhs = LogicGates.AND(LogicGates.NOT(a), LogicGates.NOT(b))
            assertEquals(lhs, rhs, "De Morgan: NOR($a,$b)")
        }
    }
}
