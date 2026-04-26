// ============================================================================
// BinarySearchTree.java — Mutable BST with Order Statistics
// ============================================================================
//
// A Binary Search Tree (BST) is a rooted binary tree where every node satisfies
// the BST property: all values in the left subtree are strictly less than the
// node's value, and all values in the right subtree are strictly greater.
//
// This gives us O(log n) expected time for insert, delete, and search (assuming
// the tree is reasonably balanced). The key insight is that the BST property
// allows us to halve the search space at each node — just like binary search on
// a sorted array.
//
//         5
//        / \
//       3   8
//      / \   \
//     1   4   9
//
// Searching for 4: start at 5 (go left) → 3 (go right) → 4 (found!)
// 3 comparisons instead of scanning all 6 nodes.
//
// Each node also stores a `size` field (the number of nodes in its subtree).
// This "augmented BST" supports two powerful operations:
//
//   kthSmallest(k) — find the k-th smallest value in O(log n)
//   rank(x)        — count how many values are less than x in O(log n)
//
// ============================================================================
// Deletion — the tricky case
// ============================================================================
//
// Deleting a node with two children requires a bit of elegance. We can't just
// remove it because both children would be orphaned. Instead, we replace it
// with its in-order successor (the smallest value in its right subtree), then
// delete the successor from the right subtree.
//
//   Before deleting 5:    After replacing with successor 7:
//         5                         7
//        / \                       / \
//       3   8                     3   8
//      / \   \                   / \   \
//     1   4   9                 1   4   9
//              ^
//        (7 was here — not shown, moved up)
//
// ============================================================================
// Size augmentation
// ============================================================================
//
// The size of a node = 1 + size(left) + size(right). We maintain this
// invariant on every insert, delete, and rotation. This enables:
//
//   kthSmallest(k):
//     if k == leftSize + 1  → current node is the answer
//     if k <= leftSize      → recurse into left subtree with same k
//     else                  → recurse into right subtree with k -= leftSize+1
//
//   rank(x):
//     walk the tree exactly like a search, accumulating left subtree sizes
//     whenever we go right.
//
// ============================================================================

package com.codingadventures.bst;

import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

/**
 * A mutable Binary Search Tree (BST) with order-statistics support.
 *
 * <p>Elements must implement {@link Comparable}. Duplicate values are ignored
 * (insert is a no-op if the value already exists).
 *
 * <p>Operations: {@link #insert}, {@link #delete}, {@link #search},
 * {@link #contains}, {@link #minValue}, {@link #maxValue},
 * {@link #predecessor}, {@link #successor}, {@link #kthSmallest},
 * {@link #rank}, {@link #toSortedList}, {@link #isValid},
 * {@link #height}, {@link #size}.
 *
 * <pre>{@code
 * BinarySearchTree<Integer> t = new BinarySearchTree<>();
 * for (int v : List.of(5, 1, 8, 3, 7)) t.insert(v);
 *
 * t.toSortedList();     // [1, 3, 5, 7, 8]
 * t.minValue();         // Optional[1]
 * t.kthSmallest(4);     // Optional[7]
 * t.rank(4);            // 2  (two values < 4: 1 and 3)
 * t.predecessor(5);     // Optional[3]
 * t.successor(5);       // Optional[7]
 *
 * t.delete(5);
 * t.contains(5);        // false
 * t.size();             // 4
 * }</pre>
 *
 * <p><b>Time complexity</b>: O(log n) expected for all operations on a
 * randomly inserted tree. Worst-case O(n) for a sorted input sequence.
 * Use {@link #fromSortedList} to build a balanced tree from a sorted list.
 *
 * @param <T> the element type; must be {@link Comparable}
 */
public class BinarySearchTree<T extends Comparable<T>> {

    // =========================================================================
    // Node (package-private for test access)
    // =========================================================================

    /**
     * A node in the BST.
     *
     * <p>{@code size} is the number of nodes in this subtree (including self),
     * maintained as an invariant by every mutation so that order-statistics
     * queries run in O(log n).
     */
    static final class Node<T> {
        T value;
        Node<T> left;
        Node<T> right;
        int size;            // subtree size, including self

