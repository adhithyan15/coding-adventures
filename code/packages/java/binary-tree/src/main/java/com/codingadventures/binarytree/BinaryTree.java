// ============================================================================
// BinaryTree.java — Generic Binary Tree with Traversal and Shape Queries
// ============================================================================
//
// A binary tree is a rooted tree where each node has at most two children,
// conventionally called "left" and "right". Unlike a BST, there is no ordering
// constraint — the tree is a structural container.
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
// [1, 2, 3, 4, 5, null, 6]
//
// Level-order maps indices to positions:
//   index i → left child at 2i+1, right child at 2i+2
//
//   i=0 (value=1): children at 1 and 2
//   i=1 (value=2): children at 3 and 4
//   i=2 (value=3): children at 5 and 6
//   i=3 (value=4): leaf
//   i=4 (value=5): leaf
//   i=5 (value=null): no node
//   i=6 (value=6): leaf
//
// This builds the tree shown above.
//
// == Traversals ==
//
//   Pre-order  (root → left → right):  1, 2, 4, 5, 3, 6
//   In-order   (left → root → right):  4, 2, 5, 1, 3, 6
//   Post-order (left → right → root):  4, 5, 2, 6, 3, 1
//   Level-order (BFS):                 1, 2, 3, 4, 5, 6
//
// == Shape predicates ==
//
//   Full tree:    every node has 0 or 2 children (no single-child nodes)
//   Complete tree: all levels filled except possibly the last, which is
//                  filled left-to-right
//   Perfect tree:  all leaves at the same depth; total nodes = 2^(h+1) - 1
//
// ============================================================================

package com.codingadventures.binarytree;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.LinkedList;
import java.util.List;

/**
 * A generic binary tree with traversal and structural predicate helpers.
 *
 * <p>Nodes are mutable: you can set {@link BinaryTreeNode#left} and
 * {@link BinaryTreeNode#right} directly, or build the tree from a level-order
 * list via {@link #fromLevelOrder}.
 *
 * <pre>{@code
 * BinaryTree<Integer> t = BinaryTree.fromLevelOrder(List.of(1, 2, 3, 4, 5, null, 6));
 *
 * t.levelOrder();   // [1, 2, 3, 4, 5, 6]
 * t.inorder();      // [4, 2, 5, 1, 3, 6]
 * t.height();       // 2
 * t.isFull();       // false (node 3 has only one child)
 * t.isComplete();   // false (null at index 5, non-null at index 6)
 * }</pre>
 *
 * @param <T> the element type
 */
public class BinaryTree<T> {

    // =========================================================================
    // Node
    // =========================================================================

    /**
     * A single node in the binary tree.
     *
     * <p>Fields are public for direct manipulation when building trees by hand.
     */
    public static final class BinaryTreeNode<T> {
        public T value;
        public BinaryTreeNode<T> left;
        public BinaryTreeNode<T> right;

        public BinaryTreeNode(T value) {
            this.value = value;
        }

        public BinaryTreeNode(T value, BinaryTreeNode<T> left, BinaryTreeNode<T> right) {
            this.value = value;
            this.left  = left;
            this.right = right;
        }

        @Override
        public String toString() {
            return "BinaryTreeNode(" + value + ")";
        }
    }

    // =========================================================================
    // Fields
    // =========================================================================

    /** The root node; null for an empty tree. */
    public BinaryTreeNode<T> root;

    // =========================================================================
    // Constructors / factory
    // =========================================================================

    /** Construct an empty tree. */
    public BinaryTree() {}

    /** Construct a tree with a given root node. */
    public BinaryTree(BinaryTreeNode<T> root) {
        this.root = root;
    }

    /** Construct a tree with a single root value. */
    public BinaryTree(T rootValue) {
        this.root = new BinaryTreeNode<>(rootValue);
    }

    /**
     * Build a binary tree from a level-order (BFS) list.
     *
     * <p>Null elements represent absent nodes. Index {@code i} maps to the node
     * whose left child is at index {@code 2i+1} and right child at {@code 2i+2}.
     *
     * <p>Example: {@code [1, 2, 3, null, 5]} builds:
     * <pre>
     *     1
     *    / \
     *   2   3
     *    \
     *     5
     * </pre>
     *
     * @param values level-order values (null means no node at that position)
     * @return a new BinaryTree
     */
    public static <T> BinaryTree<T> fromLevelOrder(List<T> values) {
        BinaryTree<T> tree = new BinaryTree<>();
        if (values == null || values.isEmpty()) return tree;
        tree.root = buildFromLevelOrder(values, 0);
        return tree;
    }

    // =========================================================================
    // Search
    // =========================================================================

    /**
     * Find the first node (in pre-order) whose value equals {@code value}.
     *
     * @return the matching node, or {@code null} if not found
     */
    public BinaryTreeNode<T> find(T value) {
        return find(root, value);
    }

