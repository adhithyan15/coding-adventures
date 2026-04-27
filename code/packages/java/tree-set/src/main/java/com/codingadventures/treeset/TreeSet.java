// ============================================================================
// TreeSet.java — Sorted Set with O(log n) Operations and Set Algebra
// ============================================================================
//
// A TreeSet is a sorted set: all elements are kept in their natural order
// (or a custom comparator order), and all operations run in O(log n) time.
//
// This implementation wraps Java's java.util.TreeMap to provide the O(log n)
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

package com.codingadventures.treeset;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.Iterator;
import java.util.List;
import java.util.NavigableMap;
import java.util.NoSuchElementException;

/**
 * A sorted set backed by a red-black tree (via {@link java.util.TreeMap}).
 *
 * <p>Elements are stored in their natural {@link Comparable} order. All
 * add, remove, contains, predecessor, and successor operations run in
 * O(log n). Set algebra operations (union, intersection, etc.) run in O(n).
 *
 * <pre>{@code
 * TreeSet<Integer> s = new TreeSet<>();
 * s.add(5); s.add(3); s.add(7); s.add(1);
 *
 * s.contains(3);          // true
 * s.min();                // 1
 * s.max();                // 7
 * s.predecessor(5);       // 3
 * s.successor(5);         // 7
 * s.rank(5);              // 2  (0-based: two elements are smaller)
 * s.kthSmallest(1);       // 1
 * s.range(3, 6);          // [3, 5]
 *
 * TreeSet<Integer> t = new TreeSet<>(List.of(4, 5, 6, 7));
 * s.union(t);             // [1, 3, 4, 5, 6, 7]
 * s.intersection(t);      // [5, 7]
 * s.difference(t);        // [1, 3]
 * }</pre>
 *
 * @param <T> the element type; must be {@link Comparable}
 */
public class TreeSet<T extends Comparable<T>> implements Iterable<T> {

    // =========================================================================
    // Fields
    // =========================================================================

    /** The backing NavigableMap (red-black tree). Dummy value is Boolean.TRUE. */
    private final java.util.TreeMap<T, Boolean> map;

    // =========================================================================
    // Constructors
    // =========================================================================

    /** Construct an empty TreeSet using natural ordering. */
    public TreeSet() {
        this.map = new java.util.TreeMap<>();
    }

    /** Construct a TreeSet pre-populated with the elements of {@code values}. */
    public TreeSet(Iterable<? extends T> values) {
        this.map = new java.util.TreeMap<>();
        for (T v : values) map.put(v, Boolean.TRUE);
    }

    /** Copy constructor — creates an independent copy. */
    public TreeSet(TreeSet<T> other) {
        this.map = new java.util.TreeMap<>(other.map);
    }

    // =========================================================================
    // Mutation
    // =========================================================================

    /**
     * Add {@code value} to the set.
     *
     * @param value the value to add; must not be null
     * @return {@code this} for chaining
     */
    public TreeSet<T> add(T value) {
        if (value == null) throw new IllegalArgumentException("Value must not be null");
        map.put(value, Boolean.TRUE);
        return this;
    }

    /**
     * Remove {@code value} from the set.
     *
     * @param value the value to remove
     * @return {@code true} if the value was present and removed
     */
    public boolean remove(T value) {
        if (value == null) return false;
        return map.remove(value) != null;
    }

    /** Alias for {@link #remove(T)} for API parity with the Python version. */
    public boolean delete(T value) {
        return remove(value);
    }

    /** Alias for {@link #remove(T)} — never throws even if absent. */
    public boolean discard(T value) {
        return remove(value);
    }

    // =========================================================================
    // Query
    // =========================================================================

    /**
     * Return {@code true} if {@code value} is in the set.
     *
     * @param value the value to test
     */
    public boolean contains(T value) {
        if (value == null) return false;
        return map.containsKey(value);
    }

