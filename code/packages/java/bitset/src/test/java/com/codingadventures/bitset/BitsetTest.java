// ============================================================================
// BitsetTest.java — Unit Tests for Bitset
// ============================================================================
//
// Test strategy:
//   1. Constructors — Bitset(size), fromInteger, fromBinaryStr
//   2. Single-bit ops — set, clear, test, toggle with bounds checks
//   3. Auto-growth — set/toggle beyond current capacity
//   4. Bulk bitwise — and, or, xor, not, andNot (including different lengths)
//   5. Counting/query — popcount, any, all, none
//   6. Iteration — iterSetBits returns correct indices
//   7. Conversion — toInteger, toBinaryStr (roundtrip)
//   8. Clean-trailing-bits invariant — not().popcount() equals length-popcount
//   9. Equality — same bits/length, different capacity, different bits
//  10. Edge cases — empty bitset, length-0, large bit indices
// ============================================================================

package com.codingadventures.bitset;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class BitsetTest {

    // =========================================================================
    // 1. Constructors
    // =========================================================================

    @Test
    void constructorCreatesZeroFilledBitset() {
        Bitset bs = new Bitset(100);
        assertEquals(100, bs.length());
        assertEquals(0, bs.popcount());
        assertFalse(bs.any());
    }

    @Test
    void constructorSizeZeroCreatesEmpty() {
        Bitset bs = new Bitset(0);
        assertEquals(0, bs.length());
        assertEquals(0, bs.capacity());
        assertEquals(0, bs.popcount());
    }

    @Test
    void capacityIsRoundedUpToMultipleOf64() {
        assertEquals(64, new Bitset(1).capacity());
        assertEquals(64, new Bitset(64).capacity());
        assertEquals(128, new Bitset(65).capacity());
        assertEquals(256, new Bitset(200).capacity());
    }

    @Test
    void fromIntegerZeroProducesEmptyBitset() {
        Bitset bs = Bitset.fromInteger(0L);
        assertEquals(0, bs.length());
        assertEquals(0, bs.popcount());
    }

    @Test
    void fromIntegerFive() {
        // 5 = binary 101: bits 0 and 2 are set, length = 3
        Bitset bs = Bitset.fromInteger(5L);
        assertEquals(3, bs.length());
        assertTrue(bs.test(0));
        assertFalse(bs.test(1));
        assertTrue(bs.test(2));
        assertEquals(2, bs.popcount());
    }

    @Test
    void fromIntegerAllBitsSet() {
        // -1L as unsigned is all 64 bits set
        Bitset bs = Bitset.fromInteger(-1L);
        assertEquals(64, bs.length());
        assertEquals(64, bs.popcount());
    }

    @Test
    void fromBinaryStrEmpty() {
        Bitset bs = Bitset.fromBinaryStr("");
        assertEquals(0, bs.length());
    }

    @Test
    void fromBinaryStr101() {
        // "101" → bit 0=1 (rightmost), bit 1=0, bit 2=1 (leftmost)
        Bitset bs = Bitset.fromBinaryStr("101");
        assertEquals(3, bs.length());
        assertTrue(bs.test(0));
        assertFalse(bs.test(1));
        assertTrue(bs.test(2));
    }

    @Test
    void fromBinaryStrAllZeros() {
        Bitset bs = Bitset.fromBinaryStr("0000");
        assertEquals(4, bs.length());
        assertEquals(0, bs.popcount());
    }

    @Test
    void fromBinaryStrInvalidCharThrows() {
        assertThrows(IllegalArgumentException.class, () ->
            Bitset.fromBinaryStr("102"));
        assertThrows(IllegalArgumentException.class, () ->
            Bitset.fromBinaryStr("abc"));
    }

    // =========================================================================
    // 2. Single-bit operations
    // =========================================================================

    @Test
    void setAndTest() {
        Bitset bs = new Bitset(10);
        assertFalse(bs.test(5));
        bs.set(5);
        assertTrue(bs.test(5));
    }

    @Test
    void setIsIdempotent() {
        Bitset bs = new Bitset(10);
        bs.set(3);
        bs.set(3); // second call is a no-op
        assertEquals(1, bs.popcount());
    }

    @Test
    void clearBit() {
        Bitset bs = new Bitset(10);
        bs.set(5);
        assertTrue(bs.test(5));
        bs.clear(5);
        assertFalse(bs.test(5));
    }

    @Test
    void clearOutOfRangeIsNoOp() {
        Bitset bs = new Bitset(5);
        assertDoesNotThrow(() -> bs.clear(100)); // no-op
        assertEquals(5, bs.length()); // length unchanged
    }

    @Test
    void testOutOfRangeReturnsFalse() {
        Bitset bs = new Bitset(5);
        assertFalse(bs.test(100));
    }

    @Test
    void toggleFlipsBit() {
        Bitset bs = new Bitset(10);
        assertFalse(bs.test(7));
        bs.toggle(7);
        assertTrue(bs.test(7));
        bs.toggle(7);
        assertFalse(bs.test(7));
    }

    // =========================================================================
    // 3. Auto-growth
    // =========================================================================

    @Test
    void setAutoGrows() {
        Bitset bs = new Bitset(5);
        assertEquals(5, bs.length());
        bs.set(200);
        assertTrue(bs.length() >= 201);
        assertTrue(bs.test(200));
    }

    @Test
    void toggleAutoGrows() {
        Bitset bs = new Bitset(0);
        bs.toggle(128);
        assertTrue(bs.length() >= 129);
        assertTrue(bs.test(128));
    }

    // =========================================================================
    // 4. Bulk bitwise operations
    // =========================================================================

    @Test
    void andReturnsIntersection() {
        // a = 0b1100, b = 0b1010 → a & b = 0b1000
        Bitset a = Bitset.fromBinaryStr("1100");
        Bitset b = Bitset.fromBinaryStr("1010");
        Bitset result = a.and(b);
        assertEquals("1000", result.toBinaryStr());
    }

    @Test
    void orReturnsUnion() {
        Bitset a = Bitset.fromBinaryStr("1100");
        Bitset b = Bitset.fromBinaryStr("1010");
        Bitset result = a.or(b);
        assertEquals("1110", result.toBinaryStr());
    }

    @Test
    void xorReturnsSymmetricDifference() {
        Bitset a = Bitset.fromBinaryStr("1100");
        Bitset b = Bitset.fromBinaryStr("1010");
        Bitset result = a.xor(b);
        assertEquals("0110", result.toBinaryStr());
    }

    @Test
    void notFlipsAllBitsWithinLength() {
        // fromBinaryStr("101") → length=3, bits 0 and 2 set
        // not() → bits 1 set only → binary "010"
        Bitset a = Bitset.fromBinaryStr("101");
        Bitset result = a.not();
        assertEquals(3, result.length());
        assertEquals("010", result.toBinaryStr());
    }

    @Test
    void notCleanTrailingBitsInvariant() {
        // A bitset of length 3 with 2 bits set.
        // not().popcount() should be 3 - 2 = 1, NOT 64 - 2 = 62.
        Bitset bs = Bitset.fromBinaryStr("101"); // 2 bits set, length 3
        Bitset notBs = bs.not();
        assertEquals(1, notBs.popcount()); // clean trailing bits preserved
    }

    @Test
    void andNotReturnsSetDifference() {
        // a = 0b1100, b = 0b1010 → a & ~b = 0b0100
        Bitset a = Bitset.fromBinaryStr("1100");
        Bitset b = Bitset.fromBinaryStr("1010");
        Bitset result = a.andNot(b);
        assertEquals("0100", result.toBinaryStr());
    }

    @Test
    void bulkOpsDifferentLengths() {
        // a has length 8, b has length 4 — b is zero-extended
        Bitset a = Bitset.fromBinaryStr("11001010"); // length 8
        Bitset b = Bitset.fromBinaryStr("1010");     // length 4
        Bitset result = a.or(b);
        assertEquals(8, result.length());
    }

    // =========================================================================
    // 5. Counting and query operations
    // =========================================================================

    @Test
    void popcountCountsSetBits() {
        Bitset bs = Bitset.fromBinaryStr("10110101"); // bits 0,2,4,5,7 = 5 bits set
        assertEquals(5, bs.popcount());
    }

    @Test
    void popcountEmptyIsZero() {
        assertEquals(0, new Bitset(0).popcount());
        assertEquals(0, new Bitset(100).popcount());
    }

    @Test
    void anyReturnsTrueWhenBitSet() {
        Bitset bs = new Bitset(100);
        assertFalse(bs.any());
        bs.set(50);
        assertTrue(bs.any());
    }

    @Test
    void allReturnsTrueWhenAllBitsSet() {
        Bitset bs = Bitset.fromBinaryStr("111");
        assertTrue(bs.all());
    }

    @Test
    void allReturnsFalseWhenSomeBitsUnset() {
        Bitset bs = Bitset.fromBinaryStr("101");
        assertFalse(bs.all());
    }

    @Test
    void allVacuousTruthForEmptyBitset() {
        assertTrue(new Bitset(0).all()); // vacuous truth
    }

    @Test
    void noneReturnsTrueWhenEmpty() {
        assertTrue(new Bitset(10).none());
    }

    @Test
    void noneReturnsFalseWhenBitSet() {
        Bitset bs = new Bitset(10);
        bs.set(5);
        assertFalse(bs.none());
    }

    // =========================================================================
    // 6. Iteration
    // =========================================================================

    @Test
    void iterSetBitsReturnsCorrectIndices() {
        // 5 = 0b101: bits 0 and 2 are set
        Bitset bs = Bitset.fromInteger(5L);
        List<Integer> bits = bs.iterSetBits();
        assertEquals(List.of(0, 2), bits);
    }

    @Test
    void iterSetBitsOnEmptyReturnsEmpty() {
        assertTrue(new Bitset(0).iterSetBits().isEmpty());
        assertTrue(new Bitset(100).iterSetBits().isEmpty());
    }

    @Test
    void iterSetBitsAscendingOrder() {
        Bitset bs = new Bitset(70);
        bs.set(0);
        bs.set(63);
        bs.set(64);
        bs.set(69);
        List<Integer> bits = bs.iterSetBits();
        assertEquals(List.of(0, 63, 64, 69), bits);
    }

    // =========================================================================
    // 7. Conversion
    // =========================================================================

    @Test
    void toIntegerRoundtrip() {
        for (long v : new long[]{0L, 1L, 5L, 42L, 0x7FFFFFFFFFFFFFFFL}) {
            Bitset bs = Bitset.fromInteger(v);
            assertEquals(v, bs.toInteger());
        }
    }

    @Test
    void toIntegerThrowsWhenTooLarge() {
        Bitset bs = new Bitset(128);
        bs.set(100); // bit 100 is beyond word 0
        assertThrows(ArithmeticException.class, bs::toInteger);
    }

    @Test
    void toBinaryStrRoundtrip() {
        String[] cases = {"", "0", "1", "101", "11001010", "0000"};
        for (String s : cases) {
            assertEquals(s, Bitset.fromBinaryStr(s).toBinaryStr());
        }
    }

    @Test
    void fromIntegerAndBinaryStrAgreement() {
        // 5 = "101"
        Bitset fromInt = Bitset.fromInteger(5L);
        Bitset fromStr = Bitset.fromBinaryStr("101");
        assertEquals(fromInt.toBinaryStr(), fromStr.toBinaryStr());
        assertEquals(fromInt.length(), fromStr.length());
        assertEquals(fromInt, fromStr);
    }

    // =========================================================================
    // 8. Equality
    // =========================================================================

    @Test
    void equalBitsets() {
        Bitset a = Bitset.fromBinaryStr("1010");
        Bitset b = Bitset.fromBinaryStr("1010");
        assertEquals(a, b);
        assertEquals(a.hashCode(), b.hashCode());
    }

    @Test
    void unequalByBits() {
        Bitset a = Bitset.fromBinaryStr("1010");
        Bitset b = Bitset.fromBinaryStr("1110");
        assertNotEquals(a, b);
    }

    @Test
    void unequalByLength() {
        // "0101" and "101" have different lengths even though bits overlap
        Bitset a = Bitset.fromBinaryStr("0101");
        Bitset b = Bitset.fromBinaryStr("101");
        assertNotEquals(a, b);
    }

    @Test
    void equalToSelf() {
        Bitset bs = Bitset.fromBinaryStr("1010");
        assertEquals(bs, bs);
    }

    // =========================================================================
    // 9. toString
    // =========================================================================

    @Test
    void toStringFormatting() {
        assertEquals("Bitset()", Bitset.fromBinaryStr("").toString());
        assertEquals("Bitset(101)", Bitset.fromBinaryStr("101").toString());
    }

    // =========================================================================
    // 10. Security — Input Validation
    // =========================================================================
    //
    // These tests verify the denial-of-service guards added to reject
    // inputs that would otherwise cause unbounded allocation or integer
    // overflow in the doubling loop.

    @Test
    void constructorNegativeSizeThrows() {
        assertThrows(IllegalArgumentException.class, () -> new Bitset(-1));
        assertThrows(IllegalArgumentException.class, () -> new Bitset(Integer.MIN_VALUE));
    }

    @Test
    void constructorExceedsMaxBitsThrows() {
        assertThrows(IllegalArgumentException.class, () -> new Bitset(Bitset.MAX_BITS + 1));
        assertThrows(IllegalArgumentException.class, () -> new Bitset(Integer.MAX_VALUE));
    }

    @Test
    void setNegativeIndexThrows() {
        Bitset bs = new Bitset(10);
        assertThrows(IllegalArgumentException.class, () -> bs.set(-1));
    }

    @Test
    void toggleNegativeIndexThrows() {
        Bitset bs = new Bitset(10);
        assertThrows(IllegalArgumentException.class, () -> bs.toggle(-1));
    }

    @Test
    void setExceedsMaxBitsThrows() {
        Bitset bs = new Bitset(10);
        assertThrows(IllegalArgumentException.class, () -> bs.set(Bitset.MAX_BITS));
        assertThrows(IllegalArgumentException.class, () -> bs.set(Integer.MAX_VALUE));
    }

    @Test
    void clearNegativeIndexThrows() {
        Bitset bs = new Bitset(10);
        assertThrows(IllegalArgumentException.class, () -> bs.clear(-1));
    }

    @Test
    void testNegativeIndexThrows() {
        Bitset bs = new Bitset(10);
        assertThrows(IllegalArgumentException.class, () -> bs.test(-1));
    }

    @Test
    void fromBinaryStrInvalidCharMessageContainsCharAndIndex() {
        // Verify the error message includes the offending character and its
        // index rather than echoing the entire (possibly huge) input string.
        IllegalArgumentException ex = assertThrows(IllegalArgumentException.class,
            () -> Bitset.fromBinaryStr("01X01"));
        String msg = ex.getMessage();
        assertTrue(msg.contains("'X'"), "message should name the bad char");
        assertTrue(msg.contains("2"),   "message should include the index");
    }
}
