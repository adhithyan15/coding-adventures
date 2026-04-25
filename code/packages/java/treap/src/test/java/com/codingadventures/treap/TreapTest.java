package com.codingadventures.treap;

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
 * Comprehensive tests for Treap (DT10).
 *
 * Every structural test verifies isValidTreap() — this ensures both BST and
 * heap properties hold throughout the treap's lifecycle.
 */
class TreapTest {

    // ─── Helpers ───────────────────────────────────────────────────────────

    /** Build a treap with a fixed seed for deterministic behavior. */
    private static Treap buildTree(long seed, int... keys) {
        Treap t = Treap.withSeed(seed);
        for (int k : keys) t = t.insert(k);
        return t;
    }

    /** Build with default seed 42. */
    private static Treap buildTree(int... keys) {
        return buildTree(42L, keys);
    }

    // ─── Empty treap ───────────────────────────────────────────────────────

    @Test
    void emptyTreap_isValid() {
        Treap t = Treap.withSeed(1L);
        assertTrue(t.isValidTreap());
        assertTrue(t.isEmpty());
        assertEquals(0, t.size());
        assertEquals(0, t.height());
    }

    @Test
    void emptyTreap_containsReturnsFalse() {
        assertFalse(Treap.withSeed(1L).contains(42));
    }

    @Test
    void emptyTreap_minMaxEmpty() {
        assertTrue(Treap.withSeed(1L).min().isEmpty());
        assertTrue(Treap.withSeed(1L).max().isEmpty());
    }

    @Test
    void emptyTreap_kthSmallestThrows() {
        assertThrows(NoSuchElementException.class, () -> Treap.withSeed(1L).kthSmallest(1));
    }

    // ─── Single element ────────────────────────────────────────────────────

    @Test
    void singleInsert_valid() {
        Treap t = buildTree(10);
        assertTrue(t.isValidTreap());
        assertEquals(1, t.size());
        assertNotNull(t.getRoot());
        assertEquals(10, t.getRoot().key());
    }

    @Test
    void singleInsert_containsValue() {
        Treap t = buildTree(42);
        assertTrue(t.contains(42));
        assertFalse(t.contains(41));
        assertFalse(t.contains(43));
    }

    @Test
    void singleInsert_minMaxEqual() {
        Treap t = buildTree(7);
        assertEquals(7, t.min().orElseThrow());
        assertEquals(7, t.max().orElseThrow());
    }

    // ─── Deterministic priorities ──────────────────────────────────────────

    @Test
    void insertWithPriority_heapPropertyHolds() {
        // Manually assign priorities so we can verify the structure
        // Keys: 5, 3, 7, 1, 4  Priorities: 0.91, 0.53, 0.75, 0.22, 0.68
        Treap t = Treap.withSeed(0L)
                .insertWithPriority(5, 0.91)
                .insertWithPriority(3, 0.53)
                .insertWithPriority(7, 0.75)
                .insertWithPriority(1, 0.22)
                .insertWithPriority(4, 0.68);

        assertTrue(t.isValidTreap());
        // Root must be 5 (highest priority 0.91)
        assertEquals(5, t.getRoot().key());
        assertEquals(0.91, t.getRoot().priority(), 1e-10);
    }

    @Test
    void insertWithPriority_sortedOrder() {
        Treap t = Treap.withSeed(0L)
                .insertWithPriority(5, 0.91)
                .insertWithPriority(3, 0.53)
                .insertWithPriority(7, 0.75)
                .insertWithPriority(1, 0.22)
                .insertWithPriority(4, 0.68);

        assertEquals(Arrays.asList(1, 3, 4, 5, 7), t.toSortedList());
    }

    // ─── Multiple inserts — invariants ─────────────────────────────────────

    @Test
    void insertAscending_alwaysValid() {
        Treap t = Treap.withSeed(42L);
        for (int i = 1; i <= 20; i++) {
            t = t.insert(i);
            assertTrue(t.isValidTreap(), "invariant failed after inserting " + i);
        }
        assertEquals(20, t.size());
    }

    @Test
    void insertDescending_alwaysValid() {
        Treap t = Treap.withSeed(42L);
        for (int i = 20; i >= 1; i--) {
            t = t.insert(i);
            assertTrue(t.isValidTreap(), "invariant failed after inserting " + i);
        }
        assertEquals(20, t.size());
    }

    @Test
    void insertMixed_alwaysValid() {
        int[] values = {10, 5, 15, 3, 7, 12, 20, 1, 6, 9, 14, 18};
        Treap t = Treap.withSeed(7L);
        for (int v : values) {
            t = t.insert(v);
            assertTrue(t.isValidTreap(), "invariant failed after inserting " + v);
        }
    }

    // ─── Duplicate handling ────────────────────────────────────────────────

    @Test
    void insertDuplicate_sizeUnchanged() {
        Treap t = buildTree(5, 5, 5);
        assertTrue(t.isValidTreap());
        assertEquals(1, t.size());
        assertTrue(t.contains(5));
    }

