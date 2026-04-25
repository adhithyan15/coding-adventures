// ============================================================================
// TreeSet.kt — Sorted Set with O(log n) Operations and Set Algebra
// ============================================================================
//
// A TreeSet is a sorted set: all elements are kept in their natural order
// (or a custom comparator order), and all operations run in O(log n) time.
//
// This implementation wraps java.util.TreeMap to provide the O(log n)
// guarantees, and adds:
//
//   - Order statistics: rank, kthSmallest, predecessor, successor
//   - Range queries: range(low, high)
//   - Set algebra: union, intersection, difference, symmetricDifference
//   - Predicates: isSubset, isSuperset, isDisjoint
//
// ============================================================================
// How a TreeSet differs from a HashSet
// ============================================================================
//
//   HashSet:
//     - O(1) average for add/contains/remove
//     - No ordering — iteration order is unpredictable
//     - Cannot do range queries, predecessor/successor, or order statistics
//
//   TreeSet:
//     - O(log n) worst-case for add/contains/remove
//     - Elements are always in sorted order
//     - Full support for range queries, predecessor/successor, rank
//
// ============================================================================
// Under the hood: Red-Black tree
// ============================================================================
//
//   java.util.TreeMap is backed by a red-black tree — a self-balancing BST
//   that keeps height ≤ 2·log₂(n+1). All O(log n) guarantees derive from
//   this. We use TreeMap<T, Boolean> as the backing store, storing `true`
//   as a dummy value for every key.
//
// ============================================================================

package com.codingadventures.treeset

/**
 * A sorted set backed by a red-black tree (via [java.util.TreeMap]).
 *
 * Elements are stored in their natural [Comparable] order. All add, remove,
 * contains, predecessor, and successor operations run in O(log n). Set algebra
 * operations (union, intersection, etc.) run in O(n).
 *
 * ```kotlin
 * val s = TreeSet<Int>()
 * s.add(5); s.add(3); s.add(7); s.add(1)
 *
 * s.contains(3)          // true
 * s.min()                // 1
 * s.max()                // 7
 * s.predecessor(5)       // 3
 * s.successor(5)         // 7
 * s.rank(5)              // 2  (0-based: two elements are smaller)
 * s.kthSmallest(1)       // 1
 * s.range(3, 6)          // [3, 5]
 *
 * val t = TreeSet<Int>(listOf(4, 5, 6, 7))
 * s.union(t)             // [1, 3, 4, 5, 6, 7]
 * s.intersection(t)      // [5, 7]
 * s.difference(t)        // [1, 3]
 * ```
 *
 * @param T the element type; must be [Comparable]
 */
class TreeSet<T : Comparable<T>>(values: Iterable<T> = emptyList()) : Iterable<T> {

    // =========================================================================
    // Fields
    // =========================================================================

    /** The backing NavigableMap (red-black tree). Dummy value is Boolean.TRUE. */
    private val map: java.util.TreeMap<T, Boolean> = java.util.TreeMap()

