// ============================================================================
// FenwickTree.java — Binary Indexed Tree (Fenwick Tree)
// ============================================================================
//
// A Fenwick tree (invented by Peter Fenwick, 1994) solves one problem with
// extraordinary elegance: prefix sums with point updates, both in O(log n).
//
// The entire algorithm rests on a single bit trick:
//
//   lowbit(i) = i & (-i)
//
// This extracts the LOWEST SET BIT of i. In two's complement, -i is the bitwise
// NOT of i plus 1, which flips all bits up to and including the lowest set bit:
//
//   i  = 0b00001100  (12)
//   -i = 0b11110100  (flip all bits, add 1)
//   i & (-i) = 0b00000100  = 4   ← the lowest set bit of 12
//
// More examples:
//   i=1  (0001): lowbit = 1
//   i=2  (0010): lowbit = 2
//   i=3  (0011): lowbit = 1
//   i=4  (0100): lowbit = 4
//   i=6  (0110): lowbit = 2
//   i=8  (1000): lowbit = 8
//
// What each cell stores:
// ----------------------
// The BIT array is 1-indexed. Cell bit[i] stores the sum of lowbit(i)
// consecutive elements of the original array, ENDING at position i.
//
//   bit[i] = sum of arr[i - lowbit(i) + 1 .. i]
//
// For n=8:
//   Index (1-based): 1    2    3    4    5    6    7    8
//   Binary:         001  010  011  100  101  110  111  1000
//   lowbit:          1    2    1    4    1    2    1    8
//   Range covered: [1]  [1,2] [3] [1,4] [5] [5,6] [7] [1,8]
//
// Prefix sum query: walk downward
// --------------------------------
// To get sum of arr[1..i], start at i, add bit[i], then jump down by
// stripping the lowest set bit: i -= lowbit(i). Repeat until i = 0.
//
//   prefixSum(7):
//     i=7 (111): add bit[7] (covers arr[7])  → i=6
//     i=6 (110): add bit[6] (covers arr[5,6]) → i=4
//     i=4 (100): add bit[4] (covers arr[1,2,3,4]) → i=0
//     Total = bit[7] + bit[6] + bit[4] = arr[1]+...+arr[7]  ✓
//
//   Steps ≤ number of set bits in i ≤ log₂(n).
//
// Point update: walk upward
// -------------------------
// To add delta to arr[i], update all BIT cells that cover position i.
// Those cells are at indices i, i+lowbit(i), i+lowbit(i+lowbit(i)), ...
//
//   update(3, delta):
//     i=3 (011): bit[3] += delta  → i=4
//     i=4 (100): bit[4] += delta  → i=8
//     i=8 (1000): bit[8] += delta → i=16 (> n, stop)
//
// Range query [l, r]:
// -------------------
// sum(l, r) = prefixSum(r) - prefixSum(l - 1)
//
// This is the standard trick for turning a prefix-sum structure into a
// range-sum structure: "how much of prefix [1..r] comes from [l..r] alone?"
//
// Complexity:
// -----------
//   update(i, delta)    — O(log n)
//   prefixSum(i)        — O(log n)
//   rangeSum(l, r)      — O(log n)
//   Construction        — O(n) via direct initialisation or O(n log n) via updates
//

package com.codingadventures.fenwicktree;

/**
 * A Fenwick Tree (Binary Indexed Tree) for prefix sums and point updates over
 * a 1-indexed array of {@code long} values.
 *
 * <p>Both {@link #update} and {@link #prefixSum} run in O(log n) time using
 * the lowbit bit trick: {@code lowbit(i) = i & (-i)}.
 *
 * <pre>{@code
 * // Create a tree for 5 elements [3, 2, -1, 6, 5]:
 * FenwickTree t = new FenwickTree(5);
 * t.update(1, 3);
 * t.update(2, 2);
 * t.update(3, -1);
 * t.update(4, 6);
 * t.update(5, 5);
 *
 * t.prefixSum(3);   // → 4  (3 + 2 + -1)
 * t.rangeSum(2, 4); // → 7  (2 + -1 + 6)
 * }</pre>
 */
