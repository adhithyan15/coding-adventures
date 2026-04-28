// ============================================================================
// BTree.kt — Self-Balancing Multi-Way Search Tree
// ============================================================================
//
// A B-tree is a generalised search tree where each node can hold many keys
// instead of just one. This is the fundamental property that makes B-trees
// ideal for disk-based storage:
//
//   - A hard drive or SSD reads data in blocks (typically 4 KiB or more).
//   - If each tree node fills exactly one block, we minimise the number of
//     I/O operations needed to find a key.
//   - A B-tree with minimum degree t=50 has nodes holding 49–99 keys and
//     50–100 children. A height-4 tree covers 50^4 ≈ 6 MILLION pages with
//     just 4 block reads per lookup.
//   - SQLite, PostgreSQL, MySQL, and most filesystems use B-tree variants.
//
// ============================================================================
// Terminology: minimum degree t
// ============================================================================
//
//   Every B-tree is parameterised by an integer t ≥ 2:
//
//     - Every non-root node holds at least t-1 keys (never under-full).
//     - Every node holds at most 2t-1 keys (never over-full).
//     - A node with k keys has exactly k+1 children (if internal).
//
//   With t=2 (minimum), nodes hold 1–3 keys and 2–4 children. This is the
//   famous "2-3-4 tree."
//
// ============================================================================
// Insertion: proactive top-down splitting
// ============================================================================
//
//   When descending the tree to find the insertion point, we pro-actively
//   split any full node we encounter. By the time we reach the leaf, every
//   ancestor is guaranteed to have room for one more key — no backtracking.
//
//   If the root itself is full, we first create a new empty root, make the
//   old root its first child, and split it. The tree height grows by 1.
//
//   Split of a full node [k0, k1, k2, k3, k4] (t=3, max = 2t-1 = 5 keys):
//
//       parent: [... X ...]
//                    |
//              [k0 k1 k2 k3 k4]
//
//       After:  parent: [... X k2 ...]
//                            /       \
//                        [k0 k1]   [k3 k4]
//
//   The median key k2 (index t-1) moves UP to the parent; left and right
//   children each get t-1 keys. Children pointers are split analogously.
//
// ============================================================================
// Deletion: three cases
// ============================================================================
//
//   Deletion is the hard part. The invariant to maintain: every non-root node
//   must have at least t-1 keys after the deletion.
//
//   We "pre-fill" any node that is too thin (t-1 keys) BEFORE descending into
//   it. Two pre-fill strategies:
//
//     3a: A sibling has ≥ t keys → ROTATE a key through the parent to gift
//         one key to the thin node. No structural change — just a rotation.
//
//     3b: No sibling has a spare key → MERGE the thin node with a sibling and
//         the separator key from the parent. The parent loses one key; if the
//         root becomes empty, the tree shrinks.
//
//   Once the path is pre-filled, deletion falls into one of three cases:
//
//     Case 1: Key is in a leaf → simply remove it.
//
//     Case 2: Key is in an internal node x:
//       2a: Left child has ≥ t keys → replace key with its in-order predecessor
//           (rightmost key in left subtree), then recursively delete predecessor.
//       2b: Right child has ≥ t keys → use successor (leftmost of right subtree).
//       2c: Both have exactly t-1 keys → merge key + right child into left child,
//           then recursively delete key from the merged node.
//
//     Case 3: Key is not in this node → descend, pre-filling the child first.
//
// ============================================================================

package com.codingadventures.btree

/**
 * A self-balancing multi-way search tree (B-tree) mapping comparable keys
 * to arbitrary values.
 *
 * O(t·log_t n) for insert, delete, and search, where [t] is the minimum
 * degree and n is the number of keys. All leaves are at exactly the same
 * depth — the tree never becomes unbalanced.
 *
 * ```kotlin
 * val tree = BTree<Int, String>(t = 2)
 * tree.insert(5, "five")
 * tree.insert(3, "three")
 * tree.insert(7, "seven")
 *
 * tree.search(3)              // "three"
 * tree.contains(5)            // true
 * tree.minKey()               // 3
 * tree.maxKey()               // 7
 * tree.rangeQuery(3, 6)       // [(3,"three"), (5,"five")]
 * tree.height                 // 1
 *
 * tree.delete(3)
 * tree.contains(3)            // false
 * tree.size                   // 2
 * ```
 *
 * @param K the key type; must be [Comparable]
 * @param V the value type
 * @param t the minimum degree; must be ≥ 2
 */
