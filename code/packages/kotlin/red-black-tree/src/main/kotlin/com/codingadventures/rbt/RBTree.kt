// ============================================================================
// RBTree.kt — Red-Black Tree (Left-Leaning, Purely Functional)
// ============================================================================
//
// Idiomatic Kotlin port of the Java red-black-tree package (DT09).
//
// Uses Sedgewick's Left-Leaning Red-Black (LLRB) algorithm for both
// insertion and deletion, implemented as a purely functional (immutable)
// data structure — every mutating operation returns a NEW tree.
//
// ─────────────────────────────────────────────────────────────────────────────
// Kotlin Idioms vs Java
// ─────────────────────────────────────────────────────────────────────────────
//
//   • `enum class Color` instead of Java's nested enum
//   • `data class Node` with `copy()` for producing modified variants
//   • `val` properties on Node (size, height, isRed) for ergonomic access
//   • Companion object factory for `RBTree.empty()`
//   • Extension functions would clutter the sealed type; standalone functions
//     inside the companion are preferred instead
//   • `when` expressions for pattern matching in balance/fixUp
//   • `require()` / `check()` for preconditions
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.rbt
// ============================================================================

package com.codingadventures.rbt

// ─────────────────────────────────────────────────────────────────────────────
// Color
// ─────────────────────────────────────────────────────────────────────────────

enum class Color { RED, BLACK }

/** Toggle a Color. */
fun Color.toggle(): Color = if (this == Color.RED) Color.BLACK else Color.RED

// ─────────────────────────────────────────────────────────────────────────────
// Node
// ─────────────────────────────────────────────────────────────────────────────
//
// Immutable node. Kotlin data class gives us `copy()` for free, which is
// exactly what we need for a functional implementation.

data class Node(
    val value: Int,
    val color: Color,
    val left: Node? = null,
    val right: Node? = null
) {
    /** True if this node is RED. (Null nodes are always considered BLACK.) */
    val isRed: Boolean get() = color == Color.RED
}

/** Null-safe red check — null is treated as BLACK (Rule 3). */
fun Node?.isRed(): Boolean = this != null && this.color == Color.RED

/** Flip the color of this node (does NOT touch children). */
fun Node.withColor(c: Color): Node = if (c == color) this else copy(color = c)

// ─────────────────────────────────────────────────────────────────────────────
// RBTree
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A purely functional Left-Leaning Red-Black Tree (DT09).
 *
 * Elements are integers. Operations returning a tree produce new instances;
 * the original is never mutated.
 */
class RBTree private constructor(val root: Node?) {

