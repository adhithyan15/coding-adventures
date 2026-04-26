// ============================================================================
// BPlusTreeTest.java — Exhaustive tests for BPlusTree<K,V> (DT12)
// ============================================================================
//
// Test strategy:
//   - Start with empty-tree edge cases.
//   - Then single-key and small-tree basics.
//   - Then split / merge mechanics verified through isValid() and structural
//     assertions (height, size, full-scan ordering).
//   - Then delete mechanics: no-underflow, borrow-right, borrow-left, merge.
//   - Then range scan and full scan against brute-force reference.
//   - Finally, a randomised stress test that mirrors every mutation in a
//     reference TreeMap and compares the entire keyspace after each step.
//
// Each test calls isValid() at the end to ensure all B+ tree invariants hold.
// ============================================================================

package com.codingadventures.bplustree;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.util.*;
import java.util.stream.Collectors;

import static org.junit.jupiter.api.Assertions.*;

class BPlusTreeTest {

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /** Extract just the keys from a list of Map.Entry. */
    private static <K, V> List<K> keys(List<Map.Entry<K, V>> entries) {
        return entries.stream().map(Map.Entry::getKey).collect(Collectors.toList());
    }

    /** Extract just the values from a list of Map.Entry. */
    private static <K, V> List<V> values(List<Map.Entry<K, V>> entries) {
        return entries.stream().map(Map.Entry::getValue).collect(Collectors.toList());
    }

