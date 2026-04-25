// ============================================================================
// Treap.kt — Randomized Binary Search Tree with Heap Priorities (DT10)
// ============================================================================
//
// Idiomatic Kotlin port of the Java treap package (DT10).
//
// A treap is a BST+heap hybrid: each node has a key (BST ordering) and a
// random priority (heap ordering). Together they uniquely determine the tree's
// shape, giving O(log n) expected height.
//
// ─────────────────────────────────────────────────────────────────────────────
// Kotlin idioms vs Java
// ─────────────────────────────────────────────────────────────────────────────
//
//   • `data class Node` with `copy()` for functional updates
//   • Destructuring declaration for split: `val (left, right) = splitNode(...)`
//   • `val` computed properties on Treap (size, height, isEmpty, min, max)
//   • Companion object with internal helpers
//   • `Pair<Node?, Node?>` instead of a SplitResult record
//   • `kotlin.random.Random` instead of java.util.Random
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.treap
// ============================================================================

package com.codingadventures.treap

import kotlin.random.Random

// ─────────────────────────────────────────────────────────────────────────────
// Node
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Immutable treap node with a BST key and a heap priority.
 *
 * Higher priority = closer to root (max-heap on priorities).
 */
data class Node(
    val key: Int,
    val priority: Double,
    val left: Node? = null,
    val right: Node? = null
)

// ─────────────────────────────────────────────────────────────────────────────
// Treap
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A purely functional Treap (DT10).
 *
 * All mutating operations return new [Treap] instances. The original is never
 * mutated.
 */
