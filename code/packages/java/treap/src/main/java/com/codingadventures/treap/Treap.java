// ============================================================================
// Treap.java — Randomized Binary Search Tree with Heap Priorities (DT10)
// ============================================================================
//
// A treap is a BST where each node carries two values:
//   - key:      determines the BST ordering (left key < node key < right key)
//   - priority: a random number; determines the heap ordering (every parent
//               priority is GREATER than its children's priorities)
//
// The name is a portmanteau: TREe + heAP.
//
// ─────────────────────────────────────────────────────────────────────────────
// Why does this work?
// ─────────────────────────────────────────────────────────────────────────────
//
// UNIQUENESS THEOREM: for any set of distinct (key, priority) pairs, there is
// exactly one treap containing them.
//
//   Proof sketch:
//   - The node with the MAXIMUM priority must be the root (heap property says
//     no other node can have a greater ancestor).
//   - All keys < root.key form the left subtree; all keys > root.key form the
//     right subtree (BST property).
//   - Apply recursively — each subtree is also uniquely determined.
//
// CONSEQUENCE: if priorities are chosen uniformly at random, the resulting
// treap has the SAME expected shape as a RANDOM BST — which has expected
// height O(log n). Unlike AVL and Red-Black trees (which guarantee worst-case
// O(log n)), treaps are probabilistically balanced: the probability of exceeding
// height c·log n decays exponentially in c.
//
// ─────────────────────────────────────────────────────────────────────────────
// Split + Merge: the core operations
// ─────────────────────────────────────────────────────────────────────────────
//
// Instead of rotations, treap operations are built from two primitives:
//
//   split(node, key) → (left, right)
//     Divide: left has all keys ≤ key, right has all keys > key.
//     Time: O(height) = O(log n) expected.
//
//   merge(left, right) → node
//     Combine: every key in left must be < every key in right.
//     The heap property guides which side's root becomes the new root:
//     whichever has the higher priority stays on top.
//     Time: O(height_left + height_right) = O(log n) expected.
//
// All higher-level operations compose from these two:
//
//   insert(key, priority):
//     (l, r) = split(root, key)
//     root   = merge(merge(l, singleton(key, priority)), r)
//
//   delete(key):
//     (l, rest) = split_strict(root, key)   // l has keys < key
//     (_, r)    = split_strict(rest, key)   // discard mid (the key itself)
//     root      = merge(l, r)
//
// ─────────────────────────────────────────────────────────────────────────────
// Functional (Immutable) Design
// ─────────────────────────────────────────────────────────────────────────────
//
// Like the RBTree (DT09), this treap is purely functional. Insert and delete
// return NEW treap objects — the original is never mutated. Nodes are Java
// records: immutable by construction.
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.treap
// ============================================================================

package com.codingadventures.treap;

import java.util.ArrayList;
import java.util.List;
import java.util.NoSuchElementException;
import java.util.Optional;
import java.util.Random;

/**
 * A purely functional Treap (DT10) — a randomized BST with heap priorities.
 *
 * <p>Keys are integers. Priorities are doubles (randomly assigned unless
 * explicitly supplied). All mutating operations return NEW treap instances.
 *
 * <p>The implementation uses split+merge internally. Insert and delete are
 * both O(log n) expected time.
 */
public final class Treap {

    // ─────────────────────────────────────────────────────────────────────────
    // Node Record
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Each node is a triple (key, priority, left, right). Immutable.
    //
    // The priority determines WHERE in the tree the node sits vertically
    // (heap property: parents always have higher priority than children).
    // The key determines WHERE horizontally (BST property).

