package com.codingadventures.btree

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.ValueSource
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Comprehensive tests for [BTree].
 *
 * Test organisation:
 * 1. Construction and empty-tree invariants
 * 2. Insert — basic, duplicate-key update, large volume
 * 3. Search and contains
 * 4. Delete — all CLRS cases (1, 2a, 2b, 2c, 3a, 3b)
 * 5. minKey / maxKey
 * 6. rangeQuery
 * 7. inorder traversal
 * 8. height
 * 9. isValid — passes for valid trees
 * 10. Stress test — 1 000 random insert/delete/search cycles
 */
class BTreeTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("Default constructor creates empty 2-3-4 tree")
    fun defaultConstructor() {
        val tree = BTree<Int, String>()
        assertEquals(0, tree.size)
        assertTrue(tree.isEmpty)
        assertEquals(0, tree.height)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Constructor with t=2 creates empty tree")
    fun constructorT2() {
        val tree = BTree<Int, String>(t = 2)
        assertEquals(0, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Constructor with t=3 creates empty tree")
    fun constructorT3() {
        val tree = BTree<Int, String>(t = 3)
        assertEquals(0, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Constructor with t=5 creates empty tree")
    fun constructorT5() {
        val tree = BTree<Int, String>(t = 5)
        assertEquals(0, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Constructor rejects t < 2")
    fun constructorRejectsBadT() {
        assertThrows<IllegalArgumentException> { BTree<Int, String>(t = 1) }
        assertThrows<IllegalArgumentException> { BTree<Int, String>(t = 0) }
        assertThrows<IllegalArgumentException> { BTree<Int, String>(t = -5) }
    }

    // =========================================================================
    // 2. Insert
    // =========================================================================

    @Test
    @DisplayName("Single insert updates size and height=0")
    fun singleInsert() {
        val tree = BTree<Int, String>(t = 2)
        tree.insert(10, "ten")
        assertEquals(1, tree.size)
        assertFalse(tree.isEmpty)
        assertEquals(0, tree.height)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Insert several keys — size grows monotonically")
    fun multipleInserts() {
        val tree = BTree<Int, String>(t = 2)
        val keys = intArrayOf(5, 3, 7, 1, 9, 2, 8)
        for ((idx, k) in keys.withIndex()) {
            tree.insert(k, "v$k")
            assertEquals(idx + 1, tree.size)
            assertTrue(tree.isValid())
        }
    }

    @Test
    @DisplayName("Inserting existing key updates value, does not grow size")
    fun duplicateKeyUpdatesValue() {
        val tree = BTree<Int, String>(t = 2)
        tree.insert(42, "original")
        tree.insert(42, "updated")
        assertEquals(1, tree.size)
        assertEquals("updated", tree.search(42))
        assertTrue(tree.isValid())
    }

    @ParameterizedTest(name = "t={0}: insert enough keys to force a root split")
    @ValueSource(ints = [2, 3, 4, 5])
    fun rootSplitForcedByFullInsertion(t: Int) {
        // A root with min-degree t holds up to 2t-1 keys before splitting.
        // Inserting 2t keys forces at least one split.
        val tree = BTree<Int, String>(t = t)
        val n = 2 * t
        for (i in 0 until n) tree.insert(i, "v$i")
        assertEquals(n, tree.size)
        assertTrue(tree.isValid())
        assertTrue(tree.height >= 1)
    }

    @Test
    @DisplayName("Sequential ascending insertion (t=2, 100 keys)")
    fun ascendingInsert100() {
        val tree = BTree<Int, String>(t = 2)
        for (i in 0 until 100) tree.insert(i, "v$i")
        assertEquals(100, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Sequential descending insertion (t=2, 100 keys)")
    fun descendingInsert100() {
        val tree = BTree<Int, String>(t = 2)
        for (i in 99 downTo 0) tree.insert(i, "v$i")
        assertEquals(100, tree.size)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Insert 200 keys with t=3 — tree stays valid throughout")
    fun insertT3Large() {
        val tree = BTree<Int, String>(t = 3)
        val keys = (0 until 200).toMutableList().also { it.shuffle(java.util.Random(42)) }
        for (k in keys) {
            tree.insert(k, "v$k")
            assertTrue(tree.isValid(), "Invalid after inserting $k")
        }
        assertEquals(200, tree.size)
    }

    // =========================================================================
    // 3. Search and contains
    // =========================================================================

    @Test
    @DisplayName("search returns correct value for inserted keys")
    fun searchHit() {
        val tree = BTree<String, Int>(t = 2)
        tree.insert("apple", 1)
        tree.insert("banana", 2)
        tree.insert("cherry", 3)
        assertEquals(1, tree.search("apple"))
        assertEquals(2, tree.search("banana"))
        assertEquals(3, tree.search("cherry"))
    }

    @Test
    @DisplayName("search returns null for absent keys")
    fun searchMiss() {
        val tree = BTree<String, Int>(t = 2)
        tree.insert("apple", 1)
        assertNull(tree.search("mango"))
    }

    @Test
    @DisplayName("contains returns true for present keys, false for absent")
    fun containsBasic() {
        val tree = BTree<Int, String>(t = 2)
        tree.insert(10, "a")
        tree.insert(20, "b")
        assertTrue(tree.contains(10))
        assertTrue(tree.contains(20))
        assertFalse(tree.contains(15))
    }

    // =========================================================================
    // 4. Delete
    // =========================================================================

    @Test
    @DisplayName("Delete from a single-element tree leaves it empty")
    fun deleteOnlyElement() {
        val tree = BTree<Int, String>(t = 2)
        tree.insert(5, "five")
        tree.delete(5)
        assertEquals(0, tree.size)
        assertTrue(tree.isEmpty)
        assertTrue(tree.isValid())
        assertFalse(tree.contains(5))
    }

    @Test
    @DisplayName("Delete absent key throws NoSuchElementException")
    fun deleteAbsentKeyThrows() {
        val tree = BTree<Int, String>(t = 2)
        tree.insert(10, "ten")
        assertThrows<NoSuchElementException> { tree.delete(99) }
    }

    @Test
    @DisplayName("Case 1 — delete key from a leaf node")
    fun deleteCase1Leaf() {
        val tree = buildTree(2, 1, 2, 3, 4, 5, 6, 7)
        val before = tree.size
        tree.delete(1)   // leftmost leaf
        assertEquals(before - 1, tree.size)
        assertFalse(tree.contains(1))
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Delete all keys one by one — tree stays valid at every step")
    fun deleteAllKeysSequentially() {
        val tree = BTree<Int, String>(t = 2)
        val n = 20
        val keys = (1..n).toMutableList()
        for (i in 1..n) tree.insert(i, "v$i")
        keys.shuffle(java.util.Random(7))
        for (k in keys) {
            tree.delete(k)
            assertFalse(tree.contains(k))
            assertTrue(tree.isValid(), "Invalid after deleting $k")
        }
        assertEquals(0, tree.size)
        assertTrue(tree.isEmpty)
    }

    @Test
    @DisplayName("Delete with merge — tree shrinks in height")
    fun deleteWithMergeShrinksHeight() {
        val tree = BTree<Int, String>(t = 2)
        for (i in 1..15) tree.insert(i, "v$i")
        assertTrue(tree.height >= 2)
        for (i in 1..14) tree.delete(i)
        assertEquals(1, tree.size)
        assertEquals(0, tree.height)
        assertTrue(tree.isValid())
    }

    @Test
    @DisplayName("Delete with t=3 — all keys removed, tree valid throughout")
    fun deleteAllT3() {
        val tree = BTree<Int, String>(t = 3)
        val n = 50
        val keys = (0 until n).map { it * 3 }.toMutableList()
        for (k in keys) tree.insert(k, "v")
        keys.shuffle(java.util.Random(13))
        for (k in keys) {
            tree.delete(k)
            assertTrue(tree.isValid(), "Invalid after deleting $k")
        }
        assertEquals(0, tree.size)
    }

    @Test
    @DisplayName("Mixed inserts and deletes preserve validity (t=2)")
    fun mixedInsertDeleteT2() {
        val tree = BTree<Int, String>(t = 2)
        val rng = java.util.Random(99)
        val reference = java.util.TreeSet<Int>()
        repeat(200) {
            val k = rng.nextInt(50)
            if (rng.nextBoolean()) {
                tree.insert(k, "v$k")
                reference.add(k)
            } else if (k in reference) {
                tree.delete(k)
                reference.remove(k)
            }
            assertEquals(reference.size, tree.size)
            assertTrue(tree.isValid())
        }
    }

    // =========================================================================
    // 5. minKey / maxKey
    // =========================================================================

    @Test
    @DisplayName("minKey and maxKey on empty tree throw")
    fun minMaxEmptyThrows() {
        val tree = BTree<Int, String>(t = 2)
        assertThrows<NoSuchElementException> { tree.minKey() }
        assertThrows<NoSuchElementException> { tree.maxKey() }
    }

    @Test
    @DisplayName("minKey and maxKey return correct values")
    fun minMaxBasic() {
        val tree = buildTree(3, 10, 5, 20, 1, 15, 30)
        assertEquals(1,  tree.minKey())
        assertEquals(30, tree.maxKey())
    }

    @Test
    @DisplayName("minKey and maxKey are correct after deletions")
    fun minMaxAfterDelete() {
        val tree = buildTree(2, 10, 5, 20, 1, 15, 30)
        tree.delete(1)
        assertEquals(5, tree.minKey())
        tree.delete(30)
        assertEquals(20, tree.maxKey())
    }

    @Test
    @DisplayName("minKey == maxKey for single-element tree")
    fun minMaxSingleElement() {
        val tree = BTree<Int, String>(t = 2)
        tree.insert(42, "x")
        assertEquals(42, tree.minKey())
        assertEquals(42, tree.maxKey())
    }

    // =========================================================================
    // 6. rangeQuery
    // =========================================================================

    @Test
    @DisplayName("rangeQuery on empty tree returns empty list")
    fun rangeQueryEmpty() {
        assertTrue(BTree<Int, String>(t = 2).rangeQuery(1, 10).isEmpty())
    }

    @Test
    @DisplayName("rangeQuery returns all matching entries in order")
    fun rangeQueryBasic() {
        val tree = buildTree(2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
        val result = tree.rangeQuery(3, 7)
        assertEquals(5, result.size)
        for ((idx, pair) in result.withIndex()) {
            assertEquals(idx + 3, pair.first)
        }
    }

    @Test
    @DisplayName("rangeQuery with single-key range returns one entry")
    fun rangeQuerySingleKey() {
        val tree = buildTree(2, 10, 20, 30)
        val result = tree.rangeQuery(20, 20)
        assertEquals(1, result.size)
        assertEquals(20, result[0].first)
    }

    @Test
    @DisplayName("rangeQuery returns empty list when range misses all keys")
    fun rangeQueryMiss() {
        val tree = buildTree(2, 10, 20, 30)
        assertTrue(tree.rangeQuery(11, 19).isEmpty())
        assertTrue(tree.rangeQuery(31, 40).isEmpty())
    }

    @Test
    @DisplayName("rangeQuery works across multiple levels")
    fun rangeQueryMultiLevel() {
        val tree = BTree<Int, String>(t = 2)
        for (i in 1..50) tree.insert(i, "v$i")
        val result = tree.rangeQuery(10, 20)
        assertEquals(11, result.size)
        for ((idx, pair) in result.withIndex()) {
            assertEquals(idx + 10, pair.first)
        }
    }

    // =========================================================================
    // 7. inorder traversal
    // =========================================================================

    @Test
    @DisplayName("inorder returns all keys in ascending order (t=2)")
    fun inorderAscendingOrderT2() {
        val tree = buildTree(2, 5, 3, 7, 1, 4, 6, 8)
        val keys = tree.inorder().map { it.first }
        assertEquals(keys.sorted(), keys)
        assertEquals(7, keys.size)
    }

    @Test
    @DisplayName("inorder on large tree with t=3 produces sorted output")
    fun inorderLargeT3() {
        val tree = BTree<Int, String>(t = 3)
        val inserted = (0 until 200).map { it * 3 }
        for (k in inserted) tree.insert(k, "v")
        val fromTree = tree.inorder().map { it.first }
        assertEquals(inserted.sorted(), fromTree)
    }

    @Test
    @DisplayName("inorder on empty tree returns empty list")
    fun inorderEmpty() {
        assertTrue(BTree<Int, String>(t = 2).inorder().isEmpty())
    }

    // =========================================================================
    // 8. height
    // =========================================================================

    @Test
    @DisplayName("Empty tree has height 0")
    fun heightEmpty() {
        assertEquals(0, BTree<Int, String>(t = 2).height)
    }

    @Test
    @DisplayName("Height grows logarithmically with t=2")
    fun heightLogarithmic() {
        val tree = BTree<Int, String>(t = 2)
        for (i in 0 until 100) tree.insert(i, "v")
        val h = tree.height
        assertTrue(h in 1..10, "Unexpected height $h")
    }

    @Test
    @DisplayName("Height of root-only tree is 0")
    fun heightRootOnly() {
        val tree = BTree<Int, String>(t = 5)
        // Insert up to 2t-2 = 8 keys — no split yet, still one root node
        for (i in 0 until 8) tree.insert(i, "v")
        assertEquals(0, tree.height)
    }

    // =========================================================================
    // 9. isValid
    // =========================================================================

    @Test
    @DisplayName("Empty tree is valid")
    fun isValidEmptyTree() {
        assertTrue(BTree<Int, String>(t = 2).isValid())
    }

    @Test
    @DisplayName("Tree is valid after many insertions with t=2")
    fun isValidAfterInsertsT2() {
        val tree = BTree<Int, String>(t = 2)
        for (i in 0 until 100) {
            tree.insert(i, "v")
            assertTrue(tree.isValid(), "Invalid after inserting $i")
        }
    }

    @Test
    @DisplayName("Tree is valid after many insertions with t=5")
    fun isValidAfterInsertsT5() {
        val tree = BTree<Int, String>(t = 5)
        for (i in 0 until 200) {
            tree.insert(i, "v")
            assertTrue(tree.isValid(), "Invalid after inserting $i")
        }
    }

    // =========================================================================
    // 10. toString
    // =========================================================================

    @Test
    @DisplayName("toString includes t, size, height")
    fun toStringContainsInfo() {
        val tree = BTree<Int, String>(t = 3)
        tree.insert(1, "one")
        val s = tree.toString()
        assertTrue(s.contains("t=3"))
        assertTrue(s.contains("size=1"))
        assertTrue(s.contains("height=0"))
    }

    // =========================================================================
    // 11. Stress test
    // =========================================================================

    @Test
    @DisplayName("Stress: 1000-element insert/delete/search with reference map")
    fun stressTest1000() {
        val tree = BTree<Int, String>(t = 3)
        val ref  = java.util.TreeMap<Int, String>()
        val rng  = java.util.Random(1234)

        // Phase 1: insert 1000 distinct keys
        val keys = (0 until 1000).toMutableList().also { it.shuffle(rng) }
        for (k in keys) {
            val v = "v$k"
            tree.insert(k, v)
            ref[k] = v
        }
        assertEquals(ref.size, tree.size)
        assertTrue(tree.isValid())

        // Phase 2: search all keys
        for (k in keys) assertEquals(ref[k], tree.search(k))

        // Phase 3: delete half the keys
        val toDelete = keys.shuffled(rng).take(500)
        for (k in toDelete) {
            tree.delete(k)
            ref.remove(k)
        }
        assertEquals(ref.size, tree.size)
        assertTrue(tree.isValid())

        // Phase 4: insert-and-delete cycle — 200 random ops
        repeat(200) {
            val k = rng.nextInt(2000)
            if (rng.nextBoolean()) {
                tree.insert(k, "v$k"); ref[k] = "v$k"
            } else if (ref.containsKey(k)) {
                tree.delete(k); ref.remove(k)
            }
        }
        assertEquals(ref.size, tree.size)
        assertTrue(tree.isValid())

        // Phase 5: in-order matches sorted reference
        val treeKeys = tree.inorder().map { it.first }
        val refKeys  = ref.keys.toList()
        assertEquals(refKeys, treeKeys)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /** Build a BTree<Int,String> with the given minimum degree and keys. */
    private fun buildTree(t: Int, vararg keys: Int): BTree<Int, String> {
        val tree = BTree<Int, String>(t = t)
        for (k in keys) tree.insert(k, "v$k")
        return tree
    }

    /** Assert that the given block throws an exception of type T. */
    private inline fun <reified T : Throwable> assertThrows(block: () -> Unit) {
        var threw = false
        try { block() } catch (e: Throwable) { if (e is T) threw = true else throw e }
        assertTrue(threw, "Expected ${T::class.simpleName} to be thrown")
    }
}