    /** Return the left child of the first node with the given value, or null. */
    public BinaryTreeNode<T> leftChild(T value) {
        BinaryTreeNode<T> node = find(value);
        return node == null ? null : node.left;
    }

    /** Return the right child of the first node with the given value, or null. */
    public BinaryTreeNode<T> rightChild(T value) {
        BinaryTreeNode<T> node = find(value);
        return node == null ? null : node.right;
    }

    // =========================================================================
    // Shape predicates
    // =========================================================================

    /**
     * Return {@code true} if every node has exactly 0 or 2 children.
     *
     * <p>A single-child node makes the tree non-full.
     *
     * <pre>
     *   Full:        Not full:
     *       1             1
     *      / \           / \
     *     2   3         2   3
     *    / \               \
     *   4   5               4
     * </pre>
     */
    public boolean isFull() {
        return isFull(root);
    }

    /**
     * Return {@code true} if all levels except possibly the last are completely
     * filled, and the last level is filled left-to-right.
     *
     * <p>Algorithm: BFS. Once we see a {@code null} position, every subsequent
     * real node (non-null) indicates incompleteness.
     *
     * <pre>
     *   Complete:       Not complete:
     *       1                1
     *      / \              / \
     *     2   3            2   3
     *    / \  /           / \   \
     *   4  5 6           4   5   6   ← node at rightmost position of this level
     * </pre>
     */
    public boolean isComplete() {
        return isComplete(root);
    }

    /**
     * Return {@code true} if all leaves are at the same depth and the tree is
     * full (every internal node has exactly 2 children).
     *
     * <p>Equivalently, a perfect tree of height {@code h} has exactly
     * {@code 2^(h+1) - 1} nodes.
     *
     * <pre>
     *   Perfect (h=2):
     *         1
     *        / \
     *       2   3
     *      / \ / \
     *     4  5 6  7    ← 2^3 - 1 = 7 nodes
     * </pre>
     */
    public boolean isPerfect() {
        int h = height();
        if (h < 0) return size() == 0;
        return size() == (1 << (h + 1)) - 1;
    }

    // =========================================================================
    // Traversals
    // =========================================================================

    /**
     * In-order traversal: left → root → right.
     *
     * <p>For a BST this produces sorted output, but this class imposes no BST
     * invariant. Here it is provided as a general structural traversal.
     */
    public List<T> inorder() {
        List<T> out = new ArrayList<>();
        inorder(root, out);
        return out;
    }

    /**
     * Pre-order traversal: root → left → right.
     *
     * <p>Used to serialise a tree — the root appears first, so a pre-order
     * sequence can be fed back into {@link #fromLevelOrder} to reconstruct.
     */
    public List<T> preorder() {
        List<T> out = new ArrayList<>();
        preorder(root, out);
        return out;
    }

    /**
     * Post-order traversal: left → right → root.
     *
     * <p>Natural for computing sizes and heights — leaf values are processed
     * before their parents, so results can be aggregated bottom-up.
     */
    public List<T> postorder() {
        List<T> out = new ArrayList<>();
        postorder(root, out);
        return out;
    }

    /**
     * Level-order traversal (BFS): visits nodes layer by layer, left to right.
     *
     * <p>Uses a queue. Each node enqueues its children before being dequeued.
     * This is the traversal order used by {@link #fromLevelOrder}.
     */
    public List<T> levelOrder() {
        List<T> out = new ArrayList<>();
        if (root == null) return out;
        Deque<BinaryTreeNode<T>> queue = new ArrayDeque<>();
        queue.add(root);
        while (!queue.isEmpty()) {
            BinaryTreeNode<T> node = queue.poll();
            out.add(node.value);
            if (node.left  != null) queue.add(node.left);
            if (node.right != null) queue.add(node.right);
        }
        return out;
    }

    // =========================================================================
    // Structural queries
    // =========================================================================

    /**
     * Return the height of the tree.
     *
     * <p>An empty tree has height {@code -1}; a single-node tree has height 0.
     * Height = length of the longest path from root to a leaf.
     */
    public int height() {
        return height(root);
    }

    /** Return the total number of nodes. */
    public int size() {
        return size(root);
    }

    /** Return {@code true} if the tree is empty. */
    public boolean isEmpty() {
        return root == null;
    }

    // =========================================================================
    // Array projection
    // =========================================================================

    /**
     * Project the tree into a level-order array of size {@code 2^(h+1) - 1},
     * with {@code null} for absent nodes.
     *
     * <p>This is the inverse of {@link #fromLevelOrder}: the output array can
     * be passed back to reconstruct the same tree structure (with the same null
     * positions representing absent nodes).
     *
     * <p>Empty tree → empty list.
     */
    public List<T> toArray() {
        int h = height();
        if (h < 0) return new ArrayList<>();
        int capacity = (1 << (h + 1)) - 1;
        List<T> result = new ArrayList<>(capacity);
        for (int i = 0; i < capacity; i++) result.add(null);
        fillArray(root, 0, result);
        return result;
    }

