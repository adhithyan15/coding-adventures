// ============================================================================
// FenwickTree.kt — Binary Indexed Tree (Fenwick Tree)
// ============================================================================
//
// A Fenwick tree (invented by Peter Fenwick, 1994) solves one problem with
// extraordinary elegance: prefix sums with point updates, both in O(log n).
//
// The entire algorithm rests on a single bit trick:
//
//   lowbit(i) = i and (-i)
//
// This extracts the LOWEST SET BIT of i. In two's complement, -i is the
// bitwise NOT of i plus 1, which flips all bits up to and including the
// lowest set bit:
//
//   i  = 0b00001100  (12)
//   -i = 0b11110100
//   i and (-i) = 0b00000100 = 4   ← the lowest set bit of 12
//
// What each cell stores:
// ----------------------
// The BIT array is 1-indexed. Cell bit[i] stores the sum of lowbit(i)
// consecutive elements of the original array, ENDING at position i.
//
//   bit[i] = sum of arr[i - lowbit(i) + 1 .. i]
//
// Prefix sum query: walk downward
// --------------------------------
//   prefixSum(7):
//     i=7 (111): add bit[7] → i=6
//     i=6 (110): add bit[6] → i=4
//     i=4 (100): add bit[4] → i=0
//     Total = arr[1]+...+arr[7]  ✓
//
// Point update: walk upward
// -------------------------
//   update(3, delta):
//     i=3: bit[3] += delta → i=4
//     i=4: bit[4] += delta → i=8
//     ...
//
// Range query [l, r]:
// -------------------
//   rangeSum(l, r) = prefixSum(r) - prefixSum(l - 1)
//

package com.codingadventures.fenwicktree

/**
 * A Fenwick Tree (Binary Indexed Tree) for prefix sums and point updates over
 * a 1-indexed array of [Long] values.
 *
 * Both [update] and [prefixSum] run in O(log n) time using the lowbit bit trick:
 * `lowbit(i) = i and (-i)`.
 *
 * ```kotlin
 * // Create a tree for [3, 2, -1, 6, 5]:
 * val t = FenwickTree(longArrayOf(3, 2, -1, 6, 5))
 * t.prefixSum(3)   // → 4  (3 + 2 + -1)
 * t.rangeSum(2, 4) // → 7  (2 + -1 + 6)
 * t.update(3, 10)  // arr[3] += 10 → arr[3] = 9
 * t.rangeSum(2, 4) // → 17
 * ```
 *
 * @property capacity the number of elements (maximum 1-based index)
 */
class FenwickTree {

    private val bit: LongArray  // 1-indexed; bit[0] unused
    val capacity: Int

    // =========================================================================
    // Constructors
    // =========================================================================

    /**
     * Construct an empty Fenwick tree of capacity [n].
     *
     * All elements are initialised to zero. Valid 1-based indices are `1..n`.
     *
     * @throws IllegalArgumentException if n < 1
     */
    constructor(n: Int) {
        require(n >= 1) { "n must be >= 1, got: $n" }
        capacity = n
        bit = LongArray(n + 1)
    }

    /**
     * Construct a Fenwick tree initialised from an existing [LongArray].
     *
     * The array is treated as 0-indexed (element 0 maps to position 1 in
     * the BIT). Construction runs in O(n) time.
     *
     * @throws IllegalArgumentException if values is empty
     */
    constructor(values: LongArray) {
        require(values.isNotEmpty()) { "values must not be empty" }
        capacity = values.size
        bit = LongArray(capacity + 1)
        // O(n) construction: copy, then propagate each cell to its parent.
        for (i in values.indices) bit[i + 1] = values[i]
        for (i in 1..capacity) {
            val parent = i + (i and -i)
            if (parent <= capacity) bit[parent] += bit[i]
        }
    }

    // =========================================================================
    // Core operations
    // =========================================================================

    /**
     * Add [delta] to the element at 1-based position [i].
     *
     * Walks upward through the BIT updating all cells that cover position [i].
     *
     * @throws IllegalArgumentException if i is outside [1, capacity]
     */
    fun update(i: Int, delta: Long) {
        checkIndex(i)
        var idx = i
        while (idx <= capacity) {
            bit[idx] += delta
            idx += idx and -idx
        }
    }

    /**
     * Return the prefix sum of elements `1..i` (1-based).
     *
     * Walks downward through the BIT stripping the lowest set bit.
     *
     * @throws IllegalArgumentException if i is outside [1, capacity]
     */
    fun prefixSum(i: Int): Long {
        checkIndex(i)
        var idx = i
        var sum = 0L
        while (idx > 0) {
            sum += bit[idx]
            idx -= idx and -idx
        }
        return sum
    }

    /**
     * Return the sum of elements in the closed range `[l, r]` (1-based).
     *
     * Computed as `prefixSum(r) - prefixSum(l - 1)`.
     *
     * @throws IllegalArgumentException if l or r are out of range, or l > r
     */
    fun rangeSum(l: Int, r: Int): Long {
        require(l <= r) { "l ($l) must be <= r ($r)" }
        checkIndex(l)
        checkIndex(r)
        return if (l == 1) prefixSum(r) else prefixSum(r) - prefixSum(l - 1)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun checkIndex(i: Int) {
        require(i in 1..capacity) { "Index $i out of range [1, $capacity]" }
    }
}
