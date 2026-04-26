// ============================================================================
// RBTree.java — Red-Black Tree (Self-Balancing BST with Color Invariants)
// ============================================================================
//
// A Red-Black tree is a binary search tree where every node carries a color
// bit (RED or BLACK) and five invariants on those colors guarantee that the
// tree height is at most 2 × log₂(n + 1) — ensuring O(log n) for all
// operations in the worst case.
//
// ─────────────────────────────────────────────────────────────────────────────
// The Five Red-Black Invariants
// ─────────────────────────────────────────────────────────────────────────────
//
//   1. COLORING:     Every node is either RED or BLACK.
//   2. ROOT:         The root is BLACK.
//   3. NULL LEAVES:  Every null pointer is treated as a BLACK NIL leaf.
//   4. RED RULE:     Red nodes may only have BLACK children.
//                    (No two consecutive red nodes on any root-to-leaf path.)
//   5. BLACK HEIGHT: Every path from a given node down to any NIL leaf
//                    passes through the same number of BLACK nodes.
//
// Rules 4 and 5 together guarantee height ≤ 2 × bh ≤ 2 × log₂(n + 1), where
// bh is the black-height of the root.
//
// ─────────────────────────────────────────────────────────────────────────────
// Design: Purely Functional (Immutable)
// ─────────────────────────────────────────────────────────────────────────────
//
// This implementation is **purely functional** — insert and delete return NEW
// tree objects and never mutate the existing structure. Node references can be
// safely shared across versions of the tree.
//
// Advantages of the functional approach:
//   - Thread-safe by construction (no locks needed for reads)
//   - Persistent data structure — old versions remain accessible
//   - Easier to reason about correctness
//
// The functional insertion algorithm comes from Chris Okasaki's classic paper
// "Red-Black Trees in a Functional Setting" (1999), which reduces the 5-case
// imperative algorithm to a single elegant balance function covering 4 cases.
//
// ─────────────────────────────────────────────────────────────────────────────
// Okasaki's Balance Function
// ─────────────────────────────────────────────────────────────────────────────
//
// After inserting a new RED node, any of four "red-red violation" patterns
// can appear in the grandparent's neighbourhood:
//
//   Pattern 1: left-left red          Pattern 2: left-right red
//       z(B)                              z(B)
//      /    \                            /    \
//    y(R)   d                          x(R)   d
//   /    \                            /    \
//  x(R)   c                          a    y(R)
//  / \                                    / \
// a   b                                  b   c
//
//   Pattern 3: right-left red         Pattern 4: right-right red
//       x(B)                              x(B)
//      /    \                            /    \
//     a    z(R)                         a    y(R)
//          /    \                            /    \
//        y(R)    d                          b    z(R)
//        / \                                     / \
//       b   c                                   c   d
//
// All four patterns produce the SAME balanced result:
//
//          y(R)
//         /    \
//       x(B)   z(B)
//       / \    / \
//      a   b  c   d
//
// This single transform handles all four violation cases.
//
// ─────────────────────────────────────────────────────────────────────────────
// Deletion
// ─────────────────────────────────────────────────────────────────────────────
//
// Deletion is more complex. Removing a BLACK node creates a "black deficit"
// on that path, violating Rule 5. We propagate a "double-black" marker up
// the tree, resolving it via 6 cases based on the sibling's color and its
// children's colors.
//
// We use Sedgewick's Left-Leaning Red-Black (LLRB) tree deletion approach for
// simplicity: additional invariant is maintained that red links only lean left,
// which reduces the deletion cases significantly.
//
// ─────────────────────────────────────────────────────────────────────────────
// Package: com.codingadventures.rbt
// ============================================================================

package com.codingadventures.rbt;

import java.util.ArrayList;
import java.util.List;
import java.util.NoSuchElementException;
import java.util.Optional;

/**
 * A purely functional Red-Black Tree (DT09).
 *
 * <p>Stores comparable integers. All mutating operations return a NEW tree
 * with the invariants restored — the original is unchanged.
 *
 * <p>Based on Okasaki's functional balance algorithm for insertion
 * (1999, "Red-Black Trees in a Functional Setting"), with a
 * Left-Leaning approach inspired by Sedgewick for deletion.
 */
public final class RBTree {

    // ─────────────────────────────────────────────────────────────────────────
    // Color Enum
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Each node carries exactly one bit of balance information: its color.
    // This is far more memory-efficient than storing a height (4 bytes) as in
    // AVL trees — though in practice Java's object overhead dwarfs this.

    public enum Color { RED, BLACK }

