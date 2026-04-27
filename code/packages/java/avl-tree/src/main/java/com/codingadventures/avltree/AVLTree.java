// ============================================================================
// AVLTree.java — Self-Balancing Binary Search Tree (AVL)
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

package com.codingadventures.avltree;

import java.util.ArrayList;
import java.util.List;
import java.util.NoSuchElementException;

/**
 * A self-balancing binary search tree using the AVL invariant.
 *
 * <p>All operations run in O(log n) time in the worst case. Each node is
 * augmented with its subtree height and size, enabling O(log n) order
 * statistics (rank and kth-smallest).
 *
 * <pre>{@code
 * AVLTree<Integer> tree = new AVLTree<>();
 * tree.insert(10);
 * tree.insert(5);
 * tree.insert(20);
 *
 * tree.contains(5);            // true
 * tree.min();                  // 5
 * tree.max();                  // 20
 * tree.kthSmallest(2);         // 10
 * tree.rank(10);               // 1  (0-based: one element is smaller)
 * tree.predecessor(10);        // 5
 * tree.successor(10);          // 20
 *
 * tree.delete(10);
 * tree.size();                 // 2
 * tree.isValid();              // true
 * }</pre>
 *
 * @param <T> the element type; must be {@link Comparable}
 */
public class AVLTree<T extends Comparable<T>> {

    // =========================================================================
    // Inner class: Node
    // =========================================================================

    /**
     * A single node in the AVL tree.
     *
     * <p>Invariants (maintained after every structural change):
     * <ul>
     *   <li>{@code height == 1 + max(height(left), height(right))} (0 for leaves)</li>
     *   <li>{@code size   == 1 + size(left)   + size(right)}   (1 for leaves)</li>
     *   <li>{@code |height(left) - height(right)| <= 1}        (AVL property)</li>
     *   <li>BST ordering: all values in left subtree < value < all in right</li>
     * </ul>
     */
    static final class Node<T> {
        T value;
        Node<T> left, right;
        int height;   // 0 for a leaf
        int size;     // 1 for a leaf

        Node(T value) {
            this.value  = value;
            this.height = 0;
            this.size   = 1;
        }
    }

    // =========================================================================
    // Fields
    // =========================================================================

    private Node<T> root;

    // =========================================================================
    // Constructor
    // =========================================================================