    /** Build a BPlusTree<Integer,String> from a vararg sequence of keys (values = "v"+key). */
    private static BPlusTree<Integer, String> treeOf(int t, int... keys) {
        BPlusTree<Integer, String> tree = new BPlusTree<>(t);
        for (int k : keys) tree.insert(k, "v" + k);
        return tree;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Empty-Tree Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void emptyTree_size() {
        BPlusTree<Integer, String> tree = new BPlusTree<>();
        assertEquals(0, tree.size());
    }

    @Test
    void emptyTree_isEmpty() {
        assertTrue(new BPlusTree<Integer, String>().isEmpty());
    }

    @Test
    void emptyTree_height() {
        assertEquals(0, new BPlusTree<Integer, String>().height());
    }

    @Test
    void emptyTree_searchReturnsNull() {
        assertNull(new BPlusTree<Integer, String>().search(42));
    }

    @Test
    void emptyTree_containsFalse() {
        assertFalse(new BPlusTree<Integer, String>().contains(1));
    }

    @Test
    void emptyTree_fullScanEmpty() {
        assertTrue(new BPlusTree<Integer, String>().fullScan().isEmpty());
    }

    @Test
    void emptyTree_rangeScanEmpty() {
        assertTrue(new BPlusTree<Integer, String>().rangeScan(1, 100).isEmpty());
    }

    @Test
    void emptyTree_iteratorEmpty() {
        assertFalse(new BPlusTree<Integer, String>().iterator().hasNext());
    }

    @Test
    void emptyTree_minKeyThrows() {
        assertThrows(NoSuchElementException.class, () -> new BPlusTree<Integer, String>().minKey());
    }

    @Test
    void emptyTree_maxKeyThrows() {
        assertThrows(NoSuchElementException.class, () -> new BPlusTree<Integer, String>().maxKey());
    }

    @Test
    void emptyTree_deleteNoOp() {
        BPlusTree<Integer, String> tree = new BPlusTree<>();
        assertDoesNotThrow(() -> tree.delete(5));
        assertEquals(0, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    void emptyTree_isValid() {
        assertTrue(new BPlusTree<Integer, String>().isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Constructor Validation
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void constructor_defaultDegreeIs2() {
        // Just verify it constructs without error and is empty.
        BPlusTree<String, Integer> tree = new BPlusTree<>();
        assertTrue(tree.isEmpty());
        assertTrue(tree.isValid());
    }

    @Test
    void constructor_degreeOne_throws() {
        assertThrows(IllegalArgumentException.class, () -> new BPlusTree<>(1));
    }

    @Test
    void constructor_degreeZero_throws() {
        assertThrows(IllegalArgumentException.class, () -> new BPlusTree<>(0));
    }

    @Test
    void constructor_degreeNegative_throws() {
        assertThrows(IllegalArgumentException.class, () -> new BPlusTree<>(-5));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2b. Null-Key and Inverted-Bounds Validation
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void nullKey_insert_throws() {
        BPlusTree<String, String> tree = new BPlusTree<>();
        assertThrows(NullPointerException.class, () -> tree.insert(null, "v"));
    }

    @Test
    void nullKey_search_throws() {
        BPlusTree<String, String> tree = new BPlusTree<>();
        assertThrows(NullPointerException.class, () -> tree.search(null));
    }

    @Test
    void nullKey_contains_throws() {
        BPlusTree<String, String> tree = new BPlusTree<>();
        assertThrows(NullPointerException.class, () -> tree.contains(null));
    }

    @Test
    void nullKey_delete_throws() {
        BPlusTree<String, String> tree = new BPlusTree<>();
        assertThrows(NullPointerException.class, () -> tree.delete(null));
    }

    @Test
    void nullKey_rangeScanLow_throws() {
        BPlusTree<Integer, String> tree = new BPlusTree<>();
        assertThrows(NullPointerException.class, () -> tree.rangeScan(null, 10));
    }

    @Test
    void nullKey_rangeScanHigh_throws() {
        BPlusTree<Integer, String> tree = new BPlusTree<>();
        assertThrows(NullPointerException.class, () -> tree.rangeScan(1, null));
    }

    @Test
    void rangeScan_invertedBounds_throws() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        assertThrows(IllegalArgumentException.class, () -> tree.rangeScan(5, 1));
    }

    @Test
    void contains_withNullValue_returnsTrue() {
        // A key inserted with a null value must still be reported as present.
        BPlusTree<Integer, String> tree = new BPlusTree<>();
        tree.insert(42, null);
        assertTrue(tree.contains(42));   // would be false if contains() used null sentinel
        assertNull(tree.search(42));     // value is null — search returns null
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Single-Key Operations
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void singleKey_insert_size1() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(42, "answer");
        assertEquals(1, tree.size());
        assertFalse(tree.isEmpty());
    }

    @Test
    void singleKey_search_found() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(42, "answer");
        assertEquals("answer", tree.search(42));
    }

    @Test
    void singleKey_contains() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(42, "answer");
        assertTrue(tree.contains(42));
        assertFalse(tree.contains(0));
    }

    @Test
    void singleKey_height0() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(42, "answer");
        assertEquals(0, tree.height()); // still a single leaf
    }

    @Test
    void singleKey_minMaxKey() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(42, "answer");
        assertEquals(42, tree.minKey());
        assertEquals(42, tree.maxKey());
    }

    @Test
    void singleKey_fullScan() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(10, "ten");
        List<Map.Entry<Integer, String>> scan = tree.fullScan();
        assertEquals(List.of(10), keys(scan));
        assertEquals(List.of("ten"), values(scan));
    }

