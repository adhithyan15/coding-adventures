// ============================================================================
// SegmentTree.java — Generic Segment Tree for Range Queries + Point Updates
// ============================================================================
//
// A segment tree is a binary tree where every node stores an AGGREGATE over a
// contiguous sub-range of an array.  Given an array A of n elements:
//
//   Leaf  nodes: tree[i] = A[j]         for some index j
//   Internal nodes: tree[i] = combine(tree[left_child], tree[right_child])
//
// This structure supports two operations in O(log n):
//   • range query:  combine all elements in A[ql..qr]
//   • point update: change A[i] to a new value, re-computing ancestors
//
// ─────────────────────────────────────────────────────────────────────────────
// The Combine Function Must Be a Monoid
// ─────────────────────────────────────────────────────────────────────────────
//
//   A monoid is an algebraic structure with:
//     1. An associative binary operation: combine(a, combine(b,c)) == combine(combine(a,b), c)
//     2. An identity element e: combine(e, x) == combine(x, e) == x
//
//   Common monoids for segment trees:
//
//   | combine  | identity         | query answers       |
//   |----------|------------------|---------------------|
//   | a + b    | 0                | range sum           |
//   | min(a,b) | +∞               | range minimum (RMQ) |
//   | max(a,b) | -∞               | range maximum       |
//   | gcd(a,b) | 0                | range GCD           |
//   | a * b    | 1                | range product       |
//   | a & b    | 0xFFFFFFFF       | range bitwise AND   |
//   | a | b    | 0                | range bitwise OR    |
//
// ─────────────────────────────────────────────────────────────────────────────
// Array-Backed Storage (1-indexed)
// ─────────────────────────────────────────────────────────────────────────────
//
// Like a heap, nodes are stored in a flat array using the parent-child formulas:
//
//   root:         index 1
//   left child:   2 * i
//   right child:  2 * i + 1
//   parent:       i / 2
//
// Why 1-indexed? Formulas 2*i and 2*i+1 are cleaner than 2*i+1 and 2*i+2.
// Index 0 is unused (or holds the identity as a sentinel).
//
// Array size: 4*n is always sufficient (covers the worst case of a non-power-
// of-2 input length without needing exact calculation).
//
// ─────────────────────────────────────────────────────────────────────────────
// Example (sum, array = [2, 1, 5, 3, 4])
// ─────────────────────────────────────────────────────────────────────────────
//
//   Internal tree array (1-indexed):
//
//   Idx:  1   2   3   4   5   6   7   8   9
//   Val: 15   8   7   3   5   3   4   2   1
//
//   Tree structure:
//
//          1
//         (15)          covers [0..4]
//        /    \
//       2      3
//      (8)    (7)       covers [0..2] and [3..4]
//     /   \   /  \
//    4     5 6    7
//   (3)  (5)(3)  (4)   covers [0..1], [2..2], [3..3], [4..4]
//   / \
//  8   9
// (2) (1)              covers [0..0] and [1..1]
//
// range query [1..3] = 1 + 5 + 3 = 9:
//   Node 2 [0..2]:  partial → recurse
//     Node 4 [0..1]:  partial → recurse
//       Node 8 [0..0]:  outside [1..3] → return 0  (identity)
//       Node 9 [1..1]:  inside  [1..3] → return 1
//     return combine(0, 1) = 1
//     Node 5 [2..2]:  inside [1..3] → return 5
//   return combine(1, 5) = 6
//   Node 3 [3..4]:  partial → recurse
//     Node 6 [3..3]:  inside [1..3] → return 3
//     Node 7 [4..4]:  outside [1..3] → return 0  (identity)
//   return combine(3, 0) = 3
//   return combine(6, 3) = 9  ✓
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.segmenttree
// ============================================================================

package com.codingadventures.segmenttree;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.function.BinaryOperator;

/**
 * A generic segment tree supporting range queries and point updates in O(log n).
 *
 * <p>Parameterized by:
 * <ul>
 *   <li>{@code T} — element type (must support the combine operation)</li>
 *   <li>{@code combine} — an associative binary operator (the monoid operation)</li>
 *   <li>{@code identity} — neutral element: {@code combine(identity, x) == x}</li>
 * </ul>
 *
 * <p>Use the static factory methods for the most common integer variants:
 * <pre>{@code
 * // Range sum:
 * SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{2, 1, 5, 3, 4});
 * st.query(1, 3);   // → 9   (1 + 5 + 3)
 * st.update(2, 7);  // A[2] is now 7
 * st.query(1, 3);   // → 11  (1 + 7 + 3)
 *
 * // Range minimum:
 * SegmentTree<Integer> rm = SegmentTree.minTree(new int[]{5, 3, 7, 1, 9});
 * rm.query(0, 3);   // → 1
 * }</pre>
 *
 * @param <T> the element type stored in the tree
 */
public class SegmentTree<T> {

    // ─────────────────────────────────────────────────────────────────────────
    // Fields
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * 1-indexed backing array.  tree[1] is the root.  tree[0] is unused.
     * Size is 4 * n, which is always sufficient for any input length n.
     */
    @SuppressWarnings("unchecked")
    private final T[] tree;

