// ============================================================================
// AVLTree.kt — Self-Balancing Binary Search Tree (AVL)
// ============================================================================
//
// An AVL tree (named after Adelson-Velsky and Landis, 1962) is a binary
// search tree that keeps itself balanced by enforcing one extra invariant:
//
//   For every node, |height(left) - height(right)| ≤ 1
//
// This single rule guarantees O(log n) height, so all standard BST operations
// run in O(log n) time in the worst case — unlike an unbalanced BST which
// degrades to O(n) on sorted input.
//
// ============================================================================
// Node augmentation: height and size
// ============================================================================
//
//   Each node stores two extra fields beyond the value and child pointers:
//
//     height: distance to the furthest leaf below (leaves have height 0)
//     size  : count of nodes in this subtree (leaves have size 1)
//
//   These are updated on every structural change, which enables:
//     - O(1) height lookups (used by the balancing code)
//     - O(log n) kthSmallest and rank (order statistics)
//
// ============================================================================
// Rebalancing: four rotation cases
// ============================================================================
//
//   When an insert or delete creates a node with balance factor (BF) ≤ -2
//   or ≥ +2, we apply one of four rotations:
//
//     BF(node) = height(left) - height(right)
//
//     Case 1 — Left-Left (BF > 1, left child is left-heavy):
//       rotate right at node
//
//     Case 2 — Left-Right (BF > 1, left child is right-heavy):
//       rotate left at left child, then rotate right at node
//
//     Case 3 — Right-Right (BF < -1, right child is right-heavy):
//       rotate left at node
//
//     Case 4 — Right-Left (BF < -1, right child is left-heavy):
//       rotate right at right child, then rotate left at node
//
//   Rotation diagrams:
//
//   Right rotation (rotateRight at y):
//
//         y             x
//        / \           / \
//       x   C    →    A   y
//      / \               / \
//     A   B             B   C
//
//   Left rotation (rotateLeft at x):
//
//       x               y
//      / \             / \
//     A   y     →    x   C
//        / \        / \
//       B   C      A   B
//
// ============================================================================

package com.codingadventures.avltree

/**
 * A self-balancing binary search tree using the AVL invariant.
 *
 * All operations run in O(log n) time in the worst case. Each node is
 * augmented with its subtree height and size, enabling O(log n) order
 * statistics (rank and kth-smallest).
 *
 * ```kotlin
 * val tree = AVLTree<Int>()
 * tree.insert(10)
 * tree.insert(5)
 * tree.insert(20)
 *
 * tree.contains(5)            // true
 * tree.min()                  // 5
 * tree.max()                  // 20
 * tree.kthSmallest(2)         // 10
 * tree.rank(10)               // 1  (0-based: one element is smaller)
 * tree.predecessor(10)        // 5
 * tree.successor(10)          // 20
 *
 * tree.delete(10)
 * tree.size                   // 2
 * tree.isValid()              // true
 * ```
 *
 * @param T the element type; must be [Comparable]
 */
class AVLTree<T : Comparable<T>> {

    // =========================================================================
    // Inner class: Node
    // =========================================================================

    /**
     * A single node in the AVL tree.
     *
     * Invariants (maintained after every structural change):
     * - height == 1 + max(height(left), height(right))  (0 for leaves)
     * - size   == 1 + size(left) + size(right)          (1 for leaves)
     * - |height(left) - height(right)| ≤ 1              (AVL property)
     * - BST ordering: all values in left < value < all in right
     */
    inner class Node(var value: T) {
        var left:   Node? = null
        var right:  Node? = null
        var height: Int   = 0   // 0 for a leaf
        var size:   Int   = 1   // 1 for a leaf
    }

    // =========================================================================
    // Fields
    // =========================================================================

    private var root: Node? = null

    /** Number of values in the tree. */
    val size: Int get() = nodeSize(root)

    /** True when the tree contains no values. */
    val isEmpty: Boolean get() = root == null

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Insert [value] into the tree.
     *
     * If the value is already present, the tree is unchanged.
     *
     * @throws IllegalArgumentException if value is null (enforced by Kotlin)
     */
    fun insert(value: T) {
        root = insert(root, value)
    }