    @Test
    void singleKey_delete_thenEmpty() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(10, "ten");
        tree.delete(10);
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
        assertNull(tree.search(10));
        assertTrue(tree.isValid());
    }

    @Test
    void singleKey_isValid() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(10, "ten");
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 4. In-Place Update (Duplicate Key)
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void update_replacesValue() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(5, "five");
        tree.insert(5, "FIVE");
        assertEquals(1, tree.size());  // still 1 key
        assertEquals("FIVE", tree.search(5));
    }

    @Test
    void update_manyDuplicates_sizeUnchanged() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        for (int i = 0; i < 100; i++) {
            tree.insert(42, "v" + i);
        }
        assertEquals(1, tree.size());
        assertEquals("v99", tree.search(42));
        assertTrue(tree.isValid());
    }

    @Test
    void update_afterSplit_replacesValue() {
        // Force a split first, then update a key on either side.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5);
        assertTrue(tree.height() >= 1);  // at least one split occurred
        tree.insert(3, "three-updated");
        assertEquals("three-updated", tree.search(3));
        assertEquals(5, tree.size());
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5. Leaf-Split Mechanics (t=2)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // t=2: max keys per node = 2*2-1 = 3.  Insert 4 keys → first leaf split.
    //
    // After inserting 1, 2, 3, 4:
    //   Leaf was [1,2,3], then 4 added gives [1,2,3,4] → split at mid=2.
    //   separator = 3.  Left=[1,2], Right=[3,4].
    //   New root (InternalNode) has keys=[3].
    //   height goes from 0 → 1.

    @Test
    void leafSplit_heightBeforeAndAfter() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(1, "one");
        tree.insert(2, "two");
        tree.insert(3, "three");
        assertEquals(0, tree.height());  // still one leaf
        tree.insert(4, "four");
        assertEquals(1, tree.height());  // root now internal
    }

    @Test
    void leafSplit_separatorStaysInRightLeaf() {
        // The separator key (3) must be findable — it lives in the right leaf.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4);
        assertEquals("v3", tree.search(3));  // 3 is in right leaf (not only in internal)
        assertTrue(tree.isValid());
    }

    @Test
    void leafSplit_fullScanOrdered() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4);
        assertEquals(List.of(1, 2, 3, 4), keys(tree.fullScan()));
    }

    @Test
    void leafSplit_linkedListIntact() {
        // After split, both leaves must be reachable via the linked list.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4);
        // fullScan uses the linked list — if it returns 4 entries they're all connected.
        assertEquals(4, tree.fullScan().size());
    }

    @Test
    void multipleSplits_sizeAndOrder() {
        // Insert 8 keys with t=2 → multiple leaf splits and at least one internal split.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5, 6, 7, 8);
        assertEquals(8, tree.size());
        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7, 8), keys(tree.fullScan()));
        assertTrue(tree.isValid());
    }

    @Test
    void multipleSplits_reverseOrder() {
        BPlusTree<Integer, String> tree = treeOf(2, 8, 7, 6, 5, 4, 3, 2, 1);
        assertEquals(8, tree.size());
        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7, 8), keys(tree.fullScan()));
        assertTrue(tree.isValid());
    }

    @Test
    void multipleSplits_randomOrder() {
        int[] shuffled = {5, 1, 8, 3, 7, 2, 6, 4};
        BPlusTree<Integer, String> tree = treeOf(2, shuffled);
        assertEquals(8, tree.size());
        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7, 8), keys(tree.fullScan()));
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 6. Height Behaviour (t=2)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // With t=2 every node can hold at most 3 keys.
    // height 0 = 1 leaf  → up to 3 keys
    // height 1 = 1 root + leaves → root can hold 3 keys → 4 leaves → 12 keys
    // The actual threshold depends on split patterns; we just verify monotonicity.

    @Test
    void height_growsMonotonically() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        int lastHeight = 0;
        for (int i = 0; i < 50; i++) {
            tree.insert(i, "v" + i);
            int h = tree.height();
            assertTrue(h >= lastHeight, "height must not decrease");
            lastHeight = h;
        }
        assertTrue(lastHeight >= 2); // must have grown past height 1 by key 50
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 7. Range Scan
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void rangeScan_exactBounds() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5);
        List<Map.Entry<Integer, String>> res = tree.rangeScan(2, 4);
        assertEquals(List.of(2, 3, 4), keys(res));
    }

    @Test
    void rangeScan_entireTree() {
        BPlusTree<Integer, String> tree = treeOf(2, 3, 1, 5, 2, 4);
        List<Map.Entry<Integer, String>> res = tree.rangeScan(1, 5);
        assertEquals(List.of(1, 2, 3, 4, 5), keys(res));
    }

    @Test
    void rangeScan_noResults() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        assertTrue(tree.rangeScan(10, 20).isEmpty());
    }

    @Test
    void rangeScan_singleKey() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        List<Map.Entry<Integer, String>> res = tree.rangeScan(2, 2);
        assertEquals(List.of(2), keys(res));
    }

    @Test
    void rangeScan_lowEqualsHigh_absent() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 3, 5);
        assertTrue(tree.rangeScan(2, 2).isEmpty());
    }

    @Test
    void rangeScan_crossesLeafBoundary() {
        // With t=2 and keys [1..6], the leaves have at most 3 keys each.
        // A range spanning two leaves must use the linked list.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5, 6);
        List<Map.Entry<Integer, String>> res = tree.rangeScan(2, 5);
        assertEquals(List.of(2, 3, 4, 5), keys(res));
    }

    @Test
    void rangeScan_leftOpenBoundary() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5);
        List<Map.Entry<Integer, String>> res = tree.rangeScan(1, 3);
        assertEquals(List.of(1, 2, 3), keys(res));
    }

    @Test
    void rangeScan_rightOpenBoundary() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5);
        List<Map.Entry<Integer, String>> res = tree.rangeScan(3, 5);
        assertEquals(List.of(3, 4, 5), keys(res));
    }

    @Test
    void rangeScan_bruteForce() {
        // Compare rangeScan against a brute-force scan for every sub-range.
        int n = 20;
        BPlusTree<Integer, String> tree = new BPlusTree<>(3);
        for (int i = 1; i <= n; i++) tree.insert(i, "v" + i);

        for (int lo = 1; lo <= n; lo++) {
            for (int hi = lo; hi <= n; hi++) {
                List<Integer> got = keys(tree.rangeScan(lo, hi));
                List<Integer> expected = new ArrayList<>();
                for (int k = lo; k <= hi; k++) expected.add(k);
                assertEquals(expected, got, "rangeScan(" + lo + "," + hi + ") wrong");
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 8. Full Scan and Iterator
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void fullScan_emptyTree() {
        assertTrue(new BPlusTree<Integer, String>().fullScan().isEmpty());
    }

    @Test
    void fullScan_sortedResult() {
        int[] shuffled = {9, 3, 7, 1, 5, 2, 8, 4, 6, 10};
        BPlusTree<Integer, String> tree = treeOf(2, shuffled);
        List<Integer> ks = keys(tree.fullScan());
        for (int i = 1; i < ks.size(); i++) {
            assertTrue(ks.get(i - 1) < ks.get(i), "fullScan not sorted at index " + i);
        }
    }

    @Test
    void iterator_matchesFullScan() {
        BPlusTree<Integer, String> tree = treeOf(2, 5, 3, 7, 1, 4, 6, 2);
        List<Map.Entry<Integer, String>> scan = tree.fullScan();
        List<Map.Entry<Integer, String>> iter = new ArrayList<>();
        for (Map.Entry<Integer, String> e : tree) iter.add(e);
        assertEquals(keys(scan), keys(iter));
        assertEquals(values(scan), values(iter));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 9. Min / Max Key
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void minMaxKey_singleKey() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        tree.insert(42, "x");
        assertEquals(42, tree.minKey());
        assertEquals(42, tree.maxKey());
    }

    @Test
    void minMaxKey_multipleKeys() {
        BPlusTree<Integer, String> tree = treeOf(2, 5, 1, 8, 3, 7);
        assertEquals(1, tree.minKey());
        assertEquals(8, tree.maxKey());
    }

    @Test
    void minMaxKey_afterInsertSmaller() {
        BPlusTree<Integer, String> tree = treeOf(2, 5, 6, 7);
        tree.insert(1, "one");
        assertEquals(1, tree.minKey());
    }

    @Test
    void minMaxKey_afterInsertLarger() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        tree.insert(100, "hundred");
        assertEquals(100, tree.maxKey());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 10. Delete — No Underflow
    // ─────────────────────────────────────────────────────────────────────────
    //
    // With t=2, non-root leaves need at least t-1 = 1 key.
    // Inserting 3 keys fills a leaf perfectly.  Deleting one leaves 2 keys ≥ 1.

    @Test
    void delete_noUnderflow_found() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        tree.delete(2);
        assertEquals(2, tree.size());
        assertNull(tree.search(2));
        assertEquals(List.of(1, 3), keys(tree.fullScan()));
        assertTrue(tree.isValid());
    }

    @Test
    void delete_noUnderflow_notPresent_noOp() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        tree.delete(99);
        assertEquals(3, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    void delete_allThree_empty() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3);
        tree.delete(1);
        tree.delete(2);
        tree.delete(3);
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 11. Delete — Borrow From Right Sibling
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Setup (t=2):
    //   Insert 1,2,3,4,5 → leaves ~[1,2] [3,4,5] with root key [3].
    //   Delete 1 → left leaf has [2].  Right sibling has [3,4,5] > t-1=1 spare keys.
    //   Borrow 3 from right, update parent separator to 4.
    //   Result: left=[2,3], right=[4,5], parent key=[4].

    @Test
    void delete_borrowFromRight() {
        // Insert enough to get a split, then delete from the left leaf.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5);
        // Understand the structure: fullScan gives [1,2,3,4,5], height >= 1
        assertTrue(tree.height() >= 1);
        // Delete 1 — if it triggers underflow, should borrow from right.
        tree.delete(1);
        assertEquals(4, tree.size());
        assertNull(tree.search(1));
        assertEquals(List.of(2, 3, 4, 5), keys(tree.fullScan()));
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 12. Delete — Borrow From Left Sibling
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void delete_borrowFromLeft() {
        // Build: [1,2,3] in left leaf, [4,5] in right leaf, root key = [4].
        // To get this, insert 1..5 with t=2.
        // Delete 4 or 5 so the right leaf has only 1 key (underflow with t=3).
        // With t=3 and 5 keys: max 5 per node, t-1=2.
        BPlusTree<Integer, String> tree = new BPlusTree<>(3);
        // Insert 1..6 to force a split with t=3 (max 5 keys → split at 6)
        for (int i = 1; i <= 6; i++) tree.insert(i, "v" + i);
        // Now delete from right side so it might need to borrow from left.
        tree.delete(6);
        tree.delete(5);
        // Right now has underflow if right leaf had only 2 and we removed 2.
        assertEquals(4, tree.size());
        assertEquals(List.of(1, 2, 3, 4), keys(tree.fullScan()));
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 13. Delete — Merge Leaves
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Merge happens when the underflowing leaf's neighbour also has exactly t-1 keys.
    // After merge, a separator is removed from the parent.

    @Test
    void delete_mergeThenHeightDecreases() {
        // Insert just enough to create height 1, then delete down to height 0.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4);
        assertEquals(1, tree.height());
        // Delete all but one key — tree should collapse back to a single leaf.
        tree.delete(2);
        tree.delete(3);
        tree.delete(4);
        assertEquals(1, tree.size());
        assertEquals(0, tree.height());
        assertTrue(tree.isValid());
    }

    @Test
    void delete_mergePreservesLinkedList() {
        // After merge, the linked list must skip the absorbed leaf.
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5, 6);
        // Delete to trigger merge.
        tree.delete(3);
        tree.delete(4);
        // fullScan walks the linked list — it must still work correctly.
        List<Integer> remaining = keys(tree.fullScan());
        assertEquals(List.of(1, 2, 5, 6), remaining);
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 14. Delete All Keys
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void deleteAll_sequential() {
        int n = 20;
        BPlusTree<Integer, String> tree = treeOf(2, intRange(1, n));
        for (int i = 1; i <= n; i++) {
            tree.delete(i);
            assertTrue(tree.isValid(), "invalid after deleting " + i);
        }
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
    }

    @Test
    void deleteAll_reverseOrder() {
        int n = 20;
        BPlusTree<Integer, String> tree = treeOf(2, intRange(1, n));
        for (int i = n; i >= 1; i--) {
            tree.delete(i);
            assertTrue(tree.isValid(), "invalid after deleting " + i);
        }
        assertEquals(0, tree.size());
    }

    @Test
    void deleteAll_randomOrder() {
        int n = 30;
        int[] keys = intRange(1, n);
        shuffleArray(keys, new Random(42));
        BPlusTree<Integer, String> tree = treeOf(2, intRange(1, n));
        for (int k : keys) {
            tree.delete(k);
            assertTrue(tree.isValid(), "invalid after deleting " + k);
        }
        assertEquals(0, tree.size());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 15. Routing Invariant (Separator Key Check)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // isValid() checks the routing invariant: for every separator keys[i],
    //   - max(children[i])  <  keys[i]
    //   - min(children[i+1]) >= keys[i]
    //
    // Note: separators may be stale after a non-structural delete (the key that was
    // a separator copy gets deleted from its leaf, but the separator stays in the
    // internal node).  This is correct B+ tree behaviour — routing still works.
    // isValid() uses the weaker routing invariant, not exact-equality.

    @Test
    void separatorInvariant_afterInserts() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        for (int i : new int[]{10, 20, 5, 15, 25, 1, 12, 18, 22}) {
            tree.insert(i, "v" + i);
            assertTrue(tree.isValid(), "isValid() failed after inserting " + i);
        }
    }

    @Test
    void separatorInvariant_afterDeletes() {
        // Deleting a key that equals a separator leaves the separator stale.
        // The routing invariant must still hold (and isValid() uses that weaker check).
        BPlusTree<Integer, String> tree = treeOf(2, 5, 10, 15, 20, 25, 30);
        for (int k : new int[]{15, 5, 25, 10}) {
            tree.delete(k);
            assertNull(tree.search(k), "search after delete should return null for " + k);
            assertTrue(tree.isValid(), "isValid() failed after deleting " + k);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 16. Various Minimum Degrees
    // ─────────────────────────────────────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(ints = {2, 3, 4, 5, 10})
    void parameterisedDegree_insertAndSearch(int t) {
        BPlusTree<Integer, String> tree = new BPlusTree<>(t);
        int n = 4 * t * t;  // enough to force several splits at any degree
        for (int i = 1; i <= n; i++) tree.insert(i, "v" + i);
        assertEquals(n, tree.size());
        for (int i = 1; i <= n; i++) assertEquals("v" + i, tree.search(i));
        assertTrue(tree.isValid());
    }

    @ParameterizedTest
    @ValueSource(ints = {2, 3, 4, 5, 10})
    void parameterisedDegree_deleteAll(int t) {
        int n = 3 * t;
        BPlusTree<Integer, String> tree = treeOf(t, intRange(1, n));
        for (int i = 1; i <= n; i++) {
            tree.delete(i);
            assertTrue(tree.isValid(), "degree=" + t + " invalid after deleting " + i);
        }
        assertEquals(0, tree.size());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 17. String Keys (non-Integer key type)
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void stringKeys_insertAndSearch() {
        BPlusTree<String, Integer> tree = new BPlusTree<>(2);
        tree.insert("banana", 2);
        tree.insert("apple", 1);
        tree.insert("cherry", 3);
        tree.insert("date", 4);
        tree.insert("elderberry", 5);

        assertEquals(1, tree.search("apple"));
        assertEquals(3, tree.search("cherry"));
        assertNull(tree.search("fig"));
    }

    @Test
    void stringKeys_fullScanSorted() {
        BPlusTree<String, Integer> tree = new BPlusTree<>(2);
        String[] words = {"fig", "apple", "cherry", "banana", "elderberry", "date"};
        for (int i = 0; i < words.length; i++) tree.insert(words[i], i);

        List<String> ks = keys(tree.fullScan());
        List<String> sorted = Arrays.stream(words).sorted().collect(Collectors.toList());
        assertEquals(sorted, ks);
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 18. Negative and Large Keys
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void negativeKeys_insertAndScan() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        for (int i = -5; i <= 5; i++) tree.insert(i, "v" + i);
        List<Integer> ks = keys(tree.fullScan());
        for (int i = 1; i < ks.size(); i++) {
            assertTrue(ks.get(i - 1) < ks.get(i));
        }
        assertEquals(11, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    void largeKeys_insertAndScan() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(4);
        tree.insert(Integer.MAX_VALUE, "max");
        tree.insert(Integer.MIN_VALUE, "min");
        tree.insert(0, "zero");
        assertEquals(Integer.MIN_VALUE, tree.minKey());
        assertEquals(Integer.MAX_VALUE, tree.maxKey());
        assertEquals(3, tree.size());
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 19. Linked-List Integrity After Mixed Operations
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void linkedList_integrityAfterMixedOps() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        Random rng = new Random(7);
        TreeMap<Integer, String> ref = new TreeMap<>();
        int[] candidates = intRange(1, 30);
        for (int i = 0; i < 60; i++) {
            int k = candidates[rng.nextInt(candidates.length)];
            if (rng.nextBoolean()) {
                tree.insert(k, "v" + k);
                ref.put(k, "v" + k);
            } else {
                tree.delete(k);
                ref.remove(k);
            }
            // After every operation, linked list must reflect sorted order.
            assertEquals(new ArrayList<>(ref.keySet()), keys(tree.fullScan()),
                    "linked list mismatch at step " + i);
        }
        assertTrue(tree.isValid());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 20. Stress Test vs TreeMap Reference
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void stress_randomOps_matchesTreeMap() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(3);
        TreeMap<Integer, String> ref = new TreeMap<>();
        Random rng = new Random(12345);

        int ops = 500;
        int keySpace = 100;

        for (int i = 0; i < ops; i++) {
            int k = rng.nextInt(keySpace);
            if (rng.nextInt(3) > 0) {  // 2/3 chance insert
                String v = "v" + k + "_" + i;
                tree.insert(k, v);
                ref.put(k, v);
            } else {
                tree.delete(k);
                ref.remove(k);
            }
            // Spot-check search.
            for (int check : new int[]{k, rng.nextInt(keySpace)}) {
                assertEquals(ref.get(check), tree.search(check),
                        "search(" + check + ") wrong at step " + i);
            }
        }

        // Full comparison.
        assertEquals(ref.size(), tree.size());
        assertEquals(new ArrayList<>(ref.keySet()), keys(tree.fullScan()));
        for (Map.Entry<Integer, String> e : ref.entrySet()) {
            assertEquals(e.getValue(), tree.search(e.getKey()));
        }
        assertTrue(tree.isValid());
    }

    @Test
    void stress_largeTree_t4() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(4);
        int n = 1000;
        // Sequential insert.
        for (int i = 1; i <= n; i++) tree.insert(i, "v" + i);
        assertEquals(n, tree.size());
        assertTrue(tree.isValid());
        // Range scan all.
        List<Map.Entry<Integer, String>> scan = tree.rangeScan(1, n);
        assertEquals(n, scan.size());
        assertEquals(1, (int) scan.get(0).getKey());
        assertEquals(n, (int) scan.get(n - 1).getKey());
        // Delete every even key.
        for (int i = 2; i <= n; i += 2) tree.delete(i);
        assertEquals(n / 2, tree.size());
        assertTrue(tree.isValid());
        // Verify only odd keys remain.
        List<Integer> ks = keys(tree.fullScan());
        for (int k : ks) assertTrue(k % 2 != 0, "even key " + k + " should be gone");
    }

    @Test
    void stress_repeatInsertDelete() {
        BPlusTree<Integer, String> tree = new BPlusTree<>(2);
        // Insert 1..20, delete 1..20, repeat 10 times.
        for (int round = 0; round < 10; round++) {
            for (int i = 1; i <= 20; i++) tree.insert(i, "r" + round + "_" + i);
            assertTrue(tree.isValid(), "invalid after insert round " + round);
            assertEquals(20, tree.size());
            for (int i = 1; i <= 20; i++) tree.delete(i);
            assertTrue(tree.isValid(), "invalid after delete round " + round);
            assertEquals(0, tree.size());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 21. toString Smoke Test
    // ─────────────────────────────────────────────────────────────────────────

    @Test
    void toString_containsSizeAndHeight() {
        BPlusTree<Integer, String> tree = treeOf(2, 1, 2, 3, 4, 5);
        String s = tree.toString();
        assertTrue(s.contains("size=5"), "toString should include size=5, got: " + s);
        assertTrue(s.contains("height="), "toString should include height, got: " + s);
        assertTrue(s.contains("t=2"), "toString should include t=2, got: " + s);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private test utilities
    // ─────────────────────────────────────────────────────────────────────────

    private static int[] intRange(int from, int to) {
        int[] arr = new int[to - from + 1];
        for (int i = 0; i < arr.length; i++) arr[i] = from + i;
        return arr;
    }

    private static void shuffleArray(int[] arr, Random rng) {
        for (int i = arr.length - 1; i > 0; i--) {
            int j = rng.nextInt(i + 1);
            int tmp = arr[i]; arr[i] = arr[j]; arr[j] = tmp;
        }
    }
}
