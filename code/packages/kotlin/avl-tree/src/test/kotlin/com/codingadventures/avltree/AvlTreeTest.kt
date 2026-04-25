package com.codingadventures.avltree

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AvlTreeTest {
    @Test
    fun rotationsRebalanceTheTree() {
        var tree = AvlTree.fromValues(listOf(10, 20, 30))

        assertEquals(listOf(10, 20, 30), tree.toSortedArray())
        assertEquals(20, tree.root?.value)
        assertTrue(tree.isValidBst())
        assertTrue(tree.isValidAvl())
        assertEquals(1, tree.height())
        assertEquals(3, tree.size())
        assertEquals(0, tree.balanceFactor(tree.root))

        tree = AvlTree.fromValues(listOf(30, 20, 10))
        assertEquals(20, tree.root?.value)
        assertTrue(tree.isValidAvl())
    }

    @Test
    fun searchAndOrderStatisticsWork() {
        val tree = AvlTree.fromValues(listOf(40, 20, 60, 10, 30, 50, 70))

        assertEquals(20, tree.search(20)?.value)
        assertTrue(tree.contains(50))
        assertEquals(10, tree.minValue())
        assertEquals(70, tree.maxValue())
        assertEquals(30, tree.predecessor(40))
        assertEquals(50, tree.successor(40))
        assertEquals(40, tree.kthSmallest(4))
        assertEquals(3, tree.rank(35))

        val deleted = tree.delete(20)
        assertFalse(deleted.contains(20))
        assertTrue(deleted.isValidAvl())
        assertTrue(tree.contains(20))
    }

    @Test
    fun emptyTreeAndDuplicatesHandleEdges() {
        val empty = AvlTree.empty<Int>()

        assertNull(empty.search(1))
        assertNull(empty.minValue())
        assertNull(empty.maxValue())
        assertNull(empty.predecessor(1))
        assertNull(empty.successor(1))
        assertNull(empty.kthSmallest(0))
        assertEquals(0, empty.rank(1))
        assertEquals(0, empty.balanceFactor(null))
        assertEquals(-1, empty.height())
        assertEquals(0, empty.size())
        assertEquals("AvlTree(root=null, size=0, height=-1)", empty.toString())

        val tree = AvlTree.fromValues(listOf(30, 20, 40, 10, 25, 35, 50))
        val duplicate = tree.insert(25)
        assertEquals(tree.toSortedArray(), duplicate.toSortedArray())
        assertEquals(tree.toSortedArray(), tree.delete(999).toSortedArray())
    }

    @Test
    fun doubleRotationsAndValidationFailuresAreCovered() {
        val leftRight = AvlTree.fromValues(listOf(30, 10, 20))
        val rightLeft = AvlTree.fromValues(listOf(10, 30, 20))

        assertEquals(20, leftRight.root?.value)
        assertEquals(20, rightLeft.root?.value)
        assertTrue(leftRight.isValidAvl())
        assertTrue(rightLeft.isValidAvl())

        val badOrder = AvlTree(AvlNode(5, left = AvlNode.leaf(6), height = 1, size = 2))
        val badRightOrder = AvlTree(AvlNode(5, right = AvlNode.leaf(4), height = 1, size = 2))
        val badHeight = AvlTree(AvlNode(5, left = AvlNode.leaf(3), height = 99, size = 2))

        assertFalse(badOrder.isValidBst())
        assertFalse(badOrder.isValidAvl())
        assertFalse(badRightOrder.isValidBst())
        assertFalse(badRightOrder.isValidAvl())
        assertFalse(badHeight.isValidAvl())
    }

    @Test
    fun deleteWithNestedSuccessorAndStaticHelpersWork() {
        val tree = AvlTree.fromValues(listOf(5, 3, 8, 7, 9, 6))

        val deleted = tree.delete(5)
        assertEquals(listOf(3, 6, 7, 8, 9), deleted.toSortedArray())
        assertTrue(deleted.isValidAvl())
        assertEquals(3, tree.kthSmallest(1))
        assertEquals(9, tree.kthSmallest(6))
        assertEquals(1, tree.rank(5))

        var root: AvlNode<Int>? = null
        root = AvlTree.insertNode(root, 2)
        root = AvlTree.insertNode(root, 1)
        root = AvlTree.insertNode(root, 3)

        assertEquals(2, AvlTree.searchNode(root, 2)?.value)
        assertEquals(1, AvlTree.minValue(root))
        assertEquals(3, AvlTree.maxValue(root))
        assertEquals(2, AvlTree.kthSmallest(root, 2))
        assertEquals(1, AvlTree.rank(root, 2))
        assertEquals(0, AvlTree.balanceFactorNode(root))
        assertTrue(AvlTree.isValidBst(root))
        assertTrue(AvlTree.isValidAvl(root))
        assertEquals(listOf(1, 2, 3), AvlTree.toSortedArray(root))
        assertEquals(listOf(2, 3), AvlTree.toSortedArray(AvlTree.deleteNode(root, 1)))
        assertNull(AvlTree.deleteNode(null, 1))
    }

    @Test
    fun referenceComparableValuesRetainSortedOrder() {
        val tree = AvlTree.fromValues(listOf("delta", "alpha", "gamma"))

        assertEquals(listOf("alpha", "delta", "gamma"), tree.toSortedArray())
        assertEquals("AvlTree(root=delta, size=3, height=1)", tree.toString())
        assertEquals("gamma", tree.successor("delta"))
        assertNull(tree.predecessor("alpha"))
    }
}
