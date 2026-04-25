package com.codingadventures.binarytree;

import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class BinaryTreeTest {
    @Test
    void buildsFromLevelOrderAndProjectsBackToArray() {
        BinaryTree<Integer> tree = BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, 6, 7));

        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7), tree.toArray());
        assertEquals(List.of(1, 2, 3, 4, 5, 6, 7), tree.levelOrder());
        assertEquals(7, tree.size());
        assertEquals(2, tree.height());
    }

    @Test
    void shapeQueriesDistinguishFullCompleteAndPerfectTrees() {
        BinaryTree<Integer> perfect = BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, 6, 7));
        assertTrue(perfect.isFull());
        assertTrue(perfect.isComplete());
        assertTrue(perfect.isPerfect());

        BinaryTree<Integer> complete = BinaryTree.fromLevelOrder(Arrays.asList(1, 2, 3, 4, null, null, null));
        assertFalse(complete.isFull());
        assertTrue(complete.isComplete());
        assertFalse(complete.isPerfect());

        BinaryTree<Integer> incomplete = BinaryTree.fromLevelOrder(Arrays.asList(1, null, 3));
        assertFalse(incomplete.isComplete());
    }

    @Test
    void traversalsAndChildLookupMatchReferenceOrder() {
        BinaryTree<Integer> tree = BinaryTree.fromLevelOrder(Arrays.asList(1, 2, 3, 4, null, 5, null));

        assertEquals(List.of(4, 2, 1, 5, 3), tree.inorder());
        assertEquals(List.of(1, 2, 4, 3, 5), tree.preorder());
        assertEquals(List.of(4, 2, 5, 3, 1), tree.postorder());
        assertEquals(List.of(1, 2, 3, 4, 5), tree.levelOrder());
        assertEquals(2, tree.leftChild(1).value());
        assertEquals(3, tree.rightChild(1).value());
        assertNull(tree.find(99));
    }

    @Test
    void emptyAndSingletonTreesExposeEdgeCases() {
        BinaryTree<String> empty = BinaryTree.empty();
        assertNull(empty.root());
        assertEquals(-1, empty.height());
        assertEquals(0, empty.size());
        assertTrue(empty.isFull());
        assertTrue(empty.isComplete());
        assertTrue(empty.isPerfect());
        assertEquals(List.of(), empty.inorder());
        assertEquals(List.of(), empty.toArray());
        assertEquals("", empty.toAscii());
        assertEquals("BinaryTree(root=null, size=0)", empty.toString());

        BinaryTree<String> single = BinaryTree.singleton("root");
        assertEquals("root", single.root().value());
        assertEquals(List.of("root"), single.toArray());
        assertEquals("BinaryTree(root=root, size=1)", single.toString());
    }

    @Test
    void asciiRenderingContainsValues() {
        BinaryTree<String> tree = BinaryTree.fromLevelOrder(List.of("root", "left", "right"));
        String ascii = tree.toAscii();

        assertTrue(ascii.contains("root"));
        assertTrue(ascii.contains("left"));
        assertTrue(ascii.contains("right"));
        assertTrue(ascii.contains("`--"));
    }

    @Test
    void staticHelpersSupportRawRootComposition() {
        BinaryTreeNode<Integer> root = new BinaryTreeNode<>(
                1,
                new BinaryTreeNode<>(2),
                new BinaryTreeNode<>(3));

        assertSame(root, BinaryTree.withRoot(root).root());
        assertEquals(2, BinaryTree.leftChild(root, 1).value());
        assertEquals(3, BinaryTree.rightChild(root, 1).value());
        assertEquals(List.of(2, 1, 3), BinaryTree.inorder(root));
        assertEquals(List.of(1, 2, 3), BinaryTree.preorder(root));
        assertEquals(List.of(2, 3, 1), BinaryTree.postorder(root));
        assertEquals(List.of(1, 2, 3), BinaryTree.levelOrder(root));
        assertEquals(Arrays.asList(1, 2, 3), BinaryTree.toArray(root));
        assertTrue(BinaryTree.isFull(root));
        assertTrue(BinaryTree.isComplete(root));
        assertTrue(BinaryTree.isPerfect(root));
        assertEquals(1, BinaryTree.height(root));
        assertEquals(3, BinaryTree.size(root));
        assertTrue(BinaryTree.toAscii(root).contains("1"));
    }
}