    @Test
    void insertDuplicate_multipleExisting() {
        Treap t = buildTree(1, 2, 3, 2, 1);
        assertTrue(t.isValidTreap());
        assertEquals(3, t.size());
    }

    // ─── Height (probabilistic) ────────────────────────────────────────────

    @Test
    void height_roughlyLogarithmic_100elements() {
        Treap t = Treap.withSeed(99L);
        for (int i = 1; i <= 100; i++) t = t.insert(i);
        assertTrue(t.isValidTreap());
        // Expected height O(log n) ≈ 7; allow generous slack (4× expected)
        assertTrue(t.height() < 40, "height=" + t.height() + " suspiciously large for 100 elements");
    }

    // ─── Contains ──────────────────────────────────────────────────────────

    @Test
    void contains_presentAndAbsent() {
        Treap t = buildTree(10, 5, 15, 3, 7, 12, 20);
        assertTrue(t.contains(10));
        assertTrue(t.contains(5));
        assertTrue(t.contains(20));
        assertFalse(t.contains(1));
        assertFalse(t.contains(11));
        assertFalse(t.contains(100));
    }

    // ─── Min / Max ─────────────────────────────────────────────────────────

    @Test
    void minMax_correct() {
        Treap t = buildTree(5, 3, 8, 1, 9, 4);
        assertEquals(1, t.min().orElseThrow());
        assertEquals(9, t.max().orElseThrow());
    }

    // ─── Predecessor / Successor ───────────────────────────────────────────

    @Test
    void predecessor_typical() {
        Treap t = buildTree(10, 5, 15, 3, 7, 12, 20);
        assertEquals(10, t.predecessor(12).orElseThrow());
        assertEquals(7,  t.predecessor(10).orElseThrow());
        assertEquals(5,  t.predecessor(7).orElseThrow());
    }

    @Test
    void predecessor_minimum_empty() {
        Treap t = buildTree(10, 5, 15);
        assertTrue(t.predecessor(5).isEmpty());
    }

    @Test
    void successor_typical() {
        Treap t = buildTree(10, 5, 15, 3, 7, 12, 20);
        assertEquals(12, t.successor(10).orElseThrow());
        assertEquals(10, t.successor(7).orElseThrow());
        assertEquals(15, t.successor(12).orElseThrow());
    }

    @Test
    void successor_maximum_empty() {
        Treap t = buildTree(10, 5, 15);
        assertTrue(t.successor(15).isEmpty());
    }

    // ─── kthSmallest ───────────────────────────────────────────────────────

    @Test
    void kthSmallest_correct() {
        Treap t = buildTree(5, 3, 8, 1, 9, 4);
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
        Treap t = buildTree(1, 2, 3);
        assertThrows(NoSuchElementException.class, () -> t.kthSmallest(0));
        assertThrows(NoSuchElementException.class, () -> t.kthSmallest(4));
    }

    // ─── toSortedList ──────────────────────────────────────────────────────

    @Test
    void toSortedList_empty() {
        assertTrue(Treap.withSeed(0L).toSortedList().isEmpty());
    }

    @Test
    void toSortedList_alwaysSorted() {
        Treap t = buildTree(7, 2, 11, 5, 3, 9, 1, 8);
        assertEquals(Arrays.asList(1, 2, 3, 5, 7, 8, 9, 11), t.toSortedList());
    }

    // ─── Split ─────────────────────────────────────────────────────────────

