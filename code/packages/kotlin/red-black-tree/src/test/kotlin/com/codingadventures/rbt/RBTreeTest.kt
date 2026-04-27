package com.codingadventures.rbt

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import kotlin.math.ceil
import kotlin.math.log2
import kotlin.random.Random
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Comprehensive tests for Kotlin RBTree (DT09).
 *
 * Every structural test verifies isValidRB() — this ensures all 5 RB invariants
 * hold throughout the tree's lifecycle.
 */
class RBTreeTest {

    // ─── Helpers ────────────────────────────────────────────────────────────

    private fun buildTree(vararg values: Int): RBTree =
        values.fold(RBTree.empty()) { t, v -> t.insert(v) }

    // ─── Empty tree ──────────────────────────────────────────────────────────

    @Test
    fun emptyTree_isValid() {
        val t = RBTree.empty()
        assertTrue(t.isValidRB())
        assertTrue(t.isEmpty)
        assertEquals(0, t.size)
        assertEquals(0, t.height)
        assertEquals(0, t.blackHeight)
    }

    @Test
    fun emptyTree_containsReturnsFalse() {
        assertFalse(RBTree.empty().contains(42))
    }

    @Test
    fun emptyTree_minMaxNull() {
        assertNull(RBTree.empty().min)
        assertNull(RBTree.empty().max)
    }

    @Test
    fun emptyTree_kthSmallestThrows() {
        assertThrows<IllegalArgumentException> { RBTree.empty().kthSmallest(1) }
    }

    // ─── Single element ──────────────────────────────────────────────────────

    @Test
    fun singleInsert_rootIsBlack() {
        val t = buildTree(10)
        assertTrue(t.isValidRB())
        assertEquals(Color.BLACK, t.root?.color)
        assertEquals(1, t.size)
    }

    @Test
    fun singleInsert_containsValue() {
        val t = buildTree(42)
        assertTrue(t.contains(42))
        assertFalse(t.contains(41))
        assertFalse(t.contains(43))
    }

    @Test
    fun singleInsert_minMaxEqual() {
        val t = buildTree(7)
        assertEquals(7, t.min)
        assertEquals(7, t.max)
    }

    // ─── Multiple inserts — invariants ───────────────────────────────────────

    @Test
    fun insertAscending_alwaysValid() {
        var t = RBTree.empty()
        for (i in 1..20) {
            t = t.insert(i)
            assertTrue(t.isValidRB(), "invariant failed after inserting $i")
        }
        assertEquals(20, t.size)
    }

    @Test
    fun insertDescending_alwaysValid() {
        var t = RBTree.empty()
        for (i in 20 downTo 1) {
            t = t.insert(i)
            assertTrue(t.isValidRB(), "invariant failed after inserting $i")
        }
        assertEquals(20, t.size)
    }

    @Test
    fun insertAlternating_alwaysValid() {
        val values = intArrayOf(10, 1, 20, 5, 15, 3, 18, 7, 12)
        var t = RBTree.empty()
        for (v in values) {
            t = t.insert(v)
            assertTrue(t.isValidRB(), "invariant failed after inserting $v")
        }
    }

    @Test
    fun insertCLRSExample_classicSequence() {
        val t = buildTree(7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13)
        assertTrue(t.isValidRB())
        assertEquals(11, t.size)
        val sorted = t.toSortedList()
        assertEquals(listOf(2, 3, 6, 7, 8, 10, 11, 13, 18, 22, 26), sorted)
    }

    // ─── Duplicate handling ──────────────────────────────────────────────────

    @Test
    fun insertDuplicate_sizeUnchanged() {
        val t = buildTree(5, 5, 5)
        assertTrue(t.isValidRB())
        assertEquals(1, t.size)
        assertTrue(t.contains(5))
    }

    @Test
    fun insertDuplicate_afterMultiple() {
        val t = buildTree(1, 2, 3, 2, 1)
        assertTrue(t.isValidRB())
        assertEquals(3, t.size)
    }

    // ─── Height bound ────────────────────────────────────────────────────────

    @Test
    fun height_bounded_by2LogN() {
        var t = RBTree.empty()
        for (i in 1..100) t = t.insert(i)
        assertTrue(t.isValidRB())
        val maxAllowed = (2 * ceil(log2(101.0))).toInt()
        assertTrue(t.height <= maxAllowed,
            "height=${t.height} exceeds 2*log2(101)=$maxAllowed")
    }

