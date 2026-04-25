// ============================================================================
// BPlusTreeTest.kt — Exhaustive tests for BPlusTree<K,V> (DT12)
// ============================================================================
//
// Test strategy mirrors the Java suite:
//   - Empty-tree edge cases.
//   - Single-key and small-tree basics.
//   - Split / merge mechanics verified through isValid() and structural
//     assertions (height, size, full-scan ordering).
//   - Delete mechanics: no-underflow, borrow-right, borrow-left, merge.
//   - Range scan and full scan against brute-force reference.
//   - Randomised stress test vs TreeMap reference.
//
// Each test calls isValid() to verify all B+ tree invariants.
// ============================================================================

package com.codingadventures.bplustree

import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import java.util.TreeMap
import kotlin.random.Random
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class BPlusTreeTest {

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────

    private fun <K, V> List<Map.Entry<K, V>>.keys(): List<K> = map { it.key }
    private fun <K, V> List<Map.Entry<K, V>>.values(): List<V> = map { it.value }

    /** Build a BPlusTree<Int,String> from a vararg sequence of keys (values = "v"+key). */
    private fun treeOf(t: Int, vararg keys: Int): BPlusTree<Int, String> {
        val tree = BPlusTree<Int, String>(t)
        for (k in keys) tree.insert(k, "v$k")
        return tree
    }

    private fun intRange(from: Int, to: Int): IntArray = IntArray(to - from + 1) { from + it }

    private fun shuffled(arr: IntArray, rng: Random): IntArray {
        val a = arr.copyOf()
        for (i in a.size - 1 downTo 1) {
            val j = rng.nextInt(i + 1)
            val tmp = a[i]; a[i] = a[j]; a[j] = tmp
        }
        return a
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Empty-Tree Edge Cases
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun emptyTree_size() = assertEquals(0, BPlusTree<Int, String>().size)

    @Test fun emptyTree_isEmpty() = assertTrue(BPlusTree<Int, String>().isEmpty)

    @Test fun emptyTree_height() = assertEquals(0, BPlusTree<Int, String>().height())

    @Test fun emptyTree_searchReturnsNull() = assertNull(BPlusTree<Int, String>().search(42))

    @Test fun emptyTree_containsFalse() = assertFalse(BPlusTree<Int, String>().contains(1))

    @Test fun emptyTree_fullScanEmpty() = assertTrue(BPlusTree<Int, String>().fullScan().isEmpty())

    @Test fun emptyTree_rangeScanEmpty() = assertTrue(BPlusTree<Int, String>().rangeScan(1, 100).isEmpty())

    @Test fun emptyTree_iteratorEmpty() = assertFalse(BPlusTree<Int, String>().iterator().hasNext())

    @Test fun emptyTree_minKeyThrows() {
        assertFailsWith<NoSuchElementException> { BPlusTree<Int, String>().minKey() }
    }

    @Test fun emptyTree_maxKeyThrows() {
        assertFailsWith<NoSuchElementException> { BPlusTree<Int, String>().maxKey() }
    }

    @Test fun emptyTree_deleteNoOp() {
        val tree = BPlusTree<Int, String>()
        tree.delete(5)
        assertEquals(0, tree.size)
        assertTrue(tree.isValid())
    }

    @Test fun emptyTree_isValid() = assertTrue(BPlusTree<Int, String>().isValid())

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Constructor Validation
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun constructor_defaultDegreeIs2() {
        val tree = BPlusTree<String, Int>()
        assertTrue(tree.isEmpty)
        assertTrue(tree.isValid())
    }

    @Test fun constructor_degreeOne_throws() {
        assertFailsWith<IllegalArgumentException> { BPlusTree<Int, String>(1) }
    }

    @Test fun constructor_degreeZero_throws() {
        assertFailsWith<IllegalArgumentException> { BPlusTree<Int, String>(0) }
    }

    @Test fun constructor_degreeNegative_throws() {
        assertFailsWith<IllegalArgumentException> { BPlusTree<Int, String>(-5) }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 2b. Null-Key and Inverted-Bounds Validation
    // ─────────────────────────────────────────────────────────────────────────

    @Suppress("UNCHECKED_CAST")
    @Test fun nullKey_insert_throws() {
        val tree = BPlusTree<String, String>()
        // Cast to bypass Kotlin's compile-time non-null check (simulates Java interop).
        assertFailsWith<NullPointerException> { (tree as BPlusTree<String?, String>).insert(null, "v") }
    }

    @Test fun rangeScan_invertedBounds_throws() {
        val tree = treeOf(2, 1, 2, 3)
        assertFailsWith<IllegalArgumentException> { tree.rangeScan(5, 1) }
    }

    @Test fun contains_withNullValue_returnsTrue() {
        // A key inserted with a null value must still be reported as present.
        val tree = BPlusTree<Int, String?>()
        tree.insert(42, null)
        assertTrue(tree.contains(42))   // would be false if contains() used null sentinel
        assertNull(tree.search(42))     // value is null — search returns null
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Single-Key Operations
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun singleKey_insert_size1() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(42, "answer")
        assertEquals(1, tree.size)
        assertFalse(tree.isEmpty)
    }

    @Test fun singleKey_search_found() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(42, "answer")
        assertEquals("answer", tree.search(42))
    }

    @Test fun singleKey_contains() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(42, "answer")
        assertTrue(tree.contains(42))
        assertFalse(tree.contains(0))
    }

    @Test fun singleKey_height0() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(42, "answer")
        assertEquals(0, tree.height())   // still a single leaf
    }

    @Test fun singleKey_minMaxKey() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(42, "answer")
        assertEquals(42, tree.minKey())
        assertEquals(42, tree.maxKey())
    }

    @Test fun singleKey_fullScan() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(10, "ten")
        val scan = tree.fullScan()
        assertEquals(listOf(10), scan.keys())
        assertEquals(listOf("ten"), scan.values())
    }

    @Test fun singleKey_delete_thenEmpty() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(10, "ten")
        tree.delete(10)
        assertEquals(0, tree.size)
        assertTrue(tree.isEmpty)
        assertNull(tree.search(10))
        assertTrue(tree.isValid())
    }

    @Test fun singleKey_isValid() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(10, "ten")
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 4. In-Place Update (Duplicate Key)
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun update_replacesValue() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(5, "five")
        tree.insert(5, "FIVE")
        assertEquals(1, tree.size)
        assertEquals("FIVE", tree.search(5))
    }

    @Test fun update_manyDuplicates_sizeUnchanged() {
        val tree = BPlusTree<Int, String>(2)
        repeat(100) { i -> tree.insert(42, "v$i") }
        assertEquals(1, tree.size)
        assertEquals("v99", tree.search(42))
        assertTrue(tree.isValid())
    }

    @Test fun update_afterSplit_replacesValue() {
        val tree = treeOf(2, 1, 2, 3, 4, 5)
        assertTrue(tree.height() >= 1)
        tree.insert(3, "three-updated")
        assertEquals("three-updated", tree.search(3))
        assertEquals(5, tree.size)
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5. Leaf-Split Mechanics (t=2)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // t=2: max keys per node = 2*2-1 = 3.  Insert 4 keys → first leaf split.

    @Test fun leafSplit_heightBeforeAndAfter() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(1, "one")
        tree.insert(2, "two")
        tree.insert(3, "three")
        assertEquals(0, tree.height())   // still one leaf
        tree.insert(4, "four")
        assertEquals(1, tree.height())   // root now internal
    }

    @Test fun leafSplit_separatorStaysInRightLeaf() {
        val tree = treeOf(2, 1, 2, 3, 4)
        assertEquals("v3", tree.search(3))   // 3 stays in the right leaf
        assertTrue(tree.isValid())
    }

    @Test fun leafSplit_fullScanOrdered() {
        val tree = treeOf(2, 1, 2, 3, 4)
        assertEquals(listOf(1, 2, 3, 4), tree.fullScan().keys())
    }

    @Test fun leafSplit_linkedListIntact() {
        val tree = treeOf(2, 1, 2, 3, 4)
        assertEquals(4, tree.fullScan().size)
    }

    @Test fun multipleSplits_sizeAndOrder() {
        val tree = treeOf(2, 1, 2, 3, 4, 5, 6, 7, 8)
        assertEquals(8, tree.size)
        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7, 8), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    @Test fun multipleSplits_reverseOrder() {
        val tree = treeOf(2, 8, 7, 6, 5, 4, 3, 2, 1)
        assertEquals(8, tree.size)
        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7, 8), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    @Test fun multipleSplits_randomOrder() {
        val tree = treeOf(2, 5, 1, 8, 3, 7, 2, 6, 4)
        assertEquals(8, tree.size)
        assertEquals(listOf(1, 2, 3, 4, 5, 6, 7, 8), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 6. Height Behaviour (t=2)
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun height_growsMonotonically() {
        val tree = BPlusTree<Int, String>(2)
        var lastHeight = 0
        for (i in 0 until 50) {
            tree.insert(i, "v$i")
            val h = tree.height()
            assertTrue(h >= lastHeight, "height must not decrease")
            lastHeight = h
        }
        assertTrue(lastHeight >= 2) // must have grown past height 1 by key 50
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 7. Range Scan
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun rangeScan_exactBounds() {
        val tree = treeOf(2, 1, 2, 3, 4, 5)
        assertEquals(listOf(2, 3, 4), tree.rangeScan(2, 4).keys())
    }

    @Test fun rangeScan_entireTree() {
        val tree = treeOf(2, 3, 1, 5, 2, 4)
        assertEquals(listOf(1, 2, 3, 4, 5), tree.rangeScan(1, 5).keys())
    }

    @Test fun rangeScan_noResults() {
        val tree = treeOf(2, 1, 2, 3)
        assertTrue(tree.rangeScan(10, 20).isEmpty())
    }

    @Test fun rangeScan_singleKey() {
        val tree = treeOf(2, 1, 2, 3)
        assertEquals(listOf(2), tree.rangeScan(2, 2).keys())
    }

    @Test fun rangeScan_lowEqualsHigh_absent() {
        val tree = treeOf(2, 1, 3, 5)
        assertTrue(tree.rangeScan(2, 2).isEmpty())
    }

    @Test fun rangeScan_crossesLeafBoundary() {
        val tree = treeOf(2, 1, 2, 3, 4, 5, 6)
        assertEquals(listOf(2, 3, 4, 5), tree.rangeScan(2, 5).keys())
    }

    @Test fun rangeScan_leftOpenBoundary() {
        val tree = treeOf(2, 1, 2, 3, 4, 5)
        assertEquals(listOf(1, 2, 3), tree.rangeScan(1, 3).keys())
    }

    @Test fun rangeScan_rightOpenBoundary() {
        val tree = treeOf(2, 1, 2, 3, 4, 5)
        assertEquals(listOf(3, 4, 5), tree.rangeScan(3, 5).keys())
    }

    @Test fun rangeScan_bruteForce() {
        val n = 20
        val tree = BPlusTree<Int, String>(3)
        for (i in 1..n) tree.insert(i, "v$i")
        for (lo in 1..n) {
            for (hi in lo..n) {
                val got = tree.rangeScan(lo, hi).keys()
                val expected = (lo..hi).toList()
                assertEquals(expected, got, "rangeScan($lo,$hi) wrong")
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 8. Full Scan and Iterator
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun fullScan_emptyTree() = assertTrue(BPlusTree<Int, String>().fullScan().isEmpty())

    @Test fun fullScan_sortedResult() {
        val tree = treeOf(2, 9, 3, 7, 1, 5, 2, 8, 4, 6, 10)
        val ks = tree.fullScan().keys()
        for (i in 1 until ks.size) assertTrue(ks[i - 1] < ks[i], "fullScan not sorted at $i")
    }

    @Test fun iterator_matchesFullScan() {
        val tree = treeOf(2, 5, 3, 7, 1, 4, 6, 2)
        val scan = tree.fullScan()
        val iter = tree.toList()
        assertEquals(scan.keys(), iter.map { it.key })
        assertEquals(scan.values(), iter.map { it.value })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 9. Min / Max Key
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun minMaxKey_singleKey() {
        val tree = BPlusTree<Int, String>(2)
        tree.insert(42, "x")
        assertEquals(42, tree.minKey())
        assertEquals(42, tree.maxKey())
    }

    @Test fun minMaxKey_multipleKeys() {
        val tree = treeOf(2, 5, 1, 8, 3, 7)
        assertEquals(1, tree.minKey())
        assertEquals(8, tree.maxKey())
    }

    @Test fun minMaxKey_afterInsertSmaller() {
        val tree = treeOf(2, 5, 6, 7)
        tree.insert(1, "one")
        assertEquals(1, tree.minKey())
    }

    @Test fun minMaxKey_afterInsertLarger() {
        val tree = treeOf(2, 1, 2, 3)
        tree.insert(100, "hundred")
        assertEquals(100, tree.maxKey())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 10. Delete — No Underflow
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun delete_noUnderflow_found() {
        val tree = treeOf(2, 1, 2, 3)
        tree.delete(2)
        assertEquals(2, tree.size)
        assertNull(tree.search(2))
        assertEquals(listOf(1, 3), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    @Test fun delete_noUnderflow_notPresent_noOp() {
        val tree = treeOf(2, 1, 2, 3)
        tree.delete(99)
        assertEquals(3, tree.size)
        assertTrue(tree.isValid())
    }

    @Test fun delete_allThree_empty() {
        val tree = treeOf(2, 1, 2, 3)
        tree.delete(1); tree.delete(2); tree.delete(3)
        assertEquals(0, tree.size)
        assertTrue(tree.isEmpty)
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 11. Delete — Borrow From Right Sibling
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun delete_borrowFromRight() {
        val tree = treeOf(2, 1, 2, 3, 4, 5)
        assertTrue(tree.height() >= 1)
        tree.delete(1)
        assertEquals(4, tree.size)
        assertNull(tree.search(1))
        assertEquals(listOf(2, 3, 4, 5), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 12. Delete — Borrow From Left Sibling
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun delete_borrowFromLeft() {
        val tree = BPlusTree<Int, String>(3)
        for (i in 1..6) tree.insert(i, "v$i")
        tree.delete(6)
        tree.delete(5)
        assertEquals(4, tree.size)
        assertEquals(listOf(1, 2, 3, 4), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 13. Delete — Merge Leaves
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun delete_mergeThenHeightDecreases() {
        val tree = treeOf(2, 1, 2, 3, 4)
        assertEquals(1, tree.height())
        tree.delete(2); tree.delete(3); tree.delete(4)
        assertEquals(1, tree.size)
        assertEquals(0, tree.height())
        assertTrue(tree.isValid())
    }

    @Test fun delete_mergePreservesLinkedList() {
        val tree = treeOf(2, 1, 2, 3, 4, 5, 6)
        tree.delete(3); tree.delete(4)
        assertEquals(listOf(1, 2, 5, 6), tree.fullScan().keys())
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 14. Delete All Keys
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun deleteAll_sequential() {
        val n = 20
        val tree = BPlusTree<Int, String>(2)
        for (i in 1..n) tree.insert(i, "v$i")
        for (i in 1..n) {
            tree.delete(i)
            assertTrue(tree.isValid(), "invalid after deleting $i")
        }
        assertEquals(0, tree.size)
    }

    @Test fun deleteAll_reverseOrder() {
        val n = 20
        val tree = BPlusTree<Int, String>(2)
        for (i in 1..n) tree.insert(i, "v$i")
        for (i in n downTo 1) {
            tree.delete(i)
            assertTrue(tree.isValid(), "invalid after deleting $i")
        }
        assertEquals(0, tree.size)
    }

    @Test fun deleteAll_randomOrder() {
        val n = 30
        val keys = shuffled(intRange(1, n), Random(42))
        val tree = BPlusTree<Int, String>(2)
        for (i in 1..n) tree.insert(i, "v$i")
        for (k in keys) {
            tree.delete(k)
            assertTrue(tree.isValid(), "invalid after deleting $k")
        }
        assertEquals(0, tree.size)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 15. Routing Invariant (Separator Key Check)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // isValid() checks the routing invariant, not strict separator equality.
    // Separators may be stale after non-structural deletes — that is correct.

    @Test fun separatorInvariant_afterInserts() {
        val tree = BPlusTree<Int, String>(2)
        for (i in intArrayOf(10, 20, 5, 15, 25, 1, 12, 18, 22)) {
            tree.insert(i, "v$i")
            assertTrue(tree.isValid(), "isValid() failed after inserting $i")
        }
    }

    @Test fun separatorInvariant_afterDeletes() {
        val tree = treeOf(2, 5, 10, 15, 20, 25, 30)
        for (k in intArrayOf(15, 5, 25, 10)) {
            tree.delete(k)
            assertNull(tree.search(k), "search after delete should return null for $k")
            assertTrue(tree.isValid(), "isValid() failed after deleting $k")
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 16. Various Minimum Degrees
    // ─────────────────────────────────────────────────────────────────────────

    @ParameterizedTest
    @ValueSource(ints = [2, 3, 4, 5, 10])
    fun parameterisedDegree_insertAndSearch(t: Int) {
        val tree = BPlusTree<Int, String>(t)
        val n = 4 * t * t
        for (i in 1..n) tree.insert(i, "v$i")
        assertEquals(n, tree.size)
        for (i in 1..n) assertEquals("v$i", tree.search(i))
        assertTrue(tree.isValid())
    }

    @ParameterizedTest
    @ValueSource(ints = [2, 3, 4, 5, 10])
    fun parameterisedDegree_deleteAll(t: Int) {
        val n = 3 * t
        val tree = BPlusTree<Int, String>(t)
        for (i in 1..n) tree.insert(i, "v$i")
        for (i in 1..n) {
            tree.delete(i)
            assertTrue(tree.isValid(), "degree=$t invalid after deleting $i")
        }
        assertEquals(0, tree.size)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 17. String Keys
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun stringKeys_insertAndSearch() {
        val tree = BPlusTree<String, Int>(2)
        tree.insert("banana", 2)
        tree.insert("apple", 1)
        tree.insert("cherry", 3)
        tree.insert("date", 4)
        tree.insert("elderberry", 5)
        assertEquals(1, tree.search("apple"))
        assertEquals(3, tree.search("cherry"))
        assertNull(tree.search("fig"))
    }

    @Test fun stringKeys_fullScanSorted() {
        val tree = BPlusTree<String, Int>(2)
        val words = arrayOf("fig", "apple", "cherry", "banana", "elderberry", "date")
        words.forEachIndexed { i, w -> tree.insert(w, i) }
        val ks = tree.fullScan().keys()
        assertEquals(words.sorted(), ks)
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 18. Negative and Large Keys
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun negativeKeys_insertAndScan() {
        val tree = BPlusTree<Int, String>(2)
        for (i in -5..5) tree.insert(i, "v$i")
        val ks = tree.fullScan().keys()
        for (i in 1 until ks.size) assertTrue(ks[i - 1] < ks[i])
        assertEquals(11, tree.size)
        assertTrue(tree.isValid())
    }

    @Test fun largeKeys_insertAndScan() {
        val tree = BPlusTree<Int, String>(4)
        tree.insert(Int.MAX_VALUE, "max")
        tree.insert(Int.MIN_VALUE, "min")
        tree.insert(0, "zero")
        assertEquals(Int.MIN_VALUE, tree.minKey())
        assertEquals(Int.MAX_VALUE, tree.maxKey())
        assertEquals(3, tree.size)
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 19. Linked-List Integrity After Mixed Operations
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun linkedList_integrityAfterMixedOps() {
        val tree = BPlusTree<Int, String>(2)
        val rng = Random(7)
        val ref = TreeMap<Int, String>()
        val candidates = intRange(1, 30)
        repeat(60) { step ->
            val k = candidates[rng.nextInt(candidates.size)]
            if (rng.nextBoolean()) {
                tree.insert(k, "v$k"); ref[k] = "v$k"
            } else {
                tree.delete(k); ref.remove(k)
            }
            assertEquals(ref.keys.toList(), tree.fullScan().keys(), "linked list mismatch at step $step")
        }
        assertTrue(tree.isValid())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 20. Stress Test vs TreeMap Reference
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun stress_randomOps_matchesTreeMap() {
        val tree = BPlusTree<Int, String>(3)
        val ref = TreeMap<Int, String>()
        val rng = Random(12345)
        val ops = 500
        val keySpace = 100

        repeat(ops) { i ->
            val k = rng.nextInt(keySpace)
            if (rng.nextInt(3) > 0) {
                val v = "v${k}_$i"
                tree.insert(k, v); ref[k] = v
            } else {
                tree.delete(k); ref.remove(k)
            }
            // Spot-check search.
            for (check in intArrayOf(k, rng.nextInt(keySpace))) {
                assertEquals(ref[check], tree.search(check), "search($check) wrong at step $i")
            }
        }

        assertEquals(ref.size, tree.size)
        assertEquals(ref.keys.toList(), tree.fullScan().keys())
        for ((k, v) in ref) assertEquals(v, tree.search(k))
        assertTrue(tree.isValid())
    }

    @Test fun stress_largeTree_t4() {
        val tree = BPlusTree<Int, String>(4)
        val n = 1000
        for (i in 1..n) tree.insert(i, "v$i")
        assertEquals(n, tree.size)
        assertTrue(tree.isValid())
        val scan = tree.rangeScan(1, n)
        assertEquals(n, scan.size)
        assertEquals(1, scan.first().key)
        assertEquals(n, scan.last().key)
        // Delete every even key.
        for (i in 2..n step 2) tree.delete(i)
        assertEquals(n / 2, tree.size)
        assertTrue(tree.isValid())
        val ks = tree.fullScan().keys()
        for (k in ks) assertTrue(k % 2 != 0, "even key $k should be gone")
    }

    @Test fun stress_repeatInsertDelete() {
        val tree = BPlusTree<Int, String>(2)
        repeat(10) { round ->
            for (i in 1..20) tree.insert(i, "r${round}_$i")
            assertTrue(tree.isValid(), "invalid after insert round $round")
            assertEquals(20, tree.size)
            for (i in 1..20) tree.delete(i)
            assertTrue(tree.isValid(), "invalid after delete round $round")
            assertEquals(0, tree.size)
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 21. toString Smoke Test
    // ─────────────────────────────────────────────────────────────────────────

    @Test fun toString_containsSizeAndHeight() {
        val tree = treeOf(2, 1, 2, 3, 4, 5)
        val s = tree.toString()
        assertTrue(s.contains("size=5"), "toString should include size=5, got: $s")
        assertTrue(s.contains("height="), "toString should include height, got: $s")
        assertTrue(s.contains("t=2"), "toString should include t=2, got: $s")
    }
}