class BTree<K : Comparable<K>, V>(val t: Int = 2) {

    init {
        require(t >= 2) { "Minimum degree t must be >= 2, got $t" }
    }

    // =========================================================================
    // Inner class: Node
    // =========================================================================

    /**
     * A single node in the B-tree.
     *
     * A node is like a mini sorted array: it holds [keys][0..n-1] in ascending
     * order and, for an internal node, n+1 child pointers in [children][0..n].
     *
     * Think of it as an airport departure board: it lists destinations (keys)
     * in order, and the gaps between destinations indicate which child (gate)
     * leads to flights in that range.
     *
     * Invariants (for minimum degree t, and this is not the root):
     * - t-1 ≤ keys.size ≤ 2t-1
     * - if isLeaf: children.isEmpty() else children.size == keys.size + 1
     * - keys is strictly sorted in ascending order
     */
    inner class Node(var isLeaf: Boolean) {
        val keys:     MutableList<K> = mutableListOf()
        val values:   MutableList<V> = mutableListOf()
        val children: MutableList<Node> = mutableListOf()

        /** Return true if this node is at maximum capacity (2t-1 keys). */
        fun isFull(): Boolean = keys.size == 2 * t - 1

        /**
         * Binary-search for the leftmost index i such that keys[i] >= key.
         *
         * If key is present, this is its index. If absent, this is the index of
         * the child to descend into.
         *
         * Example: keys = [10, 20, 30], findKeyIndex(15) → 1.
         * (Descend into children[1], which covers keys in (10, 20).)
         */
        fun findKeyIndex(key: K): Int {
            var lo = 0; var hi = keys.size
            while (lo < hi) {
                val mid = (lo + hi) ushr 1
                if (keys[mid].compareTo(key) < 0) lo = mid + 1 else hi = mid
            }
            return lo
        }
    }

    // =========================================================================
    // Fields
    // =========================================================================

    private var root: Node = Node(isLeaf = true)

    /** Number of key-value pairs currently in the tree. */
    var size: Int = 0
        private set

    /** True when the tree contains no key-value pairs. */
    val isEmpty: Boolean get() = size == 0

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Insert [key] with associated [value].
     *
     * If [key] already exists, its value is updated in place.
     *
     * Algorithm (CLRS B-TREE-INSERT):
     * 1. If the root is full, split it: create a new root, make the old root
     *    its first child, split that child. Height increases by 1.
     * 2. Call insertNonfull on the (now non-full) root.
     *
     * @throws IllegalArgumentException if key is null (Kotlin will enforce
     *   non-null at the call site for non-nullable K, but be explicit)
     */
    fun insert(key: K, value: V) {
        val r = root
        if (r.isFull()) {
            // Root is full — grow the tree upward
            val newRoot = Node(isLeaf = false)
            newRoot.children.add(r)
            splitChild(newRoot, 0)
            root = newRoot
            if (insertNonfull(newRoot, key, value)) size++
        } else {
            if (insertNonfull(r, key, value)) size++
        }
    }

    /**
     * Remove [key] from the B-tree.
     *
     * After deletion, if the root is left with no keys (due to a merge of
     * its two children), the first child becomes the new root and the tree
     * shrinks in height.
     *
     * @throws NoSuchElementException if the key is not present
     */
    fun delete(key: K) {
        if (!containsRec(root, key)) {
            throw NoSuchElementException("Key not found: $key")
        }
        deleteRec(root, key)
        size--
        // If root is now keyless but has a child, shrink the tree
        if (root.keys.isEmpty() && root.children.isNotEmpty()) {
            root = root.children[0]
        }
    }

    /**
     * Return the value associated with [key], or null if absent.
     */
    fun search(key: K): V? = searchRec(root, key)

    /** Return true if [key] is present in the tree. */
    fun contains(key: K): Boolean = containsRec(root, key)

    /**
     * Return the smallest key in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    fun minKey(): K {
        if (size == 0) throw NoSuchElementException("Tree is empty")
        return minNode(root).keys.first()
    }

    /**
     * Return the largest key in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    fun maxKey(): K {
        if (size == 0) throw NoSuchElementException("Tree is empty")
        var node = root
        while (!node.isLeaf) node = node.children.last()
        return node.keys.last()
    }

    /**
     * Return all (key, value) pairs where low <= key <= high, in ascending
     * key order.
     *
     * @param low  the inclusive lower bound
     * @param high the inclusive upper bound
     */
    fun rangeQuery(low: K, high: K): List<Pair<K, V>> {
        val result = mutableListOf<Pair<K, V>>()
        for ((k, v) in inorder()) {
            if (k.compareTo(high) > 0) break
            if (k.compareTo(low) >= 0) result.add(k to v)
        }
        return result
    }

