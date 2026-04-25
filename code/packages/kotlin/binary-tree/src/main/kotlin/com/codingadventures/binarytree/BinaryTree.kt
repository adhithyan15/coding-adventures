// ============================================================================
// BinaryTree.kt — Generic Binary Tree with Traversal and Shape Queries
// ============================================================================
//
// A binary tree is a rooted tree where each node has at most two children.
// Unlike a BST, there is no ordering constraint — this is purely structural.
//
//              1
//            /   \
//           2     3
//          / \     \
//         4   5     6
//
// == Construction ==
//
// The canonical way to specify a binary tree is level-order (BFS order):
//   [1, 2, 3, 4, 5, null, 6]
//
// Index i maps to children at 2i+1 (left) and 2i+2 (right):
//   i=0 (1)  → children at 1 and 2
//   i=1 (2)  → children at 3 and 4
//   i=2 (3)  → children at 5=null and 6
//
// == Traversals ==
//
//   In-order   (L → root → R):  4, 2, 5, 1, 3, 6
//   Pre-order  (root → L → R):  1, 2, 4, 5, 3, 6
//   Post-order (L → R → root):  4, 5, 2, 6, 3, 1
//   Level-order (BFS):          1, 2, 3, 4, 5, 6
//
// == Shape predicates ==
//
//   Full:     every node has 0 or 2 children
//   Complete: all levels filled L→R except possibly the last
//   Perfect:  all leaves at same depth; node count = 2^(h+1) - 1
//
// == Kotlin idioms ==
//
//   • Generic class with unconstrained `T` type parameter.
//   • Public `data class Node<T>` — value equality is useful for tests.
//   • `companion object` holds `fromLevelOrder` factory.
//   • Null-sentinel BFS for `isComplete` uses `LinkedList` (allows null items).
//   • Extension functions for private recursive helpers.
//
// ============================================================================

package com.codingadventures.binarytree

import java.util.LinkedList

/**
 * A generic binary tree with traversal and structural shape helpers.
 *
 * ```kotlin
 * val t = BinaryTree.fromLevelOrder(listOf(1, 2, 3, 4, 5, null, 6))
 *
 * t.levelOrder()   // [1, 2, 3, 4, 5, 6]
 * t.inorder()      // [4, 2, 5, 1, 3, 6]
 * t.height()       // 2
 * t.isFull()       // false (node 3 has only right child)
 * t.isComplete()   // false (null slot before node 6)
 * ```
 *
 * @param T the element type
 */
class BinaryTree<T> {

    // =========================================================================
    // Node
    // =========================================================================

    /**
     * A single node in the binary tree.
     *
     * Using a regular class (not data class) since equality by identity is
     * more natural for tree nodes; value equality would compare subtrees.
     */
    inner class Node(
        var value: T,
        var left:  Node? = null,
        var right: Node? = null
    ) {
        override fun toString() = "Node($value)"
    }

    // =========================================================================
    // Fields
    // =========================================================================

    /** The root node; null for an empty tree. */
    var root: Node? = null

    // =========================================================================
    // Companion (factory)
    // =========================================================================

    companion object {
        /**
         * Build a binary tree from a level-order (BFS) list.
         *
         * Null entries represent absent nodes. Index [i] maps to left child at
         * [2i+1] and right child at [2i+2].
         *
         * Example: `[1, 2, 3, null, 5]` builds:
         * ```
         *     1
         *    / \
         *   2   3
         *    \
         *     5
         * ```
         */
        fun <T> fromLevelOrder(values: List<T?>): BinaryTree<T> {
            val tree = BinaryTree<T>()
            if (values.isEmpty()) return tree
            tree.root = tree.buildFromLevelOrder(values, 0)
            return tree
        }
    }

    // =========================================================================
    // Search
    // =========================================================================

    /**
     * Find the first node (in pre-order) whose value equals [value].
     *
     * @return the matching [Node], or `null` if not found
     */
    fun find(value: T): Node? = findRec(root, value)

    /** Return the left child of the first node with [value], or null. */
    fun leftChild(value: T): Node? = find(value)?.left

    /** Return the right child of the first node with [value], or null. */
    fun rightChild(value: T): Node? = find(value)?.right

    // =========================================================================
    // Shape predicates
    // =========================================================================

    /**
     * Return `true` if every node has exactly 0 or 2 children (no lone children).
     */
    fun isFull(): Boolean = isFullRec(root)

    /**
     * Return `true` if all levels are fully filled except possibly the last,
     * which must be filled left-to-right.
     *
     * Uses a null-sentinel BFS: once a null position is seen, every subsequent
     * non-null node means the last level is not filled left-to-right.
     *
     * Requires [LinkedList] (not ArrayDeque) because we enqueue null sentinels.
     */
    fun isComplete(): Boolean {
        val root = this.root ?: return true   // empty tree is complete
        val queue: LinkedList<Node?> = LinkedList()
        queue.add(root)
        var seenNull = false
        while (queue.isNotEmpty()) {
            val node = queue.poll()
            if (node == null) {
                seenNull = true
            } else {
                if (seenNull) return false
                queue.add(node.left)    // may be null — that is intentional
                queue.add(node.right)
            }
        }
        return true
    }