public final class FenwickTree {

    private final long[] bit;   // 1-indexed BIT array; bit[0] is unused
    private final int n;        // capacity (maximum 1-based index)

    // =========================================================================
    // Constructors
    // =========================================================================

    /**
     * Construct an empty Fenwick tree of capacity {@code n}.
     *
     * <p>All elements are initialised to zero. Indices are 1-based: valid
     * positions are {@code 1..n}.
     *
     * @param n the number of elements (must be ≥ 1)
     * @throws IllegalArgumentException if n < 1
     */
    public FenwickTree(int n) {
        if (n < 1) throw new IllegalArgumentException("n must be >= 1, got: " + n);
        this.n = n;
        this.bit = new long[n + 1];
    }

    /**
     * Construct a Fenwick tree initialised from an existing array.
     *
     * <p>The array is treated as 0-indexed (element 0 maps to position 1 in
     * the BIT). Construction runs in O(n) time.
     *
     * @param values the initial values (must not be null; length ≥ 1)
     * @throws IllegalArgumentException if values is null or empty
     */
    public FenwickTree(long[] values) {
        if (values == null || values.length == 0) {
            throw new IllegalArgumentException("values must not be null or empty");
        }
        this.n = values.length;
        this.bit = new long[n + 1];
        // O(n) construction: copy, then propagate each cell to its parent.
        for (int i = 0; i < n; i++) {
            bit[i + 1] = values[i];
        }
        for (int i = 1; i <= n; i++) {
            int parent = i + (i & -i);
            if (parent <= n) bit[parent] += bit[i];
        }
    }

    // =========================================================================
    // Core operations
    // =========================================================================

    /**
     * Add {@code delta} to the element at position {@code i} (1-based).
     *
     * <p>Walks upward through the BIT updating all cells that cover position i.
     *
     * @param i     the 1-based position to update (must be in [1, n])
     * @param delta the amount to add (may be negative)
     * @throws IllegalArgumentException if i is out of range
     */
    public void update(int i, long delta) {
        checkIndex(i);
        for (; i <= n; i += i & -i) {
            bit[i] += delta;
        }
    }

    /**
     * Return the prefix sum of elements {@code 1..i} (1-based).
     *
     * <p>Walks downward through the BIT stripping the lowest set bit.
     *
     * @param i the 1-based upper bound (must be in [1, n])
     * @return the sum of arr[1] + arr[2] + ... + arr[i]
     * @throws IllegalArgumentException if i is out of range
     */
    public long prefixSum(int i) {
        checkIndex(i);
        long sum = 0;
        for (; i > 0; i -= i & -i) {
            sum += bit[i];
        }
        return sum;
    }

    /**
     * Return the sum of elements in the closed range {@code [l, r]} (1-based).
     *
     * <p>Computed as {@code prefixSum(r) - prefixSum(l - 1)}.
     *
     * @param l the 1-based left bound  (must be in [1, n])
     * @param r the 1-based right bound (must be in [l, n])
     * @return the sum of arr[l] + arr[l+1] + ... + arr[r]
     * @throws IllegalArgumentException if l or r are out of range, or l > r
     */
    public long rangeSum(int l, int r) {
        if (l > r) throw new IllegalArgumentException(
            "l (" + l + ") must be <= r (" + r + ")");
        checkIndex(l);
        checkIndex(r);
        if (l == 1) return prefixSum(r);
        return prefixSum(r) - prefixSum(l - 1);
    }

    /**
     * Return the capacity of this tree (the {@code n} passed to the constructor).
     *
     * <p>Valid 1-based indices are {@code 1..capacity()}.
     */
    public int capacity() { return n; }

    // =========================================================================
    // Helpers
    // =========================================================================

    private void checkIndex(int i) {
        if (i < 1 || i > n) {
            throw new IllegalArgumentException(
                "Index " + i + " out of range [1, " + n + "]");
        }
    }
}
