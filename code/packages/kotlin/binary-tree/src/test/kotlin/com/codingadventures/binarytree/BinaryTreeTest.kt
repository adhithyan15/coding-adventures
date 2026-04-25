package com.codingadventures.binarytree

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class BinaryTreeTest {
    @Test
    fun buildsFromLevelOrderAndProjectsBackToArray() {
        val tree = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6, 7))

        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7), tree.toArray())
        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7), tree.levelOrder())
        assertEquals(7, tree.size())
        assertEquals(2, tree.height())
    }

    @Test
    fun shapeQueriesDistinguishFullCompleteAndPerfectTrees() {
        val perfect = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, 6, 7))
        assertTrue(perfect.isFull())
        assertTrue(perfect.isComplete())
        assertTrue(perfect.isPerfect())

        val complete = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, null, null, null))
        assertFalse(complete.isFull())
        assertTrue(complete.isComplete())
        assertFalse(complete.isPerfect())

        val incomplete = BinaryTree.fromLevelOrder(listOf(1, null, 3))
        assertFalse(incomplete.isComplete())
    }

    @Test
    fun traversalsAndChildLookupMatchReferenceOrder() {
        val tree = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, null, 5, null))

        assertEquals(listOf(4, 2, 1, 5, 3), tree.inOrder())
        assertEquals(listOf(1, 2, 4, 3, 5), tree.preOrder())
        assertEquals(listOf(4, 2, 5, 3, 1), tree.postOrder())
        assertEquals(listOf(1, 2, 3, 4, 5), tree.levelOrder())
        assertEquals(2, tree.leftChild(1)?.value)
        assertEquals(3, tree.rightChild(1)?.value)
        assertNull(tree.find(99))
    }

    @Test
    fun emptyAndSingletonTreesExposeEdgeCases() {
        val empty = BinaryTree.empty<String>()
        assertNull(empty.root)
        assertEquals(-1, empty.height())
        assertEquals(0, empty.size())
        assertTrue(empty.isFull())
        assertTrue(empty.isComplete())
        assertTrue(empty.isPerfect())
        assertEquals(emptyList(), empty.inOrder())
        assertEquals(emptyList(), empty.toArray())
        assertEquals("", empty.toAscii())
        assertEquals("BinaryTree(root=null, size=0)", empty.toString())

        val single = BinaryTree.singleton("root")
        assertEquals("root", single.root?.value)
        assertEquals(listOf("root"), single.toArray())
        assertEquals("BinaryTree(root=root, size=1)", single.toString())
    }

    @Test
    fun asciiRenderingContainsValues() {
        val tree = BinaryTree.fromLevelOrder(listOf("root", "left", "right"))
        val ascii = tree.toAscii()

        assertTrue("root" in ascii)
        assertTrue("left" in ascii)
        assertTrue("right" in ascii)
        assertTrue("`--" in ascii)
    }

    @Test
    fun staticHelpersSupportRawRootComposition() {
        val root = BinaryTreeNode(
            1,
            BinaryTreeNode(2),
            BinaryTreeNode(3),
        )

        assertSame(root, BinaryTree.withRoot(root).root)
        assertEquals(2, BinaryTree.leftChild(root, 1)?.value)
        assertEquals(3, BinaryTree.rightChild(root, 1)?.value)
        assertEquals(listOf(2, 1, 3), BinaryTree.inOrder(root))
        assertEquals(listOf(1, 2, 3), BinaryTree.preOrder(root))
        assertEquals(listOf(2, 3, 1), BinaryTree.postOrder(root))
        assertEquals(listOf(1, 2, 3), BinaryTree.levelOrder(root))
        assertEquals(listOf(1, 2, 3), BinaryTree.toArray(root))
        assertTrue(BinaryTree.isFull(root))
        assertTrue(BinaryTree.isComplete(root))
        assertTrue(BinaryTree.isPerfect(root))
        assertEquals(1, BinaryTree.height(root))
        assertEquals(3, BinaryTree.size(root))
        assertTrue("1" in BinaryTree.toAscii(root))
    }
}
