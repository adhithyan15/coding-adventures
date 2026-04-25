// ============================================================================
// BPlusTree.java — B+ Tree (DT12): Database-Optimised B-Tree Variant
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
// Why This Is Better for Databases
// ─────────────────────────────────────────────────────────────────────────────
//
//   Denser internal nodes → shallower tree:
//
//     B-tree   internal:  (key + value)    fits ~37 entries per 4 KiB page
//     B+ tree  internal:  (key only)       fits ~500 entries per 4 KiB page
//
//     With 500-way branching vs 37-way:
//       B-tree:  log₃₇(1B) ≈ 6 disk reads
//       B+ tree: log₅₀₀(1B) ≈ 4 disk reads   ← 2 fewer I/Os per query!
//
//   Range scans follow the leaf linked list — pure sequential I/O:
//
//     B-tree:  find leftmost key O(log n), then inorder traversal with
//              random tree jumps back up.
//
//     B+ tree: find leftmost leaf O(log n), then follow next-pointers
//              O(1) per leaf — no upward traversal, just sequential reads.
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
// Package: com.codingadventures.bplustree
// ============================================================================

package com.codingadventures.bplustree;

import java.util.AbstractMap;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.NoSuchElementException;

/**
 * A B+ tree mapping comparable keys to values.
 *
 * <p>All data is stored in leaf nodes.  Internal nodes hold only separator keys
 * for routing.  Leaf nodes form a doubly-linked list (singly-linked here for
 * simplicity) enabling O(log n + k) range scans without tree backtracking.
 *
 * <p>Parameterised by minimum degree {@code t ≥ 2}:
 * <ul>
 *   <li>Every non-root node holds at least {@code t-1} keys.
 *   <li>Every node holds at most {@code 2t-1} keys.
 *   <li>Leaf-level keys are actual data; internal-node keys are routing copies.
 * </ul>
 *
 * <p>Time complexity (all operations): O(t · log_t n).
 *
 * <pre>{@code
 * BPlusTree<Integer, String> tree = new BPlusTree<>(2);
 * tree.insert(5, "five");
 * tree.insert(3, "three");
 * tree.insert(7, "seven");
 *
 * tree.search(3);               // → "three"
 * tree.rangeScan(3, 6);         // → [(3,"three"), (5,"five")]
 * tree.fullScan();              // → [(3,"three"), (5,"five"), (7,"seven")]
 * tree.isValid();               // → true
 * }</pre>
 *
 * @param <K> key type (must be Comparable)
 * @param <V> value type
 */
public class BPlusTree<K extends Comparable<K>, V> implements Iterable<Map.Entry<K, V>> {

    // ─────────────────────────────────────────────────────────────────────────
    // Node Interfaces
    // ─────────────────────────────────────────────────────────────────────────

    /** Marker interface for the two node types. */
    interface BPlusNode<K, V> {}

    // ─────────────────────────────────────────────────────────────────────────
    // Internal Node
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Stores ONLY separator keys — no values.
    // A node with k keys has exactly k+1 children.
    //
    //   keys[0]      keys[1]      keys[2]         ← separator keys
    //      |            |            |
    // child[0]    child[1]    child[2]    child[3] ← k+1 children
    //
    // keys[i] is the SMALLEST key in child[i+1].
    // All keys in child[i] are strictly less than keys[i].

    static final class InternalNode<K, V> implements BPlusNode<K, V> {
        /** Separator keys.  No values stored here — just routing information. */
        final List<K> keys;
        /** Children: internal or leaf nodes.  Always exactly keys.size() + 1 entries. */
        final List<BPlusNode<K, V>> children;

