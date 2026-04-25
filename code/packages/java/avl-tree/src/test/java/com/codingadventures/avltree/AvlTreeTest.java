package com.codingadventures.avltree;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class AvlTreeTest {
    @Test
    void rotationsRebalanceTheTree() {
        AvlTree<Integer> tree = AvlTree.fromValues(List.of(10, 20, 30));

        assertEquals(List.of(10, 20, 30), tree.toSortedArray());
        assertEquals(20, tree.root().value());
        assertTrue(tree.isValidBst());
        assertTrue(tree.isValidAvl());
        assertEquals(1, tree.height());
        assertEquals(3, tree.size());
        assertEquals(0, tree.balanceFactor(tree.root()));

        tree = AvlTree.fromValues(List.of(30, 20, 10));
        assertEquals(20, tree.root().value());
        assertTrue(tree.isValidAvl());
    }

    @Test
    void searchAndOrderStatisticsWork() {
        AvlTree<Integer> tree = AvlTree.fromValues(List.of(40, 20, 60, 10, 30, 50, 70));

        assertEquals(20, tree.search(20).value());
        assertTrue(tree.contains(50));
        assertEquals(10, tree.minValue());
        assertEquals(70, tree.maxValue());
        assertEquals(30, tree.predecessor(40));
        assertEquals(50, tree.successor(40));
        assertEquals(40, tree.kthSmallest(4));
        assertEquals(3, tree.rank(35));

        AvlTree<Integer> deleted = tree.delete(20);
        assertFalse(deleted.contains(20));
        assertTrue(deleted.isValidAvl());
        assertTrue(tree.contains(20));
    }

    @Test
    void emptyTreeAndDuplicatesHandleEdges() {
        AvlTree<Integer> empty = AvlTree.empty();

        assertNull(empty.search(1));
        assertNull(empty.minValue());
        assertNull(empty.maxValue());
        assertNull(empty.predecessor(1));
        assertNull(empty.successor(1));
        assertNull(empty.kthSmallest(0));
        assertEquals(0, empty.rank(1));
        assertEquals(0, empty.balanceFactor(null));
        assertEquals(-1, empty.height());
        assertEquals(0, empty.size());
        assertEquals("AvlTree(root=null, size=0, height=-1)", empty.toString());

        AvlTree<Integer> tree = AvlTree.fromValues(List.of(30, 20, 40, 10, 25, 35, 50));
        AvlTree<Integer> duplicate = tree.insert(25);
        assertEquals(tree.toSortedArray(), duplicate.toSortedArray());
        assertEquals(tree.toSortedArray(), tree.delete(999).toSortedArray());
    }

    @Test
    void doubleRotationsAndValidationFailuresAreCovered() {
        AvlTree<Integer> leftRight = AvlTree.fromValues(List.of(30, 10, 20));
        AvlTree<Integer> rightLeft = AvlTree.fromValues(List.of(10, 30, 20));

        assertEquals(20, leftRight.root().value());
        assertEquals(20, rightLeft.root().value());
        assertTrue(leftRight.isValidAvl());
        assertTrue(rightLeft.isValidAvl());

        AvlTree<Integer> badOrder = new AvlTree<>(new AvlNode<>(5, new AvlNode<>(6), null, 1, 2));
        AvlTree<Integer> badRightOrder = new AvlTree<>(new AvlNode<>(5, null, new AvlNode<>(4), 1, 2));
        AvlTree<Integer> badHeight = new AvlTree<>(new AvlNode<>(5, new AvlNode<>(3), null, 99, 2));

        assertFalse(badOrder.isValidBst());
        assertFalse(badOrder.isValidAvl());
        assertFalse(badRightOrder.isValidBst());
        assertFalse(badRightOrder.isValidAvl());
        assertFalse(badHeight.isValidAvl());
    }

    @Test
    void deleteWithNestedSuccessorAndStaticHelpersWork() {
        AvlTree<Integer> tree = AvlTree.fromValues(List.of(5, 3, 8, 7, 9, 6));

        AvlTree<Integer> deleted = tree.delete(5);
        assertEquals(List.of(3, 6, 7, 8, 9), deleted.toSortedArray());
        assertTrue(deleted.isValidAvl());
        assertEquals(3, tree.kthSmallest(1));
        assertEquals(9, tree.kthSmallest(6));
        assertEquals(1, tree.rank(5));

        AvlNode<Integer> root = null;
        root = AvlTree.insertNode(root, 2);
        root = AvlTree.insertNode(root, 1);
        root = AvlTree.insertNode(root, 3);

        assertEquals(2, AvlTree.searchNode(root, 2).value());
        assertEquals(1, AvlTree.minValue(root));
        assertEquals(3, AvlTree.maxValue(root));
        assertEquals(2, AvlTree.kthSmallest(root, 2));
        assertEquals(1, AvlTree.rank(root, 2));
        assertEquals(0, AvlTree.balanceFactorNode(root));
        assertTrue(AvlTree.isValidBst(root));
        assertTrue(AvlTree.isValidAvl(root));
        assertEquals(List.of(1, 2, 3), AvlTree.toSortedArray(root));
        assertEquals(List.of(2, 3), AvlTree.toSortedArray(AvlTree.deleteNode(root, 1)));
        assertNull(AvlTree.deleteNode(null, 1));
    }

    @Test
    void referenceComparableValuesRetainSortedOrder() {
        AvlTree<String> tree = AvlTree.fromValues(List.of("delta", "alpha", "gamma"));

        assertEquals(List.of("alpha", "delta", "gamma"), tree.toSortedArray());
        assertEquals("AvlTree(root=delta, size=3, height=1)", tree.toString());
        assertEquals("gamma", tree.successor("delta"));
        assertNull(tree.predecessor("alpha"));
    }
}