    public record Node(int key, double priority, Node left, Node right) {
        /** Return a copy of this node with new left child. */
        Node withLeft(Node l) {
            return new Node(key, priority, l, right);
        }
        /** Return a copy of this node with new right child. */
        Node withRight(Node r) {
            return new Node(key, priority, left, r);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SplitResult
    // ─────────────────────────────────────────────────────────────────────────
    //
    // split() returns two subtrees. Java records work perfectly here.

    public record SplitResult(Node left, Node right) {}

    // ─────────────────────────────────────────────────────────────────────────
    // Treap Fields
    // ─────────────────────────────────────────────────────────────────────────

    private final Node root;
    private final Random rng;   // used for randomly assigning priorities at insert

    private Treap(Node root, Random rng) {
        this.root = root;
        this.rng  = rng;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Factory Methods
    // ─────────────────────────────────────────────────────────────────────────

    /** Return an empty treap using the given Random for priority generation. */
    public static Treap empty(Random rng) {
        return new Treap(null, rng);
    }

    /** Return an empty treap with a default non-deterministic Random. */
    public static Treap empty() {
        return new Treap(null, new Random());
    }

    /** Return an empty treap seeded with a fixed value (for deterministic tests). */
    public static Treap withSeed(long seed) {
        return new Treap(null, new Random(seed));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Split
    // ─────────────────────────────────────────────────────────────────────────
    //
    // split(node, key) → (left, right)
    //
    //   left:  all keys ≤ key    (INCLUSIVE — left gets the key if present)
    //   right: all keys > key
    //
    // Intuition: walk the BST comparing node.key to the split key.
    //   - If node.key ≤ key: this node belongs to LEFT. Its right subtree might
    //     contain keys > key, so we recursively split the right subtree and
    //     graft the ≤-key part back as this node's right child.
    //   - If node.key > key: this node belongs to RIGHT. Its left subtree might
    //     contain keys ≤ key, so we recursively split the left subtree and graft
    //     the >-key part back as this node's left child.
    //
    // The BST ordering is preserved because we only move nodes within their
    // correct key-range. The heap ordering is preserved because we never
    // change priorities or parent-child relationships EXCEPT by "grafting"
    // subtrees — and in a correct treap, any subtree root already has a lower
    // priority than its ancestor, so grafting onto a new parent doesn't
    // violate the heap property.
    //
    // Time: O(height) = O(log n) expected.

    /**
     * Split the treap into two: left has all keys ≤ {@code key}, right has
     * all keys > {@code key}.
     */
    public SplitResult split(int key) {
        return splitNode(root, key);
    }

    private static SplitResult splitNode(Node node, int key) {
        if (node == null) return new SplitResult(null, null);

        if (node.key() <= key) {
            // This node belongs to LEFT.
            // Recursively split the right subtree.
            SplitResult rightSplit = splitNode(node.right(), key);
            // Graft the ≤-key portion back as this node's right child.
            Node newNode = node.withRight(rightSplit.left());
            return new SplitResult(newNode, rightSplit.right());
        } else {
            // This node belongs to RIGHT.
            // Recursively split the left subtree.
            SplitResult leftSplit = splitNode(node.left(), key);
            // Graft the >-key portion back as this node's left child.
            Node newNode = node.withLeft(leftSplit.right());
            return new SplitResult(leftSplit.left(), newNode);
        }
    }

    /**
     * Split the treap into two: left has all keys {@code < key} (strict),
     * right has all keys {@code >= key}.
     *
     * <p>Used internally by delete to isolate exactly one key.
     */
    private static SplitResult splitStrict(Node node, int key) {
        if (node == null) return new SplitResult(null, null);

        if (node.key() < key) {
            SplitResult rightSplit = splitStrict(node.right(), key);
            return new SplitResult(node.withRight(rightSplit.left()), rightSplit.right());
        } else {
            SplitResult leftSplit = splitStrict(node.left(), key);
            return new SplitResult(leftSplit.left(), node.withLeft(leftSplit.right()));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Merge
    // ─────────────────────────────────────────────────────────────────────────
    //
    // merge(left, right) → node
    //
    // Precondition: all keys in left < all keys in right.
    //
    // The heap property tells us which side's root must be on top:
    //   - If left.priority > right.priority: left's root stays on top.
    //     Its right subtree needs to merge with all of right.
    //   - Otherwise: right's root stays on top.
    //     Its left subtree needs to merge with all of left.
    //
    // This is elegant: merge is just "pick the winner by priority and
    // recursively merge the losers inner subtree with the remaining tree".
    //
    // Time: O(height_left + height_right) = O(log n) expected.

    /**
     * Merge two treaps into one. All keys in {@code left} must be less than
     * all keys in {@code right}.
     */
    public static Treap merge(Treap left, Treap right) {
        Node mergedRoot = mergeNodes(left.root, right.root);
        return new Treap(mergedRoot, left.rng);
    }

    private static Node mergeNodes(Node left, Node right) {
        if (left == null)  return right;
        if (right == null) return left;

        if (left.priority() > right.priority()) {
            // left's root stays on top; merge left.right with all of right
            return left.withRight(mergeNodes(left.right(), right));
        } else {
            // right's root stays on top; merge all of left with right.left
            return right.withLeft(mergeNodes(left, right.left()));
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insert
    // ─────────────────────────────────────────────────────────────────────────
    //
    // insert(key, priority):
    //   1. Split at key (left ≤ key, right > key).
    //   2. Create a singleton node for the new key.
    //   3. Merge: left ++ singleton ++ right.
    //
    // The singleton has the given (random) priority. If its priority is the
    // highest in the whole tree, it will bubble all the way to the root.
    // If it's the lowest, it sinks to a leaf. The exact position is determined
    // by the heap ordering during merge.
    //
    // Why this works: after split, all left keys < new key < all right keys
    // (BST property). After merging left with the singleton, the singleton is
    // in the correct BST position (it's the rightmost node of the merged-left
    // since it's larger than all left keys). After merging with right, the full
    // BST property is restored. The heap property is maintained by merge itself.

    /**
     * Return a new Treap with {@code key} inserted using a random priority.
     * If {@code key} already exists, returns the unchanged treap.
     */
    public Treap insert(int key) {
        if (contains(key)) return this;
        double priority = rng.nextDouble();
        return insertWithPriority(key, priority);
    }

    /**
     * Return a new Treap with {@code key} inserted at the given explicit
     * priority. Useful for deterministic testing.
     * If {@code key} already exists, returns the unchanged treap.
     */
    public Treap insertWithPriority(int key, double priority) {
        if (contains(key)) return this;
        // splitStrict(root, key) gives: left = keys < key, right = keys >= key.
        // Since we confirmed key is absent above, right = keys > key.
        SplitResult strict = splitStrict(root, key);
        // strict.left = keys < key, strict.right = keys >= key (all > key since key is absent)
        Node singleton = new Node(key, priority, null, null);
        Node merged = mergeNodes(mergeNodes(strict.left(), singleton), strict.right());
        return new Treap(merged, rng);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Delete
    // ─────────────────────────────────────────────────────────────────────────
    //
    // delete(key):
    //   1. splitStrict at key: left = keys < key, rest = keys >= key.
    //   2. splitStrict at key+1 (i.e., split rest into = key and > key):
    //      split rest at key INCLUSIVE: mid = keys == key (at most one node),
    //      right = keys > key.
    //   3. Discard mid, merge left and right.
    //
    // This is elegant: we slice out exactly the target key and merge the halves.
    // O(log n) expected (two splits + one merge).

    /**
     * Return a new Treap with {@code key} removed.
     * If {@code key} is not present, returns the unchanged treap.
     */
    public Treap delete(int key) {
        if (!contains(key)) return this;
        // Split into keys < key and keys >= key
        SplitResult leftPart = splitStrict(root, key);
        // Split the right part into keys == key (the target) and keys > key
        SplitResult rightPart = splitNode(leftPart.right(), key);
        // rightPart.left contains exactly the node with key (since we confirmed it exists)
        // rightPart.right contains all keys > key
        // Merge left and right, discarding the target
        Node merged = mergeNodes(leftPart.left(), rightPart.right());
        return new Treap(merged, rng);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Search / Contains
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Search ignores priorities — it's just a standard BST search.
    // The priority only determines the tree's shape (which affects performance
    // but not correctness).

    /** Return {@code true} if {@code key} is in the treap. */
    public boolean contains(int key) {
        Node node = root;
        while (node != null) {
            if      (key < node.key()) node = node.left();
            else if (key > node.key()) node = node.right();
            else                       return true;
        }
        return false;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Min / Max
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the minimum key, or empty if the treap is empty. */
    public Optional<Integer> min() {
        if (root == null) return Optional.empty();
        Node n = root;
        while (n.left() != null) n = n.left();
        return Optional.of(n.key());
    }

    /** Return the maximum key, or empty if the treap is empty. */
    public Optional<Integer> max() {
        if (root == null) return Optional.empty();
        Node n = root;
        while (n.right() != null) n = n.right();
        return Optional.of(n.key());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Predecessor / Successor
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the largest key strictly less than {@code key}, or empty. */
    public Optional<Integer> predecessor(int key) {
        Optional<Integer> best = Optional.empty();
        Node n = root;
        while (n != null) {
            if (key > n.key()) { best = Optional.of(n.key()); n = n.right(); }
            else                n = n.left();
        }
        return best;
    }

    /** Return the smallest key strictly greater than {@code key}, or empty. */
    public Optional<Integer> successor(int key) {
        Optional<Integer> best = Optional.empty();
        Node n = root;
        while (n != null) {
            if (key < n.key()) { best = Optional.of(n.key()); n = n.left(); }
            else                n = n.right();
        }
        return best;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // kthSmallest
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the k-th smallest key (1-indexed).
     *
     * @throws NoSuchElementException if k is out of range.
     */
    public int kthSmallest(int k) {
        List<Integer> sorted = toSortedList();
        if (k < 1 || k > sorted.size()) {
            throw new NoSuchElementException("k=" + k + " out of range; treap has " + sorted.size() + " elements");
        }
        return sorted.get(k - 1);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sorted Traversal
    // ─────────────────────────────────────────────────────────────────────────

    /** Return all keys in ascending order (in-order BST traversal). */
    public List<Integer> toSortedList() {
        List<Integer> result = new ArrayList<>();
        inOrder(root, result);
        return result;
    }

    private static void inOrder(Node node, List<Integer> acc) {
        if (node == null) return;
        inOrder(node.left(), acc);
        acc.add(node.key());
        inOrder(node.right(), acc);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Validation: isValidTreap
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Verify BOTH BST and heap properties hold throughout the tree:
    //
    //   BST property:  every key in left subtree < node.key < every key in right subtree
    //   Heap property: node.priority > left.priority AND node.priority > right.priority
    //
    // We track min_key and max_key bounds as we recurse (like BST validation),
    // and max_priority as the upper bound on children's priorities.

    /**
     * Return {@code true} if both BST and heap properties hold for the whole tree.
     */
    public boolean isValidTreap() {
        return checkNode(root, Integer.MIN_VALUE, Integer.MAX_VALUE, Double.MAX_VALUE);
    }

    private static boolean checkNode(Node node, int minKey, int maxKey, double maxPriority) {
        if (node == null) return true;
        // BST bounds check
        if (node.key() <= minKey || node.key() >= maxKey) return false;
        // Heap property: priority must be ≤ parent's priority
        if (node.priority() > maxPriority) return false;
        // Recurse
        return checkNode(node.left(),  minKey,    node.key(), node.priority())
            && checkNode(node.right(), node.key(), maxKey,    node.priority());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Size / Height / isEmpty
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the number of keys in the treap. */
    public int size() {
        return sizeHelper(root);
    }

    private static int sizeHelper(Node n) {
        if (n == null) return 0;
        return 1 + sizeHelper(n.left()) + sizeHelper(n.right());
    }

    /** Return the height of the treap (0 = empty, 1 = single root). */
    public int height() {
        return heightHelper(root);
    }

    private static int heightHelper(Node n) {
        if (n == null) return 0;
        return 1 + Math.max(heightHelper(n.left()), heightHelper(n.right()));
    }

    /** Return {@code true} if the treap contains no keys. */
    public boolean isEmpty() {
        return root == null;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Root Accessor (for testing)
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the root node (may be null for an empty treap). */
    public Node getRoot() {
        return root;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Static Factory: fromRoot
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Create a Treap from a raw root node and an explicit Random.
     * Used when constructing sub-treaps after split.
     */
    public static Treap fromRoot(Node root, Random rng) {
        return new Treap(root, rng);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Builder (for constructing treaps from existing nodes — e.g., after split)
    // ─────────────────────────────────────────────────────────────────────────

    /** Fluent builder for constructing a Treap from a pre-existing root node. */
    public static final class Builder {
        private Node node;
        private Random rng = new Random();

        public Builder fromNode(Node n) { this.node = n; return this; }
        public Builder withSeed(long seed) { this.rng = new Random(seed); return this; }
        public Builder withRng(Random r)   { this.rng = r; return this; }
        public Treap build() { return new Treap(node, rng); }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    @Override
    public String toString() {
        return "Treap{size=" + size() + ", height=" + height() + "}";
    }
}