        Node(T value) {
            this.value = value;
            this.size  = 1;
        }
    }

    // =========================================================================
    // Fields
    // =========================================================================

    Node<T> root;            // null for an empty tree

    // =========================================================================
    // Constructors / factory
    // =========================================================================

    /** Construct an empty BST. */
    public BinarySearchTree() {}

    /**
     * Build a balanced BST from a pre-sorted list in O(n) time.
     *
     * <p>The middle element becomes the root (or left-of-middle for even
     * lengths), guaranteeing a height of ⌊log₂ n⌋ and O(log n) operations.
     *
     * @param sortedValues a list whose elements are in ascending order
     * @return a new BST with all elements inserted
     */
    public static <T extends Comparable<T>> BinarySearchTree<T> fromSortedList(List<T> sortedValues) {
        BinarySearchTree<T> tree = new BinarySearchTree<>();
        tree.root = buildBalanced(sortedValues, 0, sortedValues.size() - 1);
        return tree;
    }

    // =========================================================================
    // Core mutation
    // =========================================================================

    /**
     * Insert {@code value} into the BST.
     *
     * <p>If the value already exists, this is a no-op (no duplicates).
     *
     * @param value the value to insert; must not be null
     * @throws IllegalArgumentException if {@code value} is null
     */
    public void insert(T value) {
        if (value == null) throw new IllegalArgumentException("Value must not be null");
        root = insert(root, value);
    }

    /**
     * Remove {@code value} from the BST.
     *
     * <p>If the value is not present, this is a no-op.
     *
     * @param value the value to remove; must not be null
     * @throws IllegalArgumentException if {@code value} is null
     */
    public void delete(T value) {
        if (value == null) throw new IllegalArgumentException("Value must not be null");
        root = delete(root, value);
    }

    // =========================================================================
    // Search
    // =========================================================================

    /**
     * Search for {@code value} in the BST.
     *
     * @return the node whose value equals {@code value}, or {@code null} if absent
     */
    public Node<T> search(T value) {
        if (value == null) return null;
        Node<T> current = root;
        while (current != null) {
            int cmp = value.compareTo(current.value);
            if      (cmp < 0) current = current.left;
            else if (cmp > 0) current = current.right;
            else              return current;
        }
        return null;
    }

    /** Return {@code true} if {@code value} is present in the BST. */
    public boolean contains(T value) {
        return search(value) != null;
    }

    // =========================================================================
    // Min / Max
    // =========================================================================

    /** Return the minimum value, or {@link Optional#empty()} if the tree is empty. */
    public Optional<T> minValue() {
        Node<T> current = root;
        while (current != null && current.left != null) current = current.left;
        return current == null ? Optional.empty() : Optional.of(current.value);
    }

    /** Return the maximum value, or {@link Optional#empty()} if the tree is empty. */
    public Optional<T> maxValue() {
        Node<T> current = root;
        while (current != null && current.right != null) current = current.right;
        return current == null ? Optional.empty() : Optional.of(current.value);
    }

    // =========================================================================
    // Predecessor / Successor
    // =========================================================================

    /**
     * Return the largest value strictly less than {@code value}, or
     * {@link Optional#empty()} if no such value exists.
     *
     * <p>Algorithm: walk the tree. When we go left (because current ≥ value)
     * the current node cannot be an answer. When we go right (because
     * current < value) the current node is a candidate — record it as the
     * best seen so far. The last recorded candidate is the predecessor.
     */
    public Optional<T> predecessor(T value) {
        if (value == null) return Optional.empty();
        Node<T> current = root;
        T best = null;
        while (current != null) {
            int cmp = value.compareTo(current.value);
            if (cmp <= 0) {
                current = current.left;
            } else {
                best = current.value;
                current = current.right;
            }
        }
        return best == null ? Optional.empty() : Optional.of(best);
    }