    // ─── Contains ────────────────────────────────────────────────────────────

    @Test
    fun contains_presentAndAbsent() {
        val t = buildTree(10, 5, 15, 3, 7, 12, 20)
        assertTrue(t.contains(10))
        assertTrue(t.contains(5))
        assertTrue(t.contains(20))
        assertFalse(t.contains(1))
        assertFalse(t.contains(11))
        assertFalse(t.contains(100))
    }

    // ─── Min / Max ───────────────────────────────────────────────────────────

    @Test
    fun minMax_correctAfterInserts() {
        val t = buildTree(5, 3, 8, 1, 9, 4)
        assertEquals(1, t.min)
        assertEquals(9, t.max)
    }

    // ─── Predecessor / Successor ─────────────────────────────────────────────

    @Test
    fun predecessor_typical() {
        val t = buildTree(10, 5, 15, 3, 7, 12, 20)
        assertEquals(10, t.predecessor(12))
        assertEquals(7,  t.predecessor(10))
        assertEquals(5,  t.predecessor(7))
    }

    @Test
    fun predecessor_minimum_returnsNull() {
        val t = buildTree(10, 5, 15)
        assertNull(t.predecessor(5))
    }

    @Test
    fun successor_typical() {
        val t = buildTree(10, 5, 15, 3, 7, 12, 20)
        assertEquals(12, t.successor(10))
        assertEquals(10, t.successor(7))
        assertEquals(15, t.successor(12))
    }

    @Test
    fun successor_maximum_returnsNull() {
        val t = buildTree(10, 5, 15)
        assertNull(t.successor(15))
    }

    // ─── kthSmallest ─────────────────────────────────────────────────────────

    @Test
    fun kthSmallest_correctOrder() {
        val t = buildTree(5, 3, 8, 1, 9, 4)
        // Sorted: 1, 3, 4, 5, 8, 9
        assertEquals(1, t.kthSmallest(1))
        assertEquals(3, t.kthSmallest(2))
        assertEquals(4, t.kthSmallest(3))
        assertEquals(5, t.kthSmallest(4))
        assertEquals(8, t.kthSmallest(5))
        assertEquals(9, t.kthSmallest(6))
    }

    @Test
    fun kthSmallest_outOfRange_throws() {
        val t = buildTree(1, 2, 3)
        assertThrows<IllegalArgumentException> { t.kthSmallest(0) }
        assertThrows<IllegalArgumentException> { t.kthSmallest(4) }
    }

    // ─── toSortedList ────────────────────────────────────────────────────────

    @Test
    fun toSortedList_empty() {
        assertTrue(RBTree.empty().toSortedList().isEmpty())
    }

    @Test
    fun toSortedList_randomInsertion() {
        val t = buildTree(7, 2, 11, 5, 3, 9, 1, 8)
        assertEquals(listOf(1, 2, 3, 5, 7, 8, 9, 11), t.toSortedList())
    }

    // ─── Delete ──────────────────────────────────────────────────────────────

    @Test
    fun delete_absentElement_unchanged() {
        val t = buildTree(5, 3, 7)
        val t2 = t.delete(99)
        assertTrue(t2.isValidRB())
        assertEquals(3, t2.size)
    }

    @Test
    fun delete_singleElement_becomesEmpty() {
        val t = buildTree(42).delete(42)
        assertTrue(t.isValidRB())
        assertTrue(t.isEmpty)
    }

    @Test
    fun delete_rootInTwoNodeTree() {
        val t = buildTree(5, 3).delete(5)
        assertTrue(t.isValidRB())
        assertEquals(1, t.size)
        assertTrue(t.contains(3))
        assertFalse(t.contains(5))
    }

    @Test
    fun delete_leafNode() {
        val t = buildTree(5, 3, 7).delete(3)
        assertTrue(t.isValidRB())
        assertEquals(2, t.size)
        assertFalse(t.contains(3))
        assertTrue(t.contains(5))
        assertTrue(t.contains(7))
    }

    @Test
    fun delete_internalNode() {
        val t = buildTree(10, 5, 15, 3, 7, 12, 20).delete(5)
        assertTrue(t.isValidRB())
        assertEquals(6, t.size)
        assertFalse(t.contains(5))
        assertTrue(t.contains(3))
        assertTrue(t.contains(7))
    }

