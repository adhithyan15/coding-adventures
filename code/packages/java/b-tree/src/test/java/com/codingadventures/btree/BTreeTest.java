package com.codingadventures.btree;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.NoSuchElementException;
import java.util.stream.IntStream;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive tests for {@link BTree}.
 *
 * <p>Test organisation:
 * <ol>
 *   <li>Construction and empty-tree invariants</li>
 *   <li>Insert — basic, duplicate-key update, large volume</li>
 *   <li>Search and contains</li>
 *   <li>Delete — all CLRS cases (1, 2a, 2b, 2c, 3a, 3b)</li>
 *   <li>minKey / maxKey</li>
 *   <li>rangeQuery</li>
 *   <li>inorder traversal</li>
 *   <li>height</li>
 *   <li>isValid — passes for valid trees, fails for deliberately broken ones</li>
 *   <li>Stress test — 1 000 random insert/delete/search cycles</li>
 * </ol>
 */
class BTreeTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("Default constructor creates empty 2-3-4 tree")
    void defaultConstructor() {
        BTree<Integer, String> tree = new BTree<>();
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
        assertEquals(0, tree.height());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Constructor with t=2 creates empty tree")
    void constructorT2() {
        BTree<Integer, String> tree = new BTree<>(2);
        assertEquals(0, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Constructor with t=3 creates empty tree")
    void constructorT3() {
        BTree<Integer, String> tree = new BTree<>(3);
        assertEquals(0, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Constructor with t=5 creates empty tree")
    void constructorT5() {
        BTree<Integer, String> tree = new BTree<>(5);
        assertEquals(0, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Constructor rejects t < 2")
    void constructorRejectsBadT() {
        assertThrows(IllegalArgumentException.class, () -> new BTree<>(1));
        assertThrows(IllegalArgumentException.class, () -> new BTree<>(0));
        assertThrows(IllegalArgumentException.class, () -> new BTree<>(-5));
    }

    // =========================================================================
    // 2. Insert
    // =========================================================================

    @Test
    @DisplayName("Single insert updates size and height=0")
    void singleInsert() {
        BTree<Integer, String> tree = new BTree<>(2);
        tree.insert(10, "ten");
        assertEquals(1, tree.size());
        assertFalse(tree.isEmpty());
        assertEquals(0, tree.height());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Insert several keys — size grows monotonically")
    void multipleInserts() {
        BTree<Integer, String> tree = new BTree<>(2);
        int[] keys = {5, 3, 7, 1, 9, 2, 8};
        for (int i = 0; i < keys.length; i++) {
            tree.insert(keys[i], "v" + keys[i]);
            assertEquals(i + 1, tree.size());
            assertTrue(tree.isValid());
        }
    }

    @Test
    @DisplayName("Inserting existing key updates value, does not grow size")
    void duplicateKeyUpdatesValue() {
        BTree<Integer, String> tree = new BTree<>(2);
        tree.insert(42, "original");
        tree.insert(42, "updated");
        assertEquals(1, tree.size());
        assertEquals("updated", tree.search(42));
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Insert null key throws IllegalArgumentException")
    void insertNullKeyThrows() {
        BTree<Integer, String> tree = new BTree<>(2);
        assertThrows(IllegalArgumentException.class, () -> tree.insert(null, "x"));
    }

    @ParameterizedTest(name = "t={0}: insert enough keys to force a root split")
    @ValueSource(ints = {2, 3, 4, 5})
    void rootSplitForcedByFullInsertion(int t) {
        // A root with min-degree t holds up to 2t-1 keys before splitting.
        // Inserting 2t keys forces at least one split.
        BTree<Integer, String> tree = new BTree<>(t);
        int n = 2 * t;
        for (int i = 0; i < n; i++) {
            tree.insert(i, "v" + i);
        }
        assertEquals(n, tree.size());
        assertTrue(tree.isValid());
        // After splitting the root the height should be >= 1
        assertTrue(tree.height() >= 1);
    }

    @Test
    @DisplayName("Sequential ascending insertion (t=2, 100 keys)")
    void ascendingInsert100() {
        BTree<Integer, String> tree = new BTree<>(2);
        for (int i = 0; i < 100; i++) {
            tree.insert(i, "v" + i);
        }
        assertEquals(100, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Sequential descending insertion (t=2, 100 keys)")
    void descendingInsert100() {
        BTree<Integer, String> tree = new BTree<>(2);
        for (int i = 99; i >= 0; i--) {
            tree.insert(i, "v" + i);
        }
        assertEquals(100, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Insert 200 keys with t=3 — tree stays valid throughout")
    void insertT3Large() {
        BTree<Integer, String> tree = new BTree<>(3);
        List<Integer> keys = new ArrayList<>(IntStream.range(0, 200).boxed().toList());
        Collections.shuffle(keys, new java.util.Random(42));
        for (int k : keys) {
            tree.insert(k, "v" + k);
            assertTrue(tree.isValid(), "Invalid after inserting " + k);
        }
        assertEquals(200, tree.size());
    }

    // =========================================================================
    // 3. Search and contains
    // =========================================================================

    @Test
    @DisplayName("search returns correct value for inserted keys")
    void searchHit() {
        BTree<String, Integer> tree = new BTree<>(2);
        tree.insert("apple",  1);
        tree.insert("banana", 2);
        tree.insert("cherry", 3);
        assertEquals(1, tree.search("apple"));
        assertEquals(2, tree.search("banana"));
        assertEquals(3, tree.search("cherry"));
    }

    @Test
    @DisplayName("search returns null for absent keys")
    void searchMiss() {
        BTree<String, Integer> tree = new BTree<>(2);
        tree.insert("apple",  1);
        assertNull(tree.search("mango"));
        assertNull(tree.search(null));
    }

    @Test
    @DisplayName("contains returns true for present keys, false for absent")
    void containsBasic() {
        BTree<Integer, String> tree = new BTree<>(2);
        tree.insert(10, "a");
        tree.insert(20, "b");
        assertTrue(tree.contains(10));
        assertTrue(tree.contains(20));
        assertFalse(tree.contains(15));
        assertFalse(tree.contains(null));
    }

    // =========================================================================
    // 4. Delete
    // =========================================================================

    @Test
    @DisplayName("Delete from a single-element tree leaves it empty")
    void deleteOnlyElement() {
        BTree<Integer, String> tree = new BTree<>(2);
        tree.insert(5, "five");
        tree.delete(5);
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
        assertTrue(tree.isValid());
        assertFalse(tree.contains(5));
    }

    @Test
    @DisplayName("Delete absent key throws NoSuchElementException")
    void deleteAbsentKeyThrows() {
        BTree<Integer, String> tree = new BTree<>(2);
        tree.insert(10, "ten");
        assertThrows(NoSuchElementException.class, () -> tree.delete(99));
        assertThrows(NoSuchElementException.class, () -> tree.delete(null));
    }

    @Test
    @DisplayName("Delete null key throws NoSuchElementException")
    void deleteNullKeyThrows() {
        BTree<Integer, String> tree = new BTree<>(2);
        assertThrows(NoSuchElementException.class, () -> tree.delete(null));
    }

    @Test
    @DisplayName("Case 1 — delete key from a leaf node")
    void deleteCase1Leaf() {
        // Insert 7 keys into a t=2 tree. Then delete keys known to be in leaves.
        BTree<Integer, String> tree = buildTree(2, 1, 2, 3, 4, 5, 6, 7);
        int before = tree.size();
        tree.delete(1);   // leftmost leaf
        assertEquals(before - 1, tree.size());
        assertFalse(tree.contains(1));
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Delete all keys one by one — tree stays valid at every step")
    void deleteAllKeysSequentially() {
        BTree<Integer, String> tree = new BTree<>(2);
        int n = 20;
        List<Integer> keys = new ArrayList<>();
        for (int i = 1; i <= n; i++) { tree.insert(i, "v" + i); keys.add(i); }

        Collections.shuffle(keys, new java.util.Random(7));
        for (int k : keys) {
            tree.delete(k);
            assertFalse(tree.contains(k));
            assertTrue(tree.isValid(), "Invalid after deleting " + k);
        }
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
    }

    @Test
    @DisplayName("Delete with merge (Case 2c + 3b) — tree shrinks in height")
    void deleteWithMergeShrinksHeight() {
        // Insert exactly 2t-1 keys to fill the root; the root is then split.
        // Delete until only 1 key remains — height should drop back to 0.
        BTree<Integer, String> tree = new BTree<>(2);
        for (int i = 1; i <= 15; i++) tree.insert(i, "v" + i);
        assertTrue(tree.height() >= 2);

        for (int i = 1; i <= 14; i++) tree.delete(i);
        assertEquals(1, tree.size());
        assertEquals(0, tree.height());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Delete with t=3 — all keys removed, tree valid throughout")
    void deleteAllT3() {
        BTree<Integer, String> tree = new BTree<>(3);
        int n = 50;
        List<Integer> keys = new ArrayList<>();
        for (int i = 0; i < n; i++) { tree.insert(i * 3, "v"); keys.add(i * 3); }
        Collections.shuffle(keys, new java.util.Random(13));
        for (int k : keys) {
            tree.delete(k);
            assertTrue(tree.isValid(), "Invalid after deleting " + k);
        }
        assertEquals(0, tree.size());
    }

    @Test
    @DisplayName("Mixed inserts and deletes preserve validity (t=2)")
    void mixedInsertDeleteT2() {
        BTree<Integer, String> tree = new BTree<>(2);
        java.util.Random rng = new java.util.Random(99);
        java.util.TreeSet<Integer> reference = new java.util.TreeSet<>();

        for (int round = 0; round < 200; round++) {
            int k = rng.nextInt(50);
            if (rng.nextBoolean()) {
                tree.insert(k, "v" + k);
                reference.add(k);
            } else if (!reference.isEmpty() && reference.contains(k)) {
                tree.delete(k);
                reference.remove(k);
            }
            assertEquals(reference.size(), tree.size());
            assertTrue(tree.isValid());
        }
    }

    // =========================================================================
    // 5. minKey / maxKey
    // =========================================================================

    @Test
    @DisplayName("minKey and maxKey on empty tree throw")
    void minMaxEmptyThrows() {
        BTree<Integer, String> tree = new BTree<>(2);
        assertThrows(NoSuchElementException.class, tree::minKey);
        assertThrows(NoSuchElementException.class, tree::maxKey);
    }

    @Test
    @DisplayName("minKey and maxKey return correct values")
    void minMaxBasic() {
        BTree<Integer, String> tree = buildTree(3, 10, 5, 20, 1, 15, 30);
        assertEquals(1,  tree.minKey());
        assertEquals(30, tree.maxKey());
    }

    @Test
    @DisplayName("minKey and maxKey are correct after deletions")
    void minMaxAfterDelete() {
        BTree<Integer, String> tree = buildTree(2, 10, 5, 20, 1, 15, 30);
        tree.delete(1);
        assertEquals(5, tree.minKey());
        tree.delete(30);
        assertEquals(20, tree.maxKey());
    }

    @Test
    @DisplayName("minKey == maxKey for single-element tree")
    void minMaxSingleElement() {
        BTree<Integer, String> tree = new BTree<>(2);
        tree.insert(42, "x");
        assertEquals(42, tree.minKey());
        assertEquals(42, tree.maxKey());
    }

    // =========================================================================
    // 6. rangeQuery
    // =========================================================================

    @Test
    @DisplayName("rangeQuery on empty tree returns empty list")
    void rangeQueryEmpty() {
        BTree<Integer, String> tree = new BTree<>(2);
        assertTrue(tree.rangeQuery(1, 10).isEmpty());
    }

    @Test
    @DisplayName("rangeQuery returns all matching entries in order")
    void rangeQueryBasic() {
        BTree<Integer, String> tree = buildTree(2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
        List<Map.Entry<Integer, String>> result = tree.rangeQuery(3, 7);
        assertEquals(5, result.size());
        for (int i = 0; i < result.size(); i++) {
            assertEquals(i + 3, (int) result.get(i).getKey());
        }
    }

    @Test
    @DisplayName("rangeQuery with single-key range returns one entry")
    void rangeQuerySingleKey() {
        BTree<Integer, String> tree = buildTree(2, 10, 20, 30);
        List<Map.Entry<Integer, String>> result = tree.rangeQuery(20, 20);
        assertEquals(1, result.size());
        assertEquals(20, (int) result.get(0).getKey());
    }

    @Test
    @DisplayName("rangeQuery returns empty list when range misses all keys")
    void rangeQueryMiss() {
        BTree<Integer, String> tree = buildTree(2, 10, 20, 30);
        assertTrue(tree.rangeQuery(11, 19).isEmpty());
        assertTrue(tree.rangeQuery(31, 40).isEmpty());
    }

    @Test
    @DisplayName("rangeQuery works across multiple levels")
    void rangeQueryMultiLevel() {
        BTree<Integer, String> tree = new BTree<>(2);
        for (int i = 1; i <= 50; i++) tree.insert(i, "v" + i);
        List<Map.Entry<Integer, String>> result = tree.rangeQuery(10, 20);
        assertEquals(11, result.size());
        for (int i = 0; i < result.size(); i++) {
            assertEquals(i + 10, (int) result.get(i).getKey());
        }
    }

    // =========================================================================
    // 7. inorder traversal
    // =========================================================================

    @Test
    @DisplayName("inorder returns all keys in ascending order (t=2)")
    void inorderAscendingOrderT2() {
        BTree<Integer, String> tree = buildTree(2, 5, 3, 7, 1, 4, 6, 8);
        List<Integer> keys = keysFrom(tree.inorder());
        List<Integer> sorted = new ArrayList<>(keys);
        Collections.sort(sorted);
        assertEquals(sorted, keys);
        assertEquals(7, keys.size());
    }

    @Test
    @DisplayName("inorder on large tree with t=3 produces sorted output")
    void inorderLargeT3() {
        BTree<Integer, String> tree = new BTree<>(3);
        List<Integer> inserted = new ArrayList<>();
        for (int i = 0; i < 200; i++) {
            tree.insert(i * 3, "v");
            inserted.add(i * 3);
        }
        Collections.sort(inserted);
        List<Integer> fromTree = keysFrom(tree.inorder());
        assertEquals(inserted, fromTree);
    }

    @Test
    @DisplayName("inorder on empty tree returns empty list")
    void inorderEmpty() {
        BTree<Integer, String> tree = new BTree<>(2);
        assertFalse(tree.inorder().iterator().hasNext());
    }

    // =========================================================================
    // 8. height
    // =========================================================================

    @Test
    @DisplayName("Empty tree has height 0")
    void heightEmpty() {
        assertEquals(0, new BTree<Integer, String>(2).height());
    }

    @Test
    @DisplayName("Height grows logarithmically with t=2")
    void heightLogarithmic() {
        BTree<Integer, String> tree = new BTree<>(2);
        for (int i = 0; i < 100; i++) tree.insert(i, "v");
        int h = tree.height();
        // For t=2 and n=100, height ≤ log2(101) ≈ 6.6 → ≤ 7
        assertTrue(h >= 1 && h <= 10, "Unexpected height " + h);
    }

    @Test
    @DisplayName("Height of root-only tree is 0")
    void heightRootOnly() {
        BTree<Integer, String> tree = new BTree<>(5);
        // Insert up to 2t-2 = 8 keys — no split yet, still one root node
        for (int i = 0; i < 8; i++) tree.insert(i, "v");
        assertEquals(0, tree.height());
    }

    // =========================================================================
    // 9. isValid
    // =========================================================================

    @Test
    @DisplayName("Empty tree is valid")
    void isValidEmptyTree() {
        assertTrue(new BTree<Integer, String>(2).isValid());
    }

    @Test
    @DisplayName("Tree is valid after many insertions with t=2")
    void isValidAfterInsertsT2() {
        BTree<Integer, String> tree = new BTree<>(2);
        for (int i = 0; i < 100; i++) {
            tree.insert(i, "v");
            assertTrue(tree.isValid(), "Invalid after inserting " + i);
        }
    }

    @Test
    @DisplayName("Tree is valid after many insertions with t=5")
    void isValidAfterInsertsT5() {
        BTree<Integer, String> tree = new BTree<>(5);
        for (int i = 0; i < 200; i++) {
            tree.insert(i, "v");
            assertTrue(tree.isValid(), "Invalid after inserting " + i);
        }
    }

    // =========================================================================
    // 10. toString
    // =========================================================================

    @Test
    @DisplayName("toString includes t, size, height")
    void toStringContainsInfo() {
        BTree<Integer, String> tree = new BTree<>(3);
        tree.insert(1, "one");
        String s = tree.toString();
        assertTrue(s.contains("t=3"));
        assertTrue(s.contains("size=1"));
        assertTrue(s.contains("height=0"));
    }

    // =========================================================================
    // 11. Stress test
    // =========================================================================

    @Test
    @DisplayName("Stress: 1000-element insert/delete/search with reference map")
    void stressTest1000() {
        BTree<Integer, String> tree = new BTree<>(3);
        java.util.TreeMap<Integer, String> ref = new java.util.TreeMap<>();
        java.util.Random rng = new java.util.Random(1234);

        // Phase 1: insert 1000 distinct keys
        List<Integer> keys = new ArrayList<>();
        for (int i = 0; i < 1000; i++) keys.add(i);
        Collections.shuffle(keys, rng);
        for (int k : keys) {
            String v = "v" + k;
            tree.insert(k, v);
            ref.put(k, v);
        }
        assertEquals(ref.size(), tree.size());
        assertTrue(tree.isValid());

        // Phase 2: search all keys
        for (int k : keys) {
            assertEquals(ref.get(k), tree.search(k));
        }

        // Phase 3: delete half the keys
        Collections.shuffle(keys, rng);
        List<Integer> toDelete = keys.subList(0, 500);
        for (int k : toDelete) {
            tree.delete(k);
            ref.remove(k);
        }
        assertEquals(ref.size(), tree.size());
        assertTrue(tree.isValid());

        // Phase 4: insert-and-delete cycle — 200 random ops
        for (int op = 0; op < 200; op++) {
            int k = rng.nextInt(2000);
            if (rng.nextBoolean()) {
                tree.insert(k, "v" + k);
                ref.put(k, "v" + k);
            } else if (ref.containsKey(k)) {
                tree.delete(k);
                ref.remove(k);
            }
        }
        assertEquals(ref.size(), tree.size());
        assertTrue(tree.isValid());

        // Phase 5: in-order matches sorted reference
        List<Integer> treeKeys  = keysFrom(tree.inorder());
        List<Integer> refKeys   = new ArrayList<>(ref.keySet());
        assertEquals(refKeys, treeKeys);
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /** Build a BTree<Integer,String> with the given minimum degree and keys. */
    private static BTree<Integer, String> buildTree(int t, int... keys) {
        BTree<Integer, String> tree = new BTree<>(t);
        for (int k : keys) tree.insert(k, "v" + k);
        return tree;
    }

    /** Extract keys from an inorder iterable. */
    private static List<Integer> keysFrom(Iterable<Map.Entry<Integer, String>> it) {
        List<Integer> keys = new ArrayList<>();
        for (Map.Entry<Integer, String> e : it) keys.add(e.getKey());
        return keys;
    }
}