    /**
     * Return the smallest value strictly greater than {@code value}, or
     * {@link Optional#empty()} if no such value exists.
     */
    public Optional<T> successor(T value) {
        if (value == null) return Optional.empty();
        Node<T> current = root;
        T best = null;
        while (current != null) {
            int cmp = value.compareTo(current.value);
            if (cmp >= 0) {
                current = current.right;
            } else {
                best = current.value;
                current = current.left;
            }
        }
        return best == null ? Optional.empty() : Optional.of(best);
    }

    // =========================================================================
    // Order statistics
    // =========================================================================

    /**
     * Return the k-th smallest value (1-indexed), or {@link Optional#empty()}
     * if {@code k} is out of range.
     *
     * <p>Example: {@code kthSmallest(1)} returns the minimum;
     * {@code kthSmallest(size())} returns the maximum.
     *
     * <p>Algorithm leverages the size augmentation:
     * <pre>
     *   leftSize = size(node.left)
     *   if k == leftSize + 1  → this node is rank k
     *   if k <= leftSize      → descend left with the same k
     *   else                  → descend right with k -= (leftSize + 1)
     * </pre>
     */
    public Optional<T> kthSmallest(int k) {
        Node<T> result = kthSmallest(root, k);
        return result == null ? Optional.empty() : Optional.of(result.value);
    }

    /**
     * Return the rank of {@code value}: the number of elements strictly less
     * than it in the BST.
     *
     * <p>If {@code value} is not in the BST, this still returns the count of
     * elements that are strictly less than it (i.e., its insertion rank).
     */
    public int rank(T value) {
        if (value == null) return 0;
        return rank(root, value);
    }

    // =========================================================================
    // Traversal / export
    // =========================================================================

    /**
     * Return all elements in sorted (ascending) order via in-order traversal.
     *
     * @return a new {@link List} of all values, sorted
     */
    public List<T> toSortedList() {
        List<T> out = new ArrayList<>(size(root));
        inorder(root, out);
        return out;
    }

    // =========================================================================
    // Structural queries
    // =========================================================================

    /**
     * Validate the BST property and size invariant throughout the tree.
     *
     * @return {@code true} if every node satisfies:
     *   <ul>
     *     <li>all values in its left subtree are strictly less than its value</li>
     *     <li>all values in its right subtree are strictly greater</li>
     *     <li>{@code node.size == 1 + size(left) + size(right)}</li>
     *   </ul>
     */
    public boolean isValid() {
        // validate() returns -1 for an empty/null subtree (valid), a non-negative
        // height for a valid non-empty subtree, or -2 as the invalid sentinel.
        return validate(root, null, null) != -2;
    }

    /**
     * Return the height of the tree.
     *
     * <p>An empty tree has height {@code -1}; a single-node tree has height 0.
     */
    public int height() {
        return height(root);
    }

    /** Return the total number of elements in the tree. */
    public int size() {
        return size(root);
    }

    /** Return {@code true} if the tree contains no elements. */
    public boolean isEmpty() {
        return root == null;
    }

    // =========================================================================
    // Object overrides
    // =========================================================================

    @Override
    public String toString() {
        Object rootVal = root == null ? null : root.value;
        return "BinarySearchTree(root=" + rootVal + ", size=" + size() + ")";
    }

    // =========================================================================
    // Private recursive helpers
    // =========================================================================

    /**
     * Recursive insert. Returns the (possibly new) root of the subtree.
     *
     * <p>We create a new path from the root to the inserted leaf, updating
     * the {@code size} of every node along the path.
     */
    private Node<T> insert(Node<T> node, T value) {
        if (node == null) return new Node<>(value);

        int cmp = value.compareTo(node.value);
        if      (cmp < 0) node.left  = insert(node.left,  value);
        else if (cmp > 0) node.right = insert(node.right, value);
        // cmp == 0 → duplicate, no-op

        node.size = 1 + size(node.left) + size(node.right);
        return node;
    }