    /**
     * Render the tree as a multi-line ASCII tree.
     *
     * <p>Example output for {@code [1, 2, 3, 4, 5, null, 6]}:
     * <pre>
     * `-- 1
     *     |-- 2
     *     |   |-- 4
     *     |   `-- 5
     *     `-- 3
     *         `-- 6
     * </pre>
     */
    public String toAscii() {
        if (root == null) return "";
        List<String> lines = new ArrayList<>();
        renderAscii(root, "", true, lines);
        return String.join("\n", lines);
    }

    // =========================================================================
    // Object overrides
    // =========================================================================

    @Override
    public String toString() {
        Object rootVal = root == null ? null : root.value;
        return "BinaryTree(root=" + rootVal + ", size=" + size() + ")";
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /** Recursive level-order construction using index arithmetic. */
    private static <T> BinaryTreeNode<T> buildFromLevelOrder(List<T> values, int index) {
        if (index >= values.size()) return null;
        T val = values.get(index);
        if (val == null) return null;
        BinaryTreeNode<T> node = new BinaryTreeNode<>(val);
        node.left  = buildFromLevelOrder(values, 2 * index + 1);
        node.right = buildFromLevelOrder(values, 2 * index + 2);
        return node;
    }

    /** Pre-order search (root first). */
    private BinaryTreeNode<T> find(BinaryTreeNode<T> node, T value) {
        if (node == null) return null;
        if (node.value == null ? value == null : node.value.equals(value)) return node;
        BinaryTreeNode<T> left = find(node.left, value);
        return left != null ? left : find(node.right, value);
    }

    /** Full-tree check: every node has 0 or 2 children. */
    private boolean isFull(BinaryTreeNode<T> node) {
        if (node == null) return true;
        if (node.left == null && node.right == null) return true;
        if (node.left == null || node.right == null) return false;
        return isFull(node.left) && isFull(node.right);
    }

    /**
     * Complete-tree check via BFS: once we see a null child slot, every
     * subsequent non-null node makes the tree incomplete.
     *
     * <p>We use {@link LinkedList} (not {@link ArrayDeque}) because we
     * intentionally enqueue {@code null} entries as sentinels for absent
     * children — {@code ArrayDeque} does not permit null elements.
     */
    private boolean isComplete(BinaryTreeNode<T> node) {
        if (node == null) return true;   // empty tree is trivially complete
        // LinkedList permits null elements, which we use to mark absent children.
        Deque<BinaryTreeNode<T>> queue = new LinkedList<>();
        queue.add(node);
        boolean seenNull = false;
        while (!queue.isEmpty()) {
            BinaryTreeNode<T> current = queue.poll();
            if (current == null) {
                seenNull = true;
            } else {
                if (seenNull) return false;
                queue.add(current.left);    // may be null — that is intentional
                queue.add(current.right);
            }
        }
        return true;
    }

    private void inorder(BinaryTreeNode<T> node, List<T> out) {
        if (node == null) return;
        inorder(node.left, out);
        out.add(node.value);
        inorder(node.right, out);
    }

    private void preorder(BinaryTreeNode<T> node, List<T> out) {
        if (node == null) return;
        out.add(node.value);
        preorder(node.left, out);
        preorder(node.right, out);
    }

    private void postorder(BinaryTreeNode<T> node, List<T> out) {
        if (node == null) return;
        postorder(node.left, out);
        postorder(node.right, out);
        out.add(node.value);
    }

    private int height(BinaryTreeNode<T> node) {
        if (node == null) return -1;
        return 1 + Math.max(height(node.left), height(node.right));
    }

    private int size(BinaryTreeNode<T> node) {
        if (node == null) return 0;
        return 1 + size(node.left) + size(node.right);
    }

    /** Fill the array at the level-order index using the index arithmetic. */
    private void fillArray(BinaryTreeNode<T> node, int index, List<T> out) {
        if (node == null || index >= out.size()) return;
        out.set(index, node.value);
        fillArray(node.left,  2 * index + 1, out);
        fillArray(node.right, 2 * index + 2, out);
    }

    /** ASCII tree renderer — produces a sideways tree with box-drawing chars. */
    private void renderAscii(BinaryTreeNode<T> node, String prefix, boolean isTail, List<String> lines) {
        String connector = isTail ? "`-- " : "|-- ";
        lines.add(prefix + connector + node.value);

        List<BinaryTreeNode<T>> children = new ArrayList<>(2);
        if (node.left  != null) children.add(node.left);
        if (node.right != null) children.add(node.right);

        String nextPrefix = prefix + (isTail ? "    " : "|   ");
        for (int i = 0; i < children.size(); i++) {
            renderAscii(children.get(i), nextPrefix, i + 1 == children.size(), lines);
        }
    }
}
