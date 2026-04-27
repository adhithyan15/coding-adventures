// ============================================================================
// BinarySearchTree.kt — Mutable BST with Order Statistics
// ============================================================================
//
// A Binary Search Tree (BST) is a rooted binary tree where every node obeys
// the BST invariant: all values in the left subtree are strictly less than
// the node's value, and all values in the right subtree are strictly greater.
//
//         5
//        / \
//       3   8
//      / \   \
//     1   4   9
//
// Each node also stores a `size` field (subtree node count), turning this
// into an order-statistics tree:
//
//   kthSmallest(k) — k-th smallest in O(log n)
//   rank(x)        — count of values strictly less than x in O(log n)
//
// ============================================================================
// Deletion
// ============================================================================
//
// Deleting a two-child node: replace it with its in-order successor (the
// minimum of its right subtree), then delete the successor from there.
//
// ============================================================================
// Kotlin idioms used
// ============================================================================
//
//   • Generic class with `T : Comparable<T>` upper bound.
//   • Inner `class Node` (not a data class — mutable children + size).
//   • `companion object` for the `fromSortedList` factory.
//   • Extension-style private helpers as top-level functions.
//   • `val size: Int` — O(1) computed from root.size (0 for null root).
//   • Null-safe field access with `?.` and `?:` operators.
//
// ============================================================================

package com.codingadventures.bst

/**
 * A mutable Binary Search Tree (BST) with order-statistics support.
 *
 * Elements must implement [Comparable]. Duplicates are ignored.
 *
 * ```kotlin
 * val t = BinarySearchTree<Int>()
 * listOf(5, 1, 8, 3, 7).forEach { t.insert(it) }
 *
 * t.toSortedList()          // [1, 3, 5, 7, 8]
 * t.kthSmallest(4)          // 7
 * t.rank(4)                 // 2  (1, 3 < 4)
 * t.predecessor(5)          // 3
 * t.successor(5)            // 7
 * t.delete(5)
 * t.contains(5)             // false
 * ```
 *
 * **Time complexity**: O(log n) expected for all operations on a randomly
 * inserted tree. Build a balanced tree from a sorted list with [fromSortedList].
 *
 * @param T the element type; must be [Comparable]
 */
class BinarySearchTree<T : Comparable<T>> {

    // =========================================================================
    // Node
    // =========================================================================

    /**
     * A single node in the BST.
     *
     * [size] is the count of nodes in this subtree (including self), cached for
     * O(1) order-statistics queries.
     */
    inner class Node(
        var value: T,
        var left:  Node? = null,
        var right: Node? = null,
        var size:  Int   = 1
    )

    // =========================================================================
    // Fields
    // =========================================================================

    var root: Node? = null
        internal set

    // =========================================================================
    // Companion (factory)
    // =========================================================================

    companion object {
        /**
         * Build a balanced BST from a pre-sorted list in O(n).
         *
         * The middle element becomes the root, guaranteeing a height of
         * ⌊log₂ n⌋ and O(log n) for all subsequent operations.
         */
        fun <T : Comparable<T>> fromSortedList(sortedValues: List<T>): BinarySearchTree<T> {
            val tree = BinarySearchTree<T>()
            tree.root = tree.buildBalanced(sortedValues, 0, sortedValues.size - 1)
            return tree
        }
    }

    // =========================================================================
    // Core mutation
    // =========================================================================

    /**
     * Insert [value] into the BST. No-op if the value already exists.
     *
     * @throws IllegalArgumentException if [value] is null (not reachable in
     *   Kotlin, but enforced for JVM callers)
     */
    fun insert(value: T) {
        root = insertRec(root, value)
    }

    /**
     * Remove [value] from the BST. No-op if not present.
     */
    fun delete(value: T) {
        root = deleteRec(root, value)
    }

    // =========================================================================
    // Search
    // =========================================================================

    /**
     * Search for [value].
     *
     * @return the matching [Node], or `null` if absent
     */
    fun search(value: T): Node? {
        var current = root
        while (current != null) {
            val cmp = value.compareTo(current.value)
            current = when {
                cmp < 0 -> current.left
                cmp > 0 -> current.right
                else    -> return current
            }
        }
        return null
    }

    /** Return `true` if [value] is present in the BST. */
    fun contains(value: T): Boolean = search(value) != null

    // =========================================================================
    // Min / Max
    // =========================================================================

    /** Return the minimum value, or `null` if the tree is empty. */
    fun minValue(): T? {
        var current = root
        while (current?.left != null) current = current.left
        return current?.value
    }

    /** Return the maximum value, or `null` if the tree is empty. */
    fun maxValue(): T? {
        var current = root
        while (current?.right != null) current = current.right
        return current?.value
    }

    // =========================================================================
    // Predecessor / Successor
    // =========================================================================

    /**
     * Return the largest value strictly less than [value], or `null` if none.
     */
    fun predecessor(value: T): T? {
        var current = root
        var best: T? = null
        while (current != null) {
            val cmp = value.compareTo(current.value)
            if (cmp <= 0) {
                current = current.left
            } else {
                best = current.value
                current = current.right
            }
        }
        return best
    }

    /**
     * Return the smallest value strictly greater than [value], or `null` if none.
     */
    fun successor(value: T): T? {
        var current = root
        var best: T? = null
        while (current != null) {
            val cmp = value.compareTo(current.value)
            if (cmp >= 0) {
                current = current.right
            } else {
                best = current.value
                current = current.left
            }
        }
        return best
    }

    // =========================================================================
    // Order statistics
    // =========================================================================