    /**
     * Recursive delete. Returns the (possibly new) root of the subtree.
     *
     * <p>For a node with two children, we replace the value with its
     * in-order successor (minimum of the right subtree) and delete the
     * successor from the right subtree. This keeps the BST property intact.
     */
    private Node<T> delete(Node<T> node, T value) {
        if (node == null) return null;

        int cmp = value.compareTo(node.value);
        if      (cmp < 0) node.left  = delete(node.left,  value);
        else if (cmp > 0) node.right = delete(node.right, value);
        else {
            // Found: splice out this node.
            if (node.left  == null) return node.right;
            if (node.right == null) return node.left;

            // Two children: replace value with successor and delete successor
            // from the right subtree.
            T successorVal = minNode(node.right).value;
            node.value = successorVal;
            node.right = delete(node.right, successorVal);
        }

        node.size = 1 + size(node.left) + size(node.right);
        return node;
    }

    /** Walk left until the leftmost node (minimum) is found. */
    private Node<T> minNode(Node<T> node) {
        while (node.left != null) node = node.left;
        return node;
    }

    /** Recursive k-th smallest helper using size augmentation. */
    private Node<T> kthSmallest(Node<T> node, int k) {
        if (node == null || k <= 0) return null;
        int leftSize = size(node.left);
        if      (k == leftSize + 1) return node;
        else if (k <= leftSize)     return kthSmallest(node.left, k);
        else                        return kthSmallest(node.right, k - leftSize - 1);
    }

    /**
     * Rank of {@code value} in the subtree rooted at {@code node}.
     *
     * <p>We accumulate the size of all left subtrees we pass through while
     * walking rightward, giving us the count of elements strictly less than
     * {@code value}.
     */
    private int rank(Node<T> node, T value) {
        if (node == null) return 0;
        int cmp = value.compareTo(node.value);
        if      (cmp < 0) return rank(node.left, value);
        else if (cmp > 0) return size(node.left) + 1 + rank(node.right, value);
        else              return size(node.left);
    }

    /**
     * In-order traversal (left → root → right) yields elements in ascending order.
     *
     * <p>This is the defining property of a BST: an in-order walk visits every
     * node exactly once in sorted order.
     */
    private void inorder(Node<T> node, List<T> out) {
        if (node == null) return;
        inorder(node.left, out);
        out.add(node.value);
        inorder(node.right, out);
    }

    /**
     * Validate BST property + size invariant. Returns subtree height if valid,
     * or -2 as a sentinel for invalid.
     *
     * <p>We pass down {@code min} and {@code max} bounds. Every node's value
     * must satisfy {@code min < value < max}. At the root, bounds are null.
     */
    private int validate(Node<T> node, T min, T max) {
        if (node == null) return -1;
        if (min != null && node.value.compareTo(min) <= 0) return -2;
        if (max != null && node.value.compareTo(max) >= 0) return -2;

        int leftH  = validate(node.left,  min,        node.value);
        int rightH = validate(node.right, node.value, max);
        if (leftH == -2 || rightH == -2) return -2;

        int expectedSize = 1 + size(node.left) + size(node.right);
        if (node.size != expectedSize) return -2;

        return 1 + Math.max(leftH, rightH);
    }

    /** Height of a subtree (-1 for null). */
    private int height(Node<T> node) {
        if (node == null) return -1;
        return 1 + Math.max(height(node.left), height(node.right));
    }

    /** Size of a subtree (0 for null), reading the cached field. */
    private static int size(Object node) {
        if (node == null) return 0;
        return ((Node<?>) node).size;
    }

    /** Build a balanced BST from a sorted subarray [lo, hi] inclusive. */
    private static <T extends Comparable<T>> Node<T> buildBalanced(List<T> values, int lo, int hi) {
        if (lo > hi) return null;
        int mid = lo + (hi - lo) / 2;
        Node<T> node = new Node<>(values.get(mid));
        node.left  = buildBalanced(values, lo, mid - 1);
        node.right = buildBalanced(values, mid + 1, hi);
        node.size  = 1 + size(node.left) + size(node.right);
        return node;
    }
}
