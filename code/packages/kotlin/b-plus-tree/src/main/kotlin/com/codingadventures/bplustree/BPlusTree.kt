// ============================================================================
// BPlusTree.kt — B+ Tree (DT12): Database-Optimised B-Tree Variant
// ============================================================================
//
// A B+ tree is a refinement of the B-tree (DT11) with two structural changes
// that make it dramatically better for database workloads:
//
//   1. ALL DATA LIVES AT THE LEAVES.
//      Internal nodes store only separator keys for routing — they hold no
//      values.  This makes internal nodes smaller, so more fit in memory and
//      the tree is shallower (higher branching factor).
//
//   2. LEAF NODES FORM A SORTED LINKED LIST.
//      Every leaf has a pointer to the next leaf.  Once you locate the starting
//      leaf for a range query, you walk the linked list without backtracking.
//
// ─────────────────────────────────────────────────────────────────────────────
// B-Tree vs B+ Tree — side-by-side
// ─────────────────────────────────────────────────────────────────────────────
//
//   B-TREE (DT11):
//     keys+values everywhere.  Finding key 20 can terminate at root.
//
//                [20:"Carol",  40:"Eve"]
//               /             |            \
//   [5:"Alice", 10:"Bob"]  [25:"Dave"]  [45:"Frank", 55:"Grace"]
//
//   B+ TREE (DT12):
//     internal nodes have separator keys only; leaves have (key, value) pairs.
//     Separator keys are COPIES of leaf keys (they stay in the leaf too).
//
//                    [20,       40]       ← no values, just routing keys
//                  /    |          \
//        leaf1      leaf2       leaf3     ← keys+values
//        ↓            ↓            ↓
//   [(5,A),(10,B),(20,C)] → [(25,D),(40,E)] → [(45,F),(55,G)] → null
//
//   KEY OBSERVATION: key 20 appears BOTH in the internal node AND in leaf1.
//   In a B-tree it would only appear in the internal node.
//
// ─────────────────────────────────────────────────────────────────────────────
// The Critical Insert Difference: Separator Key Promotion
// ─────────────────────────────────────────────────────────────────────────────
//
//   B-tree leaf split:   [1, 2, |3|, 4, 5]  →  parent gets 3; left=[1,2]; right=[4,5]
//                                                (3 is REMOVED from children)
//
//   B+ tree leaf split:  [1, 2, |3|, 4, 5]  →  parent gets 3; left=[1,2]; right=[3,4,5]
//                                                (3 STAYS in the right leaf ↑)
//
//   Internal node splits in B+ tree are the same as B-tree: the median is
//   MOVED to the parent (not copied into either child).
//
// ─────────────────────────────────────────────────────────────────────────────
// Routing Invariant vs Exact-Equality Invariant
// ─────────────────────────────────────────────────────────────────────────────
//
//   After a non-structural delete (no borrow or merge), a separator key may
//   become "stale": the key it was copied from is deleted from its leaf, but
//   the separator remains in the internal node unchanged.  This is correct
//   behaviour — routing still works:
//
//     Routing invariant: for every separator keys[i],
//       max(children[i])  < keys[i]   (all left-side keys are smaller)
//       min(children[i+1]) >= keys[i]  (all right-side keys are not smaller)
//
//   This weaker invariant is what the B+ tree guarantees and what isValid()
//   checks.  It does NOT require separator == exact minimum of right child.
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.bplustree
// ============================================================================

package com.codingadventures.bplustree

/**
 * A B+ tree mapping comparable keys to values.
 *
 * All data is stored in leaf nodes.  Internal nodes hold only separator keys
 * for routing.  Leaf nodes form a singly-linked sorted list enabling
 * O(log n + k) range scans without tree backtracking.
 *
 * Parameterised by minimum degree [t] ≥ 2:
 * - Every non-root node holds at least t-1 keys.
 * - Every node holds at most 2t-1 keys.
 * - Leaf-level keys are actual data; internal-node keys are routing copies.
 *
 * Time complexity (all operations): O(t · log_t n).
 *
 * ```kotlin
 * val tree = BPlusTree<Int, String>(t = 2)
 * tree.insert(5, "five")
 * tree.insert(3, "three")
 * tree.insert(7, "seven")
 *
 * tree.search(3)               // → "three"
 * tree.rangeScan(3, 6)         // → [(3,"three"), (5,"five")]
 * tree.fullScan()              // → [(3,"three"), (5,"five"), (7,"seven")]
 * tree.isValid()               // → true
 * ```
 *
 * @param K key type (must be Comparable)
 * @param V value type
 * @param t minimum degree (≥ 2)
 */
