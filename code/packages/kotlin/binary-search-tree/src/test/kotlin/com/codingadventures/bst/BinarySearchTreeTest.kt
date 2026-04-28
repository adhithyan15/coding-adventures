package com.codingadventures.bst

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class BinarySearchTreeTest {

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    private fun populated(): BinarySearchTree<Int> {
        val t = BinarySearchTree<Int>()
        listOf(5, 1, 8, 3, 7).forEach { t.insert(it) }
        return t
    }

    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    @Test fun emptyTreeHasSizeZeroAndHeightMinusOne() {
        val t = BinarySearchTree<Int>()
        assertEquals(0, t.size)
        assertEquals(-1, t.height())
        assertTrue(t.isEmpty)
    }

    @Test fun singleInsertProducesRootWithSizeOneAndHeightZero() {
        val t = BinarySearchTree<Int>()
        t.insert(42)
        assertEquals(1, t.size)
        assertEquals(0, t.height())
        assertFalse(t.isEmpty)
    }

    @Test fun fromSortedListBuildsBalancedTree() {
        val t = BinarySearchTree.fromSortedList(listOf(1, 2, 3, 4, 5, 6, 7))
        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7), t.toSortedList())
        assertEquals(2, t.height())
        assertEquals(7, t.size)
        assertTrue(t.isValid())
    }

    @Test fun fromSortedListEmptyProducesEmptyTree() {
        val t = BinarySearchTree.fromSortedList(emptyList<Int>())
        assertEquals(0, t.size)
        assertTrue(t.isEmpty)
    }

    // -------------------------------------------------------------------------
    // Insert
    // -------------------------------------------------------------------------

    @Test fun insertedElementsAreSortedByInorder() {
        val t = populated()
        assertEquals(listOf(1, 3, 5, 7, 8), t.toSortedList())
        assertEquals(5, t.size)
    }

    @Test fun insertDuplicateIsNoOp() {
        val t = populated()
        t.insert(5)
        assertEquals(5, t.size)
        assertEquals(listOf(1, 3, 5, 7, 8), t.toSortedList())
    }

    // -------------------------------------------------------------------------
    // Search / contains
    // -------------------------------------------------------------------------

    @Test fun searchFindsExistingValues() {
        val t = populated()
        assertNotNull(t.search(7))
        assertEquals(7, t.search(7)?.value)
    }

    @Test fun searchReturnsNullForMissingValues() {
        assertNull(populated().search(42))
    }

    @Test fun containsReturnsTrueForPresentValues() {
        val t = populated()
        assertTrue(t.contains(1))
        assertTrue(t.contains(5))
        assertTrue(t.contains(8))
    }

    @Test fun containsReturnsFalseForAbsentValues() {
        val t = populated()
        assertFalse(t.contains(0))
        assertFalse(t.contains(6))
        assertFalse(t.contains(100))
    }

    // -------------------------------------------------------------------------
    // Delete
    // -------------------------------------------------------------------------

    @Test fun deleteLeafRemovesElement() {
        val t = populated()
        t.delete(1)
        assertFalse(t.contains(1))
        assertEquals(4, t.size)
        assertTrue(t.isValid())
        assertEquals(listOf(3, 5, 7, 8), t.toSortedList())
    }

    @Test fun deleteNodeWithOneChild() {
        val t = BinarySearchTree<Int>()
        listOf(5, 3, 1).forEach { t.insert(it) }
        t.delete(3)
        assertFalse(t.contains(3))
        assertTrue(t.contains(1))
        assertTrue(t.isValid())
    }

    @Test fun deleteNodeWithTwoChildrenUsesSuccessor() {
        val t = populated()
        t.delete(5)
        assertFalse(t.contains(5))
        assertEquals(4, t.size)
        assertTrue(t.isValid())
        assertEquals(listOf(1, 3, 7, 8), t.toSortedList())
    }

    @Test fun deleteAbsentValueIsNoOp() {
        val t = populated()
        t.delete(99)
        assertEquals(5, t.size)
        assertTrue(t.isValid())
    }

    @Test fun deleteAllElementsLeavesEmptyTree() {
        val t = populated()
        listOf(1, 3, 5, 7, 8).forEach { t.delete(it) }
        assertEquals(0, t.size)
        assertTrue(t.isEmpty)
        assertNull(t.search(5))
    }

    // -------------------------------------------------------------------------
    // Min / Max
    // -------------------------------------------------------------------------

    @Test fun minValueReturnsSmallestElement() {
        assertEquals(1, populated().minValue())
    }

    @Test fun maxValueReturnsLargestElement() {
        assertEquals(8, populated().maxValue())
    }

    @Test fun minValueOnEmptyTreeReturnsNull() {
        assertNull(BinarySearchTree<Int>().minValue())
    }

    @Test fun maxValueOnEmptyTreeReturnsNull() {
        assertNull(BinarySearchTree<Int>().maxValue())
    }

    // -------------------------------------------------------------------------
    // Predecessor / Successor
    // -------------------------------------------------------------------------

    @Test fun predecessorReturnsPreviousValue() {
        val t = populated()
        assertEquals(3, t.predecessor(5))
        assertEquals(7, t.predecessor(8))
        assertEquals(1, t.predecessor(3))
    }

    @Test fun predecessorOfMinReturnsNull() {
        assertNull(populated().predecessor(1))
    }

    @Test fun successorReturnsNextValue() {
        val t = populated()
        assertEquals(7, t.successor(5))
        assertEquals(3, t.successor(1))
    }

    @Test fun successorOfMaxReturnsNull() {
        assertNull(populated().successor(8))
    }

    @Test fun predecessorAndSuccessorForAbsentValue() {
        val t = populated()   // [1, 3, 5, 7, 8]
        assertEquals(3, t.predecessor(4))
        assertEquals(5, t.successor(4))
    }

    // -------------------------------------------------------------------------
    // Order statistics: kthSmallest
    // -------------------------------------------------------------------------

    @Test fun kthSmallestReturnsCorrectRankElement() {
        val t = populated()   // sorted: [1, 3, 5, 7, 8]
        assertEquals(1, t.kthSmallest(1))
        assertEquals(3, t.kthSmallest(2))
        assertEquals(5, t.kthSmallest(3))
        assertEquals(7, t.kthSmallest(4))
        assertEquals(8, t.kthSmallest(5))
    }

    @Test fun kthSmallestOutOfRangeReturnsNull() {
        val t = populated()
        assertNull(t.kthSmallest(0))
        assertNull(t.kthSmallest(6))
        assertNull(t.kthSmallest(-1))
    }

    @Test fun kthSmallestOnEmptyTreeReturnsNull() {
        assertNull(BinarySearchTree<Int>().kthSmallest(1))
    }

    // -------------------------------------------------------------------------
    // Order statistics: rank
    // -------------------------------------------------------------------------

    @Test fun rankReturnsCountOfElementsLessThanValue() {
        val t = populated()   // [1, 3, 5, 7, 8]
        assertEquals(0, t.rank(1))
        assertEquals(1, t.rank(3))
        assertEquals(2, t.rank(5))
        assertEquals(3, t.rank(7))
        assertEquals(4, t.rank(8))
    }

    @Test fun rankForAbsentValues() {
        val t = populated()   // [1, 3, 5, 7, 8]
        assertEquals(2, t.rank(4))
        assertEquals(0, t.rank(0))
        assertEquals(5, t.rank(9))
    }

    // -------------------------------------------------------------------------
    // Validation
    // -------------------------------------------------------------------------

    @Test fun isValidReturnsTrueForCorrectlyBuiltTree() {
        assertTrue(populated().isValid())
    }

    @Test fun isValidReturnsTrueForEmptyTree() {
        assertTrue(BinarySearchTree<Int>().isValid())
    }

    @Test fun isValidDetectsViolationOfBstProperty() {
        val bad = BinarySearchTree<Int>()
        // Manually build: root=5 with left child=6 (violates BST)
        bad.root = bad.Node(5)
        bad.root!!.left = bad.Node(6).also { it.size = 1 }
        bad.root!!.size = 2
        assertFalse(bad.isValid())
    }

    @Test fun isValidDetectsSizeInvariantViolation() {
        val bad = BinarySearchTree<Int>()
        bad.root = bad.Node(5)
        bad.root!!.left = bad.Node(3).also { it.size = 1 }
        bad.root!!.size = 99   // wrong
        assertFalse(bad.isValid())
    }

    // -------------------------------------------------------------------------
    // Height
    // -------------------------------------------------------------------------

    @Test fun heightOfSingleNodeIsZero() {
        val t = BinarySearchTree<Int>()
        t.insert(1)
        assertEquals(0, t.height())
    }

    @Test fun heightOfBalancedTreeIsLog2n() {
        val t = BinarySearchTree.fromSortedList(listOf(1, 2, 3, 4, 5, 6, 7))
        assertEquals(2, t.height())
    }

    // -------------------------------------------------------------------------
    // toString
    // -------------------------------------------------------------------------

    @Test fun toStringShowsRootAndSize() {
        val empty = BinarySearchTree<Int>()
        assertEquals("BinarySearchTree(root=null, size=0)", empty.toString())

        val t = BinarySearchTree<Int>()
        t.insert(5)
        assertEquals("BinarySearchTree(root=5, size=1)", t.toString())
    }

    // -------------------------------------------------------------------------
    // Stress test
    // -------------------------------------------------------------------------

    @Test fun insertAndDeleteThousandElementsLeavesValidTree() {
        val t = BinarySearchTree<Int>()
        (1..1000).forEach { t.insert(it) }
        assertEquals(1000, t.size)

        (1..1000 step 2).forEach { t.delete(it) }  // delete odd numbers
        assertEquals(500, t.size)
        assertTrue(t.isValid())

        val sorted = t.toSortedList()
        assertEquals(500, sorted.size)
        sorted.forEachIndexed { i, v ->
            assertEquals(2 * (i + 1), v)  // only evens remain
        }
    }

    @Test fun randomInsertOrderPreservesOrderStatistics() {
        val t = BinarySearchTree<Int>()
        listOf(7, 2, 9, 1, 4, 8, 3).forEach { t.insert(it) }
        assertTrue(t.isValid())
        assertEquals(listOf(1, 2, 3, 4, 7, 8, 9), t.toSortedList())
        assertEquals(1, t.kthSmallest(1))
        assertEquals(9, t.kthSmallest(7))
        assertEquals(3, t.rank(4))   // 1, 2, 3 < 4
    }

    // -------------------------------------------------------------------------
    // String keys
    // -------------------------------------------------------------------------

    @Test fun worksWithStringKeys() {
        val t = BinarySearchTree<String>()
        listOf("banana", "apple", "cherry").forEach { t.insert(it) }
        assertEquals(listOf("apple", "banana", "cherry"), t.toSortedList())
        assertTrue(t.contains("apple"))
        assertEquals("apple", t.minValue())
        assertEquals("cherry", t.maxValue())
    }
}
