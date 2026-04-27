// ============================================================================
// SegmentTree.kt — Generic Segment Tree for Range Queries + Point Updates
// ============================================================================
//
// Idiomatic Kotlin port of the Java segment-tree package (DT05).
//
// A segment tree is a binary tree where every node stores an aggregate over a
// contiguous sub-range of an array, enabling both range queries and point updates
// in O(log n) — versus O(n) per query or O(n) per update for naive approaches.
//
// ─────────────────────────────────────────────────────────────────────────────
// Kotlin Idioms vs Java
// ─────────────────────────────────────────────────────────────────────────────
//
//   • Generic class with reified inline factory functions
//   • Lambda parameters (T, T) -> T instead of BinaryOperator<T>
//   • Companion object with factory functions sumTree / minTree / maxTree / gcdTree
//   • `val` properties (size, isEmpty)
//   • Kotlin's Int.MAX_VALUE / Int.MIN_VALUE instead of Integer constants
//   • Internal inline helpers using reified generics where appropriate
//
// ─────────────────────────────────────────────────────────────────────────────
// The Combine Function Must Be a Monoid
// ─────────────────────────────────────────────────────────────────────────────
//
//   A monoid has:
//     1. An associative binary operation: combine(a, combine(b,c)) == combine(combine(a,b), c)
//     2. An identity element e: combine(e, x) == x for all x
//
//   Common monoids for segment trees:
//
//   | combine         | identity       | query answers  |
//   |-----------------|----------------|----------------|
//   | a + b           | 0              | range sum      |
//   | minOf(a, b)     | Int.MAX_VALUE  | range minimum  |
//   | maxOf(a, b)     | Int.MIN_VALUE  | range maximum  |
//   | gcd(a, b)       | 0              | range GCD      |
//   | a * b           | 1              | range product  |
//   | a and b         | -1 (all 1s)   | range AND      |
//   | a or b          | 0              | range OR       |
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.segmenttree
// ============================================================================

package com.codingadventures.segmenttree

import kotlin.math.max
import kotlin.math.min

/**
 * A generic segment tree supporting range queries and point updates in O(log n).
 *
 * All nodes are stored in a flat array (1-indexed): root at index 1,
 * left child at `2*i`, right child at `2*i+1`.
 *
 * Example — range sum:
 * ```kotlin
 * val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))
 * st.query(1, 3)   // → 9  (1 + 5 + 3)
 * st.update(2, 7)  // arr[2] is now 7
 * st.query(1, 3)   // → 11 (1 + 7 + 3)
 * ```
 *
 * Example — range minimum:
 * ```kotlin
 * val rm = SegmentTree.minTree(intArrayOf(5, 3, 7, 1, 9))
 * rm.query(0, 3)   // → 1
 * ```
 *
 * @param T  the element type
 * @param array    source array (any length ≥ 0)
 * @param combine  associative binary operator (must be a monoid with [identity])
 * @param identity neutral element: `combine(identity, x) == x` for all x
 */