    /**
     * Return `true` if all leaves are at the same depth (i.e., the tree is
     * both full and all leaves are at depth h). A perfect tree of height h has
     * exactly 2^(h+1) - 1 nodes.
     */
    fun isPerfect(): Boolean {
        val h = height()
        if (h < 0) return size == 0
        return size == (1 shl (h + 1)) - 1
    }

    // =========================================================================
    // Traversals
    // =========================================================================

    /** In-order traversal: left → root → right. */
    fun inorder(): List<T> {
        val out = mutableListOf<T>()
        inorderRec(root, out)
        return out
    }

    /** Pre-order traversal: root → left → right. */
    fun preorder(): List<T> {
        val out = mutableListOf<T>()
        preorderRec(root, out)
        return out
    }

    /** Post-order traversal: left → right → root. */
    fun postorder(): List<T> {
        val out = mutableListOf<T>()
        postorderRec(root, out)
        return out
    }

    /** Level-order traversal (BFS): layer by layer, left to right. */
    fun levelOrder(): List<T> {
        val out = mutableListOf<T>()
        val root = this.root ?: return out
        val queue: ArrayDeque<Node> = ArrayDeque()
        queue.add(root)
        while (queue.isNotEmpty()) {
            val node = queue.removeFirst()
            out.add(node.value)
            node.left?.let  { queue.add(it) }
            node.right?.let { queue.add(it) }
        }
        return out
    }

    // =========================================================================
    // Structural queries
    // =========================================================================

    /** Height of the tree. Empty → -1; single node → 0. */
    fun height(): Int = heightRec(root)

    /** Total number of nodes. */
    val size: Int get() = sizeRec(root)

    /** True if the tree contains no nodes. */
    val isEmpty: Boolean get() = root == null

    // =========================================================================
    // Array projection
    // =========================================================================

    /**
     * Project the tree into a level-order array of size `2^(h+1) - 1` with
     * null for absent nodes. The inverse of [fromLevelOrder].
     *
     * Empty tree → empty list.
     */
    fun toArray(): List<T?> {
        val h = height()
        if (h < 0) return emptyList()
        val capacity = (1 shl (h + 1)) - 1
        val result = arrayOfNulls<Any?>(capacity)
        fillArrayRec(root, 0, result)
        @Suppress("UNCHECKED_CAST")
        return result.toList() as List<T?>
    }

    /**
     * Render the tree as a multi-line ASCII string with box-drawing connectors.
     *
     * Example for `[1, 2, 3, 4, 5, null, 6]`:
     * ```
     * `-- 1
     *     |-- 2
     *     |   |-- 4
     *     |   `-- 5
     *     `-- 3
     *         `-- 6
     * ```
     */
    fun toAscii(): String {
        val root = this.root ?: return ""
        val lines = mutableListOf<String>()
        renderAsciiRec(root, "", isTail = true, lines)
        return lines.joinToString("\n")
    }

    // =========================================================================
    // Object overrides
    // =========================================================================

    override fun toString(): String = "BinaryTree(root=${root?.value}, size=$size)"

    // =========================================================================
    // Private helpers
    // =========================================================================

    private fun buildFromLevelOrder(values: List<T?>, index: Int): Node? {
        if (index >= values.size) return null
        val value = values[index] ?: return null
        val node = Node(value)
        node.left  = buildFromLevelOrder(values, 2 * index + 1)
        node.right = buildFromLevelOrder(values, 2 * index + 2)
        return node
    }

    private fun findRec(node: Node?, value: T): Node? {
        if (node == null) return null
        if (node.value == value) return node
        return findRec(node.left, value) ?: findRec(node.right, value)
    }

    private fun isFullRec(node: Node?): Boolean {
        if (node == null) return true
        if (node.left == null && node.right == null) return true
        if (node.left == null || node.right == null) return false
        return isFullRec(node.left) && isFullRec(node.right)
    }

    private fun inorderRec(node: Node?, out: MutableList<T>) {
        if (node == null) return
        inorderRec(node.left, out)
        out.add(node.value)
        inorderRec(node.right, out)
    }

    private fun preorderRec(node: Node?, out: MutableList<T>) {
        if (node == null) return
        out.add(node.value)
        preorderRec(node.left, out)
        preorderRec(node.right, out)
    }

    private fun postorderRec(node: Node?, out: MutableList<T>) {
        if (node == null) return
        postorderRec(node.left, out)
        postorderRec(node.right, out)
        out.add(node.value)
    }

    private fun heightRec(node: Node?): Int {
        if (node == null) return -1
        return 1 + maxOf(heightRec(node.left), heightRec(node.right))
    }

    private fun sizeRec(node: Node?): Int {
        if (node == null) return 0
        return 1 + sizeRec(node.left) + sizeRec(node.right)
    }

    private fun fillArrayRec(node: Node?, index: Int, out: Array<Any?>) {
        if (node == null || index >= out.size) return
        out[index] = node.value
        fillArrayRec(node.left,  2 * index + 1, out)
        fillArrayRec(node.right, 2 * index + 2, out)
    }

    private fun renderAsciiRec(node: Node, prefix: String, isTail: Boolean, lines: MutableList<String>) {
        val connector = if (isTail) "`-- " else "|-- "
        lines.add("$prefix$connector${node.value}")
        val children = listOfNotNull(node.left, node.right)
        val nextPrefix = prefix + if (isTail) "    " else "|   "
        children.forEachIndexed { i, child ->
            renderAsciiRec(child, nextPrefix, i + 1 == children.size, lines)
        }
    }
}