    /**
     * Return the k-th smallest value (1-indexed), or `null` if out of range.
     *
     * Uses size augmentation: at each node, decide whether the answer is in
     * the current node, left subtree, or right subtree without scanning.
     */
    fun kthSmallest(k: Int): T? = kthSmallestRec(root, k)?.value

    /**
     * Return the rank of [value]: the number of elements strictly less than it.
     *
     * Works even if [value] is not in the tree.
     */
    fun rank(value: T): Int = rankRec(root, value)

    // =========================================================================
    // Traversal / export
    // =========================================================================

    /**
     * Return all elements in ascending order via in-order traversal.
     */
    fun toSortedList(): List<T> {
        val out = ArrayList<T>(nodeSize(root))
        inorderRec(root, out)
        return out
    }

    // =========================================================================
    // Structural queries
    // =========================================================================

    /**
     * Validate the BST property and size invariant throughout the tree.
     *
     * @return `true` iff every node satisfies strict left < node < right
     *   and every node's size field equals 1 + size(left) + size(right)
     */
    fun isValid(): Boolean = validateRec(root, null, null) != Int.MIN_VALUE

    /**
     * Return the height of the tree. Empty tree → -1; single node → 0.
     */
    fun height(): Int = heightRec(root)

    /** Total number of elements. O(1) via the root's cached size. */
    val size: Int get() = nodeSize(root)

    /** True if the tree contains no elements. */
    val isEmpty: Boolean get() = root == null

    // =========================================================================
    // Object overrides
    // =========================================================================

    override fun toString(): String =
        "BinarySearchTree(root=${root?.value}, size=$size)"

    // =========================================================================
    // Private recursive helpers
    // =========================================================================

    private fun insertRec(node: Node?, value: T): Node {
        if (node == null) return Node(value)
        val cmp = value.compareTo(node.value)
        when {
            cmp < 0 -> node.left  = insertRec(node.left,  value)
            cmp > 0 -> node.right = insertRec(node.right, value)
            // duplicate — no-op
        }
        node.size = 1 + nodeSize(node.left) + nodeSize(node.right)
        return node
    }

    private fun deleteRec(node: Node?, value: T): Node? {
        node ?: return null
        val cmp = value.compareTo(node.value)
        when {
            cmp < 0 -> node.left  = deleteRec(node.left,  value)
            cmp > 0 -> node.right = deleteRec(node.right, value)
            else    -> {
                if (node.left  == null) return node.right
                if (node.right == null) return node.left
                // Two children: replace with in-order successor
                val successorVal = minNode(node.right!!).value
                node.value = successorVal
                node.right = deleteRec(node.right, successorVal)
            }
        }
        node.size = 1 + nodeSize(node.left) + nodeSize(node.right)
        return node
    }

    private fun minNode(node: Node): Node {
        var current = node
        while (current.left != null) current = current.left!!
        return current
    }

    private fun kthSmallestRec(node: Node?, k: Int): Node? {
        if (node == null || k <= 0) return null
        val leftSize = nodeSize(node.left)
        return when {
            k == leftSize + 1 -> node
            k <= leftSize     -> kthSmallestRec(node.left, k)
            else              -> kthSmallestRec(node.right, k - leftSize - 1)
        }
    }

    private fun rankRec(node: Node?, value: T): Int {
        if (node == null) return 0
        val cmp = value.compareTo(node.value)
        return when {
            cmp < 0 -> rankRec(node.left, value)
            cmp > 0 -> nodeSize(node.left) + 1 + rankRec(node.right, value)
            else    -> nodeSize(node.left)
        }
    }

    private fun inorderRec(node: Node?, out: MutableList<T>) {
        if (node == null) return
        inorderRec(node.left, out)
        out.add(node.value)
        inorderRec(node.right, out)
    }

    /**
     * Validate and return height (or [Int.MIN_VALUE] as the invalid sentinel).
     *
     * Passing [min] and [max] bounds down the tree ensures every node is
     * strictly within the range of all its ancestors.
     */
    private fun validateRec(node: Node?, min: T?, max: T?): Int {
        if (node == null) return -1
        if (min != null && node.value.compareTo(min) <= 0) return Int.MIN_VALUE
        if (max != null && node.value.compareTo(max) >= 0) return Int.MIN_VALUE

        val leftH  = validateRec(node.left,  min,        node.value)
        val rightH = validateRec(node.right, node.value, max)
        if (leftH == Int.MIN_VALUE || rightH == Int.MIN_VALUE) return Int.MIN_VALUE

        val expectedSize = 1 + nodeSize(node.left) + nodeSize(node.right)
        if (node.size != expectedSize) return Int.MIN_VALUE

        return 1 + maxOf(leftH, rightH)
    }

    private fun heightRec(node: Node?): Int {
        if (node == null) return -1
        return 1 + maxOf(heightRec(node.left), heightRec(node.right))
    }

    /** Read the cached size (0 for null). */
    private fun nodeSize(node: Node?): Int = node?.size ?: 0

    /** Build a balanced BST from a sorted sublist [lo, hi] inclusive. */
    private fun buildBalanced(values: List<T>, lo: Int, hi: Int): Node? {
        if (lo > hi) return null
        val mid = lo + (hi - lo) / 2
        val node = Node(values[mid])
        node.left  = buildBalanced(values, lo, mid - 1)
        node.right = buildBalanced(values, mid + 1, hi)
        node.size  = 1 + nodeSize(node.left) + nodeSize(node.right)
        return node
    }
}
