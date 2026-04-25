package com.codingadventures.segmenttree

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import kotlin.random.Random
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/**
 * Unit tests for [SegmentTree].
 *
 * Mirrors the Java test suite. Uses Kotlin idioms:
 * - `assertFailsWith<IllegalArgumentException>` instead of `assertThrows`
 * - `intArrayOf` / `arrayOf` for input construction
 * - `kotlin.random.Random` for stress tests
 */
class SegmentTreeTest {

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Empty and Single-Element Trees
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("sumTree on empty array — size 0, isEmpty true")
    fun emptyTree_metadata() {
        val st = SegmentTree.sumTree(intArrayOf())
        assertEquals(0, st.size)
        assertTrue(st.isEmpty)
    }

    @Test
    @DisplayName("sumTree on empty array — any query throws")
    fun emptyTree_queryThrows() {
        val st = SegmentTree.sumTree(intArrayOf())
        assertFailsWith<IllegalArgumentException> { st.query(0, 0) }
    }

    @Test
    @DisplayName("sumTree on single element")
    fun singleElement_sumTree() {
        val st = SegmentTree.sumTree(intArrayOf(42))
        assertEquals(42, st.query(0, 0))
        assertEquals(1, st.size)
    }

    @Test
    @DisplayName("minTree on single element")
    fun singleElement_minTree() {
        val st = SegmentTree.minTree(intArrayOf(-7))
        assertEquals(-7, st.query(0, 0))
    }

    @Test
    @DisplayName("maxTree on single element")
    fun singleElement_maxTree() {
        val st = SegmentTree.maxTree(intArrayOf(99))
        assertEquals(99, st.query(0, 0))
    }

    @Test
    @DisplayName("update on single element")
    fun singleElement_update() {
        val st = SegmentTree.sumTree(intArrayOf(10))
        st.update(0, 55)
        assertEquals(55, st.query(0, 0))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Sum Tree — Build and All Queries
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("sumTree spec example: [2,1,5,3,4]")
    fun sumTree_specExample_allQueries() {
        val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))