    // ─────────────────────────────────────────────────────────────────────────
    // Node Record
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Immutable node. All fields are final. Children may be null (= NIL leaf).

    public record Node(int value, Color color, Node left, Node right) {

        /** Convenience: create a RED node with two null children. */
        static Node red(int v) {
            return new Node(v, Color.RED, null, null);
        }

        /** Convenience: create a BLACK node with two null children. */
        static Node black(int v) {
            return new Node(v, Color.BLACK, null, null);
        }

        /** Return a copy of this node with a new color. */
        Node withColor(Color c) {
            if (c == color) return this;
            return new Node(value, c, left, right);
        }

        /** Return a copy of this node with a new left child. */
        Node withLeft(Node l) {
            return new Node(value, color, l, right);
        }

        /** Return a copy of this node with a new right child. */
        Node withRight(Node r) {
            return new Node(value, color, left, r);
        }

        /** True if this node is RED (null nodes are considered BLACK). */
        boolean isRed() {
            return color == Color.RED;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // RBTree Fields
    // ─────────────────────────────────────────────────────────────────────────

    /** The root node. null represents an empty tree. */
    private final Node root;

    private RBTree(Node root) {
        this.root = root;
    }

    /** Return an empty Red-Black tree. */
    public static RBTree empty() {
        return new RBTree(null);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Left-Leaning Red-Black (LLRB) Core Helpers
    // ─────────────────────────────────────────────────────────────────────────
    //
    // We use Sedgewick's Left-Leaning Red-Black tree algorithm for BOTH
    // insertion and deletion. This maintains an extra invariant beyond the
    // five classic RB invariants:
    //
    //   LLRB invariant: red links only lean LEFT
    //                   (i.e., no node has a RED right child)
    //
    // This restriction maps the tree structure onto 2-3 trees (each black node
    // with a left red child represents a "3-node"), which makes the deletion
    // algorithm significantly simpler than the classic 6-case approach.
    //
    // All three helpers (rotateLeft, rotateRight, fixUp) are reused by both
    // the insert path (to maintain the LLRB invariant bottom-up) and the
    // delete path (to repair invariants after structural changes).

    /** Null-safe color check. null nodes are BLACK by convention (Rule 3). */
    private static boolean isRed(Node n) {
        return n != null && n.color() == Color.RED;
    }

    /** Toggle a color. */
    private static Color toggle(Color c) {
        return c == Color.RED ? Color.BLACK : Color.RED;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Insert — LLRB (Sedgewick-style, using fixUp bottom-up)
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Strategy:
    //   1. Descend as in a regular BST, inserting a new RED node at the leaf.
    //   2. On the way back up, call fixUp() to maintain the LLRB invariant.
    //   3. Force the root to BLACK (Rule 2).
    //
    // Using fixUp (not Okasaki's balance) ensures the tree is always
    // left-leaning after every insert, which is required for LLRB deletion
    // to work correctly.
    //
    // Time: O(log n) — tree height is bounded by 2 log₂ n.
    // Space: O(log n) new nodes created on the path from root to leaf.

    /**
     * Return a new RBTree with {@code value} inserted.
     * If {@code value} is already present, returns the unchanged tree (no duplicates).
     */
    public RBTree insert(int value) {
        Node newRoot = insertHelper(root, value);
        // Rule 2: root must be BLACK. Force it regardless of what fixUp returned.
        return new RBTree(newRoot.withColor(Color.BLACK));
    }

    private static Node insertHelper(Node h, int value) {
        if (h == null) {
            // Base case: new nodes are always RED.
            return Node.red(value);
        }

        int cmp = Integer.compare(value, h.value());
        if (cmp < 0) {
            h = h.withLeft(insertHelper(h.left(), value));
        } else if (cmp > 0) {
            h = h.withRight(insertHelper(h.right(), value));
        }
        // else: duplicate — no structural change needed

        // Restore LLRB invariant bottom-up on the way back up the stack.
        return fixUp(h);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Delete — Left-Leaning Red-Black Tree Approach
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Full Red-Black deletion has 6 cases for resolving "double-black" nodes.
    // We use a simplified variant: Left-Leaning Red-Black trees (LLRB), which
    // adds the invariant that 3-nodes are represented as left-leaning red links.
    //
    // LLRB deletion works by:
    //   1. "Lending" a red link down to the node to delete (making it red or
    //      giving it a red sibling), so that when we delete it, we only ever
    //      delete a red node — which doesn't affect black-height.
    //   2. Fixing any invariant violations on the way back up.
    //
    // Additional LLRB helpers needed:
    //   - rotateLeft(h):   make h.right the new root, h goes left
    //   - rotateRight(h):  make h.left the new root, h goes right
    //   - flipColors(h):   flip h's color and both children's colors
    //   - fixUp(h):        restore LLRB invariant after structural changes
    //   - moveRedLeft(h):  borrow from right sibling to make h.left (or its child) red
    //   - moveRedRight(h): borrow from left sibling to make h.right (or its child) red
    //
    // This gives us a clean and correct delete without tracking double-black.
    //
    // Reference: "Left-Leaning Red-Black Trees", Robert Sedgewick (2008).

    /**
     * Return a new RBTree with {@code value} removed.
     * If {@code value} is not present, returns the unchanged tree.
     */
    public RBTree delete(int value) {
        if (!contains(value)) return this;
        Node newRoot = deleteHelper(root, value);
        if (newRoot == null) return new RBTree(null);
        return new RBTree(newRoot.withColor(Color.BLACK));
    }

    // ─── LLRB Structural Helpers ───────────────────────────────────────────

    /**
     * Rotate left: the right child becomes the new root of this subtree.
     *
     * <pre>
     *   h                x
     *  / \              / \
     * a   x    →       h   c
     *    / \          / \
     *   b   c        a   b
     * </pre>
     *
     * x inherits h's color; h becomes RED (it is now the left child of x,
     * a "temporary" 3-node lean).
     */
    private static Node rotateLeft(Node h) {
        Node x = h.right();
        return new Node(x.value(), h.color(),
                new Node(h.value(), Color.RED, h.left(), x.left()),
                x.right());
    }

    /**
     * Rotate right: the left child becomes the new root.
     *
     * <pre>
     *     h              x
     *    / \            / \
     *   x   c    →     a   h
     *  / \                / \
     * a   b              b   c
     * </pre>
     *
     * x inherits h's color; h becomes RED.
     */
    private static Node rotateRight(Node h) {
        Node x = h.left();
        return new Node(x.value(), h.color(),
                x.left(),
                new Node(h.value(), Color.RED, x.right(), h.right()));
    }

    /**
     * Flip the colors of a node and both its children.
     *
     * <p>Both children MUST be non-null when this is called.
     *
     * <p>Flipping is used in two contexts:
     * <ul>
     *   <li>Splitting a "4-node" on the way UP (via fixUp): h is BLACK with two RED
     *       children → h becomes RED, children become BLACK. This propagates the
     *       virtual "4-node split" upward.
     *   <li>Borrowing for deletion (via moveRedLeft/moveRedRight): h is BLACK or RED,
     *       and we temporarily make both children RED to allow borrowing.
     * </ul>
     */
    private static Node flipColors(Node h) {
        // Simply toggle all three colors — no conditional logic needed.
        Node newLeft  = h.left()  != null ? h.left().withColor(toggle(h.left().color()))   : null;
        Node newRight = h.right() != null ? h.right().withColor(toggle(h.right().color())) : null;
        return new Node(h.value(), toggle(h.color()), newLeft, newRight);
    }

    /**
     * Restore LLRB invariants on the way up after a structural change.
     *
     * <p>LLRB invariant: red links only lean LEFT. fixUp enforces this:
     * <ol>
     *   <li>If right child is RED and left child is NOT red → rotateLeft (fix right lean).
     *   <li>If left child is RED and left-left grandchild is RED → rotateRight (fix 4-node).
     *   <li>If both children are RED → flipColors (split 4-node).
     * </ol>
     */
    private static Node fixUp(Node h) {
        // Step 1: eliminate right-leaning red links
        if (isRed(h.right()) && !isRed(h.left())) {
            h = rotateLeft(h);
        }
        // Step 2: eliminate consecutive left-leaning red links (4-node)
        if (isRed(h.left()) && isRed(h.left().left())) {
            h = rotateRight(h);
        }
        // Step 3: split 4-nodes
        if (isRed(h.left()) && isRed(h.right())) {
            h = flipColors(h);
        }
        return h;
    }

    /**
     * Make {@code h.left} or {@code h.left.left} RED by either borrowing from
     * the right sibling or by merging.
     *
     * <p>Precondition: {@code h} is RED, {@code h.left} and {@code h.left.left}
     * are both BLACK (or null).
     *
     * <p>Step 1: flipColors(h) — makes h BLACK, both children RED. This
     * "temporarily merges" h and its children into a 4-node, giving h.left
     * a virtual red sibling (h.right is now RED).
     *
     * <p>Step 2: if h.right's left child is RED, we can borrow it by
     * rotating right at h.right, then rotating left at h, then splitting
     * the resulting 4-node by flipping again.
     */
    private static Node moveRedLeft(Node h) {
        h = flipColors(h);
        // After flipColors, h.right is RED. Check if h.right has a left-leaning red.
        if (h.right() != null && isRed(h.right().left())) {
            h = h.withRight(rotateRight(h.right()));
            h = rotateLeft(h);
            h = flipColors(h);
        }
        return h;
    }

    /**
     * Make {@code h.right} or {@code h.right.left} RED by either borrowing from
     * the left sibling or by merging.
     *
     * <p>Precondition: {@code h} is RED, {@code h.right} and {@code h.right.left}
     * are both BLACK.
     */
    private static Node moveRedRight(Node h) {
        h = flipColors(h);
        // After flipColors, h.left is RED. If h.left.left is also RED,
        // we have a left-leaning 4-node we can rotate to balance.
        if (h.left() != null && isRed(h.left().left())) {
            h = rotateRight(h);
            h = flipColors(h);
        }
        return h;
    }

    /** Delete the minimum node in the subtree rooted at {@code h}. */
    private static Node deleteMin(Node h) {
        if (h.left() == null) return null; // h is the minimum; remove it
        // If h.left is not red and h.left.left is not red, borrow from sibling
        if (!isRed(h.left()) && !isRed(h.left().left())) {
            h = moveRedLeft(h);
        }
        Node newLeft = deleteMin(h.left());
        return fixUp(new Node(h.value(), h.color(), newLeft, h.right()));
    }

    /** Return the minimum value in the subtree rooted at {@code h}. */
    private static int minValue(Node h) {
        while (h.left() != null) h = h.left();
        return h.value();
    }

    /** Core recursive delete. Returns the new subtree root (may be null). */
    private static Node deleteHelper(Node h, int value) {
        if (Integer.compare(value, h.value()) < 0) {
            // ─ Go LEFT ─────────────────────────────────────────────────────
            // We need h.left (or h.left.left) to be RED so that when we
            // eventually delete, we delete a red node (doesn't harm bh).
            if (!isRed(h.left()) && !isRed(h.left().left())) {
                h = moveRedLeft(h);
            }
            return fixUp(h.withLeft(deleteHelper(h.left(), value)));
        } else {
            // ─ Go RIGHT (or delete here) ────────────────────────────────────
            // First, if left is red, rotate right to keep deletion balanced.
            if (isRed(h.left())) {
                h = rotateRight(h);
            }
            // If we found the value and there is no right child, just remove.
            if (Integer.compare(value, h.value()) == 0 && h.right() == null) {
                return null;
            }
            // Ensure h.right (or h.right.left) is RED before going right.
            if (!isRed(h.right()) && !isRed(h.right().left())) {
                h = moveRedRight(h);
            }
            if (Integer.compare(value, h.value()) == 0) {
                // Replace value with in-order successor (min of right subtree),
                // then delete the successor from the right subtree.
                int successor = minValue(h.right());
                Node newRight = deleteMin(h.right());
                h = new Node(successor, h.color(), h.left(), newRight);
            } else {
                h = h.withRight(deleteHelper(h.right(), value));
            }
            return fixUp(h);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Search / Contains
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Search is identical to BST search — color is irrelevant.
    // Time: O(log n) worst-case (tree height ≤ 2 log n).

    /**
     * Return {@code true} if {@code value} is in the tree.
     */
    public boolean contains(int value) {
        Node node = root;
        while (node != null) {
            int cmp = Integer.compare(value, node.value());
            if (cmp < 0) node = node.left();
            else if (cmp > 0) node = node.right();
            else return true;
        }
        return false;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Min / Max
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the minimum value, or empty if the tree is empty. */
    public Optional<Integer> min() {
        if (root == null) return Optional.empty();
        Node n = root;
        while (n.left() != null) n = n.left();
        return Optional.of(n.value());
    }

    /** Return the maximum value, or empty if the tree is empty. */
    public Optional<Integer> max() {
        if (root == null) return Optional.empty();
        Node n = root;
        while (n.right() != null) n = n.right();
        return Optional.of(n.value());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Predecessor / Successor
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Predecessor: largest value strictly less than `value`.
    // Successor:   smallest value strictly greater than `value`.
    //
    // Standard BST traversal with "best so far" tracking.

    /** Return the largest value strictly less than {@code value}, or empty. */
    public Optional<Integer> predecessor(int value) {
        Optional<Integer> result = Optional.empty();
        Node n = root;
        while (n != null) {
            if (value > n.value()) {
                result = Optional.of(n.value());
                n = n.right();
            } else {
                n = n.left();
            }
        }
        return result;
    }

    /** Return the smallest value strictly greater than {@code value}, or empty. */
    public Optional<Integer> successor(int value) {
        Optional<Integer> result = Optional.empty();
        Node n = root;
        while (n != null) {
            if (value < n.value()) {
                result = Optional.of(n.value());
                n = n.left();
            } else {
                n = n.right();
            }
        }
        return result;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // kthSmallest
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Uses in-order traversal. O(n) in the worst case. For a more efficient
    // O(log n) implementation, augment each node with a subtree size.

    /**
     * Return the k-th smallest element (1-indexed).
     *
     * @throws NoSuchElementException if k is out of range
     */
    public int kthSmallest(int k) {
        List<Integer> sorted = toSortedList();
        if (k < 1 || k > sorted.size()) {
            throw new NoSuchElementException("k=" + k + " out of range; tree has " + sorted.size() + " elements");
        }
        return sorted.get(k - 1);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sorted Traversal
    // ─────────────────────────────────────────────────────────────────────────

    /** Return all elements in ascending (in-order) order. */
    public List<Integer> toSortedList() {
        List<Integer> result = new ArrayList<>();
        inOrder(root, result);
        return result;
    }

    private static void inOrder(Node node, List<Integer> acc) {
        if (node == null) return;
        inOrder(node.left(), acc);
        acc.add(node.value());
        inOrder(node.right(), acc);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Validation: isValidRB
    // ─────────────────────────────────────────────────────────────────────────
    //
    // Verify all 5 Red-Black invariants:
    //   1. Every node is RED or BLACK.  (enforced by the enum)
    //   2. Root is BLACK.
    //   3. Null leaves are BLACK.       (convention — null = BLACK)
    //   4. Red nodes have only BLACK children.
    //   5. All root-to-NIL paths have the same black-height.
    //
    // Returns true iff the tree is a valid Red-Black tree.

    /**
     * Verify all 5 Red-Black invariants.
     *
     * @return {@code true} if all invariants hold; {@code false} otherwise.
     */
    public boolean isValidRB() {
        if (root == null) return true;
        // Rule 2: root must be black
        if (root.color() != Color.BLACK) return false;
        // Rules 4 and 5 checked recursively; -1 signals a violation
        return checkNode(root) != -1;
    }

    /**
     * Recursively verify the sub-tree. Returns the black-height, or -1 on violation.
     *
     * <p>The black-height of a null leaf is 1 (the null itself counts as a BLACK node).
     */
    private static int checkNode(Node node) {
        if (node == null) return 1; // NIL = BLACK, contributes 1

        // Rule 4: red node must not have red children
        if (node.color() == Color.RED) {
            if (isRed(node.left()) || isRed(node.right())) return -1;
        }

        int leftBH  = checkNode(node.left());
        int rightBH = checkNode(node.right());

        if (leftBH == -1 || rightBH == -1) return -1;

        // Rule 5: black-heights must match across left and right subtrees
        if (leftBH != rightBH) return -1;

        // This node's black-height = children's bh + (1 if this node is black)
        return leftBH + (node.color() == Color.BLACK ? 1 : 0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Black Height
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Return the black-height of the root (number of BLACK nodes on any path
     * from root to NIL, not counting the root itself if it's red, but counting
     * NIL leaves).
     *
     * <p>Returns 0 for an empty tree.
     */
    public int blackHeight() {
        return blackHeightHelper(root);
    }

    private static int blackHeightHelper(Node node) {
        if (node == null) return 0;
        int bh = blackHeightHelper(node.left());
        return bh + (node.color() == Color.BLACK ? 1 : 0);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Size / Height / isEmpty
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the number of elements in the tree. */
    public int size() {
        return sizeHelper(root);
    }

    private static int sizeHelper(Node n) {
        if (n == null) return 0;
        return 1 + sizeHelper(n.left()) + sizeHelper(n.right());
    }

    /** Return the height of the tree (0 for empty, 1 for a single root). */
    public int height() {
        return heightHelper(root);
    }

    private static int heightHelper(Node n) {
        if (n == null) return 0;
        return 1 + Math.max(heightHelper(n.left()), heightHelper(n.right()));
    }

    /** Return {@code true} if the tree contains no elements. */
    public boolean isEmpty() {
        return root == null;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Root Accessor (for testing/debugging)
    // ─────────────────────────────────────────────────────────────────────────

    /** Return the root node (may be null for an empty tree). */
    public Node getRoot() {
        return root;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // toString
    // ─────────────────────────────────────────────────────────────────────────

    @Override
    public String toString() {
        return "RBTree{size=" + size() + ", height=" + height() + ", blackHeight=" + blackHeight() + "}";
    }
}