    /** Length of the original array. */
    private final int n;

    /** Associative binary combine function (e.g. Integer::sum for range-sum). */
    private final BinaryOperator<T> combine;

    /**
     * Identity element: {@code combine(identity, x) == x} for all x.
     * Returned for "no overlap" ranges during queries.
     */
    private final T identity;

    // ─────────────────────────────────────────────────────────────────────────
    // Constructor
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Build a segment tree from the given array.
     *
     * <p>Time: O(n) — each of the ~2n nodes is visited once during build.
     * Space: O(n) — 4*n backing array.
     *
     * @param array    source array (length ≥ 0)
     * @param combine  associative binary operation
     * @param identity neutral element for {@code combine}
     */
    @SuppressWarnings("unchecked")
    public SegmentTree(T[] array, BinaryOperator<T> combine, T identity) {
        this.n        = array.length;
        this.combine  = combine;
        this.identity = identity;
        // 4*n is the worst-case number of nodes for a segment tree over n elements.
        // Use max(4, 4*n) so the array is non-empty even for n=0.
        this.tree = (T[]) new Object[Math.max(4, 4 * n)];
        // Pre-fill with identity so "empty" slots return a safe value.
        Arrays.fill(tree, identity);
        if (n > 0) {
            build(array, 1, 0, n - 1);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Build
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Recursively fill the tree array bottom-up.
    //
    //   build(node, left, right):
    //     if left == right:               ← leaf: store the array value
    //       tree[node] = array[left]
    //     else:                           ← internal: recurse then aggregate
    //       mid = (left + right) / 2
    //       build(2*node,   left, mid)
    //       build(2*node+1, mid+1, right)
    //       tree[node] = combine(tree[2*node], tree[2*node+1])

    private void build(T[] array, int node, int left, int right) {
        if (left == right) {
            // Leaf node: store the original array value.
            tree[node] = array[left];
            return;
        }
        int mid = (left + right) / 2;
        build(array, 2 * node,     left,    mid);
        build(array, 2 * node + 1, mid + 1, right);
        // Internal node: aggregate from children.
        tree[node] = combine.apply(tree[2 * node], tree[2 * node + 1]);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Range Query
    // ─────────────────────────────────────────────────────────────────────────
    //
    // query(ql, qr) returns combine of all A[i] for ql ≤ i ≤ qr.
    //
    // Three cases at each node covering [left..right]:
    //
    //   1. No overlap:      right < ql  OR  left > qr
    //      → return identity (contributes nothing to the result)
    //
    //   2. Total overlap:   ql ≤ left  AND  right ≤ qr
    //      → return tree[node]  (the whole range is inside the query)
    //
    //   3. Partial overlap: otherwise
    //      → recurse on both children, combine results
    //
    // Correctness: we never "skip" any element because case 3 forces recursion
    // into both children until we reach leaf-level total or no-overlap cases.
    //
    // Time: O(log n).  At each level, at most 4 nodes are partially overlapping;
    // all others are either totally inside or totally outside.  With O(log n)
    // levels, total work is O(4 log n) = O(log n).

    /**
     * Return the aggregate over {@code array[ql..qr]} (inclusive, 0-indexed).
     *
     * <p>Time: O(log n).
     *
     * @param ql left bound (inclusive, 0-indexed)
     * @param qr right bound (inclusive, 0-indexed)
     * @return combine(A[ql], A[ql+1], ..., A[qr])
     * @throws IllegalArgumentException if bounds are invalid
     */
    public T query(int ql, int qr) {
        // Validate: both bounds must be in [0, n-1] and ql ≤ qr.
        // This also covers the n==0 case: qr ≥ n=0 triggers the throw.
        if (ql < 0 || qr >= n || ql > qr) {
            throw new IllegalArgumentException(
                "Invalid query range [" + ql + ", " + qr + "] for array of size " + n);
        }
        return queryHelper(1, 0, n - 1, ql, qr);
    }

    private T queryHelper(int node, int left, int right, int ql, int qr) {
        // Case 1: no overlap — this node's range is entirely outside [ql..qr]
        if (right < ql || left > qr) {
            return identity;
        }
        // Case 2: total overlap — this node's range is entirely inside [ql..qr]
        if (ql <= left && right <= qr) {
            return tree[node];
        }
        // Case 3: partial overlap — recurse into both children
        int mid = (left + right) / 2;
        T leftResult  = queryHelper(2 * node,     left,    mid,   ql, qr);
        T rightResult = queryHelper(2 * node + 1, mid + 1, right, ql, qr);
        return combine.apply(leftResult, rightResult);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Point Update
    // ─────────────────────────────────────────────────────────────────────────
    //
    // update(index, value) sets A[index] = value and recomputes all ancestors.
    //
    //   update(node, left, right, idx, value):
    //     if left == right:               ← found the leaf
    //       tree[node] = value
    //       return
    //     if idx ≤ mid:
    //       update(left child, left, mid, idx, value)
    //     else:
    //       update(right child, mid+1, right, idx, value)
    //     tree[node] = combine(left child, right child)   ← recompute on the way back up
    //
    // Time: O(log n).  We trace a single root-to-leaf path (O(log n) nodes)
    // and recompute each ancestor — O(1) per node.

    /**
     * Set {@code array[index] = value} and update all ancestor nodes.
     *
     * <p>Time: O(log n).
     *
     * @param index position to update (0-indexed)
     * @param value new value
     * @throws IllegalArgumentException if index is out of range
     */
    public void update(int index, T value) {
        if (index < 0 || index >= n) {
            throw new IllegalArgumentException(
                "Index " + index + " out of range for array of size " + n);
        }
        updateHelper(1, 0, n - 1, index, value);
    }

    private void updateHelper(int node, int left, int right, int index, T value) {
        if (left == right) {
            // Found the leaf: install the new value.
            tree[node] = value;
            return;
        }
        int mid = (left + right) / 2;
        if (index <= mid) {
            updateHelper(2 * node,     left,    mid,   index, value);
        } else {
            updateHelper(2 * node + 1, mid + 1, right, index, value);
        }
        // Recompute this internal node from the (now updated) children.
        tree[node] = combine.apply(tree[2 * node], tree[2 * node + 1]);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Reconstruct Array
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Reconstruct the current array from the leaf nodes.
     *
     * <p>The returned list reflects all updates made since construction.
     * Time: O(n).
     *
     * @return list of current array values, in index order
     */
    public List<T> toList() {
        List<T> result = new ArrayList<>(n);
        collectLeaves(1, 0, n - 1, result);
        return result;
    }

    private void collectLeaves(int node, int left, int right, List<T> acc) {
        if (left == right) {
            acc.add(tree[node]);
            return;
        }
        int mid = (left + right) / 2;
        collectLeaves(2 * node,     left,    mid,   acc);
        collectLeaves(2 * node + 1, mid + 1, right, acc);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Metadata
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the length of the original array. */
    public int size() {
        return n;
    }

    /** Return {@code true} if the underlying array is empty. */
    public boolean isEmpty() {
        return n == 0;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Static Factory Methods
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Convenience builders for the four most common integer aggregate trees.

    /**
     * Build a range-sum segment tree.
     *
     * <p>Example:
     * <pre>{@code
     * SegmentTree<Integer> st = SegmentTree.sumTree(new int[]{2, 1, 5, 3, 4});
     * st.query(1, 3);  // → 9 (= 1 + 5 + 3)
     * }</pre>
     *
     * @param array source array
     * @return segment tree with sum combine and identity 0
     */
    public static SegmentTree<Integer> sumTree(int[] array) {
        Integer[] boxed = box(array);
        return new SegmentTree<>(boxed, Integer::sum, 0);
    }

    /**
     * Build a range-minimum segment tree (RMQ).
     *
     * <p>Example:
     * <pre>{@code
     * SegmentTree<Integer> st = SegmentTree.minTree(new int[]{5, 3, 7, 1, 9});
     * st.query(0, 4);  // → 1
     * }</pre>
     *
     * @param array source array
     * @return segment tree with min combine and identity Integer.MAX_VALUE
     */
    public static SegmentTree<Integer> minTree(int[] array) {
        Integer[] boxed = box(array);
        return new SegmentTree<>(boxed, Math::min, Integer.MAX_VALUE);
    }

    /**
     * Build a range-maximum segment tree.
     *
     * @param array source array
     * @return segment tree with max combine and identity Integer.MIN_VALUE
     */
    public static SegmentTree<Integer> maxTree(int[] array) {
        Integer[] boxed = box(array);
        return new SegmentTree<>(boxed, Math::max, Integer.MIN_VALUE);
    }

    /**
     * Build a range-GCD segment tree.
     *
     * <p>The identity for GCD is 0, since gcd(0, x) = x for all x ≥ 0.
     *
     * <p>Example:
     * <pre>{@code
     * SegmentTree<Integer> st = SegmentTree.gcdTree(new int[]{12, 8, 6, 4, 9});
     * st.query(0, 2);  // → 2  (gcd(12, gcd(8, 6)) = gcd(12, 2) = 2)
     * }</pre>
     *
     * @param array source array
     * @return segment tree with GCD combine and identity 0
     */
    public static SegmentTree<Integer> gcdTree(int[] array) {
        Integer[] boxed = box(array);
        return new SegmentTree<>(boxed, SegmentTree::gcd, 0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /** Euclidean GCD (non-negative integers). */
    private static int gcd(int a, int b) {
        while (b != 0) {
            int t = b;
            b = a % b;
            a = t;
        }
        return a;
    }

    /** Box a primitive int array into an Integer array. */
    private static Integer[] box(int[] array) {
        Integer[] boxed = new Integer[array.length];
        for (int i = 0; i < array.length; i++) {
            boxed[i] = array[i];
        }
        return boxed;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    @Override
    public String toString() {
        return "SegmentTree{n=" + n + ", identity=" + identity + "}";
    }
}
