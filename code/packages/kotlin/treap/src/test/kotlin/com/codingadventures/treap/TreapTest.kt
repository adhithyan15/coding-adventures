package com.codingadventures.treap

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.random.Random
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Comprehensive tests for Kotlin Treap (DT10).
 *
 * Every structural test calls isValidTreap() to verify both BST and heap
 * properties hold throughout the treap's lifecycle.
 */
class TreapTest {

    // ─── Helpers ────────────────────────────────────────────────────────────

    private fun buildTree(seed: Long = 42L, vararg keys: Int): Treap =
        keys.fold(Treap.withSeed(seed)) { t, k -> t.insert(k) }

    // ─── Empty treap ──────────────────────────────────────────────────────────

    @Test
    fun emptyTreap_isValid() {
        val t = Treap.withSeed(1L)
        assertTrue(t.isValidTreap())
        assertTrue(t.isEmpty)
        assertEquals(0, t.size)
        assertEquals(0, t.height)
    }

    @Test
    fun emptyTreap_containsReturnsFalse() {
        assertFalse(Treap.withSeed(1L).contains(42))
    }

    @Test
    fun emptyTreap_minMaxNull() {
        assertNull(Treap.withSeed(1L).min)
        assertNull(Treap.withSeed(1L).max)
    }

    @Test
    fun emptyTreap_kthSmallestThrows() {
        assertThrows<IllegalArgumentException> { Treap.withSeed(1L).kthSmallest(1) }
    }

    // ─── Single element ──────────────────────────────────────────────────────

    @Test
    fun singleInsert_valid() {
        val t = buildTree(42L, 10)
        assertTrue(t.isValidTreap())
        assertEquals(1, t.size)
        assertNotNull(t.root)
        assertEquals(10, t.root!!.key)
    }

    @Test
    fun singleInsert_containsValue() {
        val t = buildTree(42L, 42)
        assertTrue(t.contains(42))
        assertFalse(t.contains(41))
        assertFalse(t.contains(43))
    }

    @Test
    fun singleInsert_minMaxEqual() {
        val t = buildTree(42L, 7)
        assertEquals(7, t.min)
        assertEquals(7, t.max)
    }

    // ─── Deterministic priorities ─────────────────────────────────────────────

    @Test
    fun insertWithPriority_heapPropertyHolds() {
        val t = Treap.withSeed(0L)
            .insertWithPriority(5, 0.91)
            .insertWithPriority(3, 0.53)
            .insertWithPriority(7, 0.75)
            .insertWithPriority(1, 0.22)
            .insertWithPriority(4, 0.68)

        assertTrue(t.isValidTreap())
        // Root must be 5 (highest priority 0.91)
        assertEquals(5, t.root!!.key)
        assertEquals(0.91, t.root!!.priority, 1e-10)
    }

    @Test
    fun insertWithPriority_sortedOrder() {
        val t = Treap.withSeed(0L)
            .insertWithPriority(5, 0.91)
            .insertWithPriority(3, 0.53)
            .insertWithPriority(7, 0.75)
            .insertWithPriority(1, 0.22)
            .insertWithPriority(4, 0.68)

        assertEquals(listOf(1, 3, 4, 5, 7), t.toSortedList())
    }

    // ─── Multiple inserts — invariants ───────────────────────────────────────

    @Test
    fun insertAscending_alwaysValid() {
        var t = Treap.withSeed(42L)
        for (i in 1..20) {
            t = t.insert(i)
            assertTrue(t.isValidTreap(), "invariant failed after inserting $i")
        }
        assertEquals(20, t.size)
    }

    @Test
    fun insertDescending_alwaysValid() {
        var t = Treap.withSeed(42L)
        for (i in 20 downTo 1) {
            t = t.insert(i)
            assertTrue(t.isValidTreap(), "invariant failed after inserting $i")
        }
        assertEquals(20, t.size)
    }

    @Test
    fun insertMixed_alwaysValid() {
        val values = intArrayOf(10, 5, 15, 3, 7, 12, 20, 1, 6, 9, 14, 18)
        var t = Treap.withSeed(7L)
        for (v in values) {
            t = t.insert(v)
            assertTrue(t.isValidTreap(), "invariant failed after inserting $v")
        }
    }

    // ─── Duplicate handling ──────────────────────────────────────────────────

    @Test
    fun insertDuplicate_sizeUnchanged() {
        val t = buildTree(42L, 5, 5, 5)
        assertTrue(t.isValidTreap())
        assertEquals(1, t.size)
        assertTrue(t.contains(5))
    }

    @Test
    fun insertDuplicate_multipleExisting() {
        val t = buildTree(42L, 1, 2, 3, 2, 1)
        assertTrue(t.isValidTreap())
        assertEquals(3, t.size)
    }

