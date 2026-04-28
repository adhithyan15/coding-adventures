package com.codingadventures.treeset

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Comprehensive tests for [TreeSet].
 *
 * Test organisation:
 * 1. Construction
 * 2. add / remove / contains
 * 3. min / max / first / last
 * 4. predecessor / successor
 * 5. rank / byRank / kthSmallest
 * 6. range
 * 7. toList / toSortedArray
 * 8. Set algebra: union, intersection, difference, symmetricDifference
 * 9. Predicates: isSubset, isSuperset, isDisjoint
 * 10. equals / hashCode
 * 11. Iteration order
 * 12. Stress test
 */
class TreeSetTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("Empty constructor creates empty set")
    fun emptyConstructor() {
        val s = TreeSet<Int>()
        assertEquals(0, s.size)
        assertTrue(s.isEmpty)
    }

    @Test
    @DisplayName("Constructor from Iterable populates the set")
    fun iterableConstructor() {
        val s = TreeSet(listOf(3, 1, 4, 1, 5, 9, 2, 6))
        assertEquals(7, s.size)  // 1 duplicate collapsed
        assertTrue(s.contains(3))
        assertTrue(s.contains(9))
    }

    @Test
    @DisplayName("TreeSet.of() factory creates set from varargs")
    fun factoryOf() {
        val s = TreeSet.of(5, 3, 7)
        assertEquals(3, s.size)
        assertTrue(s.contains(5))
    }

    // =========================================================================
    // 2. add / remove / contains
    // =========================================================================

    @Test
    @DisplayName("add inserts elements; duplicate add is a no-op")
    fun addBasic() {
        val s = TreeSet<String>()
        s.add("apple").add("banana").add("apple")
        assertEquals(2, s.size)
        assertTrue(s.contains("apple"))
        assertTrue(s.contains("banana"))
    }

    @Test
    @DisplayName("remove returns true when element was present")
    fun removePresent() {
        val s = TreeSet(listOf(1, 2, 3))
        assertTrue(s.remove(2))
        assertFalse(s.contains(2))
        assertEquals(2, s.size)
    }

    @Test
    @DisplayName("remove returns false when element was absent")
    fun removeAbsent() {
        val s = TreeSet(listOf(1, 2, 3))
        assertFalse(s.remove(99))
        assertEquals(3, s.size)
    }

    @Test
    @DisplayName("delete and discard are aliases for remove")
    fun deleteDiscardAliases() {
        val s = TreeSet(listOf(10, 20))
        assertTrue(s.delete(10))
        assertFalse(s.discard(99))
        assertEquals(1, s.size)
    }

    // =========================================================================
    // 3. min / max / first / last
    // =========================================================================

    @Test
    @DisplayName("min and max return null on empty set")
    fun minMaxEmpty() {
        val s = TreeSet<Int>()
        assertNull(s.min())
        assertNull(s.max())
    }

    @Test
    @DisplayName("min and max return correct values")
    fun minMaxBasic() {
        val s = TreeSet(listOf(5, 3, 8, 1, 9))
        assertEquals(1, s.min())
        assertEquals(9, s.max())
        assertEquals(1, s.first())
        assertEquals(9, s.last())
    }

    // =========================================================================
    // 4. predecessor / successor
    // =========================================================================

    @Test
    @DisplayName("predecessor returns null for minimum element")
    fun predecessorOfMin() {
        val s = TreeSet(listOf(1, 3, 5))
        assertNull(s.predecessor(1))
    }

    @Test
    @DisplayName("predecessor returns largest element strictly less than value")
    fun predecessorBasic() {
        val s = TreeSet(listOf(10, 20, 30, 40))
        assertEquals(10, s.predecessor(20))
        assertEquals(20, s.predecessor(25))  // 25 not in set
        assertEquals(30, s.predecessor(40))
    }

    @Test
    @DisplayName("successor returns null for maximum element")
    fun successorOfMax() {
        val s = TreeSet(listOf(1, 3, 5))
        assertNull(s.successor(5))
    }

    @Test
    @DisplayName("successor returns smallest element strictly greater than value")
    fun successorBasic() {
        val s = TreeSet(listOf(10, 20, 30, 40))
        assertEquals(30, s.successor(20))
        assertEquals(30, s.successor(25))  // 25 not in set
        assertNull(s.successor(40))
    }

    // =========================================================================
    // 5. rank / byRank / kthSmallest
    // =========================================================================

    @Test
    @DisplayName("rank returns 0-based count of smaller elements")
    fun rankBasic() {
        val s = TreeSet(listOf(10, 20, 30, 40, 50))
        assertEquals(0, s.rank(10))
        assertEquals(1, s.rank(20))
        assertEquals(2, s.rank(30))
        assertEquals(4, s.rank(50))
    }

    @Test
    @DisplayName("rank of absent value equals insertion position")
    fun rankAbsent() {
        val s = TreeSet(listOf(10, 20, 30))
        assertEquals(1, s.rank(15))
        assertEquals(0, s.rank(5))
        assertEquals(3, s.rank(35))
    }

    @Test
    @DisplayName("byRank returns correct element (0-based)")
    fun byRankBasic() {
        val s = TreeSet(listOf(5, 3, 7, 1))
        assertEquals(1, s.byRank(0))
        assertEquals(3, s.byRank(1))
        assertEquals(5, s.byRank(2))
        assertEquals(7, s.byRank(3))
    }

    @Test
    @DisplayName("byRank returns null for out-of-range index")
    fun byRankOutOfRange() {
        val s = TreeSet(listOf(1, 2, 3))
        assertNull(s.byRank(-1))
        assertNull(s.byRank(3))
    }

    @Test
    @DisplayName("kthSmallest returns correct element (1-based)")
    fun kthSmallestBasic() {
        val s = TreeSet(listOf(5, 3, 7, 1))
        assertEquals(1, s.kthSmallest(1))
        assertEquals(3, s.kthSmallest(2))
        assertEquals(5, s.kthSmallest(3))
        assertEquals(7, s.kthSmallest(4))
        assertNull(s.kthSmallest(0))
        assertNull(s.kthSmallest(5))
    }

    // =========================================================================
    // 6. range
    // =========================================================================

    @Test
    @DisplayName("range returns elements within inclusive bounds")
    fun rangeInclusive() {
        val s = TreeSet((1..10).toList())
        assertEquals(listOf(3, 4, 5, 6, 7), s.range(3, 7))
    }

    @Test
    @DisplayName("range with equal bounds returns single element")
    fun rangeSingleElement() {
        val s = TreeSet(listOf(10, 20, 30))
        assertEquals(listOf(20), s.range(20, 20))
    }

    @Test
    @DisplayName("range returns empty list when no elements fall in range")
    fun rangeMiss() {
        val s = TreeSet(listOf(1, 2, 3))
        assertTrue(s.range(5, 10).isEmpty())
    }

    @Test
    @DisplayName("range exclusive bounds exclude endpoints")
    fun rangeExclusive() {
        val s = TreeSet(listOf(1, 2, 3, 4, 5))
        assertEquals(listOf(2, 3, 4), s.range(1, 5, inclusive = false))
    }

    // =========================================================================
    // 7. toList / toSortedArray
    // =========================================================================

    @Test
    @DisplayName("toList returns elements in ascending order")
    fun toListSorted() {
        val s = TreeSet(listOf(5, 3, 7, 1, 4, 6, 8))
        assertEquals(listOf(1, 3, 4, 5, 6, 7, 8), s.toList())
        assertEquals(s.toList(), s.toSortedArray())
    }

    // =========================================================================
    // 8. Set algebra
    // =========================================================================

    @Test
    @DisplayName("union combines all elements from both sets")
    fun union() {
        val a = TreeSet(listOf(1, 3, 5))
        val b = TreeSet(listOf(2, 3, 4))
        assertEquals(listOf(1, 2, 3, 4, 5), a.union(b).toList())
    }

    @Test
    @DisplayName("intersection keeps only shared elements")
    fun intersection() {
        val a = TreeSet(listOf(1, 2, 3, 4, 5))
        val b = TreeSet(listOf(3, 4, 5, 6, 7))
        assertEquals(listOf(3, 4, 5), a.intersection(b).toList())
    }

    @Test
    @DisplayName("intersection of disjoint sets is empty")
    fun intersectionDisjoint() {
        val a = TreeSet(listOf(1, 2, 3))
        val b = TreeSet(listOf(4, 5, 6))
        assertTrue(a.intersection(b).isEmpty)
    }

    @Test
    @DisplayName("difference removes elements present in other")
    fun difference() {
        val a = TreeSet(listOf(1, 2, 3, 4, 5))
        val b = TreeSet(listOf(3, 4, 5, 6, 7))
        assertEquals(listOf(1, 2), a.difference(b).toList())
    }

    @Test
    @DisplayName("symmetricDifference keeps elements in exactly one set")
    fun symmetricDifference() {
        val a = TreeSet(listOf(1, 2, 3, 4))
        val b = TreeSet(listOf(3, 4, 5, 6))
        assertEquals(listOf(1, 2, 5, 6), a.symmetricDifference(b).toList())
    }

    @Test
    @DisplayName("Set algebra operations do not mutate operands")
    fun algebraDoesNotMutate() {
        val a = TreeSet(listOf(1, 2, 3))
        val b = TreeSet(listOf(2, 3, 4))
        a.union(b); a.intersection(b); a.difference(b); a.symmetricDifference(b)
        assertEquals(3, a.size)
        assertEquals(3, b.size)
    }

    // =========================================================================
    // 9. Predicates
    // =========================================================================

    @Test
    @DisplayName("isSubset returns true when all elements are in other")
    fun isSubset() {
        val sub = TreeSet(listOf(2, 3))
        val sup = TreeSet(listOf(1, 2, 3, 4))
        assertTrue(sub.isSubset(sup))
        assertFalse(sup.isSubset(sub))
    }

    @Test
    @DisplayName("isSuperset is the reverse of isSubset")
    fun isSuperset() {
        val small = TreeSet(listOf(2, 3))
        val large = TreeSet(listOf(1, 2, 3, 4))
        assertTrue(large.isSuperset(small))
        assertFalse(small.isSuperset(large))
    }

    @Test
    @DisplayName("empty set is subset of every set")
    fun emptyIsSubset() {
        val empty = TreeSet<Int>()
        val some  = TreeSet(listOf(1, 2))
        assertTrue(empty.isSubset(some))
        assertFalse(some.isSubset(empty))
    }

    @Test
    @DisplayName("isDisjoint returns true for non-overlapping sets")
    fun isDisjoint() {
        val a = TreeSet(listOf(1, 2, 3))
        val b = TreeSet(listOf(4, 5, 6))
        assertTrue(a.isDisjoint(b))
    }

    @Test
    @DisplayName("isDisjoint returns false for overlapping sets")
    fun isDisjointOverlap() {
        val a = TreeSet(listOf(1, 2, 3))
        val b = TreeSet(listOf(3, 4, 5))
        assertFalse(a.isDisjoint(b))
    }

    // =========================================================================
    // 10. equals / hashCode
    // =========================================================================

    @Test
    @DisplayName("Two TreeSets with same elements are equal")
    fun equalsBasic() {
        val a = TreeSet(listOf(1, 2, 3))
        val b = TreeSet(listOf(3, 2, 1))
        assertEquals(a, b)
        assertEquals(a.hashCode(), b.hashCode())
    }

    @Test
    @DisplayName("TreeSets with different elements are not equal")
    fun notEqual() {
        val a = TreeSet(listOf(1, 2, 3))
        val b = TreeSet(listOf(1, 2, 4))
        assertNotEquals(a, b)
    }

    // =========================================================================
    // 11. Iteration order
    // =========================================================================

    @Test
    @DisplayName("Iterator yields elements in ascending order")
    fun iterationOrder() {
        val s = TreeSet(listOf(5, 2, 8, 1, 9, 3))
        assertEquals(listOf(1, 2, 3, 5, 8, 9), s.toList())
    }

    // =========================================================================
    // 12. Stress test
    // =========================================================================

    @Test
    @DisplayName("Stress: 1000 operations with reference java.util.TreeSet")
    fun stressTest() {
        val our = TreeSet<Int>()
        val ref = java.util.TreeSet<Int>()
        val rng = java.util.Random(314)

        // Phase 1: 500 inserts
        repeat(500) {
            val k = rng.nextInt(300)
            our.add(k); ref.add(k)
        }
        assertEquals(ref.size, our.size)
        assertEquals(ref.toList(), our.toList())

        // Phase 2: 200 deletes
        val keys = ref.toList().take(200)
        for (k in keys) { our.remove(k); ref.remove(k) }
        assertEquals(ref.size, our.size)

        // Phase 3: 300 mixed ops
        repeat(300) {
            val k = rng.nextInt(600)
            if (rng.nextBoolean()) { our.add(k); ref.add(k) }
            else if (ref.contains(k)) { our.remove(k); ref.remove(k) }
        }
        assertEquals(ref.size, our.size)
        assertEquals(ref.toList(), our.toList())

        // Phase 4: min/max match
        assertEquals(ref.first(), our.min())
        assertEquals(ref.last(), our.max())
    }
}
