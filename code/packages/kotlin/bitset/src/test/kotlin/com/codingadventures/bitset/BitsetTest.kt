// ============================================================================
// BitsetTest.kt — Unit Tests for Bitset
// ============================================================================
//
// Test strategy mirrors the Java implementation with idiomatic Kotlin style.
// Tests cover: constructors, single-bit ops, auto-growth, bulk bitwise ops,
// clean-trailing-bits invariant, counting/query, iteration, conversion
// roundtrips, equality, and edge cases.
// ============================================================================

package com.codingadventures.bitset

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class BitsetTest {

    // =========================================================================
    // 1. Constructors and Factory Methods
    // =========================================================================

    @Test
    fun constructorCreatesZeroFilledBitset() {
        val bs = Bitset(100)
        assertEquals(100, bs.length())
        assertEquals(0, bs.popcount())
        assertFalse(bs.any())
    }

    @Test
    fun constructorSizeZeroCreatesEmpty() {
        val bs = Bitset(0)
        assertEquals(0, bs.length())
        assertEquals(0, bs.capacity())
        assertEquals(0, bs.popcount())
    }

    @Test
    fun capacityIsRoundedUpToMultipleOf64() {
        assertEquals(64, Bitset(1).capacity())
        assertEquals(64, Bitset(64).capacity())
        assertEquals(128, Bitset(65).capacity())
        assertEquals(256, Bitset(200).capacity())
    }

    @Test
    fun fromIntegerZeroProducesEmptyBitset() {
        val bs = Bitset.fromInteger(0L)
        assertEquals(0, bs.length())
        assertEquals(0, bs.popcount())
    }

    @Test
    fun fromIntegerFive() {
        // 5 = 0b101: bits 0 and 2 set, length = 3
        val bs = Bitset.fromInteger(5L)
        assertEquals(3, bs.length())
        assertTrue(bs.test(0))
        assertFalse(bs.test(1))
        assertTrue(bs.test(2))
        assertEquals(2, bs.popcount())
    }

    @Test
    fun fromIntegerAllBitsSet() {
        // -1L as unsigned is all 64 bits set
        val bs = Bitset.fromInteger(-1L)
        assertEquals(64, bs.length())
        assertEquals(64, bs.popcount())
    }

    @Test
    fun fromBinaryStrEmpty() {
        val bs = Bitset.fromBinaryStr("")
        assertEquals(0, bs.length())
    }

    @Test
    fun fromBinaryStr101() {
        val bs = Bitset.fromBinaryStr("101")
        assertEquals(3, bs.length())
        assertTrue(bs.test(0))
        assertFalse(bs.test(1))
        assertTrue(bs.test(2))
    }

    @Test
    fun fromBinaryStrAllZeros() {
        val bs = Bitset.fromBinaryStr("0000")
        assertEquals(4, bs.length())
        assertEquals(0, bs.popcount())
    }

    @Test
    fun fromBinaryStrInvalidCharThrows() {
        assertThrows<IllegalArgumentException> { Bitset.fromBinaryStr("102") }
        assertThrows<IllegalArgumentException> { Bitset.fromBinaryStr("abc") }
    }

    // =========================================================================
    // 2. Single-bit Operations
    // =========================================================================

    @Test
    fun setAndTest() {
        val bs = Bitset(10)
        assertFalse(bs.test(5))
        bs.set(5)
        assertTrue(bs.test(5))
    }

    @Test
    fun setIsIdempotent() {
        val bs = Bitset(10)
        bs.set(3)
        bs.set(3)
        assertEquals(1, bs.popcount())
    }

    @Test
    fun clearBit() {
        val bs = Bitset(10)
        bs.set(5)
        bs.clear(5)
        assertFalse(bs.test(5))
    }

    @Test
    fun clearOutOfRangeIsNoOp() {
        val bs = Bitset(5)
        bs.clear(100) // no-op, no exception
        assertEquals(5, bs.length())
    }

    @Test
    fun testOutOfRangeReturnsFalse() {
        assertFalse(Bitset(5).test(100))
    }

    @Test
    fun toggleFlipsBit() {
        val bs = Bitset(10)
        assertFalse(bs.test(7))
        bs.toggle(7)
        assertTrue(bs.test(7))
        bs.toggle(7)
        assertFalse(bs.test(7))
    }

    // =========================================================================
    // 3. Auto-growth
    // =========================================================================

    @Test
    fun setAutoGrows() {
        val bs = Bitset(5)
        bs.set(200)
        assertTrue(bs.length() >= 201)
        assertTrue(bs.test(200))
    }

    @Test
    fun toggleAutoGrows() {
        val bs = Bitset(0)
        bs.toggle(128)
        assertTrue(bs.length() >= 129)
        assertTrue(bs.test(128))
    }

    // =========================================================================
    // 4. Bulk Bitwise Operations
    // =========================================================================

    @Test
    fun andReturnsIntersection() {
        val a = Bitset.fromBinaryStr("1100")
        val b = Bitset.fromBinaryStr("1010")
        assertEquals("1000", a.and(b).toBinaryStr())
    }

    @Test
    fun orReturnsUnion() {
        val a = Bitset.fromBinaryStr("1100")
        val b = Bitset.fromBinaryStr("1010")
        assertEquals("1110", a.or(b).toBinaryStr())
    }

    @Test
    fun xorReturnsSymmetricDifference() {
        val a = Bitset.fromBinaryStr("1100")
        val b = Bitset.fromBinaryStr("1010")
        assertEquals("0110", a.xor(b).toBinaryStr())
    }

    @Test
    fun notFlipsAllBitsWithinLength() {
        val a = Bitset.fromBinaryStr("101")
        val result = a.not()
        assertEquals(3, result.length())
        assertEquals("010", result.toBinaryStr())
    }

    @Test
    fun notCleanTrailingBitsInvariant() {
        // "101" has 2 bits set, length=3. not() should have 1 bit set, not 63.
        val bs = Bitset.fromBinaryStr("101")
        assertEquals(1, bs.not().popcount())
    }

    @Test
    fun andNotReturnsSetDifference() {
        val a = Bitset.fromBinaryStr("1100")
        val b = Bitset.fromBinaryStr("1010")
        assertEquals("0100", a.andNot(b).toBinaryStr())
    }

    @Test
    fun bulkOpsDifferentLengths() {
        val a = Bitset.fromBinaryStr("11001010") // length 8
        val b = Bitset.fromBinaryStr("1010")     // length 4
        val result = a.or(b)
        assertEquals(8, result.length())
    }

    // =========================================================================
    // 5. Counting and Query Operations
    // =========================================================================

    @Test
    fun popcountCountsSetBits() {
        // "10110101" has bits 0,2,4,5,7 set = 5 bits
        val bs = Bitset.fromBinaryStr("10110101")
        assertEquals(5, bs.popcount())
    }

    @Test
    fun anyReturnsTrueWhenBitSet() {
        val bs = Bitset(100)
        assertFalse(bs.any())
        bs.set(50)
        assertTrue(bs.any())
    }

    @Test
    fun allReturnsTrueWhenAllBitsSet() {
        assertTrue(Bitset.fromBinaryStr("111").all())
    }

    @Test
    fun allReturnsFalseWhenSomeBitsUnset() {
        assertFalse(Bitset.fromBinaryStr("101").all())
    }

    @Test
    fun allVacuousTruthForEmptyBitset() {
        assertTrue(Bitset(0).all())
    }

    @Test
    fun noneReturnsTrueWhenAllZero() {
        assertTrue(Bitset(10).none())
    }

    @Test
    fun noneReturnsFalseWhenBitSet() {
        val bs = Bitset(10)
        bs.set(5)
        assertFalse(bs.none())
    }

    // =========================================================================
    // 6. Iteration
    // =========================================================================

    @Test
    fun iterSetBitsReturnsCorrectIndices() {
        // 5 = 0b101: bits 0 and 2
        assertEquals(listOf(0, 2), Bitset.fromInteger(5L).iterSetBits())
    }

    @Test
    fun iterSetBitsOnEmptyReturnsEmpty() {
        assertTrue(Bitset(0).iterSetBits().isEmpty())
        assertTrue(Bitset(100).iterSetBits().isEmpty())
    }

    @Test
    fun iterSetBitsAscendingOrder() {
        val bs = Bitset(70)
        bs.set(0); bs.set(63); bs.set(64); bs.set(69)
        assertEquals(listOf(0, 63, 64, 69), bs.iterSetBits())
    }

    // =========================================================================
    // 7. Conversion
    // =========================================================================

    @Test
    fun toIntegerRoundtrip() {
        for (v in listOf(0L, 1L, 5L, 42L, Long.MAX_VALUE)) {
            assertEquals(v, Bitset.fromInteger(v).toInteger())
        }
    }

    @Test
    fun toIntegerThrowsWhenTooLarge() {
        val bs = Bitset(128)
        bs.set(100)
        assertThrows<ArithmeticException> { bs.toInteger() }
    }

    @Test
    fun toBinaryStrRoundtrip() {
        for (s in listOf("", "0", "1", "101", "11001010", "0000")) {
            assertEquals(s, Bitset.fromBinaryStr(s).toBinaryStr())
        }
    }

    @Test
    fun fromIntegerAndBinaryStrAgreement() {
        val fromInt = Bitset.fromInteger(5L)
        val fromStr = Bitset.fromBinaryStr("101")
        assertEquals(fromInt.toBinaryStr(), fromStr.toBinaryStr())
        assertEquals(fromInt.length(), fromStr.length())
        assertEquals(fromInt, fromStr)
    }

    // =========================================================================
    // 8. Equality
    // =========================================================================

    @Test
    fun equalBitsets() {
        val a = Bitset.fromBinaryStr("1010")
        val b = Bitset.fromBinaryStr("1010")
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
    }

    @Test
    fun unequalByBits() {
        assertNotEquals(Bitset.fromBinaryStr("1010"), Bitset.fromBinaryStr("1110"))
    }

    @Test
    fun unequalByLength() {
        assertNotEquals(Bitset.fromBinaryStr("0101"), Bitset.fromBinaryStr("101"))
    }

    // =========================================================================
    // 9. toString
    // =========================================================================

    @Test
    fun toStringFormatting() {
        assertEquals("Bitset()", Bitset.fromBinaryStr("").toString())
        assertEquals("Bitset(101)", Bitset.fromBinaryStr("101").toString())
    }

    // =========================================================================
    // 10. Security — Input Validation
    // =========================================================================
    //
    // These tests verify the denial-of-service guards added to reject
    // inputs that would otherwise cause unbounded allocation or integer
    // overflow in the doubling loop.

    @Test
    fun constructorNegativeSizeThrows() {
        assertThrows<IllegalArgumentException> { Bitset(-1) }
        assertThrows<IllegalArgumentException> { Bitset(Int.MIN_VALUE) }
    }

    @Test
    fun constructorExceedsMaxBitsThrows() {
        assertThrows<IllegalArgumentException> { Bitset(Bitset.MAX_BITS + 1) }
        assertThrows<IllegalArgumentException> { Bitset(Int.MAX_VALUE) }
    }

    @Test
    fun setNegativeIndexThrows() {
        val bs = Bitset(10)
        assertThrows<IllegalArgumentException> { bs.set(-1) }
    }

    @Test
    fun toggleNegativeIndexThrows() {
        val bs = Bitset(10)
        assertThrows<IllegalArgumentException> { bs.toggle(-1) }
    }

    @Test
    fun setExceedsMaxBitsThrows() {
        val bs = Bitset(10)
        assertThrows<IllegalArgumentException> { bs.set(Bitset.MAX_BITS) }
        assertThrows<IllegalArgumentException> { bs.set(Int.MAX_VALUE) }
    }

    @Test
    fun fromBinaryStrInvalidCharMessageContainsCharAndIndex() {
        // Verify the error message includes the offending character and its
        // index rather than echoing the entire (possibly large) input string.
        val ex = assertThrows<IllegalArgumentException> {
            Bitset.fromBinaryStr("01X01")
        }
        val msg = ex.message ?: ""
        assertTrue(msg.contains("'X'"), "message should name the bad char")
        assertTrue(msg.contains("2"),   "message should include the index")
    }
}