        assertEquals(15, st.query(0, 4))   // full range
        assertEquals(9,  st.query(1, 3))   // [1..3] = 1+5+3
        assertEquals(3,  st.query(0, 1))   // [0..1] = 2+1
        assertEquals(7,  st.query(3, 4))   // [3..4] = 3+4
        assertEquals(5,  st.query(2, 2))   // single element
    }

    @Test
    @DisplayName("sumTree spec example: update arr[2]=7, re-query")
    fun sumTree_specExample_update() {
        val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))

        assertEquals(9, st.query(1, 3))    // before: 1+5+3 = 9

        st.update(2, 7)                    // arr[2] = 7

        assertEquals(11, st.query(1, 3))   // after: 1+7+3 = 11
        assertEquals(17, st.query(0, 4))   // full: 2+1+7+3+4 = 17
    }

    @Test
    @DisplayName("sumTree: all queries match brute force for 5-element array")
    fun sumTree_bruteForce_5elements() {
        val arr = intArrayOf(2, 1, 5, 3, 4)
        val st = SegmentTree.sumTree(arr)

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r),
                    "sum[$l..$r]")
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Min Tree
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("minTree: all queries match brute force for 6-element array")
    fun minTree_bruteForce() {
        val arr = intArrayOf(5, 3, 7, 1, 9, 2)
        val st = SegmentTree.minTree(arr)

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteMin(arr, l, r), st.query(l, r), "min[$l..$r]")
            }
        }
    }

    @Test
    @DisplayName("minTree: point update propagates correctly")
    fun minTree_update() {
        val arr = intArrayOf(5, 3, 7, 1, 9, 2)
        val st = SegmentTree.minTree(arr)

        assertEquals(1, st.query(0, 5))   // global min

        arr[3] = 10
        st.update(3, 10)

        assertEquals(2, st.query(0, 5))   // new global min is 2

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteMin(arr, l, r), st.query(l, r))
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 4. Max Tree
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("maxTree: all queries match brute force")
    fun maxTree_bruteForce() {
        val arr = intArrayOf(3, -1, 4, 1, 5, 9, 2, 6)
        val st = SegmentTree.maxTree(arr)

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteMax(arr, l, r), st.query(l, r), "max[$l..$r]")
            }
        }
    }

    @Test
    @DisplayName("maxTree: update to new maximum")
    fun maxTree_update_newMax() {
        val arr = intArrayOf(1, 2, 3, 4, 5)
        val st = SegmentTree.maxTree(arr)

        assertEquals(5, st.query(0, 4))

        arr[2] = 100
        st.update(2, 100)

        assertEquals(100, st.query(0, 4))
        assertEquals(100, st.query(1, 3))
        assertEquals(5,   st.query(3, 4))   // unaffected region: max(4,5)=5
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5. GCD Tree
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("gcdTree spec example: [12, 8, 6, 4, 9]")
    fun gcdTree_specExample() {
        val st = SegmentTree.gcdTree(intArrayOf(12, 8, 6, 4, 9))

        assertEquals(2, st.query(0, 2))   // gcd(12, gcd(8, 6)) = 2
        assertEquals(1, st.query(1, 4))   // gcd(8, gcd(6, gcd(4, 9))) = 1
        assertEquals(1, st.query(3, 4))   // gcd(4, 9) = 1
        assertEquals(4, st.query(0, 1))   // gcd(12, 8) = 4
    }

    @Test
    @DisplayName("gcdTree: all queries match brute force")
    fun gcdTree_bruteForce() {
        val arr = intArrayOf(12, 8, 6, 4, 9)
        val st = SegmentTree.gcdTree(arr)

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteGcd(arr, l, r), st.query(l, r), "gcd[$l..$r]")
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 6. toList Reconstruction
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("toList returns original values")
    fun toList_afterBuild() {
        val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))
        assertEquals(listOf(2, 1, 5, 3, 4), st.toList())
    }

    @Test
    @DisplayName("toList reflects point updates")
    fun toList_afterUpdate() {
        val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))
        st.update(2, 99)
        st.update(0, -1)
        assertEquals(listOf(-1, 1, 99, 3, 4), st.toList())
    }

    @Test
    @DisplayName("toList on empty tree returns empty list")
    fun toList_emptyTree() {
        val st = SegmentTree.sumTree(intArrayOf())
        assertEquals(emptyList<Int>(), st.toList())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 7. Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("full-range query equals aggregate of all elements")
    fun fullRangeQuery() {
        val st = SegmentTree.sumTree(intArrayOf(1, 2, 3, 4, 5, 6, 7, 8, 9, 10))
        assertEquals(55, st.query(0, 9))   // 1+2+...+10 = 55
    }

    @Test
    @DisplayName("all elements equal — sum and min/max consistent")
    fun allEqual() {
        val arr = intArrayOf(7, 7, 7, 7, 7)
        assertEquals(35, SegmentTree.sumTree(arr).query(0, 4))
        assertEquals(7,  SegmentTree.minTree(arr).query(0, 4))
        assertEquals(7,  SegmentTree.maxTree(arr).query(0, 4))
    }

    @Test
    @DisplayName("large negative values")
    fun negativeValues() {
        val arr = intArrayOf(-10, -5, -20, -1, -15)
        assertEquals(-20, SegmentTree.minTree(arr).query(0, 4))
        assertEquals(-1,  SegmentTree.maxTree(arr).query(0, 4))
        assertEquals(-51, SegmentTree.sumTree(arr).query(0, 4))
    }

    @Test
    @DisplayName("non-power-of-2 input length")
    fun nonPowerOfTwo() {
        val arr = intArrayOf(1, 2, 3, 4, 5, 6, 7)
        val st = SegmentTree.sumTree(arr)

        assertEquals(28, st.query(0, 6))   // 1+2+...+7 = 28
        assertEquals(9,  st.query(1, 3))   // 2+3+4

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r))
            }
        }
    }

    @Test
    @DisplayName("update to same value leaves tree unchanged")
    fun updateSameValue() {
        val arr = intArrayOf(2, 1, 5, 3, 4)
        val st = SegmentTree.sumTree(arr)
        val before = st.query(0, 4)
        st.update(2, 5)
        assertEquals(before, st.query(0, 4))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 8. Exception Paths
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("query with ql > qr throws IllegalArgumentException")
    fun query_invalidRange_qlGtQr() {
        val st = SegmentTree.sumTree(intArrayOf(1, 2, 3))
        assertFailsWith<IllegalArgumentException> { st.query(2, 1) }
    }

    @Test
    @DisplayName("query with negative ql throws IllegalArgumentException")
    fun query_negativeLeft() {
        val st = SegmentTree.sumTree(intArrayOf(1, 2, 3))
        assertFailsWith<IllegalArgumentException> { st.query(-1, 2) }
    }

    @Test
    @DisplayName("query with qr out of bounds throws IllegalArgumentException")
    fun query_rightOutOfBounds() {
        val st = SegmentTree.sumTree(intArrayOf(1, 2, 3))
        assertFailsWith<IllegalArgumentException> { st.query(0, 3) }
    }

    @Test
    @DisplayName("update with negative index throws IllegalArgumentException")
    fun update_negativeIndex() {
        val st = SegmentTree.sumTree(intArrayOf(1, 2, 3))
        assertFailsWith<IllegalArgumentException> { st.update(-1, 5) }
    }

    @Test
    @DisplayName("update with index out of bounds throws IllegalArgumentException")
    fun update_indexOutOfBounds() {
        val st = SegmentTree.sumTree(intArrayOf(1, 2, 3))
        assertFailsWith<IllegalArgumentException> { st.update(3, 5) }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 9. Multiple Updates
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("multiple sequential updates maintain consistency")
    fun multipleUpdates() {
        val arr = intArrayOf(1, 2, 3, 4, 5)
        val st = SegmentTree.sumTree(arr)

        arr[0] = 10;  st.update(0, 10)
        arr[4] = 20;  st.update(4, 20)
        arr[2] = 0;   st.update(2, 0)

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r),
                    "After updates: sum[$l..$r]")
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 10. Random Stress Test
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("random stress: 200 elements, 200 queries + updates")
    fun stress_randomQueriesAndUpdates() {
        val rng = Random(12345L)
        val n = 200
        val arr = IntArray(n) { rng.nextInt(1000) - 500 }

        val sumSt = SegmentTree.sumTree(arr)
        val minSt = SegmentTree.minTree(arr)
        val maxSt = SegmentTree.maxTree(arr)

        // Spot-check 50 random queries before updates
        repeat(50) {
            val l = rng.nextInt(n)
            val r = l + rng.nextInt(n - l)
            assertEquals(bruteSum(arr, l, r), sumSt.query(l, r))
            assertEquals(bruteMin(arr, l, r), minSt.query(l, r))
            assertEquals(bruteMax(arr, l, r), maxSt.query(l, r))
        }

        // 200 random point updates, spot-check after each
        repeat(200) {
            val idx = rng.nextInt(n)
            val v = rng.nextInt(1000) - 500
            arr[idx] = v
            sumSt.update(idx, v); minSt.update(idx, v); maxSt.update(idx, v)

            val l = rng.nextInt(n)
            val r = l + rng.nextInt(n - l)
            assertEquals(bruteSum(arr, l, r), sumSt.query(l, r))
            assertEquals(bruteMin(arr, l, r), minSt.query(l, r))
            assertEquals(bruteMax(arr, l, r), maxSt.query(l, r))
        }
    }

    @Test
    @DisplayName("large array: 100k elements, spot queries")
    fun stress_largeArray() {
        val n = 100_000
        val rng = Random(99L)
        val arr = IntArray(n) { rng.nextInt(1_000_000) }
        val st = SegmentTree.sumTree(arr)

        assertEquals(bruteSum(arr, 0, n - 1), st.query(0, n - 1))
        assertEquals(bruteSum(arr, 40_000, 59_999), st.query(40_000, 59_999))
        assertEquals(arr[12_345], st.query(12_345, 12_345))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 11. Generic Combine
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("custom combine: range product")
    fun customCombine_product() {
        val arr = arrayOf(2, 3, 4, 5)
        val st = SegmentTree(arr, { a, b -> a * b }, 1)

        assertEquals(120, st.query(0, 3))  // 2*3*4*5 = 120
        assertEquals(24,  st.query(0, 2))  // 2*3*4 = 24
        assertEquals(20,  st.query(2, 3))  // 4*5 = 20
        assertEquals(6,   st.query(0, 1))  // 2*3 = 6
    }

    @Test
    @DisplayName("custom combine: range bitwise OR")
    fun customCombine_bitwiseOr() {
        val arr = arrayOf(0b0001, 0b0010, 0b0100, 0b1000)
        val st = SegmentTree(arr, { a, b -> a or b }, 0)

        assertEquals(0b1111, st.query(0, 3))
        assertEquals(0b0011, st.query(0, 1))
        assertEquals(0b0110, st.query(1, 2))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 12. Parameterized: array sizes 1..20
    // ─────────────────────────────────────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(ints = [1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 20])
    @DisplayName("sumTree brute-force correctness for array sizes 1..20")
    fun sumTree_allSizes_bruteForce(n: Int) {
        val arr = IntArray(n) { it + 1 }  // [1, 2, ..., n]
        val st = SegmentTree.sumTree(arr)

        for (l in arr.indices) {
            for (r in l until arr.size) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r))
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Brute-Force Helpers
    // ─────────────────────────────────────────────────────────────────────────

    private fun bruteSum(arr: IntArray, l: Int, r: Int): Int = (l..r).sumOf { arr[it] }

    private fun bruteMin(arr: IntArray, l: Int, r: Int): Int = (l..r).minOf { arr[it] }

    private fun bruteMax(arr: IntArray, l: Int, r: Int): Int = (l..r).maxOf { arr[it] }

    private fun bruteGcd(arr: IntArray, l: Int, r: Int): Int {
        var g = arr[l]
        for (i in l + 1..r) { var a = g; var b = arr[i]; while (b != 0) { val t = b; b = a % b; a = t }; g = a }
        return g
    }
}