    init {
        for (v in values) map[v] = true
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /** Number of elements in the set. */
    val size: Int get() = map.size

    /** True when the set contains no elements. */
    val isEmpty: Boolean get() = map.isEmpty()

    // =========================================================================
    // Mutation
    // =========================================================================

    /**
     * Add [value] to the set.
     *
     * @return this for chaining
     */
    fun add(value: T): TreeSet<T> {
        map[value] = true
        return this
    }

    /**
     * Remove [value] from the set.
     *
     * @return true if the value was present and removed
     */
    fun remove(value: T): Boolean = map.remove(value) != null

    /** Alias for [remove]. */
    fun delete(value: T): Boolean = remove(value)

    /** Alias for [remove] — never throws even if absent. */
    fun discard(value: T): Boolean = remove(value)

    // =========================================================================
    // Query
    // =========================================================================

    /** Return true if [value] is in the set. */
    fun contains(value: T): Boolean = map.containsKey(value)

    /** Alias for [contains]. */
    fun has(value: T): Boolean = contains(value)

    /** Return true if the set is empty. */
    fun isNotEmpty(): Boolean = !isEmpty

    // =========================================================================
    // Min / max / first / last
    // =========================================================================

    /** Return the smallest element, or null if empty. */
    fun min(): T? = if (map.isEmpty()) null else map.firstKey()

    /** Return the largest element, or null if empty. */
    fun max(): T? = if (map.isEmpty()) null else map.lastKey()

    /** Alias for [min]. */
    fun first(): T? = min()

    /** Alias for [max]. */
    fun last(): T? = max()

    // =========================================================================
    // Predecessor / successor
    // =========================================================================

    /**
     * Return the largest element strictly less than [value], or null if none
     * exists. Runs in O(log n) via [java.util.TreeMap.lowerKey].
     */
    fun predecessor(value: T): T? = map.lowerKey(value)

    /**
     * Return the smallest element strictly greater than [value], or null if
     * none exists. Runs in O(log n) via [java.util.TreeMap.higherKey].
     */
    fun successor(value: T): T? = map.higherKey(value)

    // =========================================================================
    // Order statistics
    // =========================================================================

    /**
     * Return the 0-based rank of [value]: the number of elements strictly
     * less than [value] in the set.
     *
     * If [value] is not in the set, this is the insertion position.
     * Runs in O(n) via a headMap count.
     */
    fun rank(value: T): Int = map.headMap(value).size

    /**
     * Return the element at 0-based position [rank], or null if out of range.
     * Runs in O(rank) by iterating the tree in order.
     */
    fun byRank(rank: Int): T? {
        if (rank < 0 || rank >= map.size) return null
        var i = 0
        for (key in map.keys) {
            if (i == rank) return key
            i++
        }
        return null
    }

    /**
     * Return the k-th smallest element (1-based), or null if out of range.
     * [kthSmallest](1) returns the minimum.
     */
    fun kthSmallest(k: Int): T? = if (k <= 0) null else byRank(k - 1)

    // =========================================================================
    // Range query
    // =========================================================================

    /**
     * Return all elements where [low] <= element <= [high], in ascending order.
     *
     * @param inclusive if true (default), both bounds are inclusive;
     *                  if false, both are exclusive
     */
    fun range(low: T, high: T, inclusive: Boolean = true): List<T> {
        if (low.compareTo(high) > 0) return emptyList()
        val sub = if (inclusive) map.subMap(low, true, high, true)
                  else           map.subMap(low, false, high, false)
        return sub.keys.toList()
    }

    // =========================================================================
    // Conversion
    // =========================================================================

    /** Return all elements as a sorted list. */
    fun toList(): List<T> = map.keys.toList()

    /** Alias for [toList]. */
    fun toSortedArray(): List<T> = toList()

    /** Alias for [toList]. */
    fun toArray(): List<T> = toList()

    // =========================================================================
    // Set algebra
    // =========================================================================

    /**
     * Return a new TreeSet containing all elements from this set OR [other].
     * Runs in O(n + m).
     */
    fun union(other: TreeSet<T>): TreeSet<T> {
        val result = TreeSet(this)
        for (v in other) result.add(v)
        return result
    }

    /**
     * Return a new TreeSet containing only elements present in BOTH this set
     * AND [other]. Runs in O(min(n,m) · log(max(n,m))).
     */
    fun intersection(other: TreeSet<T>): TreeSet<T> {
        val result = TreeSet<T>()
        for (v in this) if (other.contains(v)) result.add(v)
        return result
    }

    /**
     * Return a new TreeSet containing elements in this set but NOT in [other].
     * Runs in O(n · log m).
     */
    fun difference(other: TreeSet<T>): TreeSet<T> {
        val result = TreeSet<T>()
        for (v in this) if (!other.contains(v)) result.add(v)
        return result
    }

    /**
     * Return a new TreeSet containing elements in exactly one of the two sets
     * (XOR / symmetric difference). Runs in O((n+m) · log(n+m)).
     */
    fun symmetricDifference(other: TreeSet<T>): TreeSet<T> {
        val result = TreeSet<T>()
        for (v in this)  if (!other.contains(v)) result.add(v)
        for (v in other) if (!this.contains(v))  result.add(v)
        return result
    }

    // =========================================================================
    // Predicates
    // =========================================================================

    /**
     * Return true if every element of this set is also in [other].
     * Runs in O(n · log m).
     */
    fun isSubset(other: TreeSet<T>): Boolean = all { other.contains(it) }

    /**
     * Return true if every element of [other] is also in this set.
     * Runs in O(m · log n).
     */
    fun isSuperset(other: TreeSet<T>): Boolean = other.isSubset(this)

    /**
     * Return true if this set and [other] share no elements.
     * Runs in O(min(n,m) · log(max(n,m))).
     */
    fun isDisjoint(other: TreeSet<T>): Boolean {
        val small = if (size <= other.size) this else other
        val large = if (size <= other.size) other else this
        return small.none { large.contains(it) }
    }

    /**
     * Return true if this set contains exactly the same elements as [other].
     */
    fun equals(other: TreeSet<T>): Boolean = map.keys == other.map.keys

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is TreeSet<*>) return false
        return map.keys == other.map.keys
    }

    override fun hashCode(): Int = map.keys.hashCode()

    // =========================================================================
    // Iterable
    // =========================================================================

    /** Return an iterator over elements in ascending sorted order. */
    override fun iterator(): Iterator<T> = map.keys.iterator()

    // =========================================================================
    // Object
    // =========================================================================

    override fun toString(): String = "TreeSet(${map.keys})"

    // =========================================================================
    // Companion
    // =========================================================================

    companion object {
        /** Create a TreeSet from the given values. */
        fun <T : Comparable<T>> of(vararg values: T): TreeSet<T> = TreeSet(values.toList())
    }
}
