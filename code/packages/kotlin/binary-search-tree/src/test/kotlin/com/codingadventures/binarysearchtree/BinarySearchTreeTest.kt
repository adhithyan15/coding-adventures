package com.codingadventures.binarysearchtree

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertSame
import kotlin.test.assertTrue

class BinarySearchTreeTest {
    @Test
    fun insertSearchAndDeleteWorkImmutably() {
        val tree = populated()

        assertEquals(listOf(1, 3, 5, 7, 8), tree.toSortedArray())
        assertEquals(5, tree.size())
        assertTrue(tree.contains(7))
        assertEquals(7, tree.search(7)?.value)
        assertEquals(1, tree.minValue())
        assertEquals(8, tree.maxValue())
        assertEquals(3, tree.predecessor(5))
        assertEquals(7, tree.successor(5))
        assertEquals(2, tree.rank(4))
        assertEquals(7, tree.kthSmallest(4))

        val deleted = tree.delete(5)
        assertFalse(deleted.contains(5))
        assertTrue(deleted.isValid())
        assertTrue(tree.contains(5))
    }

    @Test
    fun fromSortedArrayBuildsBalancedTree() {
        val tree = BinarySearchTree.fromSortedArray(listOf(1, 2, 3, 4, 5, 6, 7))

        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7), tree.toSortedArray())
        assertEquals(2, tree.height())
        assertEquals(7, tree.size())
        assertTrue(tree.isValid())
        assertEquals(4, tree.root?.value)
    }

    @Test
    fun emptyTreeReturnsNullsAndNeutralMetrics() {
        val tree = BinarySearchTree.empty<Int>()

        assertNull(tree.search(1))
        assertNull(tree.minValue())
        assertNull(tree.maxValue())
        assertNull(tree.predecessor(1))
        assertNull(tree.successor(1))
        assertNull(tree.kthSmallest(0))
        assertNull(tree.kthSmallest(1))
        assertEquals(0, tree.rank(1))
        assertEquals(-1, tree.height())
        assertEquals(0, tree.size())
        assertEquals("BinarySearchTree(root=null, size=0)", tree.toString())
    }

    @Test
    fun duplicatesAndSingleChildDeleteKeepPersistentShape() {
        val tree = BinarySearchTree.fromSortedArray(listOf(2, 4, 6, 8))

        assertEquals(6, tree.root?.value)
        val duplicate = tree.insert(4)

        assertSame(tree.root?.left, duplicate.root?.left)
        assertEquals(tree.toSortedArray(), duplicate.toSortedArray())
        assertEquals(listOf(4, 6, 8), tree.delete(2).toSortedArray())
    }

    @Test
    fun validationCatchesBadOrderingAndStaleSizes() {
        val badOrder = BinarySearchTree(BstNode(5, left = BstNode.leaf(6), size = 2))
        val badSize = BinarySearchTree(BstNode(5, left = BstNode.leaf(3), size = 99))

        assertFalse(badOrder.isValid())
        assertFalse(badSize.isValid())
    }

    @Test
    fun staticHelpersSupportRawRootComposition() {
        var root: BstNode<Int>? = null
        root = BinarySearchTree.insertNode(root, 5)
        root = BinarySearchTree.insertNode(root, 2)
        root = BinarySearchTree.insertNode(root, 9)

        assertEquals(5, BinarySearchTree.searchNode(root, 5)?.value)
        assertEquals(2, BinarySearchTree.minValue(root))
        assertEquals(9, BinarySearchTree.maxValue(root))
        assertEquals(5, BinarySearchTree.kthSmallest(root, 2))
        assertEquals(1, BinarySearchTree.rank(root, 5))
        assertEquals(1, BinarySearchTree.height(root))
        assertTrue(BinarySearchTree.isValid(root))
        assertEquals(listOf(2, 5, 9), BinarySearchTree.toSortedArray(root))

        val deleted = BinarySearchTree.deleteNode(root, 2)
        assertEquals(listOf(5, 9), BinarySearchTree.toSortedArray(deleted))
        assertNull(BinarySearchTree.deleteNode(null, 2))
    }

    @Test
    fun referenceComparableValuesRetainSortedOrder() {
        val tree = BinarySearchTree.empty<String>()
            .insert("delta")
            .insert("alpha")
            .insert("gamma")

        assertEquals(listOf("alpha", "delta", "gamma"), tree.toSortedArray())
        assertEquals("BinarySearchTree(root=delta, size=3)", tree.toString())
        assertEquals("gamma", tree.successor("delta"))
        assertNull(tree.predecessor("alpha"))
    }

    private fun populated(): BinarySearchTree<Int> =
        listOf(5, 1, 8, 3, 7).fold(BinarySearchTree.empty()) { tree, value ->
            tree.insert(value)
        }
}