class SegmentTree<T>(
    array: Array<T>,
    private val combine: (T, T) -> T,
    private val identity: T
) {

    // ─────────────────────────────────────────────────────────────────────────
    // Storage
    // ─────────────────────────────────────────────────────────────────────────

    /** Length of the original array. */
    val size: Int = array.size

    /** True if the underlying array is empty. */
    val isEmpty: Boolean get() = size == 0

    /**
     * 1-indexed backing array.
     *
     * tree[1] is the root. tree[0] is unused.
     * Allocated as 4*n to cover all non-power-of-2 input lengths without
     * requiring exact size computation.
     */
    @Suppress("UNCHECKED_CAST")
    private val tree: Array<Any?> = arrayOfNulls<Any>(maxOf(4, 4 * size)).also { t ->
        t.fill(identity)
    }

    // Build the tree during construction.
    init {
        if (size > 0) build(array, 1, 0, size - 1)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Build
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Recursively fill tree[] bottom-up:
    //   Leaf  (left == right): tree[node] = array[left]
    //   Internal:              tree[node] = combine(tree[2*node], tree[2*node+1])

    private fun build(array: Array<T>, node: Int, left: Int, right: Int) {
        if (left == right) {
            tree[node] = array[left]
            return
        }
        val mid = (left + right) / 2
        build(array, 2 * node,     left,    mid)
        build(array, 2 * node + 1, mid + 1, right)
        tree[node] = combine(treeAt(2 * node), treeAt(2 * node + 1))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Range Query
    // ─────────────────────────────────────────────────────────────────────────
    //
    // query(ql, qr) = combine of all array[i] for ql ≤ i ≤ qr.
    //
    // Three cases:
    //   No overlap    (right < ql || left > qr) → return identity
    //   Total overlap (ql <= left && right <= qr) → return tree[node]
    //   Partial       → recurse on both children, combine results

    /**
     * Return the aggregate over `array[ql..qr]` (inclusive, 0-indexed).
     *
     * Time: O(log n).
     *
     * @throws IllegalArgumentException if bounds are invalid
     */
    fun query(ql: Int, qr: Int): T {
        require(ql >= 0 && qr < size && ql <= qr) {
            "Invalid query range [$ql, $qr] for array of size $size"
        }
        return queryHelper(1, 0, size - 1, ql, qr)
    }

    private fun queryHelper(node: Int, left: Int, right: Int, ql: Int, qr: Int): T {
        // Case 1: no overlap
        if (right < ql || left > qr) return identity
        // Case 2: total overlap
        if (ql <= left && right <= qr) return treeAt(node)
        // Case 3: partial overlap
        val mid = (left + right) / 2
        val leftResult  = queryHelper(2 * node,     left,    mid,   ql, qr)
        val rightResult = queryHelper(2 * node + 1, mid + 1, right, ql, qr)
        return combine(leftResult, rightResult)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Point Update
    // ─────────────────────────────────────────────────────────────────────────
    //
    // update(index, value) traces the root-to-leaf path, installs the new value
    // at the leaf, and recomputes all ancestors on the way back up.
    // Time: O(log n).

    /**
     * Set `array[index] = value` and update all ancestor nodes.
     *
     * Time: O(log n).
     *
     * @throws IllegalArgumentException if index is out of range
     */
    fun update(index: Int, value: T) {
        require(index in 0 until size) {
            "Index $index out of range for array of size $size"
        }
        updateHelper(1, 0, size - 1, index, value)
    }

    private fun updateHelper(node: Int, left: Int, right: Int, index: Int, value: T) {
        if (left == right) {
            tree[node] = value
            return
        }
        val mid = (left + right) / 2
        if (index <= mid) {
            updateHelper(2 * node,     left,    mid,   index, value)
        } else {
            updateHelper(2 * node + 1, mid + 1, right, index, value)
        }
        tree[node] = combine(treeAt(2 * node), treeAt(2 * node + 1))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Reconstruct Array
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Reconstruct the current array from the leaf nodes.
     * Reflects all point updates made since construction.
     * Time: O(n).
     */
    fun toList(): List<T> {
        val result = mutableListOf<T>()
        if (size > 0) collectLeaves(1, 0, size - 1, result)
        return result
    }

    private fun collectLeaves(node: Int, left: Int, right: Int, acc: MutableList<T>) {
        if (left == right) { acc.add(treeAt(node)); return }
        val mid = (left + right) / 2
        collectLeaves(2 * node,     left,    mid,   acc)
        collectLeaves(2 * node + 1, mid + 1, right, acc)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Internal Helper
    // ─────────────────────────────────────────────────────────────────────────

    @Suppress("UNCHECKED_CAST")
    private fun treeAt(i: Int): T = tree[i] as T

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    override fun toString(): String = "SegmentTree(size=$size, identity=$identity)"

    // ─────────────────────────────────────────────────────────────────────────
    // Companion Object — Factory Functions
    // ─────────────────────────────────────────────────────────────────────────

    companion object {

        /**
         * Build a range-sum segment tree.
         *
         * ```kotlin
         * val st = SegmentTree.sumTree(intArrayOf(2, 1, 5, 3, 4))
         * st.query(1, 3)   // → 9
         * ```
         */
        fun sumTree(array: IntArray): SegmentTree<Int> =
            SegmentTree(array.toTypedArray(), Int::plus, 0)

        /**
         * Build a range-minimum segment tree.
         *
         * ```kotlin
         * val st = SegmentTree.minTree(intArrayOf(5, 3, 7, 1, 9))
         * st.query(0, 4)   // → 1
         * ```
         */
        fun minTree(array: IntArray): SegmentTree<Int> =
            SegmentTree(array.toTypedArray(), ::min, Int.MAX_VALUE)

        /**
         * Build a range-maximum segment tree.
         */
        fun maxTree(array: IntArray): SegmentTree<Int> =
            SegmentTree(array.toTypedArray(), ::max, Int.MIN_VALUE)

        /**
         * Build a range-GCD segment tree.
         *
         * Identity for GCD is 0 since `gcd(0, x) = x` for all x ≥ 0.
         *
         * ```kotlin
         * val st = SegmentTree.gcdTree(intArrayOf(12, 8, 6, 4, 9))
         * st.query(0, 2)   // → 2  (gcd(12, gcd(8, 6)))
         * ```
         */
        fun gcdTree(array: IntArray): SegmentTree<Int> =
            SegmentTree(array.toTypedArray(), ::gcd, 0)

        // ─── Private helpers ─────────────────────────────────────────────────

        private fun gcd(a: Int, b: Int): Int {
            var x = a; var y = b
            while (y != 0) { val t = y; y = x % y; x = t }
            return x
        }
    }
}
