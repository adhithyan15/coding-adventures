// ============================================================================
// FenwickTreeTest.kt — Unit Tests for FenwickTree
// ============================================================================

package com.codingadventures.fenwicktree

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class FenwickTreeTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    fun constructEmpty() {
        val t = FenwickTree(5)
        assertEquals(5, t.capacity)
        assertEquals(0L, t.prefixSum(5))
    }

    @Test
    fun constructFromArray() {
        val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))
        assertEquals(5, t.capacity)
        assertEquals(3L,  t.prefixSum(1))
        assertEquals(5L,  t.prefixSum(2))
        assertEquals(4L,  t.prefixSum(3))
        assertEquals(10L, t.prefixSum(4))
        assertEquals(15L, t.prefixSum(5))
    }

    @Test
    fun constructFromArraySingleElement() {
        val t = FenwickTree(longArrayOf(42))
        assertEquals(42L, t.prefixSum(1))
    }

    @Test
    fun constructRejectsEmptyArray() {
        assertFailsWith<IllegalArgumentException> { FenwickTree(LongArray(0)) }
    }

    @Test
    fun constructRejectsZeroCapacity() {
        assertFailsWith<IllegalArgumentException> { FenwickTree(0) }
        assertFailsWith<IllegalArgumentException> { FenwickTree(-1) }
    }

    // =========================================================================
    // 2. update / prefixSum — basic
    // =========================================================================

    @Test
    fun updateAndPrefixSumSingle() {
        val t = FenwickTree(5)
        t.update(3, 10)
        assertEquals(0L,  t.prefixSum(2))
        assertEquals(10L, t.prefixSum(3))
        assertEquals(10L, t.prefixSum(5))
    }

    @Test
    fun updateMultiplePositions() {
        val t = FenwickTree(5)
        t.update(1, 3)
        t.update(2, 2)
        t.update(3, -1)
        t.update(4, 6)
        t.update(5, 5)
        assertEquals(3L,  t.prefixSum(1))
        assertEquals(5L,  t.prefixSum(2))
        assertEquals(4L,  t.prefixSum(3))
        assertEquals(10L, t.prefixSum(4))
        assertEquals(15L, t.prefixSum(5))
    }

    @Test
    fun updateNegativeDelta() {
        val t = FenwickTree(3)
        t.update(2, 10)
        t.update(2, -3)
        assertEquals(7L, t.prefixSum(2))
    }

    @Test
    fun updateAllPositions() {
        val n = 10
        val t = FenwickTree(n)
        for (i in 1..n) t.update(i, i.toLong())
        for (i in 1..n) {
            val expected = i.toLong() * (i + 1) / 2
            assertEquals(expected, t.prefixSum(i),
                "prefixSum($i) should be $expected")
        }
    }

    @Test
    fun prefixSumAtOne() {
        val t = FenwickTree(5)
        t.update(1, 7)
        assertEquals(7L, t.prefixSum(1))
    }

    // =========================================================================
    // 3. rangeSum
    // =========================================================================

    @Test
    fun rangeSumFullRange() {
        val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))
        assertEquals(15L, t.rangeSum(1, 5))
    }

    @Test
    fun rangeSumMiddle() {
        val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))
        assertEquals(7L, t.rangeSum(2, 4))  // 2 + (-1) + 6
    }

    @Test
    fun rangeSumSingleElement() {
        val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))
        assertEquals(-1L, t.rangeSum(3, 3))
        assertEquals(6L,  t.rangeSum(4, 4))
    }

    @Test
    fun rangeSumStartsAtOne() {
        val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))
        assertEquals(4L, t.rangeSum(1, 3))
    }

    @Test
    fun rangeSumAfterUpdate() {
        val t = FenwickTree(longArrayOf(1, 2, 3, 4, 5))
        t.update(3, 10)  // arr[3] becomes 13
        assertEquals(19L, t.rangeSum(2, 4))  // 2 + 13 + 4
    }

    // =========================================================================
    // 4. Edge cases and bounds
    // =========================================================================

    @Test
    fun updateRejectsOutOfRange() {
        val t = FenwickTree(5)
        assertFailsWith<IllegalArgumentException> { t.update(0, 1) }
        assertFailsWith<IllegalArgumentException> { t.update(6, 1) }
    }

    @Test
    fun prefixSumRejectsOutOfRange() {
        val t = FenwickTree(5)
        assertFailsWith<IllegalArgumentException> { t.prefixSum(0) }
        assertFailsWith<IllegalArgumentException> { t.prefixSum(6) }
    }

    @Test
    fun rangeSumRejectsLGreaterThanR() {
        val t = FenwickTree(5)
        t.update(2, 1)
        assertFailsWith<IllegalArgumentException> { t.rangeSum(3, 2) }
    }

    @Test
    fun singleElementTree() {
        val t = FenwickTree(1)
        t.update(1, 42)
        assertEquals(42L, t.prefixSum(1))
        assertEquals(42L, t.rangeSum(1, 1))
    }

    // =========================================================================
    // 5. Large dataset smoke test
    // =========================================================================

    @Test
    fun largeDataset() {
        val n = 1000
        val t = FenwickTree(n)
        for (i in 1..n) t.update(i, i.toLong())
        val expected = n.toLong() * (n + 1) / 2
        assertEquals(expected, t.prefixSum(n))
        val lo = 499L * 500 / 2
        assertEquals(expected - lo, t.rangeSum(500, n))
    }
}
