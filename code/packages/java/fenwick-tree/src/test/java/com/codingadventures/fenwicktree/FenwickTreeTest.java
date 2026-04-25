// ============================================================================
// FenwickTreeTest.java — Unit Tests for FenwickTree
// ============================================================================

package com.codingadventures.fenwicktree;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class FenwickTreeTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    void constructEmpty() {
        FenwickTree t = new FenwickTree(5);
        assertEquals(5, t.capacity());
        assertEquals(0L, t.prefixSum(5)); // all zeros
    }

    @Test
    void constructFromArray() {
        // arr = [3, 2, -1, 6, 5]  (0-indexed)
        FenwickTree t = new FenwickTree(new long[]{3, 2, -1, 6, 5});
        assertEquals(5, t.capacity());
        assertEquals(3L,  t.prefixSum(1));  // 3
        assertEquals(5L,  t.prefixSum(2));  // 3+2
        assertEquals(4L,  t.prefixSum(3));  // 3+2-1
        assertEquals(10L, t.prefixSum(4));  // 3+2-1+6
        assertEquals(15L, t.prefixSum(5));  // 3+2-1+6+5
    }

    @Test
    void constructFromArraySingleElement() {
        FenwickTree t = new FenwickTree(new long[]{42});
        assertEquals(42L, t.prefixSum(1));
    }

    @Test
    void constructRejectsEmptyArray() {
        assertThrows(IllegalArgumentException.class, () -> new FenwickTree(new long[]{}));
        assertThrows(IllegalArgumentException.class, () -> new FenwickTree((long[]) null));
    }

    @Test
    void constructRejectsZeroCapacity() {
        assertThrows(IllegalArgumentException.class, () -> new FenwickTree(0));
        assertThrows(IllegalArgumentException.class, () -> new FenwickTree(-1));
    }

    // =========================================================================
    // 2. update / prefixSum — basic
    // =========================================================================

    @Test
    void updateAndPrefixSumSingle() {
        FenwickTree t = new FenwickTree(5);
        t.update(3, 10);
        assertEquals(0L,  t.prefixSum(2));  // position 3 not yet reached
        assertEquals(10L, t.prefixSum(3));
        assertEquals(10L, t.prefixSum(5));  // no other elements
    }

    @Test
    void updateMultiplePositions() {
        FenwickTree t = new FenwickTree(5);
        t.update(1, 3);
        t.update(2, 2);
        t.update(3, -1);
        t.update(4, 6);
        t.update(5, 5);

        assertEquals(3L,  t.prefixSum(1));
        assertEquals(5L,  t.prefixSum(2));
        assertEquals(4L,  t.prefixSum(3));
        assertEquals(10L, t.prefixSum(4));
        assertEquals(15L, t.prefixSum(5));
    }

    @Test
    void updateNegativeDelta() {
        FenwickTree t = new FenwickTree(3);
        t.update(2, 10);
        t.update(2, -3);
        assertEquals(7L, t.prefixSum(2));
    }

    @Test
    void updateAllPositions() {
        // Fill array with values 1..n, then check running sum = n*(n+1)/2
        int n = 10;
        FenwickTree t = new FenwickTree(n);
        for (int i = 1; i <= n; i++) t.update(i, i);
        for (int i = 1; i <= n; i++) {
            long expected = (long) i * (i + 1) / 2;
            assertEquals(expected, t.prefixSum(i),
                "prefixSum(" + i + ") should be " + expected);
        }
    }

    @Test
    void prefixSumAtOne() {
        FenwickTree t = new FenwickTree(5);
        t.update(1, 7);
        assertEquals(7L, t.prefixSum(1));
    }

    // =========================================================================
    // 3. rangeSum
    // =========================================================================

    @Test
    void rangeSumFullRange() {
        FenwickTree t = new FenwickTree(new long[]{3, 2, -1, 6, 5});
        assertEquals(15L, t.rangeSum(1, 5));
    }

    @Test
    void rangeSumMiddle() {
        FenwickTree t = new FenwickTree(new long[]{3, 2, -1, 6, 5});
        // 2 + (-1) + 6 = 7
        assertEquals(7L, t.rangeSum(2, 4));
    }

    @Test
    void rangeSumSingleElement() {
        FenwickTree t = new FenwickTree(new long[]{3, 2, -1, 6, 5});
        assertEquals(-1L, t.rangeSum(3, 3));
        assertEquals(6L,  t.rangeSum(4, 4));
    }

    @Test
    void rangeSumStartsAtOne() {
        FenwickTree t = new FenwickTree(new long[]{3, 2, -1, 6, 5});
        assertEquals(4L, t.rangeSum(1, 3));
    }

    @Test
    void rangeSumAfterUpdate() {
        FenwickTree t = new FenwickTree(new long[]{1, 2, 3, 4, 5});
        t.update(3, 10); // arr[3] becomes 13
        // rangeSum(2,4) = 2 + 13 + 4 = 19
        assertEquals(19L, t.rangeSum(2, 4));
    }

    // =========================================================================
    // 4. Edge cases and bounds
    // =========================================================================

    @Test
    void updateRejectsOutOfRange() {
        FenwickTree t = new FenwickTree(5);
        assertThrows(IllegalArgumentException.class, () -> t.update(0, 1));
        assertThrows(IllegalArgumentException.class, () -> t.update(6, 1));
    }

    @Test
    void prefixSumRejectsOutOfRange() {
        FenwickTree t = new FenwickTree(5);
        assertThrows(IllegalArgumentException.class, () -> t.prefixSum(0));
        assertThrows(IllegalArgumentException.class, () -> t.prefixSum(6));
    }

    @Test
    void rangeSumRejectsLGreaterThanR() {
        FenwickTree t = new FenwickTree(5);
        t.update(2, 1);
        t.update(3, 1);
        assertThrows(IllegalArgumentException.class, () -> t.rangeSum(3, 2));
    }

    @Test
    void singleElementTree() {
        FenwickTree t = new FenwickTree(1);
        t.update(1, 42);
        assertEquals(42L, t.prefixSum(1));
        assertEquals(42L, t.rangeSum(1, 1));
    }

    @Test
    void largeNegativeValues() {
        FenwickTree t = new FenwickTree(3);
        t.update(1, Long.MIN_VALUE / 2);
        t.update(2, Long.MIN_VALUE / 2);
        // Should not overflow since both halves of Long.MIN_VALUE fit in long
        assertEquals(Long.MIN_VALUE, t.prefixSum(2));
    }

    // =========================================================================
    // 5. Large dataset smoke test
    // =========================================================================

    @Test
    void largeDataset() {
        int n = 1000;
        FenwickTree t = new FenwickTree(n);
        for (int i = 1; i <= n; i++) t.update(i, i);
        // sum 1..n = n*(n+1)/2
        long expected = (long) n * (n + 1) / 2;
        assertEquals(expected, t.prefixSum(n));
        // rangeSum(500..1000) = sum(1..1000) - sum(1..499) = n*(n+1)/2 - 499*500/2
        long lo = 499L * 500 / 2;
        assertEquals(expected - lo, t.rangeSum(500, n));
    }
}