class Treap private constructor(
    val root: Node?,
    private val rng: Random
) {

    companion object {

        // ─── Factories ──────────────────────────────────────────────────────

        /** Return an empty treap with a non-deterministic random source. */
        fun empty(): Treap = Treap(null, Random.Default)

        /** Return an empty treap seeded with [seed] (deterministic). */
        fun withSeed(seed: Long): Treap = Treap(null, Random(seed))

        /** Construct a Treap from a raw root node and random source. */
        fun fromRoot(root: Node?, rng: Random): Treap = Treap(root, rng)

        // ─── Split ──────────────────────────────────────────────────────────
        //
        // splitNode(node, key) → Pair(left, right)
        //   left:  all keys ≤ key (inclusive)
        //   right: all keys > key
        //
        // Walk the BST: if node.key ≤ key, node goes LEFT; recurse right.
        //               if node.key > key, node goes RIGHT; recurse left.

        internal fun splitNode(node: Node?, key: Int): Pair<Node?, Node?> {
            if (node == null) return Pair(null, null)
            return if (node.key <= key) {
                val (rl, rr) = splitNode(node.right, key)
                Pair(node.copy(right = rl), rr)
            } else {
                val (ll, lr) = splitNode(node.left, key)
                Pair(ll, node.copy(left = lr))
            }
        }

        /**
         * Strict split: left has all keys < [key], right has all keys >= [key].
         */
        internal fun splitStrict(node: Node?, key: Int): Pair<Node?, Node?> {
            if (node == null) return Pair(null, null)
            return if (node.key < key) {
                val (rl, rr) = splitStrict(node.right, key)
                Pair(node.copy(right = rl), rr)
            } else {
                val (ll, lr) = splitStrict(node.left, key)
                Pair(ll, node.copy(left = lr))
            }
        }

        // ─── Merge ──────────────────────────────────────────────────────────
        //
        // mergeNodes(left, right) → Node?
        //
        // Precondition: all keys in left < all keys in right.
        //
        // The node with the higher priority becomes the root.
        // Its "inner" subtree is merged recursively with the other treap.

        internal fun mergeNodes(left: Node?, right: Node?): Node? {
            if (left == null)  return right
            if (right == null) return left
            return if (left.priority > right.priority) {
                left.copy(right = mergeNodes(left.right, right))
            } else {
                right.copy(left = mergeNodes(left, right.left))
            }
        }

        // ─── Validation helper ───────────────────────────────────────────────

        internal fun checkNode(
            node: Node?,
            minKey: Int = Int.MIN_VALUE,
            maxKey: Int = Int.MAX_VALUE,
            maxPriority: Double = Double.MAX_VALUE
        ): Boolean {
            if (node == null) return true
            if (node.key <= minKey || node.key >= maxKey) return false
            if (node.priority > maxPriority) return false
            return checkNode(node.left,  minKey,    node.key, node.priority) &&
                   checkNode(node.right, node.key, maxKey,   node.priority)
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insert
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return a new [Treap] with [key] inserted using a random priority.
     * Duplicates are silently ignored.
     */
    fun insert(key: Int): Treap {
        if (contains(key)) return this
        return insertWithPriority(key, rng.nextDouble())
    }

    /**
     * Return a new [Treap] with [key] inserted at explicit [priority].
     * Useful for deterministic testing.
     */
    fun insertWithPriority(key: Int, priority: Double): Treap {
        if (contains(key)) return this
        val (left, right) = splitStrict(root, key)
        val singleton = Node(key, priority)
        val merged = mergeNodes(mergeNodes(left, singleton), right)
        return Treap(merged, rng)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Delete
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return a new [Treap] with [key] removed.
     * If [key] is absent, returns the unchanged treap.
     */
    fun delete(key: Int): Treap {
        if (!contains(key)) return this
        val (leftPart, rest) = splitStrict(root, key)
        val (_, rightPart) = splitNode(rest, key)
        return Treap(mergeNodes(leftPart, rightPart), rng)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Split (public)
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Split this treap into two: left has all keys ≤ [key], right has all
     * keys > [key].
     */
    fun split(key: Int): Pair<Treap, Treap> {
        val (l, r) = splitNode(root, key)
        return Pair(Treap(l, rng), Treap(r, rng))
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Search / Contains
    // ─────────────────────────────────────────────────────────────────────────

    /** Return `true` if [key] is in the treap. O(log n) expected. */
    fun contains(key: Int): Boolean {
        var node = root
        while (node != null) {
            node = when {
                key < node.key -> node.left
                key > node.key -> node.right
                else           -> return true
            }
        }
        return false
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Min / Max
    // ─────────────────────────────────────────────────────────────────────────

    /** Minimum key, or `null` if empty. */
    val min: Int? get() {
        var n = root ?: return null
        while (n.left != null) n = n.left!!
        return n.key
    }

    /** Maximum key, or `null` if empty. */
    val max: Int? get() {
        var n = root ?: return null
        while (n.right != null) n = n.right!!
        return n.key
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Predecessor / Successor
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the largest key strictly less than [key], or `null`. */
    fun predecessor(key: Int): Int? {
        var best: Int? = null
        var n = root
        while (n != null) {
            n = if (key > n.key) { best = n.key; n.right }
                else n.left
        }
        return best
    }

    /** Return the smallest key strictly greater than [key], or `null`. */
    fun successor(key: Int): Int? {
        var best: Int? = null
        var n = root
        while (n != null) {
            n = if (key < n.key) { best = n.key; n.left }
                else n.right
        }
        return best
    }

    // ─────────────────────────────────────────────────────────────────────────
    // kthSmallest
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the k-th smallest key (1-indexed).
     *
     * @throws IllegalArgumentException if k is out of range.
     */
    fun kthSmallest(k: Int): Int {
        val sorted = toSortedList()
        require(k in 1..sorted.size) {
            "k=$k out of range; treap has ${sorted.size} elements"
        }
        return sorted[k - 1]
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sorted traversal
    // ─────────────────────────────────────────────────────────────────────────

    /** Return all keys in ascending order. */
    fun toSortedList(): List<Int> {
        val result = mutableListOf<Int>()
        fun inOrder(node: Node?) {
            if (node == null) return
            inOrder(node.left)
            result.add(node.key)
            inOrder(node.right)
        }
        inOrder(root)
        return result
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Validation
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return `true` if both BST and heap properties hold for the whole treap.
     *
     * BST property:  left.key < node.key < right.key (recursively)
     * Heap property: node.priority ≥ children's priorities
     */
    fun isValidTreap(): Boolean = checkNode(root)

    // ─────────────────────────────────────────────────────────────────────────
    // Metrics
    // ─────────────────────────────────────────────────────────────────────────

    /** Number of keys. */
    val size: Int get() {
        fun sz(n: Node?): Int = if (n == null) 0 else 1 + sz(n.left) + sz(n.right)
        return sz(root)
    }

    /** Height (0 = empty, 1 = single root). */
    val height: Int get() {
        fun ht(n: Node?): Int = if (n == null) 0 else 1 + maxOf(ht(n.left), ht(n.right))
        return ht(root)
    }

    /** `true` if no keys are stored. */
    val isEmpty: Boolean get() = root == null

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    override fun toString(): String = "Treap(size=$size, height=$height)"
}

// ─────────────────────────────────────────────────────────────────────────────
// Static merge (top-level function, since Kotlin companion can't have a method
// with the same name as an object function)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Merge two treaps into one. All keys in [left] must be less than all keys
 * in [right].
 */
fun mergeTreaps(left: Treap, right: Treap): Treap {
    val mergedRoot = Treap.mergeNodes(left.root, right.root)
    return Treap.fromRoot(mergedRoot, kotlin.random.Random.Default)
}
