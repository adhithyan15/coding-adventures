package com.codingadventures.avltree

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Comprehensive tests for [AVLTree].
 *
 * Test organisation:
 * 1. Construction
 * 2. Insert — basic, duplicate, ascending/descending/random
 * 3. Contains
 * 4. Delete — leaf, one-child, two-child, rotations triggered
 * 5. min / max
 * 6. predecessor / successor
 * 7. kthSmallest
 * 8. rank
 * 9. toSortedList
 * 10. height / balanceFactor
 * 11. isValid / isValidBST
 * 12. Stress test — 1000 random operations
 */
class AVLTreeTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("Empty tree has size 0, height -1, is empty")
    fun emptyTree() {
        val tree = AVLTree<Int>()
        assertEquals(0, tree.size)
        assertEquals(-1, tree.height)
        assertTrue(tree.isEmpty)
        assertTrue(tree.isValid())
    }

    // =========================================================================
    // 2. Insert
    // =========================================================================

    @Test
    @DisplayName("Single insert: size=1, height=0")
    fun singleInsert() {
        val tree = AVLTree<Int>()
        tree.insert(10)
        assertEquals(1, tree.size)
        assertEquals(0, tree.height)
        assertFalse(tree.isEmpty)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Duplicate insert leaves tree unchanged")
    fun duplicateInsert() {
        val tree = AVLTree<Int>()
        tree.insert(5)
        tree.insert(5)
        assertEquals(1, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Ascending insertion triggers right-heavy rebalancing")
    fun ascendingInsert() {
        val tree = AVLTree<Int>()
        for (i in 1..10) tree.insert(i)
        assertEquals(10, tree.size)
        assertTrue(tree.isValid())
        assertTrue(tree.height <= 5, "Height too large: ${tree.height}")
    }

    @Test
    @DisplayName("Descending insertion triggers left-heavy rebalancing")
    fun descendingInsert() {
        val tree = AVLTree<Int>()
        for (i in 10 downTo 1) tree.insert(i)
        assertEquals(10, tree.size)
        assertTrue(tree.isValid())
        assertTrue(tree.height <= 5, "Height too large: ${tree.height}")
    }

    @Test
    @DisplayName("Random insertion maintains AVL invariant throughout")
    fun randomInsert() {
        val tree = AVLTree<Int>()
        val keys = (0 until 100).toMutableList().also { it.shuffle(java.util.Random(42)) }
        for (k in keys) {
            tree.insert(k)
            assertTrue(tree.isValid(), "Invalid after inserting $k")
        }
        assertEquals(100, tree.size)
    }

    // =========================================================================
    // 3. Contains
    // =========================================================================

    @Test
    @DisplayName("contains returns true for inserted values")
    fun containsHit() {
        val tree = AVLTree<String>()
        tree.insert("apple")
        tree.insert("banana")
        assertTrue(tree.contains("apple"))
        assertTrue(tree.contains("banana"))
    }

    @Test
    @DisplayName("contains returns false for absent values")
    fun containsMiss() {
        val tree = AVLTree<String>()
        tree.insert("apple")
        assertFalse(tree.contains("mango"))
    }

    // =========================================================================
    // 4. Delete
    // =========================================================================

    @Test
    @DisplayName("Delete the only element leaves empty tree")
    fun deleteOnlyElement() {
        val tree = AVLTree<Int>()
        tree.insert(5)
        tree.delete(5)
        assertEquals(0, tree.size)
        assertTrue(tree.isEmpty)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Delete absent value throws NoSuchElementException")
    fun deleteAbsentThrows() {
        val tree = AVLTree<Int>()
        tree.insert(1)
        assertThrowsNSE { tree.delete(99) }
    }

    @Test
    @DisplayName("Delete a leaf node maintains validity")
    fun deleteLeaf() {
        val tree = buildTree(5, 3, 7, 1, 4)
        tree.delete(1)
        assertFalse(tree.contains(1))
        assertEquals(4, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Delete node with one child maintains validity")
    fun deleteOneChildNode() {
        val tree = buildTree(5, 3, 7, 1)
        tree.delete(3)
        assertFalse(tree.contains(3))
        assertEquals(3, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Delete node with two children (uses successor)")
    fun deleteTwoChildNode() {
        val tree = buildTree(5, 3, 7, 1, 4, 6, 8)
        tree.delete(5)
        assertFalse(tree.contains(5))
        assertEquals(6, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Delete all nodes one by one — tree stays valid")
    fun deleteAllNodes() {
        val tree = AVLTree<Int>()
        val keys = (1..20).toMutableList().also { it.shuffle(java.util.Random(7)) }
        for (k in 1..20) tree.insert(k)
        for (k in keys) {
            tree.delete(k)
            assertFalse(tree.contains(k))
            assertTrue(tree.isValid(), "Invalid after deleting $k")
        }
        assertTrue(tree.isEmpty)
    }

    @Test
    @DisplayName("Mixed insert and delete preserves validity")
    fun mixedInsertDelete() {
        val tree = AVLTree<Int>()
        val ref  = java.util.TreeSet<Int>()
        val rng  = java.util.Random(99)
        repeat(200) {
            val k = rng.nextInt(50)
            if (rng.nextBoolean()) { tree.insert(k); ref.add(k) }
            else if (k in ref) { tree.delete(k); ref.remove(k) }
            assertEquals(ref.size, tree.size)
            assertTrue(tree.isValid())
        }
    }

    // =========================================================================
    // 5. min / max
    // =========================================================================

    @Test
    @DisplayName("min and max on empty tree throw")
    fun minMaxEmptyThrows() {
        val tree = AVLTree<Int>()
        assertThrowsNSE { tree.min() }
        assertThrowsNSE { tree.max() }
    }

    @Test
    @DisplayName("min and max return correct values")
    fun minMaxBasic() {
        val tree = buildTree(5, 3, 8, 1, 9, 2)
        assertEquals(1, tree.min())
        assertEquals(9, tree.max())
    }

    @Test
    @DisplayName("min and max update correctly after deletions")
    fun minMaxAfterDelete() {
        val tree = buildTree(5, 3, 7)
        tree.delete(3)
        assertEquals(5, tree.min())
        tree.delete(7)
        assertEquals(5, tree.max())
    }

    // =========================================================================
    // 6. predecessor / successor
    // =========================================================================

    @Test
    @DisplayName("predecessor returns null when value is the minimum")
    fun predecessorOfMin() {
        val tree = buildTree(5, 3, 7)
        assertNull(tree.predecessor(3))
    }

    @Test
    @DisplayName("predecessor returns correct value")
    fun predecessorBasic() {
        val tree = buildTree(5, 3, 7, 1, 4, 6, 8)
        assertEquals(4, tree.predecessor(5))
        assertEquals(1, tree.predecessor(3))
        assertEquals(6, tree.predecessor(7))
    }

    @Test
    @DisplayName("successor returns null when value is the maximum")
    fun successorOfMax() {
        val tree = buildTree(5, 3, 7)
        assertNull(tree.successor(7))
    }

    @Test
    @DisplayName("successor returns correct value")
    fun successorBasic() {
        val tree = buildTree(5, 3, 7, 1, 4, 6, 8)
        assertEquals(6, tree.successor(5))
        assertEquals(4, tree.successor(3))
        assertEquals(8, tree.successor(7))
    }

    @Test
    @DisplayName("predecessor/successor of value not in tree")
    fun predecessorSuccessorAbsent() {
        val tree = buildTree(10, 20, 30)
        assertEquals(10, tree.predecessor(15))
        assertEquals(20, tree.successor(15))
    }

    // =========================================================================
    // 7. kthSmallest
    // =========================================================================

    @Test
    @DisplayName("kthSmallest returns correct values (1-based)")
    fun kthSmallestBasic() {
        val tree = buildTree(5, 3, 7, 1, 4, 6, 8)
        assertEquals(1, tree.kthSmallest(1))
        assertEquals(3, tree.kthSmallest(2))
        assertEquals(4, tree.kthSmallest(3))
        assertEquals(5, tree.kthSmallest(4))
        assertEquals(6, tree.kthSmallest(5))
        assertEquals(7, tree.kthSmallest(6))
        assertEquals(8, tree.kthSmallest(7))
    }

    @Test
    @DisplayName("kthSmallest returns null for out-of-range k")
    fun kthSmallestOutOfRange() {
        val tree = buildTree(1, 2, 3)
        assertNull(tree.kthSmallest(0))
        assertNull(tree.kthSmallest(4))
    }

    // =========================================================================
    // 8. rank
    // =========================================================================

    @Test
    @DisplayName("rank returns 0-based position (number of smaller elements)")
    fun rankBasic() {
        val tree = buildTree(10, 20, 30, 40, 50)
        assertEquals(0, tree.rank(10))
        assertEquals(1, tree.rank(20))
        assertEquals(2, tree.rank(30))
        assertEquals(3, tree.rank(40))
        assertEquals(4, tree.rank(50))
    }

    @Test
    @DisplayName("rank of value not in tree is insertion position")
    fun rankAbsent() {
        val tree = buildTree(10, 20, 30)
        assertEquals(1, tree.rank(15))
        assertEquals(0, tree.rank(5))
        assertEquals(3, tree.rank(35))
    }

    // =========================================================================
    // 9. toSortedList
    // =========================================================================

    @Test
    @DisplayName("toSortedList returns elements in ascending order")
    fun toSortedListBasic() {
        val tree = buildTree(5, 3, 7, 1, 4, 6, 8)
        assertEquals(listOf(1, 3, 4, 5, 6, 7, 8), tree.toSortedList())
    }

    @Test
    @DisplayName("toSortedList on empty tree returns empty list")
    fun toSortedListEmpty() {
        assertTrue(AVLTree<Int>().toSortedList().isEmpty())
    }

    // =========================================================================
    // 10. height / balanceFactor
    // =========================================================================

    @Test
    @DisplayName("Height grows logarithmically with n")
    fun heightLogarithmic() {
        val tree = AVLTree<Int>()
        for (i in 0 until 100) tree.insert(i)
        assertTrue(tree.height in 6..10, "Unexpected height ${tree.height}")
    }

    @Test
    @DisplayName("Root balance factor is in {-1, 0, 1}")
    fun balanceFactorValid() {
        val tree = AVLTree<Int>()
        for (i in 0 until 30) tree.insert(i)
        assertTrue(tree.balanceFactor in -1..1, "Bad BF: ${tree.balanceFactor}")
    }

    // =========================================================================
    // 11. isValid / isValidBST
    // =========================================================================

    @Test
    @DisplayName("Empty tree is valid")
    fun isValidEmpty() {
        assertTrue(AVLTree<Int>().isValid())
        assertTrue(AVLTree<Int>().isValidBST())
    }

    @Test
    @DisplayName("isValid is true throughout ascending insertion")
    fun isValidDuringInserts() {
        val tree = AVLTree<Int>()
        for (i in 0 until 50) {
            tree.insert(i)
            assertTrue(tree.isValid(), "Invalid after inserting $i")
        }
    }

    // =========================================================================
    // 12. Stress test
    // =========================================================================

    @Test
    @DisplayName("Stress: 1000 operations with reference TreeSet")
    fun stressTest() {
        val tree = AVLTree<Int>()
        val ref  = java.util.TreeSet<Int>()
        val rng  = java.util.Random(5678)

        // Phase 1: insert 500 random values
        repeat(500) {
            val k = rng.nextInt(300)
            tree.insert(k); ref.add(k)
        }
        assertEquals(ref.size, tree.size)
        assertTrue(tree.isValid())

        // Phase 2: delete half
        val toDelete = ref.toList().take(ref.size / 2)
        for (k in toDelete) { tree.delete(k); ref.remove(k) }
        assertEquals(ref.size, tree.size)
        assertTrue(tree.isValid())

        // Phase 3: 500 mixed ops
        repeat(500) {
            val k = rng.nextInt(500)
            if (rng.nextBoolean()) { tree.insert(k); ref.add(k) }
            else if (ref.contains(k)) { tree.delete(k); ref.remove(k) }
        }
        assertEquals(ref.size, tree.size)
        assertTrue(tree.isValid())

        // Phase 4: in-order matches reference
        assertEquals(ref.toList(), tree.toSortedList())
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun buildTree(vararg values: Int): AVLTree<Int> {
        val tree = AVLTree<Int>()
        for (v in values) tree.insert(v)
        return tree
    }

    private fun assertThrowsNSE(block: () -> Unit) {
        var threw = false
        try { block() } catch (e: NoSuchElementException) { threw = true }
        assertTrue(threw, "Expected NoSuchElementException")
    }
}