    /** Alias for {@link #contains}. */
    public boolean has(T value) {
        return contains(value);
    }

    /** Return the number of elements in the set. */
    public int size() {
        return map.size();
    }

    /** Return {@code true} if the set is empty. */
    public boolean isEmpty() {
        return map.isEmpty();
    }

    // =========================================================================
    // Min / max / first / last
    // =========================================================================

    /**
     * Return the smallest element, or {@code null} if empty.
     */
    public T min() {
        return map.isEmpty() ? null : map.firstKey();
    }

    /**
     * Return the largest element, or {@code null} if empty.
     */
    public T max() {
        return map.isEmpty() ? null : map.lastKey();
    }

    /** Alias for {@link #min()}. */
    public T first() { return min(); }

    /** Alias for {@link #max()}. */
    public T last()  { return max(); }

    // =========================================================================
    // Predecessor / successor
    // =========================================================================

    /**
     * Return the largest element strictly less than {@code value}, or
     * {@code null} if none exists.
     *
     * <p>Runs in O(log n) via {@link NavigableMap#lowerKey}.
     */
    public T predecessor(T value) {
        if (value == null) return null;
        return map.lowerKey(value);
    }

    /**
     * Return the smallest element strictly greater than {@code value}, or
     * {@code null} if none exists.
     *
     * <p>Runs in O(log n) via {@link NavigableMap#higherKey}.
     */
    public T successor(T value) {
        if (value == null) return null;
        return map.higherKey(value);
    }

    // =========================================================================
    // Order statistics
    // =========================================================================

    /**
     * Return the 0-based rank of {@code value}: the number of elements
     * strictly less than {@code value} that are in the set.
     *
     * <p>If {@code value} is not in the set, this is the position it would
     * occupy if inserted.
     *
     * <p>Runs in O(n) via a headMap count. (For O(log n) rank, an
     * augmented tree such as {@link com.codingadventures.avltree.AVLTree}
     * would be needed.)
     *
     * @param value the reference value
     */
    public int rank(T value) {
        if (value == null) return 0;
        return map.headMap(value).size();
    }

    /**
     * Return the k-th smallest element (0-based).
     *
     * <p>{@code byRank(0)} returns the minimum; {@code byRank(size()-1)}
     * returns the maximum.
     *
     * <p>Runs in O(k) by iterating the underlying tree.
     *
     * @param rank the 0-based position
     * @return the element at that position, or {@code null} if out of range
     */
    public T byRank(int rank) {
        if (rank < 0 || rank >= map.size()) return null;
        // Walk the underlying map's entry set (tree order)
        int i = 0;
        for (T key : map.keySet()) {
            if (i == rank) return key;
            i++;
        }
        return null;
    }

    /**
     * Return the k-th smallest element (1-based).
     *
     * <p>{@code kthSmallest(1)} returns the minimum.
     *
     * @param k the 1-based rank
     * @return the k-th smallest element, or {@code null} if out of range
     */
    public T kthSmallest(int k) {
        if (k <= 0) return null;
        return byRank(k - 1);
    }

    // =========================================================================
    // Range query
    // =========================================================================

    /**
     * Return all elements where {@code low <= element <= high}, in ascending
     * order.
     *
     * @param low       the inclusive lower bound
     * @param high      the inclusive upper bound
     * @param inclusive if {@code true}, both bounds are inclusive; otherwise
     *                  both are exclusive
     */
    public List<T> range(T low, T high, boolean inclusive) {
        if (low.compareTo(high) > 0) return List.of();
        NavigableMap<T, Boolean> sub;
        if (inclusive) {
            sub = map.subMap(low, true, high, true);
        } else {
            sub = map.subMap(low, false, high, false);
        }
        return new ArrayList<>(sub.keySet());
    }

    /**
     * Return all elements where {@code low <= element <= high} (inclusive).
     *
     * @param low  the inclusive lower bound
     * @param high the inclusive upper bound
     */
    public List<T> range(T low, T high) {
        return range(low, high, true);
    }

