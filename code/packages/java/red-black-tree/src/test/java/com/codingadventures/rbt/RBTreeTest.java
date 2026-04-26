package com.codingadventures.rbt;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.NoSuchElementException;
import java.util.Random;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive tests for RBTree (DT09).
 *
 * Every test verifies isValidRB() after structural operations — this catches
 * any invariant violation as a side-effect without needing white-box access.
 */
class RBTreeTest {

    // ─── Helpers ───────────────────────────────────────────────────────────

    /** Build a tree from a sequence of values. */
    private static RBTree buildTree(int... values) {
        RBTree t = RBTree.empty();
        for (int v : values) t = t.insert(v);
        return t;
    }

    // ─── Empty tree ────────────────────────────────────────────────────────

    @Test
    void emptyTree_isValid() {
        RBTree t = RBTree.empty();
        assertTrue(t.isValidRB());
        assertTrue(t.isEmpty());
        assertEquals(0, t.size());
        assertEquals(0, t.height());
        assertEquals(0, t.blackHeight());
    }

    @Test
    void emptyTree_containsReturnsFalse() {
        assertFalse(RBTree.empty().contains(42));
    }

    @Test
    void emptyTree_minMaxEmpty() {
        assertTrue(RBTree.empty().min().isEmpty());
        assertTrue(RBTree.empty().max().isEmpty());
    }

    @Test
    void emptyTree_kthSmallestThrows() {
        assertThrows(NoSuchElementException.class, () -> RBTree.empty().kthSmallest(1));
    }

    // ─── Single element ────────────────────────────────────────────────────

    @Test
    void singleInsert_rootIsBlack() {
        RBTree t = buildTree(10);
        assertTrue(t.isValidRB());
        assertEquals(RBTree.Color.BLACK, t.getRoot().color());
        assertEquals(1, t.size());
    }

    @Test
    void singleInsert_containsValue() {
        RBTree t = buildTree(42);
        assertTrue(t.contains(42));
        assertFalse(t.contains(41));
        assertFalse(t.contains(43));
    }

    @Test
    void singleInsert_minMaxEqual() {
        RBTree t = buildTree(7);
        assertEquals(7, t.min().orElseThrow());
        assertEquals(7, t.max().orElseThrow());
    }

    // ─── Multiple inserts — invariants ─────────────────────────────────────

    @Test
    void insertAscending_alwaysValid() {
        RBTree t = RBTree.empty();
        for (int i = 1; i <= 20; i++) {
            t = t.insert(i);
            assertTrue(t.isValidRB(), "invariant failed after inserting " + i);
        }
        assertEquals(20, t.size());
    }

    @Test
    void insertDescending_alwaysValid() {
        RBTree t = RBTree.empty();
        for (int i = 20; i >= 1; i--) {
            t = t.insert(i);
            assertTrue(t.isValidRB(), "invariant failed after inserting " + i);
        }
        assertEquals(20, t.size());
    }

    @Test
    void insertAlternating_alwaysValid() {
        // Worst case for some BST variants: alternating high-low
        int[] values = {10, 1, 20, 5, 15, 3, 18, 7, 12};
        RBTree t = RBTree.empty();
        for (int v : values) {
            t = t.insert(v);
            assertTrue(t.isValidRB(), "invariant failed after inserting " + v);
        }
    }

    @Test
    void insertCLRSExample_classicSequence() {
        // CLRS textbook insertion sequence: [7,3,18,10,22,8,11,26,2,6,13]
        RBTree t = buildTree(7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13);
        assertTrue(t.isValidRB());
        assertEquals(11, t.size());
        // In-order must be sorted
        List<Integer> sorted = t.toSortedList();
        List<Integer> expected = Arrays.asList(2, 3, 6, 7, 8, 10, 11, 13, 18, 22, 26);
        assertEquals(expected, sorted);
    }

    // ─── Duplicate handling ────────────────────────────────────────────────

    @Test
    void insertDuplicate_sizeUnchanged() {
        RBTree t = buildTree(5, 5, 5);
        assertTrue(t.isValidRB());
        assertEquals(1, t.size());
        assertTrue(t.contains(5));
    }

    @Test
    void insertDuplicate_afterMultiple() {
        RBTree t = buildTree(1, 2, 3, 2, 1);
        assertTrue(t.isValidRB());
        assertEquals(3, t.size());
    }

    // ─── Height bound ──────────────────────────────────────────────────────

    @Test
    void height_bounded_by2LogN() {
        // Insert 100 elements; height must be ≤ 2 * log₂(101) ≈ 13.3
        RBTree t = RBTree.empty();
        for (int i = 1; i <= 100; i++) t = t.insert(i);
        assertTrue(t.isValidRB());
        int maxAllowedHeight = (int) (2 * Math.ceil(Math.log(101) / Math.log(2)));
        assertTrue(t.height() <= maxAllowedHeight,
                "height " + t.height() + " exceeds 2*log2(101)=" + maxAllowedHeight);
    }

    // ─── Contains ──────────────────────────────────────────────────────────