    /**
     * Remove [value] from the tree.
     *
     * @throws NoSuchElementException if the value is not present
     */
    fun delete(value: T) {
        if (!contains(value)) throw NoSuchElementException("Value not found: $value")
        root = delete(root, value)
    }

    /**
     * Return true if [value] is present in the tree.
     */
    fun contains(value: T): Boolean {
        var node = root
        while (node != null) {
            val cmp = value.compareTo(node.value)
            node = when {
                cmp < 0 -> node.left
                cmp > 0 -> node.right
                else    -> return true
            }
        }
        return false
    }

    /**
     * Return the smallest value in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    fun min(): T {
        val r = root ?: throw NoSuchElementException("Tree is empty")
        var node = r
        while (node.left != null) node = node.left!!
        return node.value
    }

    /**
     * Return the largest value in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    fun max(): T {
        val r = root ?: throw NoSuchElementException("Tree is empty")
        var node = r
        while (node.right != null) node = node.right!!
        return node.value
    }

    /**
     * Return the largest value strictly less than [value], or null if none exists.
     */
    fun predecessor(value: T): T? {
        var best: T? = null
        var node = root
        while (node != null) {
            node = if (value.compareTo(node.value) <= 0) {
                node.left
            } else {
                best = node.value
                node.right
            }
        }
        return best
    }

    /**
     * Return the smallest value strictly greater than [value], or null if none exists.
     */
    fun successor(value: T): T? {
        var best: T? = null
        var node = root
        while (node != null) {
            node = if (value.compareTo(node.value) >= 0) {
                node.right
            } else {
                best = node.value
                node.left
            }
        }
        return best
    }

    /**
     * Return the k-th smallest value (1-based).
     *
     * [kthSmallest](1) returns the minimum; [kthSmallest](size) returns the maximum.
     *
     * @param k the rank (1-based)
     * @return the k-th smallest value, or null if k is out of range
     */
    fun kthSmallest(k: Int): T? {
        if (k <= 0 || k > nodeSize(root)) return null
        return kthSmallest(root!!, k)
    }

    /**
     * Return the 0-based rank of [value] — the number of elements strictly
     * less than [value].
     *
     * If [value] is not in the tree, this is the position it would occupy
     * if inserted.
     */
    fun rank(value: T): Int = rank(root, value)

    /**
     * Return all values in ascending (in-order) order.
     */
    fun toSortedList(): List<T> {
        val out = mutableListOf<T>()
        inorder(root, out)
        return out
    }

    /** Height of the tree (0 for single node, -1 for empty). */
    val height: Int get() = nodeHeight(root)

    /**
     * Balance factor of the root: height(left) - height(right).
     * A valid AVL tree has every node's BF in {-1, 0, +1}.
     */
    val balanceFactor: Int get() = balanceFactor(root)

    /**
     * Validate all AVL tree invariants:
     * 1. BST ordering (left < node < right at every node)
     * 2. AVL property (|BF| ≤ 1 at every node)
     * 3. Correct height values in every node
     * 4. Correct size values in every node
     *
     * @return true if the tree is a valid AVL tree
     */
    fun isValid(): Boolean = validateAVL(root, null, null) != null

    /**
     * Return true if BST ordering holds (does NOT check AVL balance invariant).
     */
    fun isValidBST(): Boolean = validateBST(root, null, null)

    override fun toString(): String = "AVLTree(size=$size, height=$height)"

    // =========================================================================
    // Private helpers — insertion
    // =========================================================================

    private fun insert(node: Node?, value: T): Node {
        if (node == null) return Node(value)
        val cmp = value.compareTo(node.value)
        when {
            cmp < 0 -> node.left  = insert(node.left,  value)
            cmp > 0 -> node.right = insert(node.right, value)
            // cmp == 0: already present, no change
        }
        update(node)
        return rebalance(node)
    }

    // =========================================================================
    // Private helpers — deletion
    // =========================================================================

    private fun delete(node: Node?, value: T): Node? {
        node ?: return null
        val cmp = value.compareTo(node.value)
        when {
            cmp < 0 -> node.left  = delete(node.left,  value)
            cmp > 0 -> node.right = delete(node.right, value)
            else    -> {
                if (node.left  == null) return node.right
                if (node.right == null) return node.left
                // Two children: replace with in-order successor
                var successor = node.right!!
                while (successor.left != null) successor = successor.left!!
                node.value = successor.value
                node.right = delete(node.right, successor.value)
            }
        }
        update(node)
        return rebalance(node)
    }