    @Test
    fun delete_allElements_preservesInvariant() {
        val values = intArrayOf(10, 5, 15, 3, 7, 12, 20)
        var t = buildTree(*values)
        for (v in values) {
            t = t.delete(v)
            assertTrue(t.isValidRB(), "invariant failed after deleting $v")
        }
        assertTrue(t.isEmpty)
    }

    @Test
    fun delete_minElement_repeatedly() {
        var t = buildTree(1, 2, 3, 4, 5)
        for (i in 1..5) {
            t = t.delete(i)
            assertTrue(t.isValidRB(), "invariant failed after deleting $i")
            assertFalse(t.contains(i))
        }
    }

    @Test
    fun delete_maxElement_repeatedly() {
        var t = buildTree(1, 2, 3, 4, 5)
        for (i in 5 downTo 1) {
            t = t.delete(i)
            assertTrue(t.isValidRB(), "invariant failed after deleting $i")
            assertFalse(t.contains(i))
        }
    }

    @Test
    fun delete_CLRSsequence_staysValid() {
        var t = buildTree(7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13)
        for (v in intArrayOf(18, 11, 3, 7, 8)) {
            t = t.delete(v)
            assertTrue(t.isValidRB(), "invariant failed after deleting $v")
        }
        assertEquals(6, t.size)
    }

    // ─── Insert then delete — round-trip ─────────────────────────────────────

    @Test
    fun insertThenDelete_roundTrip() {
        val values = intArrayOf(50, 25, 75, 10, 30, 60, 90, 5, 15, 27, 35)
        var t = buildTree(*values)
        values.forEach { assertTrue(t.contains(it)) }
        // Delete in reverse order
        for (i in values.indices.reversed()) {
            t = t.delete(values[i])
            assertTrue(t.isValidRB())
        }
        assertTrue(t.isEmpty)
    }

    // ─── Immutability ────────────────────────────────────────────────────────

    @Test
    fun immutability_oldTreeUnchanged() {
        val original = buildTree(5, 3, 7)
        val modified = original.insert(1).insert(9)
        assertEquals(3, original.size)
        assertFalse(original.contains(1))
        assertFalse(original.contains(9))
        assertEquals(5, modified.size)
        assertTrue(modified.isValidRB())
    }

    // ─── Random stress test ──────────────────────────────────────────────────

    @Test
    fun randomInserts_alwaysValid() {
        val rng = Random(42L)
        var t = RBTree.empty()
        repeat(200) { i ->
            val v = rng.nextInt(100)
            t = t.insert(v)
            assertTrue(t.isValidRB(), "invariant failed on random insert $i (v=$v)")
        }
    }

    @Test
    fun randomDeletesAfterInserts_alwaysValid() {
        val rng = Random(7L)
        val inserted = mutableListOf<Int>()
        var t = RBTree.empty()

        repeat(100) {
            val v = rng.nextInt(50)
            t = t.insert(v)
            if (!inserted.contains(v)) inserted.add(v)
        }
        assertTrue(t.isValidRB())

        inserted.shuffle(java.util.Random(7L))
        for (v in inserted) {
            t = t.delete(v)
            assertTrue(t.isValidRB(), "invariant failed after deleting $v")
        }
        assertTrue(t.isEmpty)
    }

    // ─── Black height ─────────────────────────────────────────────────────────

    @Test
    fun blackHeight_emptyTree_zero() {
        assertEquals(0, RBTree.empty().blackHeight)
    }

    @Test
    fun blackHeight_consistentWithValidation() {
        val t = buildTree(7, 3, 18, 10, 22, 8, 11, 26, 2, 6, 13)
        assertTrue(t.isValidRB())
        assertTrue(t.blackHeight > 0)
    }

    @Test
    fun isValidRB_rootAlwaysBlack() {
        val t = buildTree(5)
        assertEquals(Color.BLACK, t.root?.color)
    }

    @Test
    fun isValidRB_largeTree() {
        var t = RBTree.empty()
        for (i in 1..63) t = t.insert(i)
        assertTrue(t.isValidRB())
    }

    // ─── Height bound parameterized ──────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(ints = [1, 3, 7, 15, 31, 63])
    fun insertN_heightBounded(n: Int) {
        var t = RBTree.empty()
        for (i in 1..n) t = t.insert(i)
        assertTrue(t.isValidRB())
        val logN = ceil(log2((n + 1).toDouble())).toInt()
        assertTrue(t.height <= 2 * logN + 1,
            "height=${t.height} logN=$logN n=$n")
    }
}
