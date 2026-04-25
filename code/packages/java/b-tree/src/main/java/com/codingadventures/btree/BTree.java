// ============================================================================
// BTree.java — Self-Balancing Multi-Way Search Tree
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

package com.codingadventures.btree;

import java.util.AbstractMap;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.NoSuchElementException;

/**
 * A self-balancing multi-way search tree (B-tree) mapping comparable keys
 * to arbitrary values.
 *
 * <p>O(t·log_t n) for insert, delete, and search, where t is the minimum
 * degree and n is the number of keys. All leaves are at exactly the same
 * depth — the tree never becomes unbalanced.
 *
 * <pre>{@code
 * BTree<Integer, String> tree = new BTree<>(2);
 * tree.insert(5, "five");
 * tree.insert(3, "three");
 * tree.insert(7, "seven");
 *
 * tree.search(3);              // "three"
 * tree.contains(5);            // true
 * tree.minKey();               // 3
 * tree.maxKey();               // 7
 * tree.rangeQuery(3, 6);       // [(3,"three"), (5,"five")]
 * tree.height();               // 1
 *
 * tree.delete(3);
 * tree.contains(3);            // false
 * tree.size();                 // 2
 * }</pre>
 *
 * @param <K> the key type; must be {@link Comparable}
 * @param <V> the value type
 */
public class BTree<K extends Comparable<K>, V> {

    // =========================================================================
    // Inner class: BTreeNode
    // =========================================================================

    /**
     * A single node in the B-tree.
     *
     * <p>A node is like a mini sorted array: it holds {@code keys[0..n-1]}
     * in ascending order and, for an internal node, {@code n+1} child pointers
     * in {@code children[0..n]}.
     *
     * <p>Think of it as an airport departure board: it lists destinations
     * (keys) in order, and the gaps between destinations indicate which child
     * (gate) leads to flights in that range.
     *
     * <p>Invariants (for minimum degree t, and this is not the root):
     * <ul>
     *   <li>{@code t-1 ≤ keys.size() ≤ 2t-1}</li>
     *   <li>{@code isLeaf ? children.isEmpty() : children.size() == keys.size()+1}</li>
     *   <li>{@code keys} is strictly sorted in ascending order</li>
     * </ul>
     */
    static final class Node<K extends Comparable<K>, V> {
        final List<K>        keys;       // sorted keys in this node
        final List<V>        values;     // values[i] corresponds to keys[i]
        final List<Node<K,V>> children;  // child pointers (empty for leaves)
        boolean              isLeaf;

        Node(boolean isLeaf) {
            this.keys     = new ArrayList<>();
            this.values   = new ArrayList<>();
            this.children = new ArrayList<>();
            this.isLeaf   = isLeaf;
        }

        /** Return true if this node is at maximum capacity (2t-1 keys). */
        boolean isFull(int t) {
            return keys.size() == 2 * t - 1;
        }

        /**
         * Binary-search for the leftmost index {@code i} such that
         * {@code keys.get(i) >= key}.
         *
         * <p>If key is present, this is its index. If absent, this is the
         * index of the child to descend into.
         *
         * <p>Example: keys = [10, 20, 30], findKeyIndex(15) → 1.
         * (Descend into children[1], which covers keys in (10, 20).)
         */
        int findKeyIndex(K key) {
            int lo = 0, hi = keys.size();
            while (lo < hi) {
                int mid = (lo + hi) >>> 1;
                if (keys.get(mid).compareTo(key) < 0) lo = mid + 1;
                else                                  hi = mid;
            }
            return lo;
        }
    }

    // =========================================================================
    // Fields
    // =========================================================================

    private final int t;       // minimum degree
    private Node<K,V> root;
    private int       size;

    // =========================================================================
    // Constructor
    // =========================================================================

    /**
     * Construct a B-tree with the given minimum degree.
     *
     * @param t the minimum degree; must be ≥ 2
     * @throws IllegalArgumentException if {@code t < 2}
     */
    public BTree(int t) {
        if (t < 2) throw new IllegalArgumentException("Minimum degree t must be >= 2, got " + t);
        this.t    = t;
        this.root = new Node<>(true);
        this.size = 0;
    }