    // =========================================================================
    // Private helpers — rotations and rebalancing
    // =========================================================================

    /**
     * Rotate right at [y]:
     *
     *       y             x
     *      / \           / \
     *     x   C    →    A   y
     *    / \               / \
     *   A   B             B   C
     */
    private fun rotateRight(y: Node): Node {
        val x = y.left!!
        val b = x.right
        x.right = y
        y.left  = b
        update(y)
        update(x)
        return x
    }

    /**
     * Rotate left at [x]:
     *
     *     x               y
     *    / \             / \
     *   A   y     →    x   C
     *      / \        / \
     *     B   C      A   B
     */
    private fun rotateLeft(x: Node): Node {
        val y = x.right!!
        val b = y.left
        y.left  = x
        x.right = b
        update(x)
        update(y)
        return y
    }

    private fun rebalance(node: Node): Node {
        val bf = balanceFactor(node)
        if (bf > 1) {
            if (balanceFactor(node.left) < 0) node.left = rotateLeft(node.left!!)
            return rotateRight(node)
        }
        if (bf < -1) {
            if (balanceFactor(node.right) > 0) node.right = rotateRight(node.right!!)
            return rotateLeft(node)
        }
        return node
    }

    private fun update(node: Node) {
        node.height = 1 + maxOf(nodeHeight(node.left), nodeHeight(node.right))
        node.size   = 1 + nodeSize(node.left) + nodeSize(node.right)
    }

    // =========================================================================
    // Private helpers — order statistics
    // =========================================================================

    private fun kthSmallest(node: Node, k: Int): T {
        val leftSize = nodeSize(node.left)
        return when {
            k == leftSize + 1 -> node.value
            k <= leftSize     -> kthSmallest(node.left!!, k)
            else              -> kthSmallest(node.right!!, k - leftSize - 1)
        }
    }

    private fun rank(node: Node?, value: T): Int {
        node ?: return 0
        val cmp = value.compareTo(node.value)
        return when {
            cmp < 0 -> rank(node.left,  value)
            cmp > 0 -> nodeSize(node.left) + 1 + rank(node.right, value)
            else    -> nodeSize(node.left)
        }
    }

    private fun inorder(node: Node?, out: MutableList<T>) {
        node ?: return
        inorder(node.left,  out)
        out.add(node.value)
        inorder(node.right, out)
    }

    // =========================================================================
    // Private helpers — utility
    // =========================================================================

    private fun nodeHeight(node: Node?): Int = node?.height ?: -1
    private fun nodeSize(node: Node?):   Int = node?.size   ?: 0
    private fun balanceFactor(node: Node?): Int =
        if (node == null) 0 else nodeHeight(node.left) - nodeHeight(node.right)

    // =========================================================================
    // Private helpers — validation
    // =========================================================================

    /**
     * Returns IntArray(height, size) if the subtree rooted at [node] is a
     * valid AVL tree (BST-ordered, balanced, correct cached values), or null
     * if any invariant is violated.
     */
    private fun validateAVL(node: Node?, min: T?, max: T?): IntArray? {
        if (node == null) return intArrayOf(-1, 0)
        if (min != null && node.value.compareTo(min) <= 0) return null
        if (max != null && node.value.compareTo(max) >= 0) return null
        val left  = validateAVL(node.left,  min,        node.value) ?: return null
        val right = validateAVL(node.right, node.value, max)        ?: return null
        val expectedHeight = 1 + maxOf(left[0], right[0])
        val expectedSize   = 1 + left[1] + right[1]
        val bf             = left[0] - right[0]
        if (node.height != expectedHeight) return null
        if (node.size   != expectedSize)   return null
        if (kotlin.math.abs(bf) > 1)       return null
        return intArrayOf(expectedHeight, expectedSize)
    }

    private fun validateBST(node: Node?, min: T?, max: T?): Boolean {
        node ?: return true
        if (min != null && node.value.compareTo(min) <= 0) return false
        if (max != null && node.value.compareTo(max) >= 0) return false
        return validateBST(node.left, min, node.value) &&
               validateBST(node.right, node.value, max)
    }
}