    /** Construct an empty AVL tree. */
    public AVLTree() {
        this.root = null;
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Insert {@code value} into the tree.
     *
     * <p>If the value is already present, the tree is unchanged.
     *
     * @param value the value to insert; must not be null
     */
    public void insert(T value) {
        if (value == null) throw new IllegalArgumentException("Value must not be null");
        root = insert(root, value);
    }

    /**
     * Remove {@code value} from the tree.
     *
     * @param value the value to remove
     * @throws NoSuchElementException if the value is not present
     */
    public void delete(T value) {
        if (!contains(value)) throw new NoSuchElementException("Value not found: " + value);
        root = delete(root, value);
    }

    /**
     * Return {@code true} if {@code value} is present in the tree.
     *
     * @param value the value to search for
     */
    public boolean contains(T value) {
        if (value == null) return false;
        Node<T> node = root;
        while (node != null) {
            int cmp = value.compareTo(node.value);
            if      (cmp < 0) node = node.left;
            else if (cmp > 0) node = node.right;
            else              return true;
        }
        return false;
    }

    /**
     * Return the smallest value in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    public T min() {
        if (root == null) throw new NoSuchElementException("Tree is empty");
        Node<T> node = root;
        while (node.left != null) node = node.left;
        return node.value;
    }

    /**
     * Return the largest value in the tree.
     *
     * @throws NoSuchElementException if the tree is empty
     */
    public T max() {
        if (root == null) throw new NoSuchElementException("Tree is empty");
        Node<T> node = root;
        while (node.right != null) node = node.right;
        return node.value;
    }

    /**
     * Return the largest value strictly less than {@code value}, or {@code null}
     * if none exists.
     *
     * @param value the reference value
     */
    public T predecessor(T value) {
        if (value == null) return null;
        T best = null;
        Node<T> node = root;
        while (node != null) {
            int cmp = value.compareTo(node.value);
            if (cmp <= 0) {
                node = node.left;
            } else {
                best = node.value;
                node = node.right;
            }
        }
        return best;
    }

    /**
     * Return the smallest value strictly greater than {@code value}, or
     * {@code null} if none exists.
     *
     * @param value the reference value
     */
    public T successor(T value) {
        if (value == null) return null;
        T best = null;
        Node<T> node = root;
        while (node != null) {
            int cmp = value.compareTo(node.value);
            if (cmp >= 0) {
                node = node.right;
            } else {
                best = node.value;
                node = node.left;
            }
        }
        return best;
    }

    /**
     * Return the k-th smallest value (1-based).
     *
     * <p>{@code kthSmallest(1)} returns the minimum; {@code kthSmallest(size())}
     * returns the maximum.
     *
     * @param k the rank (1-based)
     * @return the k-th smallest value, or {@code null} if {@code k} is out of range
     */
    public T kthSmallest(int k) {
        if (k <= 0 || k > nodeSize(root)) return null;
        return kthSmallest(root, k);
    }

    /**
     * Return the 0-based rank of {@code value} in the tree — the number of
     * elements strictly less than {@code value}.
     *
     * <p>If {@code value} is not in the tree, this is the position it would
     * occupy if inserted.
     *
     * @param value the value whose rank is requested
     */
    public int rank(T value) {
        if (value == null) return 0;
        return rank(root, value);
    }

    /**
     * Return all values in ascending (in-order) order.
     */
    public List<T> toSortedList() {
        List<T> out = new ArrayList<>(nodeSize(root));
        inorder(root, out);
        return out;
    }

    /** Return the height of the tree (0 for a single-node tree, -1 for empty). */
    public int height() {
        return nodeHeight(root);
    }

    /** Return the number of values in the tree. */
    public int size() {
        return nodeSize(root);
    }

    /** Return {@code true} if the tree contains no values. */
    public boolean isEmpty() {
        return root == null;
    }

    /**
     * Return the balance factor of the root: height(left) - height(right).
     *
     * <p>A valid AVL tree has every node with balance factor in {-1, 0, +1}.
     */
    public int balanceFactor() {
        return balanceFactor(root);
    }

    /**
     * Validate all AVL tree invariants:
     * <ol>
     *   <li>BST ordering (left < node < right at every node)</li>
     *   <li>AVL property (|BF| ≤ 1 at every node)</li>
     *   <li>Correct height values in every node</li>
     *   <li>Correct size values in every node</li>
     * </ol>
     *
     * @return {@code true} if the tree is a valid AVL tree
     */
    public boolean isValid() {
        return validateAVL(root, null, null) != null;
    }

    /**
     * Return {@code true} if the BST ordering invariant holds (but does NOT
     * check the AVL balance invariant).
     */
    public boolean isValidBST() {
        return validateBST(root, null, null);
    }

    @Override
    public String toString() {
        return "AVLTree(size=" + size() + ", height=" + height() + ")";
    }

    // =========================================================================
    // Private helpers — insertion
    // =========================================================================

    /**
     * Recursively insert {@code value} into the subtree rooted at {@code node},
     * rebalancing on the way back up.
     *
     * @return the new root of the (possibly rebalanced) subtree
     */
    private Node<T> insert(Node<T> node, T value) {
        if (node == null) return new Node<>(value);
        int cmp = value.compareTo(node.value);
        if      (cmp < 0) node.left  = insert(node.left,  value);
        else if (cmp > 0) node.right = insert(node.right, value);
        // cmp == 0: value already present — no change
        update(node);
        return rebalance(node);
    }

    // =========================================================================
    // Private helpers — deletion
    // =========================================================================

    /**
     * Recursively delete {@code value} from the subtree rooted at {@code node},
     * rebalancing on the way back up.
     *
     * <p>For a two-child node, we replace the value with the in-order successor
     * (minimum of the right subtree) and delete that successor.
     */
    private Node<T> delete(Node<T> node, T value) {
        if (node == null) return null;
        int cmp = value.compareTo(node.value);
        if (cmp < 0) {
            node.left  = delete(node.left,  value);
        } else if (cmp > 0) {
            node.right = delete(node.right, value);
        } else {
            // Found — three sub-cases
            if (node.left  == null) return node.right;
            if (node.right == null) return node.left;
            // Two children: replace with in-order successor
            Node<T> successor = node.right;
            while (successor.left != null) successor = successor.left;
            node.value = successor.value;
            node.right = delete(node.right, successor.value);
        }
        update(node);
        return rebalance(node);
    }

    // =========================================================================
    // Private helpers — rotations and rebalancing
    // =========================================================================

    /**
     * Rotate right at {@code y}.
     *
     * <pre>
     *       y             x
     *      / \           / \
     *     x   C    →    A   y
     *    / \               / \
     *   A   B             B   C
     * </pre>
     */
    private Node<T> rotateRight(Node<T> y) {
        Node<T> x = y.left;
        Node<T> B = x.right;
        x.right = y;
        y.left  = B;
        update(y);
        update(x);
        return x;
    }

    /**
     * Rotate left at {@code x}.
     *
     * <pre>
     *     x               y
     *    / \             / \
     *   A   y     →    x   C
     *      / \        / \
     *     B   C      A   B
     * </pre>
     */
    private Node<T> rotateLeft(Node<T> x) {
        Node<T> y = x.right;
        Node<T> B = y.left;
        y.left  = x;
        x.right = B;
        update(x);
        update(y);
        return y;
    }

    /**
     * Rebalance {@code node} if its balance factor has reached ±2.
     *
     * <p>Four cases:
     * <ul>
     *   <li>BF ≥ +2, left child is left-heavy or balanced  → right rotation</li>
     *   <li>BF ≥ +2, left child is right-heavy              → left-right rotation</li>
     *   <li>BF ≤ -2, right child is right-heavy or balanced → left rotation</li>
     *   <li>BF ≤ -2, right child is left-heavy              → right-left rotation</li>
     * </ul>
     */
    private Node<T> rebalance(Node<T> node) {
        int bf = balanceFactor(node);
        if (bf > 1) {
            // Left-heavy
            if (balanceFactor(node.left) < 0) {
                // Left-Right case: rotate left at left child first
                node.left = rotateLeft(node.left);
            }
            return rotateRight(node);
        }
        if (bf < -1) {
            // Right-heavy
            if (balanceFactor(node.right) > 0) {
                // Right-Left case: rotate right at right child first
                node.right = rotateRight(node.right);
            }
            return rotateLeft(node);
        }
        return node;
    }

    /** Update height and size fields of {@code node} from its children. */
    private void update(Node<T> node) {
        node.height = 1 + Math.max(nodeHeight(node.left), nodeHeight(node.right));
        node.size   = 1 + nodeSize(node.left) + nodeSize(node.right);
    }

    // =========================================================================
    // Private helpers — order statistics
    // =========================================================================

    private T kthSmallest(Node<T> node, int k) {
        int leftSize = nodeSize(node.left);
        if      (k == leftSize + 1) return node.value;
        else if (k <= leftSize)     return kthSmallest(node.left,  k);
        else                        return kthSmallest(node.right, k - leftSize - 1);
    }

    private int rank(Node<T> node, T value) {
        if (node == null) return 0;
        int cmp = value.compareTo(node.value);
        if      (cmp < 0) return rank(node.left, value);
        else if (cmp > 0) return nodeSize(node.left) + 1 + rank(node.right, value);
        else              return nodeSize(node.left);
    }

    private void inorder(Node<T> node, List<T> out) {
        if (node == null) return;
        inorder(node.left,  out);
        out.add(node.value);
        inorder(node.right, out);
    }

    // =========================================================================
    // Private helpers — utility
    // =========================================================================

    private int nodeHeight(Node<T> node) { return node == null ? -1 : node.height; }
    private int nodeSize(Node<T> node)   { return node == null ?  0 : node.size;   }
    private int balanceFactor(Node<T> node) {
        return node == null ? 0 : nodeHeight(node.left) - nodeHeight(node.right);
    }

    // =========================================================================
    // Private helpers — validation
    // =========================================================================

    /**
     * Recursively validate AVL invariants.
     *
     * @return a two-element array [height, size] if valid, or {@code null} if invalid
     */
    private int[] validateAVL(Node<T> node, T min, T max) {
        if (node == null) return new int[]{-1, 0};

        // BST ordering
        if (min != null && node.value.compareTo(min) <= 0) return null;
        if (max != null && node.value.compareTo(max) >= 0) return null;

        int[] left  = validateAVL(node.left,  min,        node.value);
        int[] right = validateAVL(node.right, node.value, max);
        if (left == null || right == null) return null;

        int expectedHeight = 1 + Math.max(left[0], right[0]);
        int expectedSize   = 1 + left[1] + right[1];
        int bf             = left[0] - right[0];

        if (node.height != expectedHeight) return null;
        if (node.size   != expectedSize)   return null;
        if (Math.abs(bf) > 1)              return null;  // AVL invariant

        return new int[]{expectedHeight, expectedSize};
    }

    private boolean validateBST(Node<T> node, T min, T max) {
        if (node == null) return true;
        if (min != null && node.value.compareTo(min) <= 0) return false;
        if (max != null && node.value.compareTo(max) >= 0) return false;
        return validateBST(node.left, min, node.value) &&
               validateBST(node.right, node.value, max);
    }
}