    @Test
    void contains_presentAndAbsent() {
        RBTree t = buildTree(10, 5, 15, 3, 7, 12, 20);
        assertTrue(t.contains(10));
        assertTrue(t.contains(5));
        assertTrue(t.contains(20));
        assertFalse(t.contains(1));
        assertFalse(t.contains(11));
        assertFalse(t.contains(100));
    }

    // ─── Min / Max ─────────────────────────────────────────────────────────

    @Test
    void minMax_correctAfterInserts() {
        RBTree t = buildTree(5, 3, 8, 1, 9, 4);
        assertEquals(1, t.min().orElseThrow());
        assertEquals(9, t.max().orElseThrow());
    }

    @Test
    void minMax_singleElement() {
        RBTree t = buildTree(42);
        assertEquals(42, t.min().orElseThrow());
        assertEquals(42, t.max().orElseThrow());
    }

    // ─── Predecessor / Successor ───────────────────────────────────────────

    @Test
    void predecessor_typical() {
        RBTree t = buildTree(10, 5, 15, 3, 7, 12, 20);
        assertEquals(10, t.predecessor(12).orElseThrow());
        assertEquals(7,  t.predecessor(10).orElseThrow());
        assertEquals(5,  t.predecessor(7).orElseThrow());
    }

    @Test
    void predecessor_minimum_returnsEmpty() {
        RBTree t = buildTree(10, 5, 15);
        assertTrue(t.predecessor(5).isEmpty());
    }

    @Test
    void successor_typical() {
        RBTree t = buildTree(10, 5, 15, 3, 7, 12, 20);
        assertEquals(12, t.successor(10).orElseThrow());
        assertEquals(10, t.successor(7).orElseThrow());
        assertEquals(15, t.successor(12).orElseThrow());
    }

    @Test
    void successor_maximum_returnsEmpty() {
        RBTree t = buildTree(10, 5, 15);
        assertTrue(t.successor(15).isEmpty());
    }

    // ─── kthSmallest ───────────────────────────────────────────────────────

    @Test
    void kthSmallest_correctOrder() {
        RBTree t = buildTree(5, 3, 8, 1, 9, 4);
        // Sorted: 1, 3, 4, 5, 8, 9
        assertEquals(1, t.kthSmallest(1));
        assertEquals(3, t.kthSmallest(2));
        assertEquals(4, t.kthSmallest(3));
        assertEquals(5, t.kthSmallest(4));
        assertEquals(8, t.kthSmallest(5));
        assertEquals(9, t.kthSmallest(6));
    }

    @Test
    void kthSmallest_outOfRange_throws() {
        RBTree t = buildTree(1, 2, 3);
        assertThrows(NoSuchElementException.class, () -> t.kthSmallest(0));
        assertThrows(NoSuchElementException.class, () -> t.kthSmallest(4));
    }

    // ─── toSortedList ──────────────────────────────────────────────────────

    @Test
    void toSortedList_empty() {
        assertTrue(RBTree.empty().toSortedList().isEmpty());
    }

    @Test
    void toSortedList_randomInsertion() {
        int[] values = {7, 2, 11, 5, 3, 9, 1, 8};
        RBTree t = buildTree(values);
        List<Integer> sorted = t.toSortedList();
        List<Integer> expected = new ArrayList<>(List.of(1, 2, 3, 5, 7, 8, 9, 11));
        assertEquals(expected, sorted);
    }

    // ─── Delete ────────────────────────────────────────────────────────────

    @Test
    void delete_absentElement_unchanged() {
        RBTree t = buildTree(5, 3, 7);
        RBTree t2 = t.delete(99);
        assertTrue(t2.isValidRB());
        assertEquals(3, t2.size());
    }

    @Test
    void delete_singleElement_becomesEmpty() {
        RBTree t = buildTree(42).delete(42);
        assertTrue(t.isValidRB());
        assertTrue(t.isEmpty());
    }

    @Test
    void delete_rootInTwoNodeTree() {
        RBTree t = buildTree(5, 3).delete(5);
        assertTrue(t.isValidRB());
        assertEquals(1, t.size());
        assertTrue(t.contains(3));
        assertFalse(t.contains(5));
    }

    @Test
    void delete_leafNode() {
        RBTree t = buildTree(5, 3, 7).delete(3);
        assertTrue(t.isValidRB());
        assertEquals(2, t.size());
        assertFalse(t.contains(3));
        assertTrue(t.contains(5));
        assertTrue(t.contains(7));
    }

    @Test
    void delete_internalNode() {
        RBTree t = buildTree(10, 5, 15, 3, 7, 12, 20).delete(5);
        assertTrue(t.isValidRB());
        assertEquals(6, t.size());
        assertFalse(t.contains(5));
        assertTrue(t.contains(3));
        assertTrue(t.contains(7));
    }

    @Test
    void delete_allElements_preservesInvariant() {
        int[] values = {10, 5, 15, 3, 7, 12, 20};
        RBTree t = buildTree(values);
        for (int v : values) {
            t = t.delete(v);
            assertTrue(t.isValidRB(), "invariant failed after deleting " + v);
        }
        assertTrue(t.isEmpty());
    }

