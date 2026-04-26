package com.codingadventures.binarysearchtree;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class BinarySearchTreeTest {
    @Test
    void insertSearchAndDeleteWorkImmutably() {
        BinarySearchTree<Integer> tree = populated();

        assertEquals(List.of(1, 3, 5, 7, 8), tree.toSortedArray());
        assertEquals(5, tree.size());
        assertTrue(tree.contains(7));
        assertEquals(7, tree.search(7).value());
        assertEquals(1, tree.minValue());
        assertEquals(8, tree.maxValue());
        assertEquals(3, tree.predecessor(5));
        assertEquals(7, tree.successor(5));
        assertEquals(2, tree.rank(4));
        assertEquals(7, tree.kthSmallest(4));

        BinarySearchTree<Integer> deleted = tree.delete(5);
        assertFalse(deleted.contains(5));
        assertTrue(deleted.isValid());
        assertTrue(tree.contains(5));
    }

    @Test
    void fromSortedArrayBuildsBalancedTree() {
        BinarySearchTree<Integer> tree = BinarySearchTree.fromSortedArray(List.of(1, 2, 3, 4, 5, 6, 7));

        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7), tree.toSortedArray());
        assertEquals(2, tree.height());
        assertEquals(7, tree.size());
        assertTrue(tree.isValid());
        assertEquals(4, tree.root().value());
    }

    @Test
    void emptyTreeReturnsNullsAndNeutralMetrics() {
        BinarySearchTree<Integer> tree = BinarySearchTree.empty();

        assertNull(tree.search(1));
        assertNull(tree.minValue());
        assertNull(tree.maxValue());
        assertNull(tree.predecessor(1));
        assertNull(tree.successor(1));
        assertNull(tree.kthSmallest(0));
        assertNull(tree.kthSmallest(1));
        assertEquals(0, tree.rank(1));
        assertEquals(-1, tree.height());
        assertEquals(0, tree.size());
        assertEquals("BinarySearchTree(root=null, size=0)", tree.toString());
    }

    @Test
    void duplicatesAndSingleChildDeleteKeepPersistentShape() {
        BinarySearchTree<Integer> tree = BinarySearchTree.fromSortedArray(List.of(2, 4, 6, 8));

        assertEquals(6, tree.root().value());
        BinarySearchTree<Integer> duplicate = tree.insert(4);

        assertSame(tree.root().left(), duplicate.root().left());
        assertEquals(tree.toSortedArray(), duplicate.toSortedArray());
        assertEquals(List.of(4, 6, 8), tree.delete(2).toSortedArray());
    }

    @Test
    void validationCatchesBadOrderingAndStaleSizes() {
        BinarySearchTree<Integer> badOrder = new BinarySearchTree<>(
                new BstNode<>(5, new BstNode<>(6), null, 2));
        BinarySearchTree<Integer> badSize = new BinarySearchTree<>(
                new BstNode<>(5, new BstNode<>(3), null, 99));

        assertFalse(badOrder.isValid());
        assertFalse(badSize.isValid());
    }

    @Test
    void staticHelpersSupportRawRootComposition() {
        BstNode<Integer> root = null;
        root = BinarySearchTree.insertNode(root, 5);
        root = BinarySearchTree.insertNode(root, 2);
        root = BinarySearchTree.insertNode(root, 9);

        assertEquals(5, BinarySearchTree.searchNode(root, 5).value());
        assertEquals(2, BinarySearchTree.minValue(root));
        assertEquals(9, BinarySearchTree.maxValue(root));
        assertEquals(5, BinarySearchTree.kthSmallest(root, 2));
        assertEquals(1, BinarySearchTree.rank(root, 5));
        assertEquals(1, BinarySearchTree.height(root));
        assertTrue(BinarySearchTree.isValid(root));
        assertEquals(List.of(2, 5, 9), BinarySearchTree.toSortedArray(root));

        BstNode<Integer> deleted = BinarySearchTree.deleteNode(root, 2);
        assertEquals(List.of(5, 9), BinarySearchTree.toSortedArray(deleted));
        assertNull(BinarySearchTree.deleteNode(null, 2));
    }

    @Test
    void referenceComparableValuesRetainSortedOrder() {
        BinarySearchTree<String> tree = BinarySearchTree.<String>empty()
                .insert("delta")
                .insert("alpha")
                .insert("gamma");

        assertEquals(List.of("alpha", "delta", "gamma"), tree.toSortedArray());
        assertEquals("BinarySearchTree(root=delta, size=3)", tree.toString());
        assertEquals("gamma", tree.successor("delta"));
        assertNull(tree.predecessor("alpha"));
    }

    private static BinarySearchTree<Integer> populated() {
        BinarySearchTree<Integer> tree = BinarySearchTree.empty();
        for (int value : List.of(5, 1, 8, 3, 7)) {
            tree = tree.insert(value);
        }
        return tree;
    }
}
