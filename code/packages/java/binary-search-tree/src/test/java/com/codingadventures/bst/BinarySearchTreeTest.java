package com.codingadventures.bst;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class BinarySearchTreeTest {

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    private BinarySearchTree<Integer> populated() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        for (int v : new int[]{5, 1, 8, 3, 7}) t.insert(v);
        return t;
    }

    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    @Test
    void emptyTreeHasSizeZeroAndHeightMinusOne() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        assertEquals(0, t.size());
        assertEquals(-1, t.height());
        assertTrue(t.isEmpty());
    }

    @Test
    void singleInsertProducesRootWithSizeOneAndHeightZero() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        t.insert(42);
        assertEquals(1, t.size());
        assertEquals(0, t.height());
        assertFalse(t.isEmpty());
    }

    @Test
    void fromSortedListBuildsBalancedTree() {
        BinarySearchTree<Integer> t = BinarySearchTree.fromSortedList(List.of(1, 2, 3, 4, 5, 6, 7));
        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7), t.toSortedList());
        assertEquals(2, t.height());   // balanced: height = floor(log2(7)) = 2
        assertEquals(7, t.size());
        assertTrue(t.isValid());
    }

    @Test
    void fromSortedListEmptyProducesEmptyTree() {
        BinarySearchTree<Integer> t = BinarySearchTree.fromSortedList(List.<Integer>of());
        assertEquals(0, t.size());
        assertTrue(t.isEmpty());
    }

    // -------------------------------------------------------------------------
    // Insert
    // -------------------------------------------------------------------------

    @Test
    void insertedElementsAreSortedByInorder() {
        BinarySearchTree<Integer> t = populated();
        assertEquals(List.of(1, 3, 5, 7, 8), t.toSortedList());
        assertEquals(5, t.size());
    }

    @Test
    void insertDuplicateIsNoOp() {
        BinarySearchTree<Integer> t = populated();
        t.insert(5);
        assertEquals(5, t.size());
        assertEquals(List.of(1, 3, 5, 7, 8), t.toSortedList());
    }

    @Test
    void insertNullThrows() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        assertThrows(IllegalArgumentException.class, () -> t.insert(null));
    }

    // -------------------------------------------------------------------------
    // Search / contains
    // -------------------------------------------------------------------------

    @Test
    void searchFindsExistingValues() {
        BinarySearchTree<Integer> t = populated();
        assertNotNull(t.search(7));
        assertEquals(7, t.search(7).value);
    }

    @Test
    void searchReturnsNullForMissingValues() {
        BinarySearchTree<Integer> t = populated();
        assertNull(t.search(42));
    }

    @Test
    void containsReturnsTrueForPresentValues() {
        BinarySearchTree<Integer> t = populated();
        assertTrue(t.contains(1));
        assertTrue(t.contains(5));
        assertTrue(t.contains(8));
    }

    @Test
    void containsReturnsFalseForAbsentValues() {
        BinarySearchTree<Integer> t = populated();
        assertFalse(t.contains(0));
        assertFalse(t.contains(6));
        assertFalse(t.contains(100));
    }

    // -------------------------------------------------------------------------
    // Delete
    // -------------------------------------------------------------------------

    @Test
    void deleteLeafRemovesElement() {
        BinarySearchTree<Integer> t = populated();
        t.delete(1);
        assertFalse(t.contains(1));
        assertEquals(4, t.size());
        assertTrue(t.isValid());
        assertEquals(List.of(3, 5, 7, 8), t.toSortedList());
    }

    @Test
    void deleteNodeWithOneChild() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        for (int v : new int[]{5, 3, 1}) t.insert(v);
        t.delete(3);   // 3 has only left child 1
        assertFalse(t.contains(3));
        assertTrue(t.contains(1));
        assertTrue(t.isValid());
    }

    @Test
    void deleteNodeWithTwoChildrenUsesSuccessor() {
        BinarySearchTree<Integer> t = populated();  // root is 5
        t.delete(5);
        assertFalse(t.contains(5));
        assertEquals(4, t.size());
        assertTrue(t.isValid());
        assertEquals(List.of(1, 3, 7, 8), t.toSortedList());
    }

    @Test
    void deleteAbsentValueIsNoOp() {
        BinarySearchTree<Integer> t = populated();
        t.delete(99);
        assertEquals(5, t.size());
        assertTrue(t.isValid());
    }

    @Test
    void deleteNullThrows() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        assertThrows(IllegalArgumentException.class, () -> t.delete(null));
    }

    @Test
    void deleteAllElementsLeavesEmptyTree() {
        BinarySearchTree<Integer> t = populated();
        for (int v : new int[]{1, 3, 5, 7, 8}) t.delete(v);
        assertEquals(0, t.size());
        assertTrue(t.isEmpty());
        assertNull(t.search(5));
    }

    // -------------------------------------------------------------------------
    // Min / Max
    // -------------------------------------------------------------------------

    @Test
    void minValueReturnsSmallestElement() {
        assertEquals(1, populated().minValue().orElseThrow());
    }

    @Test
    void maxValueReturnsLargestElement() {
        assertEquals(8, populated().maxValue().orElseThrow());
    }

    @Test
    void minValueOnEmptyTreeReturnsEmpty() {
        assertTrue(new BinarySearchTree<Integer>().minValue().isEmpty());
    }

    @Test
    void maxValueOnEmptyTreeReturnsEmpty() {
        assertTrue(new BinarySearchTree<Integer>().maxValue().isEmpty());
    }

    // -------------------------------------------------------------------------
    // Predecessor / Successor
    // -------------------------------------------------------------------------

    @Test
    void predecessorReturnsPreviousValue() {
        BinarySearchTree<Integer> t = populated();
        assertEquals(3, t.predecessor(5).orElseThrow());
        assertEquals(7, t.predecessor(8).orElseThrow());
        assertEquals(1, t.predecessor(3).orElseThrow());
    }

    @Test
    void predecessorOfMinReturnsEmpty() {
        assertTrue(populated().predecessor(1).isEmpty());
    }

    @Test
    void successorReturnsNextValue() {
        BinarySearchTree<Integer> t = populated();
        assertEquals(7, t.successor(5).orElseThrow());
        assertEquals(3, t.successor(1).orElseThrow());
    }

    @Test
    void successorOfMaxReturnsEmpty() {
        assertTrue(populated().successor(8).isEmpty());
    }

    @Test
    void predecessorAndSuccessorForAbsentValue() {
        BinarySearchTree<Integer> t = populated();   // [1, 3, 5, 7, 8]
        // 4 is not in tree; predecessor(4) = 3, successor(4) = 5
        assertEquals(3, t.predecessor(4).orElseThrow());
        assertEquals(5, t.successor(4).orElseThrow());
    }

    // -------------------------------------------------------------------------
    // Order statistics: kthSmallest
    // -------------------------------------------------------------------------

    @Test
    void kthSmallestReturnsCorrectRankElement() {
        BinarySearchTree<Integer> t = populated();   // sorted: [1, 3, 5, 7, 8]
        assertEquals(1, t.kthSmallest(1).orElseThrow());
        assertEquals(3, t.kthSmallest(2).orElseThrow());
        assertEquals(5, t.kthSmallest(3).orElseThrow());
        assertEquals(7, t.kthSmallest(4).orElseThrow());
        assertEquals(8, t.kthSmallest(5).orElseThrow());
    }

    @Test
    void kthSmallestOutOfRangeReturnsEmpty() {
        BinarySearchTree<Integer> t = populated();
        assertTrue(t.kthSmallest(0).isEmpty());
        assertTrue(t.kthSmallest(6).isEmpty());
        assertTrue(t.kthSmallest(-1).isEmpty());
    }

    @Test
    void kthSmallestOnEmptyTreeReturnsEmpty() {
        assertTrue(new BinarySearchTree<Integer>().kthSmallest(1).isEmpty());
    }

    // -------------------------------------------------------------------------
    // Order statistics: rank
    // -------------------------------------------------------------------------

    @Test
    void rankReturnsCountOfElementsLessThanValue() {
        BinarySearchTree<Integer> t = populated();   // [1, 3, 5, 7, 8]
        assertEquals(0, t.rank(1));   // nothing less than 1
        assertEquals(1, t.rank(3));   // 1 < 3
        assertEquals(2, t.rank(5));   // 1, 3 < 5
        assertEquals(3, t.rank(7));   // 1, 3, 5 < 7
        assertEquals(4, t.rank(8));   // 1, 3, 5, 7 < 8
    }

    @Test
    void rankForAbsentValues() {
        BinarySearchTree<Integer> t = populated();   // [1, 3, 5, 7, 8]
        assertEquals(2, t.rank(4));   // 1, 3 < 4
        assertEquals(0, t.rank(0));
        assertEquals(5, t.rank(9));
    }

    // -------------------------------------------------------------------------
    // Validation
    // -------------------------------------------------------------------------

    @Test
    void isValidReturnsTrueForCorrectlyBuiltTree() {
        assertTrue(populated().isValid());
    }

    @Test
    void isValidReturnsTrueForEmptyTree() {
        assertTrue(new BinarySearchTree<Integer>().isValid());
    }

    @Test
    void isValidDetectsViolationOfBstProperty() {
        // Manually build an invalid tree: root=5 with left child=6 (violates BST)
        BinarySearchTree<Integer> bad = new BinarySearchTree<>();
        bad.root = new BinarySearchTree.Node<>(5);
        bad.root.left = new BinarySearchTree.Node<>(6);
        bad.root.left.size = 1;
        bad.root.size = 2;
        assertFalse(bad.isValid());
    }

    @Test
    void isValidDetectsSizeInvariantViolation() {
        BinarySearchTree<Integer> bad = new BinarySearchTree<>();
        bad.root = new BinarySearchTree.Node<>(5);
        bad.root.left = new BinarySearchTree.Node<>(3);
        bad.root.left.size = 1;
        bad.root.size = 99;   // wrong size
        assertFalse(bad.isValid());
    }

    // -------------------------------------------------------------------------
    // Height
    // -------------------------------------------------------------------------

    @Test
    void heightOfSingleNodeIsZero() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        t.insert(1);
        assertEquals(0, t.height());
    }

    @Test
    void heightOfBalancedTreeIsLog2n() {
        // 7 elements → balanced height = 2
        BinarySearchTree<Integer> t = BinarySearchTree.fromSortedList(List.of(1, 2, 3, 4, 5, 6, 7));
        assertEquals(2, t.height());
    }

    // -------------------------------------------------------------------------
    // toString
    // -------------------------------------------------------------------------

    @Test
    void toStringShowsRootAndSize() {
        BinarySearchTree<Integer> empty = new BinarySearchTree<>();
        assertEquals("BinarySearchTree(root=null, size=0)", empty.toString());

        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        t.insert(5);
        assertEquals("BinarySearchTree(root=5, size=1)", t.toString());
    }

    // -------------------------------------------------------------------------
    // Stress test
    // -------------------------------------------------------------------------

    @Test
    void insertAndDeleteThousandElementsLeavesValidTree() {
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        for (int i = 1; i <= 1000; i++) t.insert(i);
        assertEquals(1000, t.size());

        for (int i = 1; i <= 1000; i += 2) t.delete(i);  // delete odd numbers
        assertEquals(500, t.size());
        assertTrue(t.isValid());

        List<Integer> sorted = t.toSortedList();
        assertEquals(500, sorted.size());
        for (int i = 0; i < sorted.size(); i++) {
            assertEquals(2 * (i + 1), sorted.get(i));  // only evens remain
        }
    }

    @Test
    void randomInsertOrderPreservesOrderStatistics() {
        // Insert [7, 2, 9, 1, 4, 8, 3] — not sorted
        BinarySearchTree<Integer> t = new BinarySearchTree<>();
        for (int v : new int[]{7, 2, 9, 1, 4, 8, 3}) t.insert(v);

        assertTrue(t.isValid());
        assertEquals(List.of(1, 2, 3, 4, 7, 8, 9), t.toSortedList());
        assertEquals(1, t.kthSmallest(1).orElseThrow());
        assertEquals(9, t.kthSmallest(7).orElseThrow());
        assertEquals(3, t.rank(4));   // 1, 2, 3 < 4
    }

    // -------------------------------------------------------------------------
    // String keys (ensures generic Comparable works)
    // -------------------------------------------------------------------------

    @Test
    void worksWithStringKeys() {
        BinarySearchTree<String> t = new BinarySearchTree<>();
        t.insert("banana");
        t.insert("apple");
        t.insert("cherry");

        assertEquals(List.of("apple", "banana", "cherry"), t.toSortedList());
        assertTrue(t.contains("apple"));
        assertEquals("apple", t.minValue().orElseThrow());
        assertEquals("cherry", t.maxValue().orElseThrow());
    }
}
