package com.codingadventures.treeset;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive tests for {@link TreeSet}.
 *
 * <p>Test organisation:
 * <ol>
 *   <li>Construction</li>
 *   <li>add / remove / contains</li>
 *   <li>min / max / first / last</li>
 *   <li>predecessor / successor</li>
 *   <li>rank / byRank / kthSmallest</li>
 *   <li>range</li>
 *   <li>toList / toSortedArray</li>
 *   <li>Set algebra: union, intersection, difference, symmetricDifference</li>
 *   <li>Predicates: isSubset, isSuperset, isDisjoint</li>
 *   <li>equals / hashCode</li>
 *   <li>Iteration order</li>
 *   <li>Stress test</li>
 * </ol>
 */
class TreeSetTest {

    // =========================================================================
    // 1. Construction
    // =========================================================================

    @Test
    @DisplayName("Empty constructor creates empty set")
    void emptyConstructor() {
        TreeSet<Integer> s = new TreeSet<>();
        assertEquals(0, s.size());
        assertTrue(s.isEmpty());
    }

    @Test
    @DisplayName("Constructor from Iterable populates the set")
    void iterableConstructor() {
        TreeSet<Integer> s = new TreeSet<>(List.of(3, 1, 4, 1, 5, 9, 2, 6));
        // Duplicates are collapsed
        assertEquals(7, s.size());
        assertTrue(s.contains(3));
        assertTrue(s.contains(9));
    }

    @Test
    @DisplayName("Copy constructor creates independent copy")
    void copyConstructor() {
        TreeSet<Integer> original = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> copy = new TreeSet<>(original);
        copy.add(99);
        assertFalse(original.contains(99));
        assertEquals(3, original.size());
    }

    // =========================================================================
    // 2. add / remove / contains
    // =========================================================================

    @Test
    @DisplayName("add inserts elements; duplicate add is a no-op")
    void addBasic() {
        TreeSet<String> s = new TreeSet<>();
        s.add("apple").add("banana").add("apple");
        assertEquals(2, s.size());
        assertTrue(s.contains("apple"));
        assertTrue(s.contains("banana"));
    }

    @Test
    @DisplayName("add null throws IllegalArgumentException")
    void addNullThrows() {
        assertThrows(IllegalArgumentException.class, () -> new TreeSet<Integer>().add(null));
    }