class BPlusTree<K : Comparable<K>, V>(val t: Int = 2) : Iterable<Map.Entry<K, V>> {

    init {
        require(t >= 2) { "Minimum degree must be ≥ 2, got $t" }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Node Types (sealed hierarchy)
    // ─────────────────────────────────────────────────────────────────────────

    /** Sealed hierarchy for the two node types. */
    private sealed class BPlusNode<K, V>

    // ─────────────────────────────────────────────────────────────────────────
    // Internal Node
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Stores ONLY separator keys — no values.
    // A node with k keys has exactly k+1 children.
    //
    //   keys[0]      keys[1]      keys[2]          ← separator keys
    //      |            |            |
    // child[0]    child[1]    child[2]    child[3]  ← k+1 children
    //
    // keys[i] is the SMALLEST key in children[i+1] (or a stale copy after delete).
    // All keys in children[i] are strictly less than keys[i].

    private class InternalNode<K, V> : BPlusNode<K, V>() {
        /** Separator keys.  No values stored here — just routing information. */
        val keys: MutableList<K> = mutableListOf()
        /** Children: internal or leaf nodes.  Always exactly keys.size + 1 entries. */
        val children: MutableList<BPlusNode<K, V>> = mutableListOf()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Leaf Node
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Stores the actual (key, value) pairs in sorted order.
    // Has a next-pointer to the neighbouring leaf — forming the linked list
    // that makes B+ range scans sequential.

    private class LeafNode<K, V> : BPlusNode<K, V>() {
        /** Sorted keys in this leaf. */
        val keys: MutableList<K> = mutableListOf()
        /** values[i] is the value associated with keys[i]. */
        val values: MutableList<V> = mutableListOf()
        /** Pointer to the next leaf node.  null for the rightmost leaf. */
        var next: LeafNode<K, V>? = null
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SplitResult
    // ─────────────────────────────────────────────────────────────────────────
    //
    // When a node overflows after an insert, insertInto() returns this class:
    //   promotedKey  — the key to insert into the parent
    //   rightNode    — the new right sibling
    //
    // For a LEAF split:  promotedKey is the smallest key of rightNode,
    //                    AND rightNode still contains that key.
    //
    // For an INTERNAL split: promotedKey is the median, which is removed
    //                         from both children (it moves entirely to the parent).

    private data class SplitResult<K, V>(val promotedKey: K, val rightNode: BPlusNode<K, V>)

    // ─────────────────────────────────────────────────────────────────────────
    // Fields
    // ─────────────────────────────────────────────────────────────────────────

    /** The root node.  Starts as an empty LeafNode. */
    private var root: BPlusNode<K, V>

    /**
     * Leftmost leaf — start of the linked list.
     * fullScan() starts here; minKey() returns firstLeaf.keys[0].
     */
    private var firstLeaf: LeafNode<K, V>

    /** Total number of (key, value) pairs currently stored. */
    var size: Int = 0
        private set

    /** True if no keys are stored. */
    val isEmpty: Boolean get() = size == 0

    init {
        val emptyLeaf = LeafNode<K, V>()
        root = emptyLeaf
        firstLeaf = emptyLeaf
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Search (Point Lookup)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Unlike a B-tree (where a key can be in any node), B+ tree search always
    // descends to a leaf.  Internal nodes are PURE routing — they can never
    // terminate a search.
    //
    // Time: O(t · log_t n) — height traversal with O(t) work at each level.

    /**
     * Return the value for [key], or `null` if absent.
     *
     * Always descends to a leaf — never terminates at an internal node.
     * Time: O(t · log_t n).
     *
     * @throws NullPointerException if [key] is null (Kotlin non-null type enforces this
     *   at compile time for non-nullable K; this guard is for Java interop callers)
     */
    fun search(key: K): V? {
        requireNonNullKey(key)
        val leaf = findLeaf(root, key)
        val i = leafIndexOf(leaf, key)
        return if (i >= 0) leaf.values[i] else null
    }

    /**
     * Return `true` if [key] is present.
     *
     * Implemented directly (not via [search]) so that a key whose associated value
     * happens to be `null` is still reported as present.
     */
    fun contains(key: K): Boolean {
        requireNonNullKey(key)
        val leaf = findLeaf(root, key)
        return leafIndexOf(leaf, key) >= 0
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insert
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Strategy: top-down insert using recursive "return split result" approach.
    //
    //   1. Descend to the target leaf.
    //   2. Insert (key, value) in sorted order.
    //   3. If the leaf overflows (size == 2t-1):
    //        splitLeaf → returns (separator, rightLeaf)
    //        separator is the SMALLEST KEY OF rightLeaf — and stays there.
    //   4. Propagate splits upward: each overflowing internal node also splits.
    //      For internal splits: the median key is MOVED to the parent.
    //   5. If the root splits, create a new root with one key and two children.
    //
    // Time: O(t · log_t n).

    /**
     * Insert or update the mapping from [key] to [value].
     *
     * If [key] already exists, its value is replaced.
     * Time: O(t · log_t n).
     *
     * @throws NullPointerException if [key] is null (guards Java interop)
     */
    fun insert(key: K, value: V) {
        requireNonNullKey(key)
        val split = insertInto(root, key, value)
        if (split != null) {
            // Root overflowed and was split.  Create a new root.
            val newRoot = InternalNode<K, V>()
            newRoot.keys.add(split.promotedKey)
            newRoot.children.add(root)
            newRoot.children.add(split.rightNode)
            root = newRoot
        }
    }

    /**
     * Recursively insert into the subtree rooted at [node].
     *
     * @return a SplitResult if this node overflowed and split; null otherwise
     */
    private fun insertInto(node: BPlusNode<K, V>, key: K, value: V): SplitResult<K, V>? {
        return when (node) {
            is LeafNode -> {
                // ─── Leaf: insert into the sorted position ───────────────────
                val pos = leafInsertPosition(node, key)
                if (pos < node.keys.size && node.keys[pos].compareTo(key) == 0) {
                    // Key already exists: update value in-place, no split.
                    node.values[pos] = value
                    return null
                }
                node.keys.add(pos, key)
                node.values.add(pos, value)
                size++
                // Leaf is now full (2t-1 keys → must split)?
                if (node.keys.size > 2 * t - 1) splitLeaf(node) else null
            }

            is InternalNode -> {
                // ─── Internal node: find the correct child, recurse ──────────
                val i = findChildIndex(node, key)
                val childSplit = insertInto(node.children[i], key, value) ?: return null
                // Child split: insert the promoted key and right-child pointer.
                node.keys.add(i, childSplit.promotedKey)
                node.children.add(i + 1, childSplit.rightNode)
                // Does this internal node overflow too?
                if (node.keys.size > 2 * t - 1) splitInternal(node) else null
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Leaf Split
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Split a full leaf into two leaves:
    //
    //   Before (t=2, max=3 keys):  [k0, k1, k2, k3]
    //   mid = 2
    //
    //   left (original):  [k0, k1]
    //   right (new):      [k2, k3]   ← k2 (separator) STAYS in right leaf
    //   promoted:         k2         ← also goes UP to parent
    //
    //   Linked list:  ... → left → right → (old next) → ...

    private fun splitLeaf(leaf: LeafNode<K, V>): SplitResult<K, V> {
        val mid = leaf.keys.size / 2           // right gets keys[mid..end]
        val separator = leaf.keys[mid]

        val right = LeafNode<K, V>()
        right.keys.addAll(leaf.keys.subList(mid, leaf.keys.size))
        right.values.addAll(leaf.values.subList(mid, leaf.values.size))
        right.next = leaf.next                 // maintain linked list

        leaf.keys.subList(mid, leaf.keys.size).clear()
        leaf.values.subList(mid, leaf.values.size).clear()
        leaf.next = right                      // left now points to right

        // separator = keys[mid] = smallest key in right leaf.
        // It goes UP to the parent AND stays in the right leaf.
        return SplitResult(separator, right)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Internal Node Split
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Split a full internal node:
    //
    //   Before (t=2, max=3 keys):  keys=[k0,k1,k2,k3]  children=[c0,c1,c2,c3,c4]
    //   mid = 2  (the median)
    //
    //   left (original):  keys=[k0,k1]  children=[c0,c1,c2]
    //   right (new):      keys=[k3]     children=[c3,c4]
    //   promoted:         k2            ← MOVED to parent, removed from both halves

    private fun splitInternal(node: InternalNode<K, V>): SplitResult<K, V> {
        val mid = node.keys.size / 2           // index of the median key
        val separator = node.keys[mid]

        val right = InternalNode<K, V>()
        right.keys.addAll(node.keys.subList(mid + 1, node.keys.size))
        right.children.addAll(node.children.subList(mid + 1, node.children.size))

        // Trim the left node (original): remove median AND everything to the right.
        node.keys.subList(mid, node.keys.size).clear()
        node.children.subList(mid + 1, node.children.size).clear()

        // separator is REMOVED from both halves and goes entirely to the parent.
        return SplitResult(separator, right)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Delete
    // ─────────────────────────────────────────────────────────────────────────
    //
    // B+ tree deletion always removes from a leaf (data lives at leaves).
    // If the leaf underflows (size < t-1), fix it by:
    //   1. Borrow from the right sibling (steal its leftmost key/value).
    //   2. Borrow from the left sibling  (steal its rightmost key/value).
    //   3. Merge with the right sibling  (delete separator from parent).
    //   4. Merge with the left sibling   (delete separator from parent).
    //   Propagate underflow fix upward if the parent lost a key.
    //
    // Separator keys in internal nodes may be stale after a non-merging delete,
    // but the routing invariant still holds.
    //
    // Time: O(t · log_t n).

    /**
     * Remove the mapping for [key].
     *
     * No-op if [key] is not present.
     * Time: O(t · log_t n).
     *
     * @throws NullPointerException if [key] is null (guards Java interop)
     */
    fun delete(key: K) {
        requireNonNullKey(key)
        val deleted = deleteFrom(root, key, null, -1)
        if (!deleted) return
        // If root is an internal node with no keys, its only child becomes the new root.
        val r = root
        if (r is InternalNode && r.keys.isEmpty()) {
            root = r.children[0]
        }
    }

    /**
     * Recursively delete [key] from the subtree rooted at [node].
     *
     * @param node    current subtree root
     * @param key     key to delete
     * @param parent  parent of this node (null for root)
     * @param idx     index of this node in parent.children (-1 for root)
     * @return true if the key was found and deleted
     */
    private fun deleteFrom(
        node: BPlusNode<K, V>,
        key: K,
        parent: InternalNode<K, V>?,
        idx: Int
    ): Boolean {
        return when (node) {
            is LeafNode -> {
                // ─── Leaf: remove the key directly ───────────────────────────
                val pos = leafIndexOf(node, key)
                if (pos < 0) return false      // key not found
                node.keys.removeAt(pos)
                node.values.removeAt(pos)
                size--
                // Fix underflow if not root and below minimum.
                if (parent != null && node.keys.size < t - 1) {
                    fixLeafUnderflow(parent, idx, node)
                }
                true
            }

            is InternalNode -> {
                // ─── Internal node: find the right child and recurse ─────────
                val i = findChildIndex(node, key)
                val deleted = deleteFrom(node.children[i], key, node, i)
                if (!deleted) return false
                // Fix underflow in internal node if it occurs.
                if (parent != null && node.keys.size < t - 1) {
                    fixInternalUnderflow(parent, idx, node)
                }
                true
            }
        }
    }

    /**
     * Repair underflow in a leaf node.
     *
     * @param parent the parent of the underflowing leaf
     * @param idx    the index of the underflowing leaf in parent.children
     * @param leaf   the underflowing leaf
     */
    @Suppress("UNCHECKED_CAST")
    private fun fixLeafUnderflow(parent: InternalNode<K, V>, idx: Int, leaf: LeafNode<K, V>) {
        // Try borrowing from the right sibling.
        if (idx + 1 < parent.children.size) {
            val right = parent.children[idx + 1] as LeafNode<K, V>
            if (right.keys.size > t - 1) {
                // Borrow the leftmost key from the right sibling.
                leaf.keys.add(right.keys.removeAt(0))
                leaf.values.add(right.values.removeAt(0))
                // Update the separator in the parent to the new smallest key of right.
                parent.keys[idx] = right.keys[0]
                return
            }
        }

        // Try borrowing from the left sibling.
        if (idx > 0) {
            val left = parent.children[idx - 1] as LeafNode<K, V>
            if (left.keys.size > t - 1) {
                // Borrow the rightmost key from the left sibling.
                val last = left.keys.size - 1
                leaf.keys.add(0, left.keys.removeAt(last))
                leaf.values.add(0, left.values.removeAt(last))
                // Update the separator in the parent to the new smallest key of leaf.
                parent.keys[idx - 1] = leaf.keys[0]
                return
            }
        }

        // Neither sibling has spare keys — must merge.
        if (idx + 1 < parent.children.size) {
            // Merge leaf with its RIGHT sibling.
            val right = parent.children[idx + 1] as LeafNode<K, V>
            // Move all right's keys/values into leaf.
            leaf.keys.addAll(right.keys)
            leaf.values.addAll(right.values)
            leaf.next = right.next             // skip right in the linked list
            // Remove the separator key and the right child from the parent.
            parent.keys.removeAt(idx)
            parent.children.removeAt(idx + 1)
        } else {
            // Merge leaf with its LEFT sibling.
            val left = parent.children[idx - 1] as LeafNode<K, V>
            left.keys.addAll(leaf.keys)
            left.values.addAll(leaf.values)
            left.next = leaf.next              // skip leaf in linked list
            parent.keys.removeAt(idx - 1)
            parent.children.removeAt(idx)
        }
    }

    /**
     * Repair underflow in an internal node.
     *
     * @param parent the parent of the underflowing internal node
     * @param idx    the index of the underflowing node in parent.children
     * @param node   the underflowing internal node
     */
    @Suppress("UNCHECKED_CAST")
    private fun fixInternalUnderflow(parent: InternalNode<K, V>, idx: Int, node: InternalNode<K, V>) {
        // Try borrowing from the right sibling.
        if (idx + 1 < parent.children.size) {
            val right = parent.children[idx + 1] as InternalNode<K, V>
            if (right.keys.size > t - 1) {
                // Rotate: parent's separator goes down to node; right's first key goes up.
                node.keys.add(parent.keys[idx])
                parent.keys[idx] = right.keys.removeAt(0)
                node.children.add(right.children.removeAt(0))
                return
            }
        }

        // Try borrowing from the left sibling.
        if (idx > 0) {
            val left = parent.children[idx - 1] as InternalNode<K, V>
            if (left.keys.size > t - 1) {
                // Rotate: parent's separator goes down to node; left's last key goes up.
                node.keys.add(0, parent.keys[idx - 1])
                parent.keys[idx - 1] = left.keys.removeAt(left.keys.size - 1)
                node.children.add(0, left.children.removeAt(left.children.size - 1))
                return
            }
        }

        // Must merge.
        if (idx + 1 < parent.children.size) {
            // Merge with RIGHT sibling.
            val right = parent.children[idx + 1] as InternalNode<K, V>
            val separatorDown = parent.keys.removeAt(idx)
            parent.children.removeAt(idx + 1)
            node.keys.add(separatorDown)
            node.keys.addAll(right.keys)
            node.children.addAll(right.children)
        } else {
            // Merge with LEFT sibling.
            val left = parent.children[idx - 1] as InternalNode<K, V>
            val separatorDown = parent.keys.removeAt(idx - 1)
            parent.children.removeAt(idx)
            left.keys.add(separatorDown)
            left.keys.addAll(node.keys)
            left.children.addAll(node.children)
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Range Scan — the Killer Feature
    // ─────────────────────────────────────────────────────────────────────────
    //
    // 1. Use the tree to find the first leaf that may contain keys ≥ low.   O(log n)
    // 2. Walk the leaf linked list, collecting keys in [low..high].          O(k)
    //
    // Total: O(t · log_t n + k).

    /**
     * Return all (key, value) pairs where `low ≤ key ≤ high`, sorted by key.
     *
     * Time: O(t · log_t n + k) where k = number of results.
     *
     * Note: all matched entries are materialised into a single list.  For very
     * wide ranges on large trees, prefer iterating via [iterator] instead.
     *
     * @param low  inclusive lower bound (must be ≤ high)
     * @param high inclusive upper bound (must be ≥ low)
     * @return sorted list of matching entries
     * @throws NullPointerException     if [low] or [high] is null (guards Java interop)
     * @throws IllegalArgumentException if `low > high`
     */
    fun rangeScan(low: K, high: K): List<Map.Entry<K, V>> {
        requireNonNullKey(low)
        requireNonNullKey(high)
        require(low <= high) { "rangeScan: low must be ≤ high, got low=$low high=$high" }
        val results = mutableListOf<Map.Entry<K, V>>()
        var leaf: LeafNode<K, V>? = findLeaf(root, low)

        while (leaf != null) {
            for (i in leaf.keys.indices) {
                val k = leaf.keys[i]
                if (k > high) return results    // past the end
                if (k >= low) results.add(java.util.AbstractMap.SimpleImmutableEntry(k, leaf.values[i]))
            }
            leaf = leaf.next
        }
        return results
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Full Scan
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Walk the entire leaf linked list from left to right.  O(n).

    /**
     * Return all (key, value) pairs in sorted key order.
     *
     * Walks the leaf linked list from [firstLeaf] to the end.
     * Time: O(n).
     */
    fun fullScan(): List<Map.Entry<K, V>> {
        val results = mutableListOf<Map.Entry<K, V>>()
        var leaf: LeafNode<K, V>? = firstLeaf
        while (leaf != null) {
            for (i in leaf.keys.indices) {
                results.add(java.util.AbstractMap.SimpleImmutableEntry(leaf.keys[i], leaf.values[i]))
            }
            leaf = leaf.next
        }
        return results
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Min Key / Max Key
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the smallest key.  O(1) — just read firstLeaf.keys[0].
     *
     * @throws NoSuchElementException if the tree is empty
     */
    fun minKey(): K {
        if (isEmpty) throw NoSuchElementException("Tree is empty")
        return firstLeaf.keys[0]
    }

    /**
     * Return the largest key.  O(log_t n) — follow the rightmost path.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    fun maxKey(): K {
        if (isEmpty) throw NoSuchElementException("Tree is empty")
        var node: BPlusNode<K, V> = root
        while (node is InternalNode) {
            node = node.children.last()
        }
        val leaf = node as LeafNode<K, V>
        return leaf.keys.last()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Metadata
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Height of the tree.  An empty tree or a single-leaf tree has height 0.
     * Each additional level of internal nodes adds 1.
     */
    fun height(): Int {
        var h = 0
        var node: BPlusNode<K, V> = root
        while (node is InternalNode) {
            h++
            node = node.children[0]
        }
        return h
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Iterator
    // ─────────────────────────────────────────────────────────────────────────

    /** Iterate all (key, value) pairs in ascending key order. */
    override fun iterator(): Iterator<Map.Entry<K, V>> = fullScan().iterator()

    // ─────────────────────────────────────────────────────────────────────────
    // Validation
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Verify all B+ tree invariants:
    //   1. All leaves at the same depth.
    //   2. Non-root internal nodes: t-1 ≤ keys ≤ 2t-1.
    //   3. All leaves: t-1 ≤ keys ≤ 2t-1 (except root-leaf).
    //   4. Leaf linked list covers all keys in sorted order.
    //   5. Routing invariant: for each separator keys[i]:
    //        max(children[i]) < keys[i]  AND  min(children[i+1]) >= keys[i].
    //      (Exact equality not required — separators may be stale after delete.)

    /**
     * Verify all B+ tree invariants.  O(n).  For testing and debugging only.
     *
     * @return true if all invariants hold
     */
    fun isValid(): Boolean {
        // 1. All leaves at the same depth.
        val leafDepth = computeLeafDepth(root, 0)
        if (leafDepth < 0) return false

        // 2 & 3. Key count invariants.
        if (!validateKeyCount(root, isRoot = true)) return false

        // 4. Linked list is sorted and contains every key exactly once.
        val leafKeys = mutableListOf<K>()
        var leaf: LeafNode<K, V>? = firstLeaf
        while (leaf != null) {
            for (i in 1 until leaf.keys.size) {
                if (leaf.keys[i - 1] >= leaf.keys[i]) return false
            }
            leafKeys.addAll(leaf.keys)
            leaf = leaf.next
        }
        for (i in 1 until leafKeys.size) {
            if (leafKeys[i - 1] >= leafKeys[i]) return false
        }
        if (leafKeys.size != size) return false

        // 5. Routing invariant for separator keys.
        if (!validateSeparators(root)) return false

        return true
    }

    /** Returns the depth at which leaves are found, or -1 if inconsistent. */
    private fun computeLeafDepth(node: BPlusNode<K, V>, depth: Int): Int {
        if (node is LeafNode) return depth
        val internal = node as InternalNode
        var firstDepth = -1
        for (child in internal.children) {
            val d = computeLeafDepth(child, depth + 1)
            if (d < 0) return -1
            if (firstDepth < 0) firstDepth = d
            else if (firstDepth != d) return -1
        }
        return firstDepth
    }

    /** Verify key count invariants (t-1 ≤ keys ≤ 2t-1 for non-root nodes). */
    private fun validateKeyCount(node: BPlusNode<K, V>, isRoot: Boolean): Boolean {
        return when (node) {
            is LeafNode -> {
                if (!isRoot && node.keys.size < t - 1) return false
                if (node.keys.size > 2 * t - 1) return false
                true
            }
            is InternalNode -> {
                if (!isRoot && node.keys.size < t - 1) return false
                if (node.keys.size > 2 * t - 1) return false
                node.children.all { validateKeyCount(it, isRoot = false) }
            }
        }
    }

    /**
     * Verify the routing invariant for separator keys.
     *
     * For each separator keys[i] in an internal node:
     * - The maximum key in children[i] must be strictly less than keys[i].
     * - The minimum key in children[i+1] must be ≥ keys[i].
     *
     * Note: exact separator == min(right child) is NOT required; separators
     * may be stale after a non-structural delete.
     */
    private fun validateSeparators(node: BPlusNode<K, V>): Boolean {
        if (node is LeafNode) return true
        val internal = node as InternalNode
        for (i in internal.keys.indices) {
            val sep = internal.keys[i]
            // All keys in children[i] must be < sep.
            if (rightmostLeafKey(internal.children[i]) >= sep) return false
            // All keys in children[i+1] must be >= sep.
            if (leftmostLeafKey(internal.children[i + 1]) < sep) return false
        }
        return internal.children.all { validateSeparators(it) }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Find the leaf that should contain [key].
     * Always descends to a leaf — never stops at an internal node.
     */
    private fun findLeaf(node: BPlusNode<K, V>, key: K): LeafNode<K, V> {
        var current = node
        while (current is InternalNode) {
            current = current.children[findChildIndex(current, key)]
        }
        return current as LeafNode<K, V>
    }

    /**
     * Find the child index to follow for [key] in an internal node.
     *
     * Returns i such that all keys in children[i] are ≤ key and all keys in
     * children[i+1] are > key (for the relevant separators).
     *
     * Specifically: return the number of separator keys that are ≤ key.
     * keys[i] = smallest key in children[i+1], so we go right if key >= keys[i].
     */
    private fun findChildIndex(node: InternalNode<K, V>, key: K): Int {
        var i = 0
        while (i < node.keys.size && key >= node.keys[i]) i++
        return i
    }

    /**
     * Find the index of [key] in a sorted leaf (binary search).
     *
     * @return index i if leaf.keys[i] == key, or -1 if absent
     */
    private fun leafIndexOf(leaf: LeafNode<K, V>, key: K): Int {
        var lo = 0
        var hi = leaf.keys.size - 1
        while (lo <= hi) {
            val mid = (lo + hi).ushr(1)
            val cmp = leaf.keys[mid].compareTo(key)
            when {
                cmp < 0 -> lo = mid + 1
                cmp > 0 -> hi = mid - 1
                else    -> return mid
            }
        }
        return -1
    }

    /**
     * Find the sorted insertion position for [key] in a leaf.
     *
     * @return index where key should be inserted (0 to leaf.keys.size)
     */
    private fun leafInsertPosition(leaf: LeafNode<K, V>, key: K): Int {
        var lo = 0
        var hi = leaf.keys.size
        while (lo < hi) {
            val mid = (lo + hi).ushr(1)
            if (leaf.keys[mid] < key) lo = mid + 1 else hi = mid
        }
        return lo
    }

    /**
     * Guard against null keys at every public entry point.
     *
     * Kotlin's non-null type system prevents null K from Kotlin callers at compile time,
     * but Java interop callers can pass null at runtime.  Reject explicitly with a clear message.
     */
    private fun requireNonNullKey(key: K) {
        if (key == null) throw NullPointerException("key must not be null")
    }

    /** Return the leftmost (minimum) key reachable from this node. */
    private fun leftmostLeafKey(node: BPlusNode<K, V>): K {
        var current = node
        while (current is InternalNode) current = current.children[0]
        return (current as LeafNode<K, V>).keys[0]
    }

    /** Return the rightmost (maximum) key reachable from this node. */
    private fun rightmostLeafKey(node: BPlusNode<K, V>): K {
        var current = node
        while (current is InternalNode) current = current.children.last()
        val leaf = current as LeafNode<K, V>
        return leaf.keys.last()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    override fun toString(): String = "BPlusTree{size=$size, height=${height()}, t=$t}"
}