    /** Construct a B-tree with the default minimum degree of 2 (a 2-3-4 tree). */
    public BTree() {
        this(2);
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Insert {@code key} with associated {@code value}.
     *
     * <p>If {@code key} already exists, its value is updated in place.
     *
     * <p>Algorithm (CLRS B-TREE-INSERT):
     * <ol>
     *   <li>If the root is full, split it: create a new root, make the old
     *       root its first child, split that child. Height increases by 1.
     *   <li>Call {@code insertNonfull} on the (now non-full) root.
     * </ol>
     *
     * @param key   the key; must not be null
     * @param value the value
     */
    public void insert(K key, V value) {
        if (key == null) throw new IllegalArgumentException("Key must not be null");
        Node<K,V> r = root;
        if (r.isFull(t)) {
            // Root is full — grow the tree upward
            Node<K,V> newRoot = new Node<>(false);
            newRoot.children.add(r);
            splitChild(newRoot, 0);
            root = newRoot;
            if (insertNonfull(newRoot, key, value)) size++;
        } else {
            if (insertNonfull(r, key, value)) size++;
        }
    }

    /**
     * Remove {@code key} from the B-tree.
     *
     * <p>After deletion, if the root is left with no keys (due to a merge
     * of its two children), the first child becomes the new root and the
     * tree shrinks in height.
     *
     * @param key the key to remove
     * @throws NoSuchElementException if the key is not present
     */
    public void delete(K key) {
        if (key == null || !contains(root, key)) {
            throw new NoSuchElementException("Key not found: " + key);
        }
        deleteRec(root, key);
        size--;
        // If root is now keyless but has a child, shrink the tree
        if (root.keys.isEmpty() && !root.children.isEmpty()) {
            root = root.children.get(0);
        }
    }

    /**
     * Return the value associated with {@code key}, or {@code null} if absent.
     *
     * @param key the key to look up
     */
    public V search(K key) {
        if (key == null) return null;
        return searchRec(root, key);
    }

    /** Return {@code true} if {@code key} is present in the tree. */
    public boolean contains(K key) {
        if (key == null) return false;
        return contains(root, key);
    }

    /**
     * Return the smallest key in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    public K minKey() {
        if (size == 0) throw new NoSuchElementException("Tree is empty");
        return minNode(root).keys.get(0);
    }

    /**
     * Return the largest key in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    public K maxKey() {
        if (size == 0) throw new NoSuchElementException("Tree is empty");
        Node<K,V> node = root;
        while (!node.isLeaf) node = node.children.get(node.children.size() - 1);
        return node.keys.get(node.keys.size() - 1);
    }

    /**
     * Return all {@code (key, value)} pairs where {@code low <= key <= high},
     * in ascending key order.
     *
     * @param low  the inclusive lower bound
     * @param high the inclusive upper bound
     */
    public List<Map.Entry<K,V>> rangeQuery(K low, K high) {
        List<Map.Entry<K,V>> result = new ArrayList<>();
        for (Map.Entry<K,V> entry : inorder()) {
            if (entry.getKey().compareTo(high) > 0) break;
            if (entry.getKey().compareTo(low) >= 0) result.add(entry);
        }
        return result;
    }

    /**
     * Return an {@link Iterable} over all {@code (key, value)} pairs in
     * ascending key order.
     *
     * <p>The in-order traversal generalises BST in-order to B-trees: for a
     * node with keys {@code [k0, k1, k2]} and children {@code [c0, c1, c2, c3]},
     * we yield all from c0, then k0, then all from c1, then k1, and so on.
     */
    public Iterable<Map.Entry<K,V>> inorder() {
        List<Map.Entry<K,V>> result = new ArrayList<>(size);
        collectInorder(root, result);
        return result;
    }

    /**
     * Return the height of the tree.
     *
     * <p>A single-node tree (leaf) has height 0; each additional level adds 1.
     * All paths from root to leaf have exactly this length — the key B-tree
     * invariant.
     */
    public int height() {
        Node<K,V> node = root;
        int h = 0;
        while (!node.isLeaf) { node = node.children.get(0); h++; }
        return h;
    }

    /** Return the number of key-value pairs in the tree. */
    public int size() { return size; }

    /** Return {@code true} if the tree contains no key-value pairs. */
    public boolean isEmpty() { return size == 0; }

    /**
     * Validate all B-tree structural invariants.
     *
     * <p>Invariants checked:
     * <ol>
     *   <li>Key count bounds: {@code t-1 ≤ keys ≤ 2t-1} for non-root nodes</li>
     *   <li>Root has at least 1 key (unless tree is empty)</li>
     *   <li>Keys within each node are strictly increasing</li>
     *   <li>Keys respect BST ordering between parents and children</li>
     *   <li>Internal nodes have exactly {@code keys.size()+1} children</li>
     *   <li>All leaves are at the same depth</li>
     * </ol>
     *
     * @return {@code true} if the tree is structurally valid
     */
    public boolean isValid() {
        if (size == 0) return true;
        int[] leafDepth = {-1};
        return validate(root, null, null, 0, leafDepth, true);
    }

    @Override
    public String toString() {
        return "BTree(t=" + t + ", size=" + size + ", height=" + height() + ")";
    }

    // =========================================================================
    // Private helpers — insertion
    // =========================================================================

    /**
     * Split {@code parent.children[childIndex]}, which must be full.
     *
     * <p>The median key (index t-1) is promoted to the parent. The left child
     * retains keys[0..t-2]; the right child gets keys[t..2t-2]. Children
     * (if the node is internal) are split the same way.
     *
     * <p>This is O(t) work — we copy t-1 keys and (for internal nodes) t pointers.
     */
    private void splitChild(Node<K,V> parent, int childIndex) {
        Node<K,V> child = parent.children.get(childIndex);
        Node<K,V> right = new Node<>(child.isLeaf);

        int mid = t - 1;   // index of the median key in child.keys

        // Promote median to parent
        parent.keys.add(childIndex, child.keys.get(mid));
        parent.values.add(childIndex, child.values.get(mid));
        parent.children.add(childIndex + 1, right);

        // Right node: upper half of keys/values/children
        right.keys.addAll(  child.keys.subList(  mid + 1, child.keys.size()));
        right.values.addAll(child.values.subList(mid + 1, child.values.size()));
        if (!child.isLeaf) {
            right.children.addAll(child.children.subList(t, child.children.size()));
            // Keep only first t children in child (left)
            while (child.children.size() > t) child.children.remove(child.children.size() - 1);
        }

        // Left node: lower half (trim to first t-1 keys)
        while (child.keys.size() > mid)   child.keys.remove(child.keys.size() - 1);
        while (child.values.size() > mid) child.values.remove(child.values.size() - 1);
    }

    /**
     * Insert {@code key} into the subtree rooted at {@code node}, assuming
     * {@code node} is NOT full.
     *
     * <p>Returns {@code true} if this was a new key (size should increase),
     * {@code false} if an existing key was updated.
     */
    private boolean insertNonfull(Node<K,V> node, K key, V value) {
        int i = node.findKeyIndex(key);

        // Check for exact match at this node
        if (i < node.keys.size() && node.keys.get(i).compareTo(key) == 0) {
            node.values.set(i, value);   // update in place
            return false;
        }

        if (node.isLeaf) {
            // Sorted insertion at position i
            node.keys.add(i, key);
            node.values.add(i, value);
            return true;
        }

        // Internal: pre-split child[i] if full (proactive top-down splitting)
        if (node.children.get(i).isFull(t)) {
            splitChild(node, i);
            // After split, node.keys[i] is the promoted median
            int cmp = key.compareTo(node.keys.get(i));
            if (cmp == 0) {
                node.values.set(i, value);
                return false;
            } else if (cmp > 0) {
                i++;  // descend into the right half
            }
        }
        return insertNonfull(node.children.get(i), key, value);
    }

    // =========================================================================
    // Private helpers — search
    // =========================================================================

    private V searchRec(Node<K,V> node, K key) {
        int i = node.findKeyIndex(key);
        if (i < node.keys.size() && node.keys.get(i).compareTo(key) == 0) {
            return node.values.get(i);
        }
        if (node.isLeaf) return null;
        return searchRec(node.children.get(i), key);
    }

    private boolean contains(Node<K,V> node, K key) {
        int i = node.findKeyIndex(key);
        if (i < node.keys.size() && node.keys.get(i).compareTo(key) == 0) return true;
        if (node.isLeaf) return false;
        return contains(node.children.get(i), key);
    }

    // =========================================================================
    // Private helpers — min/max
    // =========================================================================

    private Node<K,V> minNode(Node<K,V> node) {
        while (!node.isLeaf) node = node.children.get(0);
        return node;
    }

    // =========================================================================
    // Private helpers — deletion
    // =========================================================================

    /**
     * Recursively delete {@code key} from the subtree rooted at {@code node}.
     *
     * <p>Precondition: {@code node} has at least t keys (guaranteed by
     * {@code ensureMinKeys} on every descent), unless {@code node} is the root.
     */
    private void deleteRec(Node<K,V> node, K key) {
        int i = node.findKeyIndex(key);
        boolean found = i < node.keys.size() && node.keys.get(i).compareTo(key) == 0;

        if (found) {
            if (node.isLeaf) {
                // Case 1: key is in a leaf — simply remove it
                node.keys.remove(i);
                node.values.remove(i);
            } else {
                Node<K,V> leftChild  = node.children.get(i);
                Node<K,V> rightChild = node.children.get(i + 1);

                if (leftChild.keys.size() >= t) {
                    // Case 2a: left child has spare key — use predecessor
                    Node<K,V> predNode = maxNode(leftChild);
                    K predKey = predNode.keys.get(predNode.keys.size() - 1);
                    V predVal = predNode.values.get(predNode.values.size() - 1);
                    node.keys.set(i, predKey);
                    node.values.set(i, predVal);
                    deleteRec(leftChild, predKey);

                } else if (rightChild.keys.size() >= t) {
                    // Case 2b: right child has spare key — use successor
                    Node<K,V> succNode = minNode(rightChild);
                    K succKey = succNode.keys.get(0);
                    V succVal = succNode.values.get(0);
                    node.keys.set(i, succKey);
                    node.values.set(i, succVal);
                    deleteRec(rightChild, succKey);

                } else {
                    // Case 2c: both have t-1 keys — merge
                    Node<K,V> merged = mergeChildren(node, i);
                    deleteRec(merged, key);
                }
            }
        } else {
            // Key not in this node; descend, pre-filling if needed (Case 3)
            if (node.isLeaf) return;   // key not present (shouldn't reach here)

            i = ensureMinKeys(node, i);
            deleteRec(node.children.get(i), key);
        }
    }

    private Node<K,V> maxNode(Node<K,V> node) {
        while (!node.isLeaf) node = node.children.get(node.children.size() - 1);
        return node;
    }

    /**
     * Merge {@code parent.children[leftIdx]} with {@code parent.children[leftIdx+1]},
     * pulling down the separator key from the parent.
     *
     * <p>The merged node = left.keys + [separator] + right.keys.
     * The separator is removed from the parent, and the right child pointer
     * is removed from the parent's children list.
     *
     * <p>Returns the merged node (which is at {@code parent.children[leftIdx]}).
     */
    private Node<K,V> mergeChildren(Node<K,V> parent, int leftIdx) {
        Node<K,V> left  = parent.children.get(leftIdx);
        Node<K,V> right = parent.children.get(leftIdx + 1);

        // Pull down separator from parent
        left.keys.add(parent.keys.remove(leftIdx));
        left.values.add(parent.values.remove(leftIdx));
        parent.children.remove(leftIdx + 1);

        // Append right's keys/values/children to left
        left.keys.addAll(right.keys);
        left.values.addAll(right.values);
        if (!left.isLeaf) left.children.addAll(right.children);

        return left;
    }

    /**
     * Ensure that {@code parent.children[childIdx]} has at least t keys.
     *
     * <p>If the child is already fat enough, returns {@code childIdx} unchanged.
     *
     * <p>Otherwise:
     * <ul>
     *   <li><b>Case 3a</b>: Borrow from a sibling with ≥ t keys (rotate through parent).</li>
     *   <li><b>Case 3b</b>: Merge with a sibling (pulls separator down from parent).</li>
     * </ul>
     *
     * @return the (possibly shifted) child index to descend into
     */
    private int ensureMinKeys(Node<K,V> parent, int childIdx) {
        Node<K,V> child = parent.children.get(childIdx);
        if (child.keys.size() >= t) return childIdx;

        // Try to borrow from left sibling
        if (childIdx > 0) {
            Node<K,V> leftSib = parent.children.get(childIdx - 1);
            if (leftSib.keys.size() >= t) {
                // Rotate right: pull parent separator down to child front
                child.keys.add(0, parent.keys.get(childIdx - 1));
                child.values.add(0, parent.values.get(childIdx - 1));
                // Move left sibling's last key up to parent
                int ls = leftSib.keys.size() - 1;
                parent.keys.set(childIdx - 1, leftSib.keys.remove(ls));
                parent.values.set(childIdx - 1, leftSib.values.remove(ls));
                // Move left sibling's last child to child's first child
                if (!leftSib.isLeaf) {
                    child.children.add(0, leftSib.children.remove(leftSib.children.size() - 1));
                }
                return childIdx;
            }
        }

        // Try to borrow from right sibling
        if (childIdx < parent.children.size() - 1) {
            Node<K,V> rightSib = parent.children.get(childIdx + 1);
            if (rightSib.keys.size() >= t) {
                // Rotate left: pull parent separator down to child end
                child.keys.add(parent.keys.get(childIdx));
                child.values.add(parent.values.get(childIdx));
                // Move right sibling's first key up to parent
                parent.keys.set(childIdx, rightSib.keys.remove(0));
                parent.values.set(childIdx, rightSib.values.remove(0));
                // Move right sibling's first child to child's last child
                if (!rightSib.isLeaf) {
                    child.children.add(rightSib.children.remove(0));
                }
                return childIdx;
            }
        }

        // Must merge (Case 3b): no sibling has a spare key
        if (childIdx > 0) {
            mergeChildren(parent, childIdx - 1);
            return childIdx - 1;   // merged node is now at childIdx - 1
        } else {
            mergeChildren(parent, childIdx);
            return childIdx;       // merged node stays at childIdx
        }
    }

    // =========================================================================
    // Private helpers — in-order traversal
    // =========================================================================

    /**
     * Collect (key, value) pairs in ascending order into {@code result}.
     *
     * <p>For a node with keys [k0, k1, k2] and children [c0, c1, c2, c3]:
     * traverse c0, emit k0, traverse c1, emit k1, traverse c2, emit k2,
     * traverse c3.
     */
    private void collectInorder(Node<K,V> node, List<Map.Entry<K,V>> result) {
        if (node.isLeaf) {
            for (int i = 0; i < node.keys.size(); i++) {
                result.add(new AbstractMap.SimpleImmutableEntry<>(node.keys.get(i), node.values.get(i)));
            }
            return;
        }
        for (int i = 0; i < node.keys.size(); i++) {
            collectInorder(node.children.get(i), result);
            result.add(new AbstractMap.SimpleImmutableEntry<>(node.keys.get(i), node.values.get(i)));
        }
        collectInorder(node.children.get(node.children.size() - 1), result);
    }

    // =========================================================================
    // Private helpers — validation
    // =========================================================================

    /**
     * Recursively validate B-tree invariants.
     *
     * @param node          current node
     * @param minKey        lower bound for keys (exclusive); null means no bound
     * @param maxKey        upper bound for keys (exclusive); null means no bound
     * @param depth         current depth from root
     * @param leafDepth     one-element array holding the expected leaf depth
     *                      (-1 means not yet set)
     * @param isRoot        true if this is the root node
     * @return true if the subtree is valid
     */
    private boolean validate(Node<K,V> node, K minKey, K maxKey, int depth,
                              int[] leafDepth, boolean isRoot) {
        int n = node.keys.size();

        // Check key count bounds
        if (isRoot) {
            if (size > 0 && n < 1) return false;
        } else {
            if (n < t - 1 || n > 2 * t - 1) return false;
        }

        // Check keys are sorted and within bounds
        for (int j = 0; j < n; j++) {
            K k = node.keys.get(j);
            if (minKey != null && k.compareTo(minKey) <= 0) return false;
            if (maxKey != null && k.compareTo(maxKey) >= 0) return false;
            if (j > 0 && k.compareTo(node.keys.get(j - 1)) <= 0) return false;
        }

        if (node.isLeaf) {
            // Check child count
            if (!node.children.isEmpty()) return false;
            // Record/check leaf depth
            if (leafDepth[0] == -1) leafDepth[0] = depth;
            else if (leafDepth[0] != depth) return false;
        } else {
            // Internal: children.size() must be keys.size() + 1
            if (node.children.size() != n + 1) return false;
            for (int j = 0; j <= n; j++) {
                K lo = (j > 0)  ? node.keys.get(j - 1) : minKey;
                K hi = (j < n)  ? node.keys.get(j)     : maxKey;
                if (!validate(node.children.get(j), lo, hi, depth + 1, leafDepth, false)) {
                    return false;
                }
            }
        }
        return true;
    }
}