    companion object {
        /** Return an empty Red-Black tree. */
        fun empty(): RBTree = RBTree(null)

        // ─── LLRB Structural Helpers ──────────────────────────────────────────

        /**
         * Rotate left: right child becomes the new root.
         *
         * ```
         *   h              x
         *  / \            / \
         * a   x    →     h   c
         *    / \        / \
         *   b   c      a   b
         * ```
         * x inherits h's color; h becomes RED.
         */
        internal fun rotateLeft(h: Node): Node {
            val x = h.right!!
            return Node(x.value, h.color,
                Node(h.value, Color.RED, h.left, x.left),
                x.right)
        }

        /**
         * Rotate right: left child becomes the new root.
         *
         * ```
         *     h              x
         *    / \            / \
         *   x   c    →     a   h
         *  / \                / \
         * a   b              b   c
         * ```
         * x inherits h's color; h becomes RED.
         */
        internal fun rotateRight(h: Node): Node {
            val x = h.left!!
            return Node(x.value, h.color,
                x.left,
                Node(h.value, Color.RED, x.right, h.right))
        }

        /**
         * Flip the colors of [h] and both its children.
         *
         * Both children must be non-null. Used to:
         * - Split 4-nodes on the way up (BLACK h, RED children → RED h, BLACK children)
         * - Borrow during deletion (toggle all three)
         */
        internal fun flipColors(h: Node): Node {
            val newLeft  = h.left?.withColor(h.left.color.toggle())
            val newRight = h.right?.withColor(h.right.color.toggle())
            return Node(h.value, h.color.toggle(), newLeft, newRight)
        }

        /**
         * Restore LLRB invariant after a structural change (called bottom-up).
         *
         * Three steps:
         * 1. Right child is RED and left is NOT red → rotateLeft (fix right-lean)
         * 2. Left child AND left-left grandchild are RED → rotateRight (fix 4-node)
         * 3. Both children RED → flipColors (split 4-node)
         */
        internal fun fixUp(h: Node): Node {
            var n = h
            if (n.right.isRed() && !n.left.isRed()) n = rotateLeft(n)
            if (n.left.isRed() && n.left?.left.isRed()) n = rotateRight(n)
            if (n.left.isRed() && n.right.isRed()) n = flipColors(n)
            return n
        }

        // ─── Insert helper (LLRB) ─────────────────────────────────────────────

        internal fun insertHelper(h: Node?, value: Int): Node {
            if (h == null) return Node(value, Color.RED)
            return when {
                value < h.value -> fixUp(h.copy(left  = insertHelper(h.left,  value)))
                value > h.value -> fixUp(h.copy(right = insertHelper(h.right, value)))
                else            -> h  // duplicate: no-op
            }
        }

        // ─── Delete helpers (LLRB) ────────────────────────────────────────────

        /**
         * Make h.left or h.left.left RED by borrowing from the right sibling
         * or merging.
         *
         * Precondition: h is RED, h.left and h.left.left are both BLACK.
         */
        internal fun moveRedLeft(h: Node): Node {
            var n = flipColors(h)
            // If right sibling has a left-leaning red child, borrow it.
            if (n.right != null && n.right.left.isRed()) {
                n = n.copy(right = rotateRight(n.right))
                n = rotateLeft(n)
                n = flipColors(n)
            }
            return n
        }

        /**
         * Make h.right or h.right.left RED by borrowing from the left sibling
         * or merging.
         *
         * Precondition: h is RED, h.right and h.right.left are both BLACK.
         */
        internal fun moveRedRight(h: Node): Node {
            var n = flipColors(h)
            if (n.left != null && n.left.left.isRed()) {
                n = rotateRight(n)
                n = flipColors(n)
            }
            return n
        }

        /** Return the minimum value in the subtree rooted at [h]. */
        internal fun minValue(h: Node): Int {
            var n = h
            while (n.left != null) n = n.left!!
            return n.value
        }

        /** Delete the minimum node in the subtree. Returns null if now empty. */
        internal fun deleteMin(h: Node): Node? {
            if (h.left == null) return null  // h is the minimum
            var n = h
            if (!n.left.isRed() && !n.left?.left.isRed()) {
                n = moveRedLeft(n)
            }
            val newLeft = n.left?.let { deleteMin(it) }
            return fixUp(n.copy(left = newLeft))
        }

        /** Core recursive delete. Returns null when the subtree becomes empty. */
        internal fun deleteHelper(h: Node, value: Int): Node? {
            if (value < h.value) {
                // ─ Go LEFT ─────────────────────────────────────────────────────
                var n = h
                if (!n.left.isRed() && !n.left?.left.isRed()) {
                    n = moveRedLeft(n)
                }
                val newLeft = n.left?.let { deleteHelper(it, value) }
                return fixUp(n.copy(left = newLeft))
            } else {
                // ─ Go RIGHT (or delete here) ────────────────────────────────────
                var n = h
                if (n.left.isRed()) n = rotateRight(n)
                // Found it and no right child → delete
                if (value == n.value && n.right == null) return null
                // Ensure right side has a red to work with
                if (!n.right.isRed() && !n.right?.left.isRed()) {
                    n = moveRedRight(n)
                }
                if (value == n.value) {
                    // Replace with in-order successor, delete successor from right
                    val successorVal = minValue(n.right!!)
                    val newRight = n.right.let { deleteMin(it) }
                    return fixUp(Node(successorVal, n.color, n.left, newRight))
                } else {
                    val newRight = n.right?.let { deleteHelper(it, value) }
                    return fixUp(n.copy(right = newRight))
                }
            }
        }

        // ─── Validation helper ────────────────────────────────────────────────

        /**
         * Recursively verify RB invariants. Returns black-height or -1 on violation.
         * Null leaves count as 1 (they are black per Rule 3).
         */
        internal fun checkNode(node: Node?): Int {
            if (node == null) return 1
            // Rule 4: red node must not have red children
            if (node.color == Color.RED) {
                if (node.left.isRed() || node.right.isRed()) return -1
            }
            val leftBH  = checkNode(node.left)
            val rightBH = checkNode(node.right)
            if (leftBH == -1 || rightBH == -1) return -1
            if (leftBH != rightBH) return -1
            return leftBH + if (node.color == Color.BLACK) 1 else 0
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insert
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return a new [RBTree] with [value] inserted.
     * Duplicates are silently ignored (no-op).
     */
    fun insert(value: Int): RBTree {
        val newRoot = insertHelper(root, value)
        // Rule 2: root must be BLACK
        return RBTree(newRoot.withColor(Color.BLACK))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Delete
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return a new [RBTree] with [value] removed.
     * If [value] is not present, returns the unchanged tree.
     */
    fun delete(value: Int): RBTree {
        if (!contains(value)) return this
        val newRoot = root?.let { deleteHelper(it, value) }
        return RBTree(newRoot?.withColor(Color.BLACK))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Search / Contains
    // ─────────────────────────────────────────────────────────────────────────

    /** Return `true` if [value] is in the tree. O(log n). */
    fun contains(value: Int): Boolean {
        var node = root
        while (node != null) {
            node = when {
                value < node.value -> node.left
                value > node.value -> node.right
                else               -> return true
            }
        }
        return false
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Min / Max
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the minimum value, or `null` if empty. */
    val min: Int? get() {
        var n = root ?: return null
        while (n.left != null) n = n.left!!
        return n.value
    }

    /** Return the maximum value, or `null` if empty. */
    val max: Int? get() {
        var n = root ?: return null
        while (n.right != null) n = n.right!!
        return n.value
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Predecessor / Successor
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the largest value strictly less than [value], or `null`. */
    fun predecessor(value: Int): Int? {
        var best: Int? = null
        var n = root
        while (n != null) {
            n = if (value > n.value) { best = n.value; n.right }
                else n.left
        }
        return best
    }

    /** Return the smallest value strictly greater than [value], or `null`. */
    fun successor(value: Int): Int? {
        var best: Int? = null
        var n = root
        while (n != null) {
            n = if (value < n.value) { best = n.value; n.left }
                else n.right
        }
        return best
    }

    // ─────────────────────────────────────────────────────────────────────────
    // kthSmallest
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the k-th smallest element (1-indexed).
     *
     * @throws NoSuchElementException if k is out of range.
     */
    fun kthSmallest(k: Int): Int {
        val sorted = toSortedList()
        require(k in 1..sorted.size) {
            "k=$k out of range; tree has ${sorted.size} elements"
        }
        return sorted[k - 1]
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sorted traversal
    // ─────────────────────────────────────────────────────────────────────────

    /** Return all elements in ascending order. */
    fun toSortedList(): List<Int> {
        val result = mutableListOf<Int>()
        fun inOrder(node: Node?) {
            if (node == null) return
            inOrder(node.left)
            result.add(node.value)
            inOrder(node.right)
        }
        inOrder(root)
        return result
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Validation
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Verify all 5 Red-Black invariants:
     * 1. Every node is RED or BLACK.
     * 2. Root is BLACK.
     * 3. Null leaves are BLACK.
     * 4. Red nodes have only BLACK children.
     * 5. All root-to-NIL paths have the same black-height.
     */
    fun isValidRB(): Boolean {
        if (root == null) return true
        if (root.color != Color.BLACK) return false  // Rule 2
        return checkNode(root) != -1
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Metrics
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the black-height of the root (number of BLACK nodes on any path
     * from root down to NIL, counting each black node — NOT the NIL itself
     * in this count; consistent with the Java implementation).
     */
    val blackHeight: Int get() {
        fun bh(n: Node?): Int {
            if (n == null) return 0
            return bh(n.left) + if (n.color == Color.BLACK) 1 else 0
        }
        return bh(root)
    }

    /** Number of elements in the tree. */
    val size: Int get() {
        fun sz(n: Node?): Int = if (n == null) 0 else 1 + sz(n.left) + sz(n.right)
        return sz(root)
    }

    /** Height of the tree (0 for empty, 1 for a single root). */
    val height: Int get() {
        fun ht(n: Node?): Int = if (n == null) 0 else 1 + maxOf(ht(n.left), ht(n.right))
        return ht(root)
    }

    /** `true` if the tree has no elements. */
    val isEmpty: Boolean get() = root == null

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    override fun toString(): String =
        "RBTree(size=$size, height=$height, blackHeight=$blackHeight)"
}