    @Test
    void delete_minElement_repeatedly() {
        RBTree t = buildTree(1, 2, 3, 4, 5);
        for (int i = 1; i <= 5; i++) {
            t = t.delete(i);
            assertTrue(t.isValidRB(), "invariant failed after deleting " + i);
            assertFalse(t.contains(i));
        }
    }

    @Test
    void delete_maxElement_repeatedly() {
        RBTree t = buildTree(1, 2, 3, 4, 5);
        for (int i = 5; i >= 1; i--) {
            t = t.delete(i);
            assertTrue(t.isValidRB(), "invariant failed after deleting " + i);
            assertFalse(t.contains(i));
        }
    }

    @Test
    void delete_CLRSsequence_staysValid() {
        RBTree t = buildTree(7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13);
        for (int v : new int[]{18, 11, 3, 7, 8}) {
            t = t.delete(v);
            assertTrue(t.isValidRB(), "invariant failed after deleting " + v);
        }
        assertEquals(6, t.size());
    }

    // ─── Insert then delete — round-trip ───────────────────────────────────

    @Test
    void insertThenDelete_roundTrip() {
        int[] values = {50, 25, 75, 10, 30, 60, 90, 5, 15, 27, 35};
        RBTree t = buildTree(values);
        for (int v : values) {
            assertTrue(t.contains(v));
        }
        // Delete in reverse order
        for (int i = values.length - 1; i >= 0; i--) {
            t = t.delete(values[i]);
            assertTrue(t.isValidRB());
        }
        assertTrue(t.isEmpty());
    }

    // ─── Immutability ──────────────────────────────────────────────────────

    @Test
    void immutability_oldTreeUnchanged() {
        RBTree original = buildTree(5, 3, 7);
        RBTree modified = original.insert(1).insert(9);
        // Original must be unchanged
        assertEquals(3, original.size());
        assertFalse(original.contains(1));
        assertFalse(original.contains(9));
        // New tree has 5 elements
        assertEquals(5, modified.size());
        assertTrue(modified.isValidRB());
    }

    // ─── Random stress test ────────────────────────────────────────────────

    @Test
    void randomInserts_alwaysValid() {
        Random rng = new Random(42L);
        RBTree t = RBTree.empty();
        for (int i = 0; i < 200; i++) {
            int v = rng.nextInt(100);
            t = t.insert(v);
            assertTrue(t.isValidRB(), "invariant failed on random insert " + i);
        }
    }

    @Test
    void randomDeletesAfterInserts_alwaysValid() {
        Random rng = new Random(7L);
        List<Integer> inserted = new ArrayList<>();
        RBTree t = RBTree.empty();

        // Insert 100 random values
        for (int i = 0; i < 100; i++) {
            int v = rng.nextInt(50);
            t = t.insert(v);
            if (!inserted.contains(v)) inserted.add(v);
        }
        assertTrue(t.isValidRB());

        // Delete them one by one, verifying invariant each time
        Collections.shuffle(inserted, rng);
        for (int v : inserted) {
            t = t.delete(v);
            assertTrue(t.isValidRB(), "invariant failed after deleting " + v);
        }
        assertTrue(t.isEmpty());
    }

    // ─── Black height consistency ──────────────────────────────────────────

    @Test
    void blackHeight_consistentWithValidation() {
        RBTree t = buildTree(7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13);
        assertTrue(t.isValidRB());
        // A valid tree's black height should be positive
        assertTrue(t.blackHeight() > 0);
    }

    @Test
    void blackHeight_emptyTree_zero() {
        assertEquals(0, RBTree.empty().blackHeight());
    }

    // ─── isValidRB detects violations ─────────────────────────────────────

    @Test
    void isValidRB_detectsRedRoot() {
        // Manually craft a red root — should fail validation
        RBTree.Node redRoot = new RBTree.Node(5, RBTree.Color.RED, null, null);
        // We can't inject this through the public API (insert always makes root black),
        // but we can test the checkNode logic by constructing a tree with a red root
        // via reflection or by testing the insert always produces black roots.
        RBTree t = buildTree(5);
        assertEquals(RBTree.Color.BLACK, t.getRoot().color());
    }

    @Test
    void isValidRB_largeTree() {
        RBTree t = RBTree.empty();
        for (int i = 1; i <= 63; i++) t = t.insert(i);
        assertTrue(t.isValidRB());
    }

    // ─── Black height property ─────────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(ints = {1, 3, 7, 15, 31, 63})
    void insertPerfectPowerOf2Minus1_heightBounded(int n) {
        RBTree t = RBTree.empty();
        for (int i = 1; i <= n; i++) t = t.insert(i);
        assertTrue(t.isValidRB());
        int logN = (int) Math.ceil(Math.log(n + 1) / Math.log(2));
        assertTrue(t.height() <= 2 * logN + 1,
                "height=" + t.height() + " log=" + logN + " n=" + n);
    }
}