    // ─── Height ──────────────────────────────────────────────────────────────

    @Test
    fun height_roughlyLogarithmic_100elements() {
        var t = Treap.withSeed(99L)
        for (i in 1..100) t = t.insert(i)
        assertTrue(t.isValidTreap())
        assertTrue(t.height < 40, "height=${t.height} suspiciously large")
    }

    // ─── Contains ────────────────────────────────────────────────────────────

    @Test
    fun contains_presentAndAbsent() {
        val t = buildTree(42L, 10, 5, 15, 3, 7, 12, 20)
        assertTrue(t.contains(10))
        assertTrue(t.contains(5))
        assertTrue(t.contains(20))
        assertFalse(t.contains(1))
        assertFalse(t.contains(11))
        assertFalse(t.contains(100))
    }

    // ─── Min / Max ───────────────────────────────────────────────────────────

    @Test
    fun minMax_correct() {
        val t = buildTree(42L, 5, 3, 8, 1, 9, 4)
        assertEquals(1, t.min)
        assertEquals(9, t.max)
    }

    // ─── Predecessor / Successor ─────────────────────────────────────────────

    @Test
    fun predecessor_typical() {
        val t = buildTree(42L, 10, 5, 15, 3, 7, 12, 20)
        assertEquals(10, t.predecessor(12))
        assertEquals(7,  t.predecessor(10))
        assertEquals(5,  t.predecessor(7))
    }

    @Test
    fun predecessor_minimum_null() {
        val t = buildTree(42L, 10, 5, 15)
        assertNull(t.predecessor(5))
    }

    @Test
    fun successor_typical() {
        val t = buildTree(42L, 10, 5, 15, 3, 7, 12, 20)
        assertEquals(12, t.successor(10))
        assertEquals(10, t.successor(7))
        assertEquals(15, t.successor(12))
    }

    @Test
    fun successor_maximum_null() {
        val t = buildTree(42L, 10, 5, 15)
        assertNull(t.successor(15))
    }

    // ─── kthSmallest ─────────────────────────────────────────────────────────

    @Test
    fun kthSmallest_correct() {
        val t = buildTree(42L, 5, 3, 8, 1, 9, 4)
        assertEquals(1, t.kthSmallest(1))
        assertEquals(3, t.kthSmallest(2))
        assertEquals(4, t.kthSmallest(3))
        assertEquals(5, t.kthSmallest(4))
        assertEquals(8, t.kthSmallest(5))
        assertEquals(9, t.kthSmallest(6))
    }

    @Test
    fun kthSmallest_outOfRange_throws() {
        val t = buildTree(42L, 1, 2, 3)
        assertThrows<IllegalArgumentException> { t.kthSmallest(0) }
        assertThrows<IllegalArgumentException> { t.kthSmallest(4) }
    }

    // ─── toSortedList ────────────────────────────────────────────────────────

    @Test
    fun toSortedList_empty() {
        assertTrue(Treap.withSeed(0L).toSortedList().isEmpty())
    }

    @Test
    fun toSortedList_alwaysSorted() {
        val t = buildTree(42L, 7, 2, 11, 5, 3, 9, 1, 8)
        assertEquals(listOf(1, 2, 3, 5, 7, 8, 9, 11), t.toSortedList())
    }

    // ─── Split ───────────────────────────────────────────────────────────────