    /**
     * Return a list of all (key, value) pairs in ascending key order.
     *
     * The in-order traversal generalises BST in-order to B-trees: for a node
     * with keys [k0, k1, k2] and children [c0, c1, c2, c3], we yield all from
     * c0, then k0, then all from c1, then k1, and so on.
     */
    fun inorder(): List<Pair<K, V>> {
        val result = mutableListOf<Pair<K, V>>()
        collectInorder(root, result)
        return result
    }

    /**
     * Return the height of the tree.
     *
     * A single-node tree (leaf) has height 0; each additional level adds 1.
     * All paths from root to leaf have exactly this length — the key B-tree
     * invariant.
     */
    val height: Int
        get() {
            var node = root; var h = 0
            while (!node.isLeaf) { node = node.children[0]; h++ }
            return h
        }

    /**
     * Validate all B-tree structural invariants.
     *
     * Invariants checked:
     * 1. Key count bounds: t-1 ≤ keys ≤ 2t-1 for non-root nodes
     * 2. Root has at least 1 key (unless tree is empty)
     * 3. Keys within each node are strictly increasing
     * 4. Keys respect BST ordering between parents and children
     * 5. Internal nodes have exactly keys.size+1 children
     * 6. All leaves are at the same depth
     *
     * @return true if the tree is structurally valid
     */
    fun isValid(): Boolean {
        if (size == 0) return true
        val leafDepth = intArrayOf(-1)
        return validate(root, null, null, 0, leafDepth, isRoot = true)
    }

    override fun toString(): String = "BTree(t=$t, size=$size, height=$height)"

    // =========================================================================
    // Private helpers — insertion
    // =========================================================================

    /**
     * Split parent.children[childIndex], which must be full.
     *
     * The median key (index t-1) is promoted to the parent. The left child
     * retains keys[0..t-2]; the right child gets keys[t..2t-2]. Children
     * (if the node is internal) are split the same way.
     *
     * This is O(t) work — we copy t-1 keys and (for internal nodes) t pointers.
     */
    private fun splitChild(parent: Node, childIndex: Int) {
        val child = parent.children[childIndex]
        val right = Node(isLeaf = child.isLeaf)
        val mid   = t - 1   // index of the median key in child.keys

        // Promote median to parent
        parent.keys.add(childIndex, child.keys[mid])
        parent.values.add(childIndex, child.values[mid])
        parent.children.add(childIndex + 1, right)

        // Right node: upper half of keys/values/children
        right.keys.addAll(child.keys.subList(mid + 1, child.keys.size))
        right.values.addAll(child.values.subList(mid + 1, child.values.size))
        if (!child.isLeaf) {
            right.children.addAll(child.children.subList(t, child.children.size))
            // Keep only first t children in child (left)
            while (child.children.size > t) child.children.removeAt(child.children.size - 1)
        }

        // Left node: lower half (trim to first t-1 keys)
        while (child.keys.size > mid)   child.keys.removeAt(child.keys.size - 1)
        while (child.values.size > mid) child.values.removeAt(child.values.size - 1)
    }

    /**
     * Insert [key] into the subtree rooted at [node], assuming [node] is NOT
     * full.
     *
     * Returns true if this was a new key (size should increase), false if an
     * existing key was updated.
     */
    private fun insertNonfull(node: Node, key: K, value: V): Boolean {
        var i = node.findKeyIndex(key)

        // Check for exact match at this node
        if (i < node.keys.size && node.keys[i].compareTo(key) == 0) {
            node.values[i] = value   // update in place
            return false
        }

        if (node.isLeaf) {
            // Sorted insertion at position i
            node.keys.add(i, key)
            node.values.add(i, value)
            return true
        }

        // Internal: pre-split child[i] if full (proactive top-down splitting)
        if (node.children[i].isFull()) {
            splitChild(node, i)
            // After split, node.keys[i] is the promoted median
            val cmp = key.compareTo(node.keys[i])
            when {
                cmp == 0 -> { node.values[i] = value; return false }
                cmp > 0  -> i++   // descend into the right half
            }
        }
        return insertNonfull(node.children[i], key, value)
    }