        InternalNode() {
            this.keys     = new ArrayList<>();
            this.children = new ArrayList<>();
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Leaf Node
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Stores the actual (key, value) pairs in sorted order.
    // Has a next-pointer to the neighbouring leaf — forming the linked list
    // that makes B+ range scans sequential.

    static final class LeafNode<K, V> implements BPlusNode<K, V> {
        /** Sorted keys in this leaf. */
        final List<K> keys;
        /** values[i] is the value associated with keys[i]. */
        final List<V> values;
        /** Pointer to the next leaf node.  null for the rightmost leaf. */
        LeafNode<K, V> next;

        LeafNode() {
            this.keys   = new ArrayList<>();
            this.values = new ArrayList<>();
            this.next   = null;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SplitResult
    // ─────────────────────────────────────────────────────────────────────────
    //
    // When a node overflows after an insert, split() returns this record:
    //   promotedKey  — the key to insert into the parent
    //   rightNode    — the new right sibling
    //
    // For a LEAF split:  promotedKey is the smallest key of rightNode,
    //                    AND rightNode still contains that key.
    //
    // For an INTERNAL split: promotedKey is the median, which is removed
    //                         from both children (it moves entirely to the parent).

    private record SplitResult<K, V>(K promotedKey, BPlusNode<K, V> rightNode) {}

    // ─────────────────────────────────────────────────────────────────────────
    // BPlusTree Fields
    // ─────────────────────────────────────────────────────────────────────────

    /** The root node.  Starts as an empty LeafNode. */
    private BPlusNode<K, V> root;

    /**
     * Leftmost leaf — start of the linked list.
     * fullScan() starts here; minKey() returns firstLeaf.keys.get(0).
     */
    private LeafNode<K, V> firstLeaf;

    /** Minimum degree.  Every non-root node has between t-1 and 2t-1 keys. */
    private final int t;

    /** Total number of (key, value) pairs currently stored. */
    private int size;

    // ─────────────────────────────────────────────────────────────────────────
    // Constructors
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Create an empty B+ tree with the given minimum degree.
     *
     * @param t minimum degree (t ≥ 2).  Higher t → fewer levels, wider nodes.
     * @throws IllegalArgumentException if t < 2
     */
    public BPlusTree(int t) {
        if (t < 2) throw new IllegalArgumentException("Minimum degree must be ≥ 2, got " + t);
        this.t         = t;
        LeafNode<K, V> emptyLeaf = new LeafNode<>();
        this.root      = emptyLeaf;
        this.firstLeaf = emptyLeaf;
        this.size      = 0;
    }

    /** Create an empty B+ tree with minimum degree 2 (the minimum possible). */
    public BPlusTree() {
        this(2);
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
     * Return the value for {@code key}, or {@code null} if absent.
     *
     * <p>Always descends to a leaf — never terminates at an internal node.
     * Time: O(t · log_t n).
     *
     * @throws NullPointerException if {@code key} is null
     */
    public V search(K key) {
        requireNonNullKey(key);
        LeafNode<K, V> leaf = findLeaf(root, key);
        int i = leafIndexOf(leaf, key);
        return i >= 0 ? leaf.values.get(i) : null;
    }

    /**
     * Return {@code true} if {@code key} is present.
     *
     * <p>Implemented directly (not via {@link #search}) so that a key whose
     * associated value happens to be {@code null} is still reported as present.
     *
     * @throws NullPointerException if {@code key} is null
     */
    public boolean contains(K key) {
        requireNonNullKey(key);
        LeafNode<K, V> leaf = findLeaf(root, key);
        return leafIndexOf(leaf, key) >= 0;
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
    //        split_leaf → returns (separator, rightLeaf)
    //        separator is the SMALLEST KEY OF rightLeaf — and stays there.
    //   4. Propagate splits upward: each overflowing internal node also splits.
    //      For internal splits: the median key is MOVED to the parent.
    //   5. If the root splits, create a new root with one key and two children.
    //
    // Time: O(t · log_t n).

    /**
     * Insert or update the mapping from {@code key} to {@code value}.
     *
     * <p>If {@code key} already exists, its value is replaced.
     * Time: O(t · log_t n).
     *
     * @throws NullPointerException if {@code key} is null
     */
    public void insert(K key, V value) {
        requireNonNullKey(key);
        SplitResult<K, V> split = insertInto(root, key, value);
        if (split != null) {
            // Root overflowed and was split.  Create a new root.
            InternalNode<K, V> newRoot = new InternalNode<>();
            newRoot.keys.add(split.promotedKey());
            newRoot.children.add(root);
            newRoot.children.add(split.rightNode());
            root = newRoot;
        }
    }

    /**
     * Recursively insert into the subtree rooted at {@code node}.
     *
     * @return a SplitResult if this node overflowed and split; null otherwise
     */
    private SplitResult<K, V> insertInto(BPlusNode<K, V> node, K key, V value) {
        if (node instanceof LeafNode<K, V> leaf) {
            // ─── Leaf: insert into the sorted position ───────────────────────
            int pos = leafInsertPosition(leaf, key);
            if (pos < leaf.keys.size() && leaf.keys.get(pos).compareTo(key) == 0) {
                // Key already exists: update value in-place, no split.
                leaf.values.set(pos, value);
                return null;
            }
            leaf.keys.add(pos, key);
            leaf.values.add(pos, value);
            size++;

            // Leaf is now full (2t-1 keys → must split)?
            if (leaf.keys.size() > 2 * t - 1) {
                return splitLeaf(leaf);
            }
            return null;  // no split

        } else {
            // ─── Internal node: find the correct child, recurse ─────────────
            InternalNode<K, V> internal = (InternalNode<K, V>) node;
            int i = findChildIndex(internal, key);
            SplitResult<K, V> childSplit = insertInto(internal.children.get(i), key, value);

            if (childSplit == null) return null;  // child did not split

            // Child split: insert the promoted key and right-child pointer.
            internal.keys.add(i, childSplit.promotedKey());
            internal.children.add(i + 1, childSplit.rightNode());

            // Does this internal node overflow too?
            if (internal.keys.size() > 2 * t - 1) {
                return splitInternal(internal);
            }
            return null;  // no split
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
    //
    // KEY DIFFERENCE from B-tree:
    //   B-tree:   [k0,k1,|k2|,k3,k4] → parent gets k2; left=[k0,k1]; right=[k3,k4]
    //   B+ tree:  [k0,k1,|k2|,k3,k4] → parent gets k2; left=[k0,k1]; right=[k2,k3,k4]
    //                                   ↑ k2 stays in right leaf!

    private SplitResult<K, V> splitLeaf(LeafNode<K, V> leaf) {
        int mid = leaf.keys.size() / 2;  // right gets keys[mid..end]
        K separator = leaf.keys.get(mid);

        // Create the new right leaf.
        LeafNode<K, V> right = new LeafNode<>();
        right.keys.addAll(leaf.keys.subList(mid, leaf.keys.size()));
        right.values.addAll(leaf.values.subList(mid, leaf.values.size()));
        right.next = leaf.next;  // maintain linked list

        // Trim the original (left) leaf.
        leaf.keys.subList(mid, leaf.keys.size()).clear();
        leaf.values.subList(mid, leaf.values.size()).clear();
        leaf.next = right;  // left now points to right

        // separator = keys[mid] = smallest key in right leaf.
        // It goes UP to the parent AND stays in the right leaf.
        return new SplitResult<>(separator, right);
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
    //
    // KEY DIFFERENCE from leaf split:
    //   The median k2 is REMOVED from both children and given entirely to the parent.
    //   (Internal nodes are routing only — they don't store data.)

    private SplitResult<K, V> splitInternal(InternalNode<K, V> node) {
        int mid = node.keys.size() / 2;  // index of the median key
        K separator = node.keys.get(mid);

        // Create the new right internal node.
        InternalNode<K, V> right = new InternalNode<>();
        right.keys.addAll(node.keys.subList(mid + 1, node.keys.size()));
        right.children.addAll(node.children.subList(mid + 1, node.children.size()));

        // Trim the left node (original): remove median AND everything to the right.
        node.keys.subList(mid, node.keys.size()).clear();
        node.children.subList(mid + 1, node.children.size()).clear();

        // separator is REMOVED from both halves and goes entirely to the parent.
        return new SplitResult<>(separator, right);
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
    // but the tree still routes correctly.  On merge, we remove the separator.
    //
    // Time: O(t · log_t n).

    /**
     * Remove the mapping for {@code key}.
     *
     * <p>No-op if {@code key} is not present.
     * Time: O(t · log_t n).
     *
     * @throws NullPointerException if {@code key} is null
     */
    public void delete(K key) {
        requireNonNullKey(key);
        boolean deleted = deleteFrom(root, key, null, -1);
        // If root is an internal node with no keys, its only child becomes the new root.
        if (!deleted) return;
        if (root instanceof InternalNode<K, V> internalRoot && internalRoot.keys.isEmpty()) {
            root = internalRoot.children.get(0);
        }
    }

    /**
     * Recursively delete {@code key} from the subtree rooted at {@code node}.
     *
     * @param node    current subtree root
     * @param key     key to delete
     * @param parent  parent of this node (null for root)
     * @param idx     index of this node in parent.children (-1 for root)
     * @return true if the key was found and deleted
     */
    private boolean deleteFrom(BPlusNode<K, V> node, K key,
                               InternalNode<K, V> parent, int idx) {
        if (node instanceof LeafNode<K, V> leaf) {
            // ─── Leaf: remove the key directly ───────────────────────────────
            int pos = leafIndexOf(leaf, key);
            if (pos < 0) return false;  // key not found
            leaf.keys.remove(pos);
            leaf.values.remove(pos);
            size--;

            // Fix underflow if not root and below minimum.
            if (parent != null && leaf.keys.size() < t - 1) {
                fixLeafUnderflow(parent, idx, leaf);
            }
            return true;

        } else {
            // ─── Internal node: find the right child and recurse ────────────
            InternalNode<K, V> internal = (InternalNode<K, V>) node;
            int i = findChildIndex(internal, key);
            boolean deleted = deleteFrom(internal.children.get(i), key, internal, i);

            if (!deleted) return false;

            // Fix underflow in internal node if it occurs.
            if (parent != null && internal.keys.size() < t - 1) {
                fixInternalUnderflow(parent, idx, internal);
            }
            return true;
        }
    }

    /**
     * Repair underflow in a leaf node.
     *
     * @param parent the parent of the underflowing leaf
     * @param idx    the index of the underflowing leaf in parent.children
     * @param leaf   the underflowing leaf
     */
    private void fixLeafUnderflow(InternalNode<K, V> parent, int idx,
                                  LeafNode<K, V> leaf) {
        // Try borrowing from the right sibling.
        if (idx + 1 < parent.children.size()) {
            LeafNode<K, V> right = (LeafNode<K, V>) parent.children.get(idx + 1);
            if (right.keys.size() > t - 1) {
                // Borrow the leftmost key from the right sibling.
                leaf.keys.add(right.keys.remove(0));
                leaf.values.add(right.values.remove(0));
                // Update the separator in the parent to the new smallest key of right.
                parent.keys.set(idx, right.keys.get(0));
                return;
            }
        }

        // Try borrowing from the left sibling.
        if (idx > 0) {
            LeafNode<K, V> left = (LeafNode<K, V>) parent.children.get(idx - 1);
            if (left.keys.size() > t - 1) {
                // Borrow the rightmost key from the left sibling.
                int last = left.keys.size() - 1;
                leaf.keys.add(0, left.keys.remove(last));
                leaf.values.add(0, left.values.remove(last));
                // Update the separator in the parent to the new smallest key of leaf.
                parent.keys.set(idx - 1, leaf.keys.get(0));
                return;
            }
        }

        // Neither sibling has spare keys — must merge.
        if (idx + 1 < parent.children.size()) {
            // Merge leaf with its RIGHT sibling.
            LeafNode<K, V> right = (LeafNode<K, V>) parent.children.get(idx + 1);
            // Move all right's keys into leaf.
            leaf.keys.addAll(right.keys);
            leaf.values.addAll(right.values);
            leaf.next = right.next;  // skip right in the linked list
            // Remove the separator key and the right child from the parent.
            parent.keys.remove(idx);
            parent.children.remove(idx + 1);
        } else {
            // Merge leaf with its LEFT sibling.
            LeafNode<K, V> left = (LeafNode<K, V>) parent.children.get(idx - 1);
            left.keys.addAll(leaf.keys);
            left.values.addAll(leaf.values);
            left.next = leaf.next;  // skip leaf in linked list
            parent.keys.remove(idx - 1);
            parent.children.remove(idx);
        }
    }

    /**
     * Repair underflow in an internal node.
     *
     * @param parent the parent of the underflowing internal node
     * @param idx    the index of the underflowing node in parent.children
     * @param node   the underflowing internal node
     */
    private void fixInternalUnderflow(InternalNode<K, V> parent, int idx,
                                      InternalNode<K, V> node) {
        // Try borrowing from the right sibling.
        if (idx + 1 < parent.children.size()) {
            InternalNode<K, V> right = (InternalNode<K, V>) parent.children.get(idx + 1);
            if (right.keys.size() > t - 1) {
                // Rotate: parent's separator goes down to node; right's first key goes up.
                node.keys.add(parent.keys.get(idx));
                parent.keys.set(idx, right.keys.remove(0));
                node.children.add(right.children.remove(0));
                return;
            }
        }

        // Try borrowing from the left sibling.
        if (idx > 0) {
            InternalNode<K, V> left = (InternalNode<K, V>) parent.children.get(idx - 1);
            if (left.keys.size() > t - 1) {
                // Rotate: parent's separator goes down to node; left's last key goes up.
                node.keys.add(0, parent.keys.get(idx - 1));
                parent.keys.set(idx - 1, left.keys.remove(left.keys.size() - 1));
                node.children.add(0, left.children.remove(left.children.size() - 1));
                return;
            }
        }

        // Must merge.
        if (idx + 1 < parent.children.size()) {
            // Merge with RIGHT sibling.
            InternalNode<K, V> right = (InternalNode<K, V>) parent.children.get(idx + 1);
            K separatorDown = parent.keys.remove(idx);
            parent.children.remove(idx + 1);
            node.keys.add(separatorDown);
            node.keys.addAll(right.keys);
            node.children.addAll(right.children);
        } else {
            // Merge with LEFT sibling.
            InternalNode<K, V> left = (InternalNode<K, V>) parent.children.get(idx - 1);
            K separatorDown = parent.keys.remove(idx - 1);
            parent.children.remove(idx);
            left.keys.add(separatorDown);
            left.keys.addAll(node.keys);
            left.children.addAll(node.children);
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
    //
    // This is dramatically faster than a B-tree range scan because:
    //   - No backtracking up the tree.
    //   - Sequential memory access on the leaf level → CPU cache-friendly and
    //     OS prefetch-friendly on disk.

    /**
     * Return all (key, value) pairs where {@code low ≤ key ≤ high}, sorted by key.
     *
     * <p>Time: O(t · log_t n + k) where k = number of results.
     *
     * <p>Note: all matched entries are materialised into a single list.  For very
     * wide ranges on large trees, prefer iterating via {@link #iterator()} instead.
     *
     * @param low  inclusive lower bound (must be ≤ high)
     * @param high inclusive upper bound (must be ≥ low)
     * @return sorted list of matching entries
     * @throws NullPointerException     if {@code low} or {@code high} is null
     * @throws IllegalArgumentException if {@code low.compareTo(high) > 0}
     */
    public List<Map.Entry<K, V>> rangeScan(K low, K high) {
        requireNonNullKey(low);
        requireNonNullKey(high);
        if (low.compareTo(high) > 0)
            throw new IllegalArgumentException(
                "rangeScan: low must be ≤ high, got low=" + low + " high=" + high);
        List<Map.Entry<K, V>> results = new ArrayList<>();
        LeafNode<K, V> leaf = findLeaf(root, low);

        // Walk the linked list until we pass 'high'.
        while (leaf != null) {
            for (int i = 0; i < leaf.keys.size(); i++) {
                K k = leaf.keys.get(i);
                if (k.compareTo(high) > 0) return results;  // past the end
                if (k.compareTo(low) >= 0) {
                    results.add(new AbstractMap.SimpleImmutableEntry<>(k, leaf.values.get(i)));
                }
            }
            leaf = leaf.next;
        }
        return results;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Full Scan
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Walk the entire leaf linked list from left to right.
    // O(n) — trivially sequential.

    /**
     * Return all (key, value) pairs in sorted key order.
     *
     * <p>Walks the leaf linked list from {@code firstLeaf} to the end.
     * Time: O(n).
     */
    public List<Map.Entry<K, V>> fullScan() {
        List<Map.Entry<K, V>> results = new ArrayList<>();
        LeafNode<K, V> leaf = firstLeaf;
        while (leaf != null) {
            for (int i = 0; i < leaf.keys.size(); i++) {
                results.add(new AbstractMap.SimpleImmutableEntry<>(leaf.keys.get(i), leaf.values.get(i)));
            }
            leaf = leaf.next;
        }
        return results;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Min Key / Max Key
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the smallest key.  O(1) — just read firstLeaf.keys.get(0).
     *
     * @throws NoSuchElementException if the tree is empty
     */
    public K minKey() {
        if (isEmpty()) throw new NoSuchElementException("Tree is empty");
        return firstLeaf.keys.get(0);
    }

    /**
     * Return the largest key.  O(log_t n) — follow the rightmost path.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    public K maxKey() {
        if (isEmpty()) throw new NoSuchElementException("Tree is empty");
        BPlusNode<K, V> node = root;
        while (node instanceof InternalNode<K, V> internal) {
            node = internal.children.get(internal.children.size() - 1);
        }
        LeafNode<K, V> leaf = (LeafNode<K, V>) node;
        return leaf.keys.get(leaf.keys.size() - 1);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Metadata
    // ─────────────────────────────────────────────────────────────────────────

    /** Total number of (key, value) pairs stored. */
    public int size()    { return size; }

    /** True if no keys are stored. */
    public boolean isEmpty() { return size == 0; }

    /**
     * Height of the tree.  An empty tree or a single-leaf tree has height 0.
     * Each additional level of internal nodes adds 1.
     */
    public int height() {
        int h = 0;
        BPlusNode<K, V> node = root;
        while (node instanceof InternalNode<K, V> internal) {
            h++;
            node = internal.children.get(0);
        }
        return h;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Iterator
    // ─────────────────────────────────────────────────────────────────────────

    /** Iterate all (key, value) pairs in ascending key order. */
    @Override
    public Iterator<Map.Entry<K, V>> iterator() {
        return fullScan().iterator();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Validation
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Verify all B+ tree invariants:
    //   1. All leaves at the same depth.
    //   2. Non-root internal nodes: t-1 ≤ keys ≤ 2t-1.
    //   3. All leaves: t-1 ≤ keys ≤ 2t-1 (except root-leaf).
    //   4. Leaf linked list covers all keys in sorted order.
    //   5. Separator keys in internal nodes match the smallest key of the right child.

    /**
     * Verify all B+ tree invariants.  O(n).  For testing and debugging only.
     *
     * <p>Checks:
     * <ol>
     *   <li>All leaves are at the same depth.
     *   <li>Key-count bounds: every non-root node has t-1 ≤ keys ≤ 2t-1.
     *   <li>Leaf linked list is globally sorted and contains exactly {@code size} keys.
     *   <li>Routing invariant: for every internal node and every separator index i,
     *       the maximum key in {@code children[i]} is strictly less than {@code keys[i]},
     *       and the minimum key in {@code children[i+1]} is ≥ {@code keys[i]}.
     *       (Separator keys may be stale copies — not necessarily the exact minimum of
     *       the right child — after a non-structural delete.  The weaker routing-invariant
     *       check is the correct criterion for a B+ tree.)
     * </ol>
     *
     * @return true if all invariants hold
     */
    public boolean isValid() {
        if (root == null) return false;

        // 1. All leaves at the same depth.
        int leafDepth = computeLeafDepth(root, 0);
        if (leafDepth < 0) return false;

        // 2 & 3. Key count invariants (skip for root which may have 0 keys).
        if (!validateKeyCount(root, true)) return false;

        // 4. Linked list is sorted and contains every key exactly once.
        List<K> leafKeys = new ArrayList<>();
        LeafNode<K, V> leaf = firstLeaf;
        while (leaf != null) {
            // Each leaf's own keys must be sorted.
            for (int i = 1; i < leaf.keys.size(); i++) {
                if (leaf.keys.get(i - 1).compareTo(leaf.keys.get(i)) >= 0) return false;
            }
            leafKeys.addAll(leaf.keys);
            leaf = leaf.next;
        }
        // Global sorted order across all leaves.
        for (int i = 1; i < leafKeys.size(); i++) {
            if (leafKeys.get(i - 1).compareTo(leafKeys.get(i)) >= 0) return false;
        }
        if (leafKeys.size() != size) return false;

        // 5. Routing invariant for separator keys in internal nodes.
        if (!validateSeparators(root)) return false;

        return true;
    }

    /** Returns the depth at which leaves are found, or -1 if inconsistent. */
    private int computeLeafDepth(BPlusNode<K, V> node, int depth) {
        if (node instanceof LeafNode) return depth;
        InternalNode<K, V> internal = (InternalNode<K, V>) node;
        int firstDepth = -1;
        for (BPlusNode<K, V> child : internal.children) {
            int d = computeLeafDepth(child, depth + 1);
            if (d < 0) return -1;
            if (firstDepth < 0) firstDepth = d;
            else if (firstDepth != d) return -1;
        }
        return firstDepth;
    }

    /** Verify key count invariants (t-1 ≤ keys ≤ 2t-1 for non-root nodes). */
    private boolean validateKeyCount(BPlusNode<K, V> node, boolean isRoot) {
        if (node instanceof LeafNode<K, V> leaf) {
            if (!isRoot && leaf.keys.size() < t - 1) return false;
            if (leaf.keys.size() > 2 * t - 1) return false;
            return true;
        }
        InternalNode<K, V> internal = (InternalNode<K, V>) node;
        if (!isRoot && internal.keys.size() < t - 1) return false;
        if (internal.keys.size() > 2 * t - 1) return false;
        for (BPlusNode<K, V> child : internal.children) {
            if (!validateKeyCount(child, false)) return false;
        }
        return true;
    }

    /**
     * Verify the routing invariant for separator keys.
     *
     * <p>For each separator {@code keys[i]} in an internal node:
     * <ul>
     *   <li>The maximum key in {@code children[i]} must be <em>strictly less</em> than
     *       {@code keys[i]}.
     *   <li>The minimum key in {@code children[i+1]} must be ≥ {@code keys[i]}.
     * </ul>
     *
     * <p>Note: in a B+ tree the separator need NOT equal the exact minimum of the right
     * child — after a non-structural delete the separator may be stale (the deleted key
     * was a separator copy, but the right child's new minimum is larger).  The weaker
     * routing invariant still guarantees correct search behaviour.
     */
    private boolean validateSeparators(BPlusNode<K, V> node) {
        if (node instanceof LeafNode) return true;
        InternalNode<K, V> internal = (InternalNode<K, V>) node;
        for (int i = 0; i < internal.keys.size(); i++) {
            K sep = internal.keys.get(i);
            // All keys in children[i] must be strictly less than sep.
            K rightmostLeft = rightmostLeafKey(internal.children.get(i));
            if (rightmostLeft.compareTo(sep) >= 0) return false;
            // All keys in children[i+1] must be >= sep.
            K leftmostRight = leftmostLeafKey(internal.children.get(i + 1));
            if (leftmostRight.compareTo(sep) < 0) return false;
        }
        for (BPlusNode<K, V> child : internal.children) {
            if (!validateSeparators(child)) return false;
        }
        return true;
    }

    /** Return the leftmost (minimum) key reachable from this node. */
    private K leftmostLeafKey(BPlusNode<K, V> node) {
        while (node instanceof InternalNode<K, V> internal) {
            node = internal.children.get(0);
        }
        return ((LeafNode<K, V>) node).keys.get(0);
    }

    /** Return the rightmost (maximum) key reachable from this node. */
    private K rightmostLeafKey(BPlusNode<K, V> node) {
        while (node instanceof InternalNode<K, V> internal) {
            node = internal.children.get(internal.children.size() - 1);
        }
        LeafNode<K, V> leaf = (LeafNode<K, V>) node;
        return leaf.keys.get(leaf.keys.size() - 1);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Private Helpers
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Guard against null keys at every public entry point.
     *
     * <p>Null keys would cause a NullPointerException deep inside compareTo()
     * with a confusing stack trace.  Reject them explicitly at the boundary.
     */
    private static <K> void requireNonNullKey(K key) {
        if (key == null) throw new NullPointerException("key must not be null");
    }

    /**
     * Find the leaf that should contain {@code key}.
     * Always descends to a leaf — never stops at an internal node.
     */
    private LeafNode<K, V> findLeaf(BPlusNode<K, V> node, K key) {
        while (node instanceof InternalNode<K, V> internal) {
            node = internal.children.get(findChildIndex(internal, key));
        }
        return (LeafNode<K, V>) node;
    }

    /**
     * Find the child index to follow for {@code key} in an internal node.
     *
     * <p>Returns i such that all keys in children[i] are ≤ key and all keys in
     * children[i+1] are > key (for the relevant separators).
     *
     * <p>Specifically: return the number of separator keys strictly ≤ key.
     * This is the correct child to follow in a B+ tree where:
     *   keys[i] = smallest key in children[i+1].
     */
    private int findChildIndex(InternalNode<K, V> node, K key) {
        int i = 0;
        while (i < node.keys.size() && key.compareTo(node.keys.get(i)) >= 0) {
            i++;
        }
        return i;
    }

    /**
     * Find the position in a sorted leaf where {@code key} sits (binary search).
     *
     * @return index i if {@code leaf.keys.get(i).equals(key)}, or -(i+1) if absent
     */
    private int leafIndexOf(LeafNode<K, V> leaf, K key) {
        int lo = 0, hi = leaf.keys.size() - 1;
        while (lo <= hi) {
            int mid = (lo + hi) >>> 1;
            int cmp = leaf.keys.get(mid).compareTo(key);
            if      (cmp < 0) lo = mid + 1;
            else if (cmp > 0) hi = mid - 1;
            else return mid;
        }
        return -1;
    }

    /**
     * Find the sorted insertion position for {@code key} in a leaf.
     *
     * @return index where key should be inserted (0 to leaf.keys.size())
     */
    private int leafInsertPosition(LeafNode<K, V> leaf, K key) {
        int lo = 0, hi = leaf.keys.size();
        while (lo < hi) {
            int mid = (lo + hi) >>> 1;
            if (leaf.keys.get(mid).compareTo(key) < 0) lo = mid + 1;
            else hi = mid;
        }
        return lo;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    @Override
    public String toString() {
        return "BPlusTree{size=" + size + ", height=" + height() + ", t=" + t + "}";
    }
}