    // =========================================================================
    // Conversion
    // =========================================================================

    /** Return all elements as a sorted list. */
    public List<T> toList() {
        return new ArrayList<>(map.keySet());
    }

    /** Alias for {@link #toList()}. */
    public List<T> toSortedArray() {
        return toList();
    }

    // =========================================================================
    // Set algebra
    // =========================================================================

    /**
     * Return a new TreeSet containing all elements from this set OR {@code other}.
     *
     * <p>Runs in O(n + m).
     */
    public TreeSet<T> union(TreeSet<T> other) {
        TreeSet<T> result = new TreeSet<>(this);
        for (T v : other) result.add(v);
        return result;
    }

    /**
     * Return a new TreeSet containing only elements present in BOTH this set
     * AND {@code other}.
     *
     * <p>Runs in O(min(n, m) · log(max(n, m))).
     */
    public TreeSet<T> intersection(TreeSet<T> other) {
        TreeSet<T> result = new TreeSet<>();
        for (T v : this) {
            if (other.contains(v)) result.add(v);
        }
        return result;
    }

    /**
     * Return a new TreeSet containing elements in this set but NOT in {@code other}.
     *
     * <p>Runs in O(n · log m).
     */
    public TreeSet<T> difference(TreeSet<T> other) {
        TreeSet<T> result = new TreeSet<>();
        for (T v : this) {
            if (!other.contains(v)) result.add(v);
        }
        return result;
    }

    /**
     * Return a new TreeSet containing elements in exactly one of the two sets
     * (XOR / symmetric difference).
     *
     * <p>Runs in O((n + m) · log(n + m)).
     */
    public TreeSet<T> symmetricDifference(TreeSet<T> other) {
        TreeSet<T> result = new TreeSet<>();
        for (T v : this)  { if (!other.contains(v)) result.add(v); }
        for (T v : other) { if (!this.contains(v))  result.add(v); }
        return result;
    }

    // =========================================================================
    // Predicates
    // =========================================================================

    /**
     * Return {@code true} if every element of this set is also in {@code other}.
     *
     * <p>Runs in O(n · log m).
     */
    public boolean isSubset(TreeSet<T> other) {
        for (T v : this) {
            if (!other.contains(v)) return false;
        }
        return true;
    }

    /**
     * Return {@code true} if every element of {@code other} is also in this set.
     *
     * <p>Runs in O(m · log n).
     */
    public boolean isSuperset(TreeSet<T> other) {
        return other.isSubset(this);
    }

    /**
     * Return {@code true} if this set and {@code other} share no elements.
     *
     * <p>Runs in O(min(n, m) · log(max(n, m))).
     */
    public boolean isDisjoint(TreeSet<T> other) {
        // Iterate the smaller set, check against the larger
        TreeSet<T> small = size() <= other.size() ? this : other;
        TreeSet<T> large = size() <= other.size() ? other : this;
        for (T v : small) {
            if (large.contains(v)) return false;
        }
        return true;
    }

    /**
     * Return {@code true} if this set contains exactly the same elements as
     * {@code other}.
     */
    public boolean equals(TreeSet<T> other) {
        return map.keySet().equals(other.map.keySet());
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof TreeSet)) return false;
        @SuppressWarnings("unchecked")
        TreeSet<T> other = (TreeSet<T>) o;
        return map.keySet().equals(other.map.keySet());
    }

    @Override
    public int hashCode() {
        return map.keySet().hashCode();
    }

    // =========================================================================
    // Iterable / Iterator
    // =========================================================================

    /**
     * Return an iterator over elements in ascending sorted order.
     */
    @Override
    public Iterator<T> iterator() {
        return map.keySet().iterator();
    }

    // =========================================================================
    // Object
    // =========================================================================

    @Override
    public String toString() {
        return "TreeSet(" + map.keySet() + ")";
    }
}