    // =========================================================================
    // Private helpers — search
    // =========================================================================

    private fun searchRec(node: Node, key: K): V? {
        val i = node.findKeyIndex(key)
        if (i < node.keys.size && node.keys[i].compareTo(key) == 0) return node.values[i]
        if (node.isLeaf) return null
        return searchRec(node.children[i], key)
    }

    private fun containsRec(node: Node, key: K): Boolean {
        val i = node.findKeyIndex(key)
        if (i < node.keys.size && node.keys[i].compareTo(key) == 0) return true
        if (node.isLeaf) return false
        return containsRec(node.children[i], key)
    }

    // =========================================================================
    // Private helpers — min/max
    // =========================================================================

    private fun minNode(node: Node): Node {
        var n = node
        while (!n.isLeaf) n = n.children[0]
        return n
    }

    // =========================================================================
    // Private helpers — deletion
    // =========================================================================

    /**
     * Recursively delete [key] from the subtree rooted at [node].
     *
     * Precondition: [node] has at least t keys (guaranteed by ensureMinKeys on
     * every descent), unless [node] is the root.
     */
    private fun deleteRec(node: Node, key: K) {
        val i     = node.findKeyIndex(key)
        val found = i < node.keys.size && node.keys[i].compareTo(key) == 0

        if (found) {
            if (node.isLeaf) {
                // Case 1: key is in a leaf — simply remove it
                node.keys.removeAt(i)
                node.values.removeAt(i)
            } else {
                val leftChild  = node.children[i]
                val rightChild = node.children[i + 1]

                when {
                    leftChild.keys.size >= t -> {
                        // Case 2a: left child has spare key — use predecessor
                        val predNode = maxNode(leftChild)
                        val predKey  = predNode.keys.last()
                        val predVal  = predNode.values.last()
                        node.keys[i]   = predKey
                        node.values[i] = predVal
                        deleteRec(leftChild, predKey)
                    }
                    rightChild.keys.size >= t -> {
                        // Case 2b: right child has spare key — use successor
                        val succNode = minNode(rightChild)
                        val succKey  = succNode.keys.first()
                        val succVal  = succNode.values.first()
                        node.keys[i]   = succKey
                        node.values[i] = succVal
                        deleteRec(rightChild, succKey)
                    }
                    else -> {
                        // Case 2c: both have t-1 keys — merge
                        val merged = mergeChildren(node, i)
                        deleteRec(merged, key)
                    }
                }
            }
        } else {
            // Key not in this node; descend, pre-filling if needed (Case 3)
            if (node.isLeaf) return   // key not present (shouldn't reach here)
            val newIdx = ensureMinKeys(node, i)
            deleteRec(node.children[newIdx], key)
        }
    }

    private fun maxNode(node: Node): Node {
        var n = node
        while (!n.isLeaf) n = n.children.last()
        return n
    }

    /**
     * Merge parent.children[leftIdx] with parent.children[leftIdx+1], pulling
     * down the separator key from the parent.
     *
     * The merged node = left.keys + [separator] + right.keys.
     * The separator is removed from the parent, and the right child pointer is
     * removed from the parent's children list.
     *
     * Returns the merged node (which is at parent.children[leftIdx]).
     */
    private fun mergeChildren(parent: Node, leftIdx: Int): Node {
        val left  = parent.children[leftIdx]
        val right = parent.children[leftIdx + 1]

        // Pull down separator from parent
        left.keys.add(parent.keys.removeAt(leftIdx))
        left.values.add(parent.values.removeAt(leftIdx))
        parent.children.removeAt(leftIdx + 1)

        // Append right's keys/values/children to left
        left.keys.addAll(right.keys)
        left.values.addAll(right.values)
        if (!left.isLeaf) left.children.addAll(right.children)

        return left
    }