    @Test
    @DisplayName("remove returns true when element was present")
    void removePresent() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2, 3));
        assertTrue(s.remove(2));
        assertFalse(s.contains(2));
        assertEquals(2, s.size());
    }

    @Test
    @DisplayName("remove returns false when element was absent")
    void removeAbsent() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2, 3));
        assertFalse(s.remove(99));
        assertEquals(3, s.size());
    }

    @Test
    @DisplayName("delete and discard are aliases for remove")
    void deleteDiscardAliases() {
        TreeSet<Integer> s = new TreeSet<>(List.of(10, 20));
        assertTrue(s.delete(10));
        assertFalse(s.discard(99));  // absent — no-op
        assertEquals(1, s.size());
    }

    @Test
    @DisplayName("contains returns false for null")
    void containsNull() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2));
        assertFalse(s.contains(null));
    }

    // =========================================================================
    // 3. min / max / first / last
    // =========================================================================

    @Test
    @DisplayName("min and max return null on empty set")
    void minMaxEmpty() {
        TreeSet<Integer> s = new TreeSet<>();
        assertNull(s.min());
        assertNull(s.max());
    }

    @Test
    @DisplayName("min and max return correct values")
    void minMaxBasic() {
        TreeSet<Integer> s = new TreeSet<>(List.of(5, 3, 8, 1, 9));
        assertEquals(1, s.min());
        assertEquals(9, s.max());
        assertEquals(1, s.first());
        assertEquals(9, s.last());
    }

    // =========================================================================
    // 4. predecessor / successor
    // =========================================================================

    @Test
    @DisplayName("predecessor returns null for minimum element")
    void predecessorOfMin() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 3, 5));
        assertNull(s.predecessor(1));
    }

    @Test
    @DisplayName("predecessor returns largest element strictly less than value")
    void predecessorBasic() {
        TreeSet<Integer> s = new TreeSet<>(List.of(10, 20, 30, 40));
        assertEquals(10, s.predecessor(20));
        assertEquals(20, s.predecessor(25));  // 25 not in set
        assertEquals(30, s.predecessor(40));
    }

    @Test
    @DisplayName("successor returns null for maximum element")
    void successorOfMax() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 3, 5));
        assertNull(s.successor(5));
    }

    @Test
    @DisplayName("successor returns smallest element strictly greater than value")
    void successorBasic() {
        TreeSet<Integer> s = new TreeSet<>(List.of(10, 20, 30, 40));
        assertEquals(30, s.successor(20));
        assertEquals(30, s.successor(25));  // 25 not in set
        assertNull(s.successor(40));
    }

    // =========================================================================
    // 5. rank / byRank / kthSmallest
    // =========================================================================

    @Test
    @DisplayName("rank returns 0-based count of smaller elements")
    void rankBasic() {
        TreeSet<Integer> s = new TreeSet<>(List.of(10, 20, 30, 40, 50));
        assertEquals(0, s.rank(10));
        assertEquals(1, s.rank(20));
        assertEquals(2, s.rank(30));
        assertEquals(4, s.rank(50));
    }

    @Test
    @DisplayName("rank of absent value equals insertion position")
    void rankAbsent() {
        TreeSet<Integer> s = new TreeSet<>(List.of(10, 20, 30));
        assertEquals(1, s.rank(15));
        assertEquals(0, s.rank(5));
        assertEquals(3, s.rank(35));
    }

    @Test
    @DisplayName("byRank returns correct element (0-based)")
    void byRankBasic() {
        TreeSet<Integer> s = new TreeSet<>(List.of(5, 3, 7, 1));
        assertEquals(1, s.byRank(0));
        assertEquals(3, s.byRank(1));
        assertEquals(5, s.byRank(2));
        assertEquals(7, s.byRank(3));
    }

    @Test
    @DisplayName("byRank returns null for out-of-range index")
    void byRankOutOfRange() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2, 3));
        assertNull(s.byRank(-1));
        assertNull(s.byRank(3));
    }

    @Test
    @DisplayName("kthSmallest returns correct element (1-based)")
    void kthSmallestBasic() {
        TreeSet<Integer> s = new TreeSet<>(List.of(5, 3, 7, 1));
        assertEquals(1, s.kthSmallest(1));
        assertEquals(3, s.kthSmallest(2));
        assertEquals(5, s.kthSmallest(3));
        assertEquals(7, s.kthSmallest(4));
        assertNull(s.kthSmallest(0));
        assertNull(s.kthSmallest(5));
    }

    // =========================================================================
    // 6. range
    // =========================================================================

    @Test
    @DisplayName("range returns elements within inclusive bounds")
    void rangeInclusive() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2, 3, 4, 5, 6, 7, 8, 9, 10));
        assertEquals(List.of(3, 4, 5, 6, 7), s.range(3, 7));
    }

    @Test
    @DisplayName("range with equal bounds returns single element")
    void rangeSingleElement() {
        TreeSet<Integer> s = new TreeSet<>(List.of(10, 20, 30));
        assertEquals(List.of(20), s.range(20, 20));
    }

    @Test
    @DisplayName("range returns empty list when no elements fall in range")
    void rangeMiss() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2, 3));
        assertTrue(s.range(5, 10).isEmpty());
    }

    @Test
    @DisplayName("range exclusive bounds exclude endpoints")
    void rangeExclusive() {
        TreeSet<Integer> s = new TreeSet<>(List.of(1, 2, 3, 4, 5));
        assertEquals(List.of(2, 3, 4), s.range(1, 5, false));
    }

    // =========================================================================
    // 7. toList / toSortedArray
    // =========================================================================

    @Test
    @DisplayName("toList returns elements in ascending order")
    void toListSorted() {
        TreeSet<Integer> s = new TreeSet<>(List.of(5, 3, 7, 1, 4, 6, 8));
        assertEquals(List.of(1, 3, 4, 5, 6, 7, 8), s.toList());
        assertEquals(s.toList(), s.toSortedArray());
    }

    // =========================================================================
    // 8. Set algebra
    // =========================================================================

    @Test
    @DisplayName("union combines all elements from both sets")
    void union() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 3, 5));
        TreeSet<Integer> b = new TreeSet<>(List.of(2, 3, 4));
        assertEquals(List.of(1, 2, 3, 4, 5), a.union(b).toList());
    }

    @Test
    @DisplayName("intersection keeps only shared elements")
    void intersection() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3, 4, 5));
        TreeSet<Integer> b = new TreeSet<>(List.of(3, 4, 5, 6, 7));
        assertEquals(List.of(3, 4, 5), a.intersection(b).toList());
    }

    @Test
    @DisplayName("intersection of disjoint sets is empty")
    void intersectionDisjoint() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> b = new TreeSet<>(List.of(4, 5, 6));
        assertTrue(a.intersection(b).isEmpty());
    }

    @Test
    @DisplayName("difference removes elements present in other")
    void difference() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3, 4, 5));
        TreeSet<Integer> b = new TreeSet<>(List.of(3, 4, 5, 6, 7));
        assertEquals(List.of(1, 2), a.difference(b).toList());
    }

    @Test
    @DisplayName("symmetricDifference keeps elements in exactly one set")
    void symmetricDifference() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3, 4));
        TreeSet<Integer> b = new TreeSet<>(List.of(3, 4, 5, 6));
        assertEquals(List.of(1, 2, 5, 6), a.symmetricDifference(b).toList());
    }

    @Test
    @DisplayName("Set algebra operations do not mutate operands")
    void algebraDoesNotMutate() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> b = new TreeSet<>(List.of(2, 3, 4));
        a.union(b);
        a.intersection(b);
        a.difference(b);
        a.symmetricDifference(b);
        assertEquals(3, a.size());
        assertEquals(3, b.size());
    }

    // =========================================================================
    // 9. Predicates
    // =========================================================================

    @Test
    @DisplayName("isSubset returns true when all elements are in other")
    void isSubset() {
        TreeSet<Integer> sub = new TreeSet<>(List.of(2, 3));
        TreeSet<Integer> sup = new TreeSet<>(List.of(1, 2, 3, 4));
        assertTrue(sub.isSubset(sup));
        assertFalse(sup.isSubset(sub));
    }

    @Test
    @DisplayName("isSuperset is the reverse of isSubset")
    void isSuperset() {
        TreeSet<Integer> small = new TreeSet<>(List.of(2, 3));
        TreeSet<Integer> large = new TreeSet<>(List.of(1, 2, 3, 4));
        assertTrue(large.isSuperset(small));
        assertFalse(small.isSuperset(large));
    }

    @Test
    @DisplayName("empty set is subset of every set")
    void emptyIsSubset() {
        TreeSet<Integer> empty = new TreeSet<>();
        TreeSet<Integer> some  = new TreeSet<>(List.of(1, 2));
        assertTrue(empty.isSubset(some));
        assertFalse(some.isSubset(empty));
    }

    @Test
    @DisplayName("isDisjoint returns true for non-overlapping sets")
    void isDisjoint() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> b = new TreeSet<>(List.of(4, 5, 6));
        assertTrue(a.isDisjoint(b));
    }

    @Test
    @DisplayName("isDisjoint returns false for overlapping sets")
    void isDisjointOverlap() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> b = new TreeSet<>(List.of(3, 4, 5));
        assertFalse(a.isDisjoint(b));
    }

    // =========================================================================
    // 10. equals / hashCode
    // =========================================================================

    @Test
    @DisplayName("Two TreeSets with same elements are equal")
    void equalsBasic() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> b = new TreeSet<>(List.of(3, 2, 1));
        assertEquals(a, b);
        assertEquals(a.hashCode(), b.hashCode());
    }

    @Test
    @DisplayName("TreeSets with different elements are not equal")
    void notEqual() {
        TreeSet<Integer> a = new TreeSet<>(List.of(1, 2, 3));
        TreeSet<Integer> b = new TreeSet<>(List.of(1, 2, 4));
        assertNotEquals(a, b);
    }

    // =========================================================================
    // 11. Iteration order
    // =========================================================================

    @Test
    @DisplayName("Iterator yields elements in ascending order")
    void iterationOrder() {
        TreeSet<Integer> s = new TreeSet<>(List.of(5, 2, 8, 1, 9, 3));
        java.util.List<Integer> actual = new java.util.ArrayList<>();
        for (int v : s) actual.add(v);
        assertEquals(List.of(1, 2, 3, 5, 8, 9), actual);
    }

    // =========================================================================
    // 12. Stress test
    // =========================================================================

    @Test
    @DisplayName("Stress: 1000 operations with reference java.util.TreeSet")
    void stressTest() {
        TreeSet<Integer> our = new TreeSet<>();
        java.util.TreeSet<Integer> ref = new java.util.TreeSet<>();
        java.util.Random rng = new java.util.Random(314);

        // Phase 1: 500 inserts
        for (int i = 0; i < 500; i++) {
            int k = rng.nextInt(300);
            our.add(k);
            ref.add(k);
        }
        assertEquals(ref.size(), our.size());
        assertEquals(new java.util.ArrayList<>(ref), our.toList());

        // Phase 2: 200 deletes
        java.util.List<Integer> keys = new java.util.ArrayList<>(ref).subList(0, 200);
        for (int k : keys) { our.remove(k); ref.remove(k); }
        assertEquals(ref.size(), our.size());

        // Phase 3: 300 mixed ops
        for (int i = 0; i < 300; i++) {
            int k = rng.nextInt(600);
            if (rng.nextBoolean()) { our.add(k); ref.add(k); }
            else if (ref.contains(k)) { our.remove(k); ref.remove(k); }
        }
        assertEquals(ref.size(), our.size());
        assertEquals(new java.util.ArrayList<>(ref), our.toList());

        // Phase 4: min/max match
        assertEquals(ref.first(), our.min());
        assertEquals(ref.last(), our.max());
    }
}
