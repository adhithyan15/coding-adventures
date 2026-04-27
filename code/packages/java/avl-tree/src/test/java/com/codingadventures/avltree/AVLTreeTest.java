package com.codingadventures.avltree;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.NoSuchElementException;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive tests for {@link AVLTree}.
 *
 * <p>Test organisation:
 * <ol>
 *   <li>Construction</li>
 *   <li>Insert — basic, duplicate, ascending/descending/random</li>
 *   <li>Contains</li>
 *   <li>Delete — leaf, one-child, two-child, rotations triggered</li>
 *   <li>min / max</li>
 *   <li>predecessor / successor</li>
 *   <li>kthSmallest</li>
 *   <li>rank</li>
 *   <li>toSortedList</li>
 *   <li>height / balanceFactor</li>
 *   <li>isValid / isValidBST</li>
 *   <li>Stress test — 1000 random operations</li>
 * </ol>
 */
class AVLTreeTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("Empty tree has size 0, height -1, is empty")
    void emptyTree() {
        AVLTree<Integer> tree = new AVLTree<>();
        assertEquals(0, tree.size());
        assertEquals(-1, tree.height());
        assertTrue(tree.isEmpty());
        assertTrue(tree.isValid());
    }

    // =========================================================================
    // 2. Insert
    // =========================================================================

    @Test
    @DisplayName("Single insert: size=1, height=0")
    void singleInsert() {
        AVLTree<Integer> tree = new AVLTree<>();
        tree.insert(10);
        assertEquals(1, tree.size());
        assertEquals(0, tree.height());
        assertFalse(tree.isEmpty());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Duplicate insert leaves tree unchanged")
    void duplicateInsert() {
        AVLTree<Integer> tree = new AVLTree<>();
        tree.insert(5);
        tree.insert(5);
        assertEquals(1, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Insert null throws IllegalArgumentException")
    void insertNullThrows() {
        assertThrows(IllegalArgumentException.class, () -> new AVLTree<Integer>().insert(null));
    }

    @Test
    @DisplayName("Ascending insertion triggers right-heavy rebalancing")
    void ascendingInsert() {
        AVLTree<Integer> tree = new AVLTree<>();
        for (int i = 1; i <= 10; i++) tree.insert(i);
        assertEquals(10, tree.size());
        assertTrue(tree.isValid());
        // Height should be O(log n), not O(n)
        assertTrue(tree.height() <= 5, "Height too large: " + tree.height());
    }

    @Test
    @DisplayName("Descending insertion triggers left-heavy rebalancing")
    void descendingInsert() {
        AVLTree<Integer> tree = new AVLTree<>();
        for (int i = 10; i >= 1; i--) tree.insert(i);
        assertEquals(10, tree.size());
        assertTrue(tree.isValid());
        assertTrue(tree.height() <= 5, "Height too large: " + tree.height());
    }

    @Test
    @DisplayName("Random insertion maintains AVL invariant throughout")
    void randomInsert() {
        AVLTree<Integer> tree = new AVLTree<>();
        List<Integer> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) keys.add(i);
        Collections.shuffle(keys, new java.util.Random(42));
        for (int k : keys) {
            tree.insert(k);
            assertTrue(tree.isValid(), "Invalid after inserting " + k);
        }
        assertEquals(100, tree.size());
    }

    // =========================================================================
    // 3. Contains
    // =========================================================================

    @Test
    @DisplayName("contains returns true for inserted values")
    void containsHit() {
        AVLTree<String> tree = new AVLTree<>();
        tree.insert("apple");
        tree.insert("banana");
        assertTrue(tree.contains("apple"));
        assertTrue(tree.contains("banana"));
    }

    @Test
    @DisplayName("contains returns false for absent values and null")
    void containsMiss() {
        AVLTree<String> tree = new AVLTree<>();
        tree.insert("apple");
        assertFalse(tree.contains("mango"));
        assertFalse(tree.contains(null));
    }

    // =========================================================================
    // 4. Delete
    // =========================================================================

    @Test
    @DisplayName("Delete the only element leaves empty tree")
    void deleteOnlyElement() {
        AVLTree<Integer> tree = new AVLTree<>();
        tree.insert(5);
        tree.delete(5);
        assertEquals(0, tree.size());
        assertTrue(tree.isEmpty());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Delete absent value throws NoSuchElementException")
    void deleteAbsentThrows() {
        AVLTree<Integer> tree = new AVLTree<>();
        tree.insert(1);
        assertThrows(NoSuchElementException.class, () -> tree.delete(99));
    }

    @Test
    @DisplayName("Delete a leaf node maintains validity")
    void deleteLeaf() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1, 4);
        tree.delete(1);  // leaf
        assertFalse(tree.contains(1));
        assertEquals(4, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Delete node with one child maintains validity")
    void deleteOneChildNode() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1);
        // 3 has only a left child (1)
        tree.delete(3);
        assertFalse(tree.contains(3));
        assertEquals(3, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Delete node with two children (uses successor)")
    void deleteTwoChildNode() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1, 4, 6, 8);
        tree.delete(5);  // root with two children
        assertFalse(tree.contains(5));
        assertEquals(6, tree.size());
        assertTrue(tree.isValid());
    }

    @Test
    @DisplayName("Delete all nodes one by one — tree stays valid")
    void deleteAllNodes() {
        AVLTree<Integer> tree = new AVLTree<>();
        List<Integer> keys = new ArrayList<>();
        for (int i = 1; i <= 20; i++) { tree.insert(i); keys.add(i); }
        Collections.shuffle(keys, new java.util.Random(7));
        for (int k : keys) {
            tree.delete(k);
            assertFalse(tree.contains(k));
            assertTrue(tree.isValid(), "Invalid after deleting " + k);
        }
        assertTrue(tree.isEmpty());
    }

    @Test
    @DisplayName("Mixed insert and delete preserves validity")
    void mixedInsertDelete() {
        AVLTree<Integer> tree = new AVLTree<>();
        java.util.TreeSet<Integer> ref = new java.util.TreeSet<>();
        java.util.Random rng = new java.util.Random(99);
        for (int i = 0; i < 200; i++) {
            int k = rng.nextInt(50);
            if (rng.nextBoolean()) {
                tree.insert(k);
                ref.add(k);
            } else if (ref.contains(k)) {
                tree.delete(k);
                ref.remove(k);
            }
            assertEquals(ref.size(), tree.size());
            assertTrue(tree.isValid());
        }
    }

    // =========================================================================
    // 5. min / max
    // =========================================================================

    @Test
    @DisplayName("min and max on empty tree throw")
    void minMaxEmptyThrows() {
        AVLTree<Integer> tree = new AVLTree<>();
        assertThrows(NoSuchElementException.class, tree::min);
        assertThrows(NoSuchElementException.class, tree::max);
    }

    @Test
    @DisplayName("min and max return correct values")
    void minMaxBasic() {
        AVLTree<Integer> tree = buildTree(5, 3, 8, 1, 9, 2);
        assertEquals(1, tree.min());
        assertEquals(9, tree.max());
    }

    @Test
    @DisplayName("min and max update correctly after deletions")
    void minMaxAfterDelete() {
        AVLTree<Integer> tree = buildTree(5, 3, 7);
        tree.delete(3);
        assertEquals(5, tree.min());
        tree.delete(7);
        assertEquals(5, tree.max());
    }

    // =========================================================================
    // 6. predecessor / successor
    // =========================================================================

    @Test
    @DisplayName("predecessor returns null when value is the minimum")
    void predecessorOfMin() {
        AVLTree<Integer> tree = buildTree(5, 3, 7);
        assertNull(tree.predecessor(3));
    }

    @Test
    @DisplayName("predecessor returns correct value")
    void predecessorBasic() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1, 4, 6, 8);
        assertEquals(4, tree.predecessor(5));
        assertEquals(1, tree.predecessor(3));
        assertEquals(6, tree.predecessor(7));
    }

    @Test
    @DisplayName("successor returns null when value is the maximum")
    void successorOfMax() {
        AVLTree<Integer> tree = buildTree(5, 3, 7);
        assertNull(tree.successor(7));
    }

    @Test
    @DisplayName("successor returns correct value")
    void successorBasic() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1, 4, 6, 8);
        assertEquals(6, tree.successor(5));
        assertEquals(4, tree.successor(3));
        assertEquals(8, tree.successor(7));
    }

    @Test
    @DisplayName("predecessor/successor of value not in tree")
    void predecessorSuccessorAbsent() {
        AVLTree<Integer> tree = buildTree(10, 20, 30);
        assertEquals(10, tree.predecessor(15));  // largest < 15
        assertEquals(20, tree.successor(15));    // smallest > 15
    }

    // =========================================================================
    // 7. kthSmallest
    // =========================================================================

    @Test
    @DisplayName("kthSmallest returns correct values (1-based)")
    void kthSmallestBasic() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1, 4, 6, 8);
        assertEquals(1, tree.kthSmallest(1));
        assertEquals(3, tree.kthSmallest(2));
        assertEquals(4, tree.kthSmallest(3));
        assertEquals(5, tree.kthSmallest(4));
        assertEquals(6, tree.kthSmallest(5));
        assertEquals(7, tree.kthSmallest(6));
        assertEquals(8, tree.kthSmallest(7));
    }

    @Test
    @DisplayName("kthSmallest returns null for out-of-range k")
    void kthSmallestOutOfRange() {
        AVLTree<Integer> tree = buildTree(1, 2, 3);
        assertNull(tree.kthSmallest(0));
        assertNull(tree.kthSmallest(4));
    }

    // =========================================================================
    // 8. rank
    // =========================================================================

    @Test
    @DisplayName("rank returns 0-based position (number of smaller elements)")
    void rankBasic() {
        AVLTree<Integer> tree = buildTree(10, 20, 30, 40, 50);
        assertEquals(0, tree.rank(10));
        assertEquals(1, tree.rank(20));
        assertEquals(2, tree.rank(30));
        assertEquals(3, tree.rank(40));
        assertEquals(4, tree.rank(50));
    }

    @Test
    @DisplayName("rank of value not in tree is insertion position")
    void rankAbsent() {
        AVLTree<Integer> tree = buildTree(10, 20, 30);
        assertEquals(1, tree.rank(15));  // between 10 and 20
        assertEquals(0, tree.rank(5));   // before 10
        assertEquals(3, tree.rank(35));  // after 30
    }

    // =========================================================================
    // 9. toSortedList
    // =========================================================================

    @Test
    @DisplayName("toSortedList returns elements in ascending order")
    void toSortedListBasic() {
        AVLTree<Integer> tree = buildTree(5, 3, 7, 1, 4, 6, 8);
        assertEquals(List.of(1, 3, 4, 5, 6, 7, 8), tree.toSortedList());
    }

    @Test
    @DisplayName("toSortedList on empty tree returns empty list")
    void toSortedListEmpty() {
        assertTrue(new AVLTree<Integer>().toSortedList().isEmpty());
    }

    @Test
    @DisplayName("toSortedList produces sorted output after random inserts")
    void toSortedListRandom() {
        AVLTree<Integer> tree = new AVLTree<>();
        List<Integer> inserted = new ArrayList<>();
        for (int i = 0; i < 50; i++) inserted.add(i * 3);
        Collections.shuffle(inserted, new java.util.Random(13));
        for (int k : inserted) tree.insert(k);
        List<Integer> sorted = new ArrayList<>(inserted);
        Collections.sort(sorted);
        assertEquals(sorted, tree.toSortedList());
    }

    // =========================================================================
    // 10. height / balanceFactor
    // =========================================================================

    @Test
    @DisplayName("Height grows logarithmically with n")
    void heightLogarithmic() {
        AVLTree<Integer> tree = new AVLTree<>();
        for (int i = 0; i < 100; i++) tree.insert(i);
        int h = tree.height();
        assertTrue(h >= 6 && h <= 10, "Unexpected height " + h);
    }

    @Test
    @DisplayName("Root balance factor is in {-1, 0, 1}")
    void balanceFactorValid() {
        AVLTree<Integer> tree = new AVLTree<>();
        for (int i = 0; i < 30; i++) tree.insert(i);
        int bf = tree.balanceFactor();
        assertTrue(bf >= -1 && bf <= 1, "Bad BF: " + bf);
    }

    // =========================================================================
    // 11. isValid / isValidBST
    // =========================================================================

    @Test
    @DisplayName("Empty tree is valid")
    void isValidEmpty() {
        assertTrue(new AVLTree<Integer>().isValid());
        assertTrue(new AVLTree<Integer>().isValidBST());
    }

    @Test
    @DisplayName("isValid is true throughout ascending insertion")
    void isValidDuringInserts() {
        AVLTree<Integer> tree = new AVLTree<>();
        for (int i = 0; i < 50; i++) {
            tree.insert(i);
            assertTrue(tree.isValid(), "Invalid after inserting " + i);
        }
    }

    // =========================================================================
    // 12. Stress test
    // =========================================================================

    @Test
    @DisplayName("Stress: 1000 operations with reference TreeSet")
    void stressTest() {
        AVLTree<Integer> tree = new AVLTree<>();
        java.util.TreeSet<Integer> ref = new java.util.TreeSet<>();
        java.util.Random rng = new java.util.Random(5678);

        // Phase 1: insert 500 random values
        for (int i = 0; i < 500; i++) {
            int k = rng.nextInt(300);
            tree.insert(k);
            ref.add(k);
        }
        assertEquals(ref.size(), tree.size());
        assertTrue(tree.isValid());

        // Phase 2: delete 250 values that exist
        List<Integer> toDelete = new ArrayList<>(ref).subList(0, Math.min(250, ref.size()));
        for (int k : toDelete) {
            tree.delete(k);
            ref.remove(k);
        }
        assertEquals(ref.size(), tree.size());
        assertTrue(tree.isValid());

        // Phase 3: 500 mixed ops
        for (int i = 0; i < 500; i++) {
            int k = rng.nextInt(500);
            if (rng.nextBoolean()) {
                tree.insert(k);
                ref.add(k);
            } else if (ref.contains(k)) {
                tree.delete(k);
                ref.remove(k);
            }
        }
        assertEquals(ref.size(), tree.size());
        assertTrue(tree.isValid());

        // Phase 4: in-order matches sorted reference
        assertEquals(new ArrayList<>(ref), tree.toSortedList());
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private static AVLTree<Integer> buildTree(int... values) {
        AVLTree<Integer> tree = new AVLTree<>();
        for (int v : values) tree.insert(v);
        return tree;
    }
}