    @Test
    fun split_correctPartition() {
        val t = buildTree(42L, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
        val (left, right) = t.split(5)
        for (k in left.toSortedList()) assertTrue(k <= 5, "left has key $k > 5")
        for (k in right.toSortedList()) assertTrue(k > 5, "right has key $k <= 5")
        assertEquals(5, left.size)
        assertEquals(5, right.size)
    }

    @Test
    fun split_atMin_leftEmpty() {
        val t = buildTree(42L, 3, 5, 7)
        val (left, right) = t.split(2)
        assertNull(left.root)
        assertNotNull(right.root)
    }

    @Test
    fun split_atMax_rightEmpty() {
        val t = buildTree(42L, 3, 5, 7)
        val (left, right) = t.split(7)
        assertNotNull(left.root)
        assertNull(right.root)
    }

    // ─── mergeTreaps ─────────────────────────────────────────────────────────

    @Test
    fun mergeTreaps_reconstructsOriginal() {
        val original = buildTree(42L, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
        val (left, right) = original.split(5)
        val merged = mergeTreaps(left, right)
        assertTrue(merged.isValidTreap())
        assertEquals(10, merged.size)
        assertEquals((1..10).toList(), merged.toSortedList())
    }

    @Test
    fun mergeTreaps_withEmptyLeft() {
        val left  = Treap.withSeed(1L)
        val right = buildTree(42L, 1, 2, 3)
        val merged = mergeTreaps(left, right)
        assertTrue(merged.isValidTreap())
        assertEquals(3, merged.size)
    }

    @Test
    fun mergeTreaps_withEmptyRight() {
        val left  = buildTree(42L, 1, 2, 3)
        val right = Treap.withSeed(1L)
        val merged = mergeTreaps(left, right)
        assertTrue(merged.isValidTreap())
        assertEquals(3, merged.size)
    }

    // ─── Delete ──────────────────────────────────────────────────────────────

    @Test
    fun delete_absentKey_unchanged() {
        val t = buildTree(42L, 5, 3, 7)
        val t2 = t.delete(99)
        assertTrue(t2.isValidTreap())
        assertEquals(3, t2.size)
    }

    @Test
    fun delete_singleElement_becomesEmpty() {
        val t = buildTree(42L, 42).delete(42)
        assertTrue(t.isValidTreap())
        assertTrue(t.isEmpty)
    }

    @Test
    fun delete_leafKey() {
        val t = buildTree(42L, 5, 3, 7).delete(3)
        assertTrue(t.isValidTreap())
        assertEquals(2, t.size)
        assertFalse(t.contains(3))
        assertTrue(t.contains(5))
        assertTrue(t.contains(7))
    }

    @Test
    fun delete_rootKey() {
        val t = Treap.withSeed(0L)
            .insertWithPriority(5, 0.9)
            .insertWithPriority(3, 0.5)
            .insertWithPriority(7, 0.6)
        val t2 = t.delete(5)
        assertTrue(t2.isValidTreap())
        assertEquals(2, t2.size)
        assertFalse(t2.contains(5))
    }

    @Test
    fun delete_allElements_preservesInvariant() {
        val values = intArrayOf(10, 5, 15, 3, 7, 12, 20)
        var t = buildTree(42L, *values)
        for (v in values) {
            t = t.delete(v)
            assertTrue(t.isValidTreap(), "invariant failed after deleting $v")
        }
        assertTrue(t.isEmpty)
    }

    @Test
    fun delete_minElement_repeatedly() {
        var t = buildTree(42L, 1, 2, 3, 4, 5)
        for (i in 1..5) {
            t = t.delete(i)
            assertTrue(t.isValidTreap(), "invariant failed after deleting $i")
            assertFalse(t.contains(i))
        }
    }

    @Test
    fun delete_maxElement_repeatedly() {
        var t = buildTree(42L, 1, 2, 3, 4, 5)
        for (i in 5 downTo 1) {
            t = t.delete(i)
            assertTrue(t.isValidTreap(), "invariant failed after deleting $i")
            assertFalse(t.contains(i))
        }
    }

    // ─── Round-trip ──────────────────────────────────────────────────────────

    @Test
    fun insertThenDelete_roundTrip() {
        val values = intArrayOf(50, 25, 75, 10, 30, 60, 90, 5, 15, 27, 35)
        var t = buildTree(42L, *values)
        values.forEach { assertTrue(t.contains(it)) }
        for (i in values.indices.reversed()) {
            t = t.delete(values[i])
            assertTrue(t.isValidTreap())
        }
        assertTrue(t.isEmpty)
    }

    // ─── Immutability ────────────────────────────────────────────────────────

    @Test
    fun immutability_oldTreapUnchanged() {
        val original = buildTree(42L, 5, 3, 7)
        val modified = original.insert(1).insert(9)
        assertEquals(3, original.size)
        assertFalse(original.contains(1))
        assertFalse(original.contains(9))
        assertEquals(5, modified.size)
        assertTrue(modified.isValidTreap())
    }

    // ─── Random stress ───────────────────────────────────────────────────────

    @Test
    fun randomInserts_alwaysValid() {
        val javaRng = java.util.Random(42L)
        var t = Treap.withSeed(99L)
        repeat(200) { i ->
            val v = javaRng.nextInt(100)
            t = t.insert(v)
            assertTrue(t.isValidTreap(), "invariant failed on insert $i (v=$v)")
        }
    }

    @Test
    fun randomDeletesAfterInserts_alwaysValid() {
        val javaRng = java.util.Random(7L)
        val inserted = mutableListOf<Int>()
        var t = Treap.withSeed(55L)

        repeat(100) {
            val v = javaRng.nextInt(50)
            t = t.insert(v)
            if (!inserted.contains(v)) inserted.add(v)
        }
        assertTrue(t.isValidTreap())

        inserted.shuffle(javaRng)
        for (v in inserted) {
            t = t.delete(v)
            assertTrue(t.isValidTreap(), "invariant failed after deleting $v")
        }
        assertTrue(t.isEmpty)
    }
}