    @Test
    void split_correctPartition() {
        Treap t = buildTree(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        Treap.SplitResult parts = t.split(5);
        Treap left  = new Treap.Builder().fromNode(parts.left()).build();
        Treap right = new Treap.Builder().fromNode(parts.right()).build();

        // All left keys ≤ 5
        for (int k : left.toSortedList()) {
            assertTrue(k <= 5, "left has key " + k + " > 5");
        }
        // All right keys > 5
        for (int k : right.toSortedList()) {
            assertTrue(k > 5, "right has key " + k + " <= 5");
        }
        assertEquals(5, left.size());
        assertEquals(5, right.size());
    }

    @Test
    void split_atMin_leftEmpty() {
        Treap t = buildTree(3, 5, 7);
        Treap.SplitResult parts = t.split(2);
        assertNull(parts.left());
        assertNotNull(parts.right());
    }

    @Test
    void split_atMax_rightEmpty() {
        Treap t = buildTree(3, 5, 7);
        Treap.SplitResult parts = t.split(7);
        assertNotNull(parts.left());
        assertNull(parts.right());
    }

    // ─── Merge ─────────────────────────────────────────────────────────────

    @Test
    void merge_reconstructsTreap() {
        Treap original = buildTree(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        Treap.SplitResult parts = original.split(5);

        Treap left  = new Treap.Builder().fromNode(parts.left()).withSeed(42L).build();
        Treap right = new Treap.Builder().fromNode(parts.right()).withSeed(42L).build();
        Treap merged = Treap.merge(left, right);

        assertTrue(merged.isValidTreap());
        assertEquals(10, merged.size());
        assertEquals(Arrays.asList(1, 2, 3, 4, 5, 6, 7, 8, 9, 10), merged.toSortedList());
    }

    @Test
    void merge_withEmptyLeft() {
        Treap left  = Treap.withSeed(1L);
        Treap right = buildTree(1, 2, 3);
        Treap merged = Treap.merge(left, right);
        assertTrue(merged.isValidTreap());
        assertEquals(3, merged.size());
    }

    @Test
    void merge_withEmptyRight() {
        Treap left  = buildTree(1, 2, 3);
        Treap right = Treap.withSeed(1L);
        Treap merged = Treap.merge(left, right);
        assertTrue(merged.isValidTreap());
        assertEquals(3, merged.size());
    }

    // ─── Delete ────────────────────────────────────────────────────────────

    @Test
    void delete_absentKey_unchanged() {
        Treap t = buildTree(5, 3, 7);
        Treap t2 = t.delete(99);
        assertTrue(t2.isValidTreap());
        assertEquals(3, t2.size());
    }

    @Test
    void delete_singleElement_becomesEmpty() {
        Treap t = buildTree(42).delete(42);
        assertTrue(t.isValidTreap());
        assertTrue(t.isEmpty());
    }

    @Test
    void delete_leafKey() {
        Treap t = buildTree(5, 3, 7).delete(3);
        assertTrue(t.isValidTreap());
        assertEquals(2, t.size());
        assertFalse(t.contains(3));
        assertTrue(t.contains(5));
        assertTrue(t.contains(7));
    }

    @Test
    void delete_rootKey() {
        // Using explicit priorities: root is the key with highest priority
        Treap t = Treap.withSeed(0L)
                .insertWithPriority(5, 0.9)   // root
                .insertWithPriority(3, 0.5)
                .insertWithPriority(7, 0.6);
        Treap t2 = t.delete(5);
        assertTrue(t2.isValidTreap());
        assertEquals(2, t2.size());
        assertFalse(t2.contains(5));
    }

    @Test
    void delete_allElements_preservesInvariant() {
        int[] values = {10, 5, 15, 3, 7, 12, 20};
        Treap t = buildTree(values);
        for (int v : values) {
            t = t.delete(v);
            assertTrue(t.isValidTreap(), "invariant failed after deleting " + v);
        }
        assertTrue(t.isEmpty());
    }

    @Test
    void delete_minElement_repeatedly() {
        Treap t = buildTree(1, 2, 3, 4, 5);
        for (int i = 1; i <= 5; i++) {
            t = t.delete(i);
            assertTrue(t.isValidTreap(), "invariant failed after deleting " + i);
            assertFalse(t.contains(i));
        }
    }

    @Test
    void delete_maxElement_repeatedly() {
        Treap t = buildTree(1, 2, 3, 4, 5);
        for (int i = 5; i >= 1; i--) {
            t = t.delete(i);
            assertTrue(t.isValidTreap(), "invariant failed after deleting " + i);
            assertFalse(t.contains(i));
        }
    }

    // ─── Insert then delete — round-trip ───────────────────────────────────

    @Test
    void insertThenDelete_roundTrip() {
        int[] values = {50, 25, 75, 10, 30, 60, 90, 5, 15, 27, 35};
        Treap t = buildTree(values);
        for (int v : values) assertTrue(t.contains(v));
        for (int i = values.length - 1; i >= 0; i--) {
            t = t.delete(values[i]);
            assertTrue(t.isValidTreap());
        }
        assertTrue(t.isEmpty());
    }

    // ─── Immutability ──────────────────────────────────────────────────────

    @Test
    void immutability_oldTreapUnchanged() {
        Treap original = buildTree(5, 3, 7);
        Treap modified = original.insert(1).insert(9);
        assertEquals(3, original.size());
        assertFalse(original.contains(1));
        assertFalse(original.contains(9));
        assertEquals(5, modified.size());
        assertTrue(modified.isValidTreap());
    }

    // ─── Random stress test ────────────────────────────────────────────────

    @Test
    void randomInserts_alwaysValid() {
        Random rng = new Random(42L);
        Treap t = Treap.withSeed(99L);
        for (int i = 0; i < 200; i++) {
            int v = rng.nextInt(100);
            t = t.insert(v);
            assertTrue(t.isValidTreap(), "invariant failed on random insert " + i);
        }
    }

    @Test
    void randomDeletesAfterInserts_alwaysValid() {
        Random rng = new Random(7L);
        List<Integer> inserted = new ArrayList<>();
        Treap t = Treap.withSeed(55L);

        for (int i = 0; i < 100; i++) {
            int v = rng.nextInt(50);
            t = t.insert(v);
            if (!inserted.contains(v)) inserted.add(v);
        }
        assertTrue(t.isValidTreap());

        Collections.shuffle(inserted, rng);
        for (int v : inserted) {
            t = t.delete(v);
            assertTrue(t.isValidTreap(), "invariant failed after deleting " + v);
        }
        assertTrue(t.isEmpty());
    }
}