    /**
     * Ensure that parent.children[childIdx] has at least t keys.
     *
     * If the child is already fat enough, returns childIdx unchanged.
     *
     * Otherwise:
     * - Case 3a: Borrow from a sibling with ≥ t keys (rotate through parent).
     * - Case 3b: Merge with a sibling (pulls separator down from parent).
     *
     * @return the (possibly shifted) child index to descend into
     */
    private fun ensureMinKeys(parent: Node, childIdx: Int): Int {
        val child = parent.children[childIdx]
        if (child.keys.size >= t) return childIdx

        // Try to borrow from left sibling
        if (childIdx > 0) {
            val leftSib = parent.children[childIdx - 1]
            if (leftSib.keys.size >= t) {
                // Rotate right: pull parent separator down to child front
                child.keys.add(0, parent.keys[childIdx - 1])
                child.values.add(0, parent.values[childIdx - 1])
                // Move left sibling's last key up to parent
                val ls = leftSib.keys.size - 1
                parent.keys[childIdx - 1]   = leftSib.keys.removeAt(ls)
                parent.values[childIdx - 1] = leftSib.values.removeAt(ls)
                // Move left sibling's last child to child's first child
                if (!leftSib.isLeaf) {
                    child.children.add(0, leftSib.children.removeAt(leftSib.children.size - 1))
                }
                return childIdx
            }
        }

        // Try to borrow from right sibling
        if (childIdx < parent.children.size - 1) {
            val rightSib = parent.children[childIdx + 1]
            if (rightSib.keys.size >= t) {
                // Rotate left: pull parent separator down to child end
                child.keys.add(parent.keys[childIdx])
                child.values.add(parent.values[childIdx])
                // Move right sibling's first key up to parent
                parent.keys[childIdx]   = rightSib.keys.removeAt(0)
                parent.values[childIdx] = rightSib.values.removeAt(0)
                // Move right sibling's first child to child's last child
                if (!rightSib.isLeaf) {
                    child.children.add(rightSib.children.removeAt(0))
                }
                return childIdx
            }
        }

        // Must merge (Case 3b): no sibling has a spare key
        return if (childIdx > 0) {
            mergeChildren(parent, childIdx - 1)
            childIdx - 1   // merged node is now at childIdx - 1
        } else {
            mergeChildren(parent, childIdx)
            childIdx       // merged node stays at childIdx
        }
    }

    // =========================================================================
    // Private helpers — in-order traversal
    // =========================================================================

    /**
     * Collect (key, value) pairs in ascending order into [result].
     *
     * For a node with keys [k0, k1, k2] and children [c0, c1, c2, c3]:
     * traverse c0, emit k0, traverse c1, emit k1, traverse c2, emit k2,
     * traverse c3.
     */
    private fun collectInorder(node: Node, result: MutableList<Pair<K, V>>) {
        if (node.isLeaf) {
            for (i in node.keys.indices) result.add(node.keys[i] to node.values[i])
            return
        }
        for (i in node.keys.indices) {
            collectInorder(node.children[i], result)
            result.add(node.keys[i] to node.values[i])
        }
        collectInorder(node.children.last(), result)
    }

    // =========================================================================
    // Private helpers — validation
    // =========================================================================

    /**
     * Recursively validate B-tree invariants.
     *
     * @param node       current node
     * @param minKey     lower bound for keys (exclusive); null means no bound
     * @param maxKey     upper bound for keys (exclusive); null means no bound
     * @param depth      current depth from root
     * @param leafDepth  single-element array holding the expected leaf depth (-1 = not set)
     * @param isRoot     true if this is the root node
     * @return true if the subtree is valid
     */
    private fun validate(
        node:      Node,
        minKey:    K?,
        maxKey:    K?,
        depth:     Int,
        leafDepth: IntArray,
        isRoot:    Boolean
    ): Boolean {
        val n = node.keys.size

        // Check key count bounds
        if (isRoot) {
            if (size > 0 && n < 1) return false
        } else {
            if (n < t - 1 || n > 2 * t - 1) return false
        }

        // Check keys are sorted and within bounds
        for (j in 0 until n) {
            val k = node.keys[j]
            if (minKey != null && k.compareTo(minKey) <= 0) return false
            if (maxKey != null && k.compareTo(maxKey) >= 0) return false
            if (j > 0 && k.compareTo(node.keys[j - 1]) <= 0) return false
        }

        if (node.isLeaf) {
            // Check child count
            if (node.children.isNotEmpty()) return false
            // Record/check leaf depth
            if (leafDepth[0] == -1) leafDepth[0] = depth
            else if (leafDepth[0] != depth) return false
        } else {
            // Internal: children.size must be keys.size + 1
            if (node.children.size != n + 1) return false
            for (j in 0..n) {
                val lo = if (j > 0) node.keys[j - 1] else minKey
                val hi = if (j < n) node.keys[j]     else maxKey
                if (!validate(node.children[j], lo, hi, depth + 1, leafDepth, isRoot = false)) {
                    return false
                }
            }
        }
        return true
    }
}
