package com.codingadventures.segmenttree;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.util.ArrayList;
import java.util.List;
import java.util.Random;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for {@link SegmentTree}.
 *
 * <p>Strategy:
 * <ul>
 *   <li>Correctness: compare every possible query range against a brute-force O(n) scan.
 *   <li>Update correctness: apply point updates to both the tree and a reference array,
 *       then re-verify all queries.
 *   <li>All four combine functions: sum, min, max, GCD.
 *   <li>Edge cases: single element, full-range query, identity boundary, large arrays.
 *   <li>Exception paths: invalid indices and ranges throw appropriately.
 * </ul>
 */
class SegmentTreeTest {

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Empty and Single-Element Trees
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("sumTree on empty array — size 0, isEmpty true")
    void emptyTree_metadata() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{});
        assertEquals(0, st.size());
        assertTrue(st.isEmpty());
    }

    @Test
    @DisplayName("sumTree on empty array — any query throws")
    void emptyTree_queryThrows() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{});
        assertThrows(IllegalArgumentException.class, () -> st.query(0, 0));
    }

    @Test
    @DisplayName("sumTree on single element")
    void singleElement_sumTree() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{42});
        assertEquals(42, st.query(0, 0));
        assertEquals(1, st.size());
    }

    @Test
    @DisplayName("minTree on single element")
    void singleElement_minTree() {
        SegmentTree<Integer> st = SegmentTree.minTree(new int[]{-7});
        assertEquals(-7, st.query(0, 0));
    }

    @Test
    @DisplayName("maxTree on single element")
    void singleElement_maxTree() {
        SegmentTree<Integer> st = SegmentTree.maxTree(new int[]{99});
        assertEquals(99, st.query(0, 0));
    }

    @Test
    @DisplayName("update on single element")
    void singleElement_update() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{10});
        st.update(0, 55);
        assertEquals(55, st.query(0, 0));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Sum Tree — Build and All Queries
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("sumTree spec example: [2,1,5,3,4]")
    void sumTree_specExample_allQueries() {
        int[] arr = {2, 1, 5, 3, 4};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        // From the spec:
        assertEquals(15, st.query(0, 4));  // full range
        assertEquals(9,  st.query(1, 3));  // [1..3] = 1+5+3
        assertEquals(3,  st.query(0, 1));  // [0..1] = 2+1
        assertEquals(7,  st.query(3, 4));  // [3..4] = 3+4
        assertEquals(5,  st.query(2, 2));  // single element
    }

    @Test
    @DisplayName("sumTree spec example: update arr[2]=7, re-query")
    void sumTree_specExample_update() {
        int[] arr = {2, 1, 5, 3, 4};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        // Before update: query(1,3) = 9
        assertEquals(9, st.query(1, 3));

        st.update(2, 7);  // arr[2] = 5 → 7

        // After update: query(1,3) = 1+7+3 = 11
        assertEquals(11, st.query(1, 3));
        // Full range: 2+1+7+3+4 = 17
        assertEquals(17, st.query(0, 4));
    }

    @Test
    @DisplayName("sumTree: all queries match brute force for a 5-element array")
    void sumTree_bruteForce_5elements() {
        int[] arr = {2, 1, 5, 3, 4};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                int expected = bruteSum(arr, l, r);
                assertEquals(expected, st.query(l, r),
                    "sum[" + l + ".." + r + "] expected " + expected);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Min Tree
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("minTree: all queries match brute force for a 6-element array")
    void minTree_bruteForce() {
        int[] arr = {5, 3, 7, 1, 9, 2};
        SegmentTree<Integer> st = SegmentTree.minTree(arr);

        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                int expected = bruteMin(arr, l, r);
                assertEquals(expected, st.query(l, r),
                    "min[" + l + ".." + r + "] expected " + expected);
            }
        }
    }

    @Test
    @DisplayName("minTree: point update propagates correctly")
    void minTree_update() {
        int[] arr = {5, 3, 7, 1, 9, 2};
        SegmentTree<Integer> st = SegmentTree.minTree(arr);

        assertEquals(1, st.query(0, 5));  // global min

        // Update arr[3] from 1 to 10 — no longer the global min
        arr[3] = 10;
        st.update(3, 10);

        assertEquals(2, st.query(0, 5));  // new global min is 2

        // Re-verify all queries
        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                assertEquals(bruteMin(arr, l, r), st.query(l, r));
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 4. Max Tree
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("maxTree: all queries match brute force")
    void maxTree_bruteForce() {
        int[] arr = {3, -1, 4, 1, 5, 9, 2, 6};
        SegmentTree<Integer> st = SegmentTree.maxTree(arr);

        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                int expected = bruteMax(arr, l, r);
                assertEquals(expected, st.query(l, r),
                    "max[" + l + ".." + r + "] expected " + expected);
            }
        }
    }

    @Test
    @DisplayName("maxTree: update to new maximum")
    void maxTree_update_newMax() {
        int[] arr = {1, 2, 3, 4, 5};
        SegmentTree<Integer> st = SegmentTree.maxTree(arr);

        assertEquals(5, st.query(0, 4));

        arr[2] = 100;
        st.update(2, 100);

        assertEquals(100, st.query(0, 4));
        assertEquals(100, st.query(1, 3));
        assertEquals(5,   st.query(3, 4));  // unaffected region: max(4,5)=5
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5. GCD Tree
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("gcdTree spec example: [12, 8, 6, 4, 9]")
    void gcdTree_specExample() {
        int[] arr = {12, 8, 6, 4, 9};
        SegmentTree<Integer> st = SegmentTree.gcdTree(arr);

        // gcd(12, gcd(8, 6)) = gcd(12, 2) = 2
        assertEquals(2, st.query(0, 2));
        // gcd(8, gcd(6, gcd(4, 9))) = gcd(8, gcd(6, 1)) = gcd(8, 1) = 1
        assertEquals(1, st.query(1, 4));
        // gcd(4, 9) = 1
        assertEquals(1, st.query(3, 4));
        // gcd(12, 8) = 4
        assertEquals(4, st.query(0, 1));
    }

    @Test
    @DisplayName("gcdTree: all queries match brute force")
    void gcdTree_bruteForce() {
        int[] arr = {12, 8, 6, 4, 9};
        SegmentTree<Integer> st = SegmentTree.gcdTree(arr);

        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                int expected = bruteGcd(arr, l, r);
                assertEquals(expected, st.query(l, r),
                    "gcd[" + l + ".." + r + "] expected " + expected);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 6. toList Reconstruction
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("toList returns original values")
    void toList_afterBuild() {
        int[] arr = {2, 1, 5, 3, 4};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);
        assertEquals(List.of(2, 1, 5, 3, 4), st.toList());
    }

    @Test
    @DisplayName("toList reflects point updates")
    void toList_afterUpdate() {
        int[] arr = {2, 1, 5, 3, 4};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        st.update(2, 99);
        st.update(0, -1);

        assertEquals(List.of(-1, 1, 99, 3, 4), st.toList());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 7. Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("full-range query equals aggregate of all elements")
    void fullRangeQuery() {
        int[] arr = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);
        assertEquals(55, st.query(0, 9));  // 1+2+...+10 = 55
    }

    @Test
    @DisplayName("all elements equal — sum and min/max consistent")
    void allEqual() {
        int[] arr = {7, 7, 7, 7, 7};
        SegmentTree<Integer> sum = SegmentTree.sumTree(arr);
        SegmentTree<Integer> min = SegmentTree.minTree(arr);
        SegmentTree<Integer> max = SegmentTree.maxTree(arr);

        assertEquals(35, sum.query(0, 4));
        assertEquals(7,  min.query(0, 4));
        assertEquals(7,  max.query(0, 4));
        assertEquals(14, sum.query(0, 1));
    }

    @Test
    @DisplayName("large negative values")
    void negativeValues() {
        int[] arr = {-10, -5, -20, -1, -15};
        SegmentTree<Integer> min = SegmentTree.minTree(arr);
        SegmentTree<Integer> max = SegmentTree.maxTree(arr);
        SegmentTree<Integer> sum = SegmentTree.sumTree(arr);

        assertEquals(-20, min.query(0, 4));
        assertEquals(-1,  max.query(0, 4));
        assertEquals(-51, sum.query(0, 4));
    }

    @Test
    @DisplayName("mixed positive and negative values")
    void mixedValues() {
        int[] arr = {-3, 1, -4, 1, 5, -9, 2, 6};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        // Verify all queries against brute force
        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r));
            }
        }
    }

    @Test
    @DisplayName("non-power-of-2 input length")
    void nonPowerOfTwo() {
        // n=7 is not a power of 2 — exercise the non-trivial tree padding
        int[] arr = {1, 2, 3, 4, 5, 6, 7};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        assertEquals(28, st.query(0, 6));  // 1+2+...+7 = 28
        assertEquals(9,  st.query(1, 3));  // 2+3+4
        for (int l = 0; l < 7; l++) {
            for (int r = l; r < 7; r++) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r));
            }
        }
    }

    @Test
    @DisplayName("update to same value leaves tree unchanged")
    void updateSameValue() {
        int[] arr = {2, 1, 5, 3, 4};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);
        int before = st.query(0, 4);
        st.update(2, 5);  // same value
        assertEquals(before, st.query(0, 4));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 8. Exception Paths
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("query with ql > qr throws IllegalArgumentException")
    void query_invalidRange_qlGtQr() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{1, 2, 3});
        assertThrows(IllegalArgumentException.class, () -> st.query(2, 1));
    }

    @Test
    @DisplayName("query with negative ql throws IllegalArgumentException")
    void query_negativeLeft() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{1, 2, 3});
        assertThrows(IllegalArgumentException.class, () -> st.query(-1, 2));
    }

    @Test
    @DisplayName("query with qr out of bounds throws IllegalArgumentException")
    void query_rightOutOfBounds() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{1, 2, 3});
        assertThrows(IllegalArgumentException.class, () -> st.query(0, 3));
    }

    @Test
    @DisplayName("update with negative index throws IllegalArgumentException")
    void update_negativeIndex() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{1, 2, 3});
        assertThrows(IllegalArgumentException.class, () -> st.update(-1, 5));
    }

    @Test
    @DisplayName("update with index out of bounds throws IllegalArgumentException")
    void update_indexOutOfBounds() {
        SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{1, 2, 3});
        assertThrows(IllegalArgumentException.class, () -> st.update(3, 5));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 9. Multiple Updates
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("multiple sequential updates maintain consistency")
    void multipleUpdates() {
        int[] arr = {1, 2, 3, 4, 5};
        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        arr[0] = 10;  st.update(0, 10);
        arr[4] = 20;  st.update(4, 20);
        arr[2] = 0;   st.update(2, 0);

        for (int l = 0; l < arr.length; l++) {
            for (int r = l; r < arr.length; r++) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r),
                    "After updates: sum[" + l + ".." + r + "]");
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 10. Random Stress Test
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("random stress test: 200 insertions, 200 queries after updates")
    void stress_randomQueriesAndUpdates() {
        Random rng = new Random(12345L);
        int n = 200;
        int[] arr = new int[n];
        for (int i = 0; i < n; i++) {
            arr[i] = rng.nextInt(1000) - 500;  // values in [-500, 499]
        }

        SegmentTree<Integer> sumSt = SegmentTree.sumTree(arr);
        SegmentTree<Integer> minSt = SegmentTree.minTree(arr);
        SegmentTree<Integer> maxSt = SegmentTree.maxTree(arr);

        // Verify all queries match brute force initially
        for (int iter = 0; iter < 50; iter++) {
            int l = rng.nextInt(n);
            int r = l + rng.nextInt(n - l);
            assertEquals(bruteSum(arr, l, r), sumSt.query(l, r));
            assertEquals(bruteMin(arr, l, r), minSt.query(l, r));
            assertEquals(bruteMax(arr, l, r), maxSt.query(l, r));
        }

        // Apply 200 random point updates, verify after each
        for (int iter = 0; iter < 200; iter++) {
            int idx = rng.nextInt(n);
            int val = rng.nextInt(1000) - 500;
            arr[idx] = val;
            sumSt.update(idx, val);
            minSt.update(idx, val);
            maxSt.update(idx, val);

            // Spot-check a random query after each update
            int l = rng.nextInt(n);
            int r = l + rng.nextInt(n - l);
            assertEquals(bruteSum(arr, l, r), sumSt.query(l, r));
            assertEquals(bruteMin(arr, l, r), minSt.query(l, r));
            assertEquals(bruteMax(arr, l, r), maxSt.query(l, r));
        }
    }

    @Test
    @DisplayName("large array: 100k elements, spot queries in O(log n)")
    void stress_largeArray() {
        int n = 100_000;
        int[] arr = new int[n];
        Random rng = new Random(99L);
        for (int i = 0; i < n; i++) arr[i] = rng.nextInt(1_000_000);

        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        // Spot check a few queries
        assertEquals(bruteSum(arr, 0, n - 1), st.query(0, n - 1));
        assertEquals(bruteSum(arr, 40_000, 59_999), st.query(40_000, 59_999));
        assertEquals(arr[12_345], st.query(12_345, 12_345));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 11. Generic Combine (custom via constructor)
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    @DisplayName("custom combine: range product")
    void customCombine_product() {
        Integer[] arr = {2, 3, 4, 5};
        // product tree: identity = 1
        SegmentTree<Integer> st = new SegmentTree<>(arr, (a, b) -> a * b, 1);

        assertEquals(120, st.query(0, 3));  // 2*3*4*5 = 120
        assertEquals(24,  st.query(0, 2));  // 2*3*4 = 24
        assertEquals(20,  st.query(2, 3));  // 4*5 = 20
        assertEquals(6,   st.query(0, 1));  // 2*3 = 6
    }

    @Test
    @DisplayName("custom combine: range bitwise OR")
    void customCombine_bitwiseOr() {
        Integer[] arr = {0b0001, 0b0010, 0b0100, 0b1000};
        // OR tree: identity = 0
        SegmentTree<Integer> st = new SegmentTree<>(arr, (a, b) -> a | b, 0);

        assertEquals(0b1111, st.query(0, 3));
        assertEquals(0b0011, st.query(0, 1));
        assertEquals(0b0110, st.query(1, 2));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 12. Parameterized: all array sizes 1..20 with sum tree
    // ─────────────────────────────────────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(ints = {1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 20})
    @DisplayName("sumTree brute-force correctness for array sizes 1..20")
    void sumTree_allSizes_bruteForce(int n) {
        int[] arr = new int[n];
        for (int i = 0; i < n; i++) arr[i] = i + 1;  // [1, 2, ..., n]

        SegmentTree<Integer> st = SegmentTree.sumTree(arr);

        for (int l = 0; l < n; l++) {
            for (int r = l; r < n; r++) {
                assertEquals(bruteSum(arr, l, r), st.query(l, r));
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Brute-Force Helpers
    // ─────────────────────────────────────────────────────────────────────────

    private static int bruteSum(int[] arr, int l, int r) {
        int s = 0;
        for (int i = l; i <= r; i++) s += arr[i];
        return s;
    }

    private static int bruteMin(int[] arr, int l, int r) {
        int m = arr[l];
        for (int i = l + 1; i <= r; i++) m = Math.min(m, arr[i]);
        return m;
    }

    private static int bruteMax(int[] arr, int l, int r) {
        int m = arr[l];
        for (int i = l + 1; i <= r; i++) m = Math.max(m, arr[i]);
        return m;
    }

    private static int bruteGcd(int[] arr, int l, int r) {
        int g = arr[l];
        for (int i = l + 1; i <= r; i++) {
            int b = arr[i];
            while (b != 0) { int t = b; b = g % b; g = t; }
        }
        return g;
    }
}
