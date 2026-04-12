"""
B-Tree — Self-Balancing Search Tree
====================================

What problem does a B-tree solve?
----------------------------------
A binary search tree (BST) works beautifully in RAM, where every memory access
costs about the same.  But a database with millions of rows must live on disk.
A random disk seek takes ~10 ms; a cache hit takes ~10 ns — a factor of 1 000 000.

The B-tree's design is driven by one insight:

    **Read one big block at a time, not one tiny node.**

A modern SSD reads 4 KiB in a single I/O.  If each tree node fills exactly one
block, we can store hundreds of keys per node.  A height-4 B-tree with 500 keys
per node covers 500^4 = 62 BILLION records with just 4 block reads per lookup.
SQLite, PostgreSQL, MySQL, and virtually every filesystem on earth use B-trees.

The minimum degree t
---------------------
Every B-tree is parameterised by an integer t ≥ 2 called the **minimum degree**.

    - Every node except the root holds between t-1 and 2t-1 keys.
    - The root holds between 1 and 2t-1 keys.
    - A node with k keys has exactly k+1 children (if it is not a leaf).

With t=2 (the minimum), internal nodes hold 1–3 keys and 2–4 children.
This is called a **2-3-4 tree**.

With t=50, nodes hold 49–99 keys and 50–100 children.  Much more disk-friendly.

Visual example (t=2, keys 1–7 inserted in order):

              [4]
           /       \\
       [2]           [6]
      /   \\         /   \\
    [1]  [3]     [5]   [7]

Every leaf is at the SAME depth.  This is the key invariant.

Proactive top-down splitting
-----------------------------
There are two strategies for keeping nodes within their 2t-1 key limit:

  1. **Bottom-up (lazy)**: Insert into leaves; split nodes on the way back up
     using a recursive call stack.  Simple but requires backtracking.

  2. **Top-down (proactive)**: As we descend to find the insertion point,
     split any FULL node we encounter along the way.  By the time we reach the
     leaf, every ancestor is guaranteed to have room for an additional key
     (because we split it if it was full).  No backtracking needed.

This implementation uses strategy 2 (CLRS Algorithm B-TREE-INSERT).

How splitting works
-------------------
When a node is "full" (it has 2t-1 keys), we split it into two nodes of t-1
keys each, and the MEDIAN key (index t-1) is promoted to the parent.

Example: split a full node [1, 2, 3, 4, 5] with t=3 (max 5 keys):

    parent: [..., X, ...]
                 |
          [1, 2, 3, 4, 5]

    After split:

    parent: [..., X, 3, ...]
                 /      \\
            [1, 2]    [4, 5]

The median key (3) moves up.  The left half keeps keys < median; the right half
keeps keys > median.  The children are split similarly.

Delete — the hard part
-----------------------
Deletion has three main cases:

  **Case 1** — key k is in a LEAF node:
    Simply remove it.  (We pre-filled the node on the way down if needed,
    so removing one key still leaves ≥ t-1 keys — the minimum.)

  **Case 2** — key k is in an INTERNAL node x:
    Let y = x.children[i] (the child immediately before k) and
        z = x.children[i+1] (the child immediately after k).

    **2a**: y has ≥ t keys → find k', the in-order predecessor of k (the
            rightmost key in the subtree rooted at y).  Replace k with k'
            in x, then recursively delete k' from y.

    **2b**: z has ≥ t keys → symmetric: use in-order successor k'' from z.

    **2c**: Both y and z have exactly t-1 keys → merge k and all of z into y,
            giving y a total of 2t-1 keys.  Remove k and z from x, then
            recursively delete k from the merged y.

  **Case 3** — k is NOT in the current internal node x, and we must descend
               into child x.children[i]:
    If x.children[i] has only t-1 keys, we need to "pre-fill" it before
    descending (otherwise a future delete from that subtree would leave it
    under-full):

    **3a**: If an immediate sibling has ≥ t keys, rotate a key through the
            parent to give x.children[i] one extra key (like a B-tree rotation).

    **3b**: If no sibling has a spare key, merge x.children[i] with a sibling
            and the separator key from x, reducing x's key count by 1.
"""

from __future__ import annotations

import bisect
from collections.abc import Iterator
from dataclasses import dataclass, field
from typing import Any


# ---------------------------------------------------------------------------
# BTreeNode
# ---------------------------------------------------------------------------

@dataclass
class BTreeNode:
    """A single node in the B-tree.

    A B-tree node is like a "mini sorted array" that can have many keys.
    Think of it as an airport departure board: it lists destinations (keys)
    in sorted order, and the gaps between destinations tell you which gate
    (child pointer) leads to flights in that range.

    Attributes:
        keys:     Sorted list of keys stored in this node.
        values:   Parallel list; values[i] corresponds to keys[i].
        children: List of child BTreeNode pointers.  For a node with k keys,
                  children has k+1 entries.  Empty for leaf nodes.
        is_leaf:  True if this node has no children.

    Invariants (for minimum degree t):
        - t-1 ≤ len(keys) ≤ 2t-1  (except root: 1 ≤ len(keys) ≤ 2t-1)
        - len(children) == len(keys) + 1  (if not is_leaf)
        - len(children) == 0              (if is_leaf)
        - keys are sorted in ascending order
        - All leaves are at the same depth
    """

    keys: list[Any] = field(default_factory=list)
    values: list[Any] = field(default_factory=list)
    children: list["BTreeNode"] = field(default_factory=list)
    is_leaf: bool = True

    def is_full(self, t: int) -> bool:
        """Return True if this node has reached its maximum capacity 2t-1."""
        return len(self.keys) == 2 * t - 1

    def find_key_index(self, key: Any) -> int:
        """Binary-search for the leftmost position where keys[i] >= key.

        If key is present, this returns its index.
        If key is absent, this returns the index of the child to descend into.

        Example: keys = [10, 20, 30], find_key_index(15) → 1
          (descend into children[1], which covers keys in (10, 20))
        """
        return bisect.bisect_left(self.keys, key)


# ---------------------------------------------------------------------------
# BTree
# ---------------------------------------------------------------------------

class BTree:
    """B-Tree: a self-balancing multi-way search tree.

    The B-tree maintains a sorted map of (key, value) pairs and guarantees
    O(log n) worst-case time for insert, delete, and search — and crucially,
    the O(log n) is measured in disk I/Os, not CPU operations, because the
    logarithm base is t (up to hundreds), not 2.

    Parameters:
        t: Minimum degree.  Must be ≥ 2.
           - t=2  → 2-3-4 tree (nodes hold 1–3 keys, 2–4 children)
           - t=3  → nodes hold 2–5 keys, 3–6 children
           - t=50 → nodes hold 49–99 keys, 50–100 children

    Usage::

        tree = BTree(t=2)
        tree.insert(5, "five")
        tree.insert(3, "three")
        tree.insert(7, "seven")

        tree.search(3)           # → "three"
        tree[5]                  # → "five"
        tree.min_key()           # → 3
        tree.max_key()           # → 7
        tree.range_query(3, 6)   # → [(3, "three"), (5, "five")]
        tree.height()            # → 1
        tree.is_valid()          # → True

        del tree[3]
        3 in tree                # → False
    """

    def __init__(self, t: int = 2) -> None:
        if t < 2:
            raise ValueError(f"Minimum degree t must be >= 2, got {t}")
        self._t = t
        self._root: BTreeNode = BTreeNode(is_leaf=True)
        self._size: int = 0

    # -----------------------------------------------------------------------
    # Internal helpers
    # -----------------------------------------------------------------------

    def _split_child(self, parent: BTreeNode, child_index: int) -> None:
        """Split parent.children[child_index] (which must be full) into two nodes.

        Before:
            parent: [..., sep_left, sep_right, ...]
                                   |
                        child (2t-1 keys): [k0, k1, ..., k_{t-2}, k_{t-1}, k_t, ..., k_{2t-2}]

        After:
            parent: [..., sep_left, k_{t-1}, sep_right, ...]
                                   /                \\
                        left (t-1):             right (t-1):
                     [k0, ..., k_{t-2}]     [k_t, ..., k_{2t-2}]

        The median key k_{t-1} is PROMOTED to the parent.
        The child is split into two nodes of t-1 keys each.
        Children are split in the same way (left gets first t, right gets last t).

        This is O(t) work — we copy t-1 keys/values/children.
        """
        t = self._t
        child = parent.children[child_index]

        # Create the new right sibling
        right = BTreeNode(is_leaf=child.is_leaf)

        # The median key moves up to the parent
        mid = t - 1  # index of the median in child.keys
        median_key = child.keys[mid]
        median_val = child.values[mid]

        # Right node gets the upper half of child's keys/values
        right.keys = child.keys[mid + 1:]
        right.values = child.values[mid + 1:]

        # If child is internal, right node gets the upper half of children
        if not child.is_leaf:
            right.children = child.children[t:]
            child.children = child.children[:t]

        # Trim child (left node) to hold only the lower half
        child.keys = child.keys[:mid]
        child.values = child.values[:mid]

        # Insert the median key into the parent
        parent.keys.insert(child_index, median_key)
        parent.values.insert(child_index, median_val)

        # Insert the new right node into parent's children list
        parent.children.insert(child_index + 1, right)

    def _insert_nonfull(self, node: BTreeNode, key: Any, value: Any) -> bool:
        """Insert key into the subtree rooted at node, assuming node is not full.

        Returns True if a new key was inserted, False if an existing key was
        updated (so the caller can adjust self._size correctly).

        We walk DOWN the tree without backtracking.  At each internal node, if
        the child we're about to descend into is full, we split it first.
        After the split, we decide which of the two resulting nodes to descend
        into based on the newly promoted median key.

        At the leaf, we do a sorted insertion.

        The invariant maintained: when we call _insert_nonfull on a node, that
        node is guaranteed to be NOT full.  Because we pre-split children before
        descending, the child we land on is also not full.
        """
        t = self._t
        i = node.find_key_index(key)

        # Check if the key already exists at this node (exact match)
        if i < len(node.keys) and node.keys[i] == key:
            node.values[i] = value  # update in place
            return False  # not a new insertion

        if node.is_leaf:
            # Insert at position i to maintain sorted order
            node.keys.insert(i, key)
            node.values.insert(i, value)
            return True
        else:
            # i is the child index to descend into
            # Pre-split the child if it's full (proactive top-down splitting)
            if node.children[i].is_full(t):
                self._split_child(node, i)
                # After split, parent.keys[i] is the promoted median.
                # Decide whether to go left (i) or right (i+1).
                if key == node.keys[i]:
                    # The median itself is our key — update it
                    node.values[i] = value
                    return False
                elif key > node.keys[i]:
                    i += 1  # descend into the right half

            return self._insert_nonfull(node.children[i], key, value)

    def _search(self, node: BTreeNode, key: Any) -> Any | None:
        """Recursively search for key in the subtree rooted at node.

        At each node, binary-search for the key.  If found, return the value.
        If not found at this node and this is a leaf, return None.
        Otherwise, recurse into the appropriate child.

        This terminates early if the key is found in an internal node —
        unlike B+ trees, B-trees store data at every level.
        """
        i = node.find_key_index(key)

        # Key found at this node?
        if i < len(node.keys) and node.keys[i] == key:
            return node.values[i]

        # Key not here; if leaf, it doesn't exist
        if node.is_leaf:
            return None

        # Descend into the appropriate child
        return self._search(node.children[i], key)

    def _contains(self, node: BTreeNode, key: Any) -> bool:
        """Recursively check if key exists in the subtree."""
        i = node.find_key_index(key)
        if i < len(node.keys) and node.keys[i] == key:
            return True
        if node.is_leaf:
            return False
        return self._contains(node.children[i], key)

    def _inorder(self, node: BTreeNode) -> Iterator[tuple[Any, Any]]:
        """Yield (key, value) pairs in sorted order via in-order traversal.

        For a node with keys [k0, k1, k2] and children [c0, c1, c2, c3]:
          yield all from c0, yield k0, yield all from c1, yield k1, ...

        This is the standard in-order traversal generalised from BSTs.
        """
        if node.is_leaf:
            yield from zip(node.keys, node.values)
            return
        for i, (k, v) in enumerate(zip(node.keys, node.values)):
            yield from self._inorder(node.children[i])
            yield (k, v)
        # Don't forget the last child
        yield from self._inorder(node.children[-1])

    def _min_node(self, node: BTreeNode) -> BTreeNode:
        """Descend to the leftmost leaf (holds the minimum key)."""
        while not node.is_leaf:
            node = node.children[0]
        return node

    def _max_node(self, node: BTreeNode) -> BTreeNode:
        """Descend to the rightmost leaf (holds the maximum key)."""
        while not node.is_leaf:
            node = node.children[-1]
        return node

    def _height(self, node: BTreeNode) -> int:
        """Recursively compute height (0 = leaf, 1 = parent of leaves, ...)."""
        if node.is_leaf:
            return 0
        return 1 + self._height(node.children[0])

    # -----------------------------------------------------------------------
    # Delete helpers
    # -----------------------------------------------------------------------

    def _predecessor(self, node: BTreeNode) -> tuple[Any, Any]:
        """Return (key, value) of the in-order predecessor (rightmost in subtree)."""
        n = self._max_node(node)
        return (n.keys[-1], n.values[-1])

    def _successor(self, node: BTreeNode) -> tuple[Any, Any]:
        """Return (key, value) of the in-order successor (leftmost in subtree)."""
        n = self._min_node(node)
        return (n.keys[0], n.values[0])

    def _merge_children(
        self, parent: BTreeNode, left_idx: int
    ) -> BTreeNode:
        """Merge parent.children[left_idx] with parent.children[left_idx+1].

        The separator key at parent.keys[left_idx] is pulled down into the
        merged node.  The right sibling is discarded.

        Before:
            parent: [..., sep, ...]
                        /       \\
                  left (t-1)   right (t-1)

        After:
            parent: [...] (sep removed)
                        |
                merged (2t-1): left.keys + [sep] + right.keys

        Returns the merged node.
        """
        left = parent.children[left_idx]
        right = parent.children[left_idx + 1]

        # Pull down the separator
        sep_key = parent.keys[left_idx]
        sep_val = parent.values[left_idx]

        # Merge: left + [sep] + right
        left.keys.append(sep_key)
        left.values.append(sep_val)
        left.keys.extend(right.keys)
        left.values.extend(right.values)
        if not left.is_leaf:
            left.children.extend(right.children)

        # Remove sep from parent; remove right child pointer
        parent.keys.pop(left_idx)
        parent.values.pop(left_idx)
        parent.children.pop(left_idx + 1)

        return left

    def _ensure_min_keys(self, parent: BTreeNode, child_idx: int) -> int:
        """Ensure parent.children[child_idx] has at least t keys before descending.

        If the child is "thin" (has only t-1 keys), we either:
          3a: Borrow a key from a sibling that has ≥ t keys (rotate through parent).
          3b: Merge with a sibling (pulling down the separator from the parent).

        Returns the (possibly adjusted) child index after the operation.
        Merging may shift children indices if we merge with the LEFT sibling.
        """
        t = self._t
        child = parent.children[child_idx]

        if len(child.keys) >= t:
            return child_idx  # already fat enough, nothing to do

        # Try to borrow from the left sibling
        if child_idx > 0:
            left_sib = parent.children[child_idx - 1]
            if len(left_sib.keys) >= t:
                # Case 3a: rotate right (borrow from left)
                # Pull down parent separator into child (prepend)
                child.keys.insert(0, parent.keys[child_idx - 1])
                child.values.insert(0, parent.values[child_idx - 1])
                # Move left sibling's last key up to parent
                parent.keys[child_idx - 1] = left_sib.keys[-1]
                parent.values[child_idx - 1] = left_sib.values[-1]
                left_sib.keys.pop()
                left_sib.values.pop()
                # Move left sibling's last child to child's first child
                if not left_sib.is_leaf:
                    child.children.insert(0, left_sib.children.pop())
                return child_idx

        # Try to borrow from the right sibling
        if child_idx < len(parent.children) - 1:
            right_sib = parent.children[child_idx + 1]
            if len(right_sib.keys) >= t:
                # Case 3a: rotate left (borrow from right)
                # Pull down parent separator into child (append)
                child.keys.append(parent.keys[child_idx])
                child.values.append(parent.values[child_idx])
                # Move right sibling's first key up to parent
                parent.keys[child_idx] = right_sib.keys.pop(0)
                parent.values[child_idx] = right_sib.values.pop(0)
                # Move right sibling's first child to child's last child
                if not right_sib.is_leaf:
                    child.children.append(right_sib.children.pop(0))
                return child_idx

        # Case 3b: must merge — no sibling has a spare key
        if child_idx > 0:
            # Merge child with its LEFT sibling
            self._merge_children(parent, child_idx - 1)
            return child_idx - 1  # merged node is now at child_idx - 1
        else:
            # Merge child with its RIGHT sibling
            self._merge_children(parent, child_idx)
            return child_idx  # merged node stays at child_idx

    def _delete(self, node: BTreeNode, key: Any) -> bool:
        """Recursively delete key from the subtree rooted at node.

        Returns True if deleted, False if not found.

        Precondition: node has at least t keys (guaranteed by _ensure_min_keys
        before every recursive descent), UNLESS node is the root (the root is
        allowed to have as few as 1 key).
        """
        t = self._t
        i = node.find_key_index(key)
        found = i < len(node.keys) and node.keys[i] == key

        if found:
            if node.is_leaf:
                # Case 1: key is in a leaf — just remove it
                node.keys.pop(i)
                node.values.pop(i)
                return True
            else:
                # Key is in an internal node
                left_child = node.children[i]
                right_child = node.children[i + 1]

                if len(left_child.keys) >= t:
                    # Case 2a: left child has a spare key — use predecessor
                    pred_key, pred_val = self._predecessor(left_child)
                    node.keys[i] = pred_key
                    node.values[i] = pred_val
                    return self._delete(left_child, pred_key)

                elif len(right_child.keys) >= t:
                    # Case 2b: right child has a spare key — use successor
                    succ_key, succ_val = self._successor(right_child)
                    node.keys[i] = succ_key
                    node.values[i] = succ_val
                    return self._delete(right_child, succ_key)

                else:
                    # Case 2c: both children have exactly t-1 keys — merge
                    merged = self._merge_children(node, i)
                    return self._delete(merged, key)

        else:
            # Key is not in this node; descend into appropriate child
            if node.is_leaf:
                return False  # key not found

            # Ensure the child has enough keys before descending (Case 3)
            i = self._ensure_min_keys(node, i)

            # If root became empty after a merge, tree shrinks in height
            # (handled by the caller — see delete() public method)

            return self._delete(node.children[i], key)

    # -----------------------------------------------------------------------
    # Validation helpers
    # -----------------------------------------------------------------------

    def _validate(
        self,
        node: BTreeNode,
        min_key: Any,
        max_key: Any,
        depth: int,
        expected_leaf_depth: list[int],
        is_root: bool,
    ) -> bool:
        """Recursively validate B-tree invariants.

        Checks:
          1. Key count bounds: t-1 ≤ len(keys) ≤ 2t-1 (root: 1 ≤ len(keys))
          2. keys is sorted and within the allowed range [min_key, max_key]
          3. If internal: len(children) == len(keys) + 1
          4. All leaves are at the same depth
        """
        t = self._t
        n_keys = len(node.keys)

        # Invariant 1: key count
        if is_root:
            if self._size > 0 and n_keys < 1:
                return False
        else:
            if n_keys < t - 1 or n_keys > 2 * t - 1:
                return False

        # Invariant 2: keys sorted and within bounds
        for j, k in enumerate(node.keys):
            if min_key is not None and k <= min_key:
                return False
            if max_key is not None and k >= max_key:
                return False
            if j > 0 and node.keys[j] <= node.keys[j - 1]:
                return False  # not strictly increasing

        # Invariant 3: child count
        if node.is_leaf:
            if node.children:
                return False
            # Record or check leaf depth
            if expected_leaf_depth[0] == -1:
                expected_leaf_depth[0] = depth
            elif expected_leaf_depth[0] != depth:
                return False  # Invariant 4: all leaves same depth
        else:
            if len(node.children) != n_keys + 1:
                return False
            # Recurse into children with tightened bounds
            for j, child in enumerate(node.children):
                lo = node.keys[j - 1] if j > 0 else min_key
                hi = node.keys[j] if j < n_keys else max_key
                if not self._validate(child, lo, hi, depth + 1, expected_leaf_depth, False):
                    return False

        return True

    # -----------------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------------

    def insert(self, key: Any, value: Any) -> None:
        """Insert key with associated value into the B-tree.

        If the key already exists, its value is updated in place.
        O(t · log_t n) time — t work per node visited, log_t n nodes visited.

        Algorithm (CLRS B-TREE-INSERT):
          1. If the root is full, split it first.
             - Create a new empty root, make the old root its first child.
             - Split the old (now first) child.
             - Height increases by 1.
          2. Call _insert_nonfull on the (guaranteed non-full) root.

        Example::

            tree = BTree(t=2)
            tree.insert(10, "a")
            tree.insert(20, "b")
            tree.insert(5,  "c")
            tree.search(10)  # → "a"
        """
        root = self._root
        if root.is_full(self._t):
            # Root is full — must grow the tree upward
            new_root = BTreeNode(is_leaf=False)
            new_root.children.append(root)
            self._split_child(new_root, 0)
            self._root = new_root
            is_new = self._insert_nonfull(new_root, key, value)
        else:
            is_new = self._insert_nonfull(root, key, value)

        if is_new:
            self._size += 1

    def delete(self, key: Any) -> None:
        """Remove key from the B-tree.

        Raises KeyError if key is not found.
        O(t · log_t n) time.

        After deletion, if the root becomes empty (all its keys were merged into
        a child), the tree's height shrinks by 1 and the first child becomes the
        new root.

        Example::

            tree = BTree(t=2)
            tree.insert(5, "five")
            tree.delete(5)
            5 in tree  # → False
        """
        if not self._contains(self._root, key):
            raise KeyError(key)

        self._delete(self._root, key)
        self._size -= 1

        # If the root has no keys but has a child, shrink the tree
        if not self._root.keys and self._root.children:
            self._root = self._root.children[0]

    def search(self, key: Any) -> Any | None:
        """Return the value associated with key, or None if not found.

        O(t · log_t n) time.

        Example::

            tree = BTree(t=2)
            tree.insert(42, "the answer")
            tree.search(42)   # → "the answer"
            tree.search(99)   # → None
        """
        return self._search(self._root, key)

    def __contains__(self, key: Any) -> bool:
        """Support ``key in tree`` syntax.

        O(t · log_t n) time.
        """
        return self._contains(self._root, key)

    def __getitem__(self, key: Any) -> Any:
        """Return tree[key]. Raises KeyError if not found."""
        result = self._search(self._root, key)
        if result is None and not self._contains(self._root, key):
            raise KeyError(key)
        return result

    def __setitem__(self, key: Any, value: Any) -> None:
        """Support tree[key] = value (insert or update)."""
        self.insert(key, value)

    def __delitem__(self, key: Any) -> None:
        """Support del tree[key]. Raises KeyError if not found."""
        self.delete(key)

    def __len__(self) -> int:
        """Return the number of (key, value) pairs in the tree. O(1)."""
        return self._size

    def __bool__(self) -> bool:
        """Return True if the tree is non-empty."""
        return self._size > 0

    def min_key(self) -> Any:
        """Return the smallest key in the tree.

        Raises ValueError if the tree is empty.
        O(log_t n) time — walk down leftmost spine to leaf.
        """
        if not self._size:
            raise ValueError("Tree is empty")
        node = self._min_node(self._root)
        return node.keys[0]

    def max_key(self) -> Any:
        """Return the largest key in the tree.

        Raises ValueError if the tree is empty.
        O(log_t n) time — walk down rightmost spine to leaf.
        """
        if not self._size:
            raise ValueError("Tree is empty")
        node = self._max_node(self._root)
        return node.keys[-1]

    def range_query(self, low: Any, high: Any) -> list[tuple[Any, Any]]:
        """Return all (key, value) pairs where low <= key <= high, in sorted order.

        O(t · log_t n + k) time where k is the number of results.

        The implementation uses the inorder generator and stops early once
        keys exceed `high`.  This is O(log_t n) to reach the first result
        plus O(k) to collect results.

        Example::

            tree = BTree(t=2)
            for k, v in [(1,"a"), (3,"c"), (5,"e"), (7,"g")]:
                tree.insert(k, v)
            tree.range_query(2, 6)
            # → [(3, "c"), (5, "e")]
        """
        result = []
        for k, v in self._inorder(self._root):
            if k > high:
                break
            if k >= low:
                result.append((k, v))
        return result

    def inorder(self) -> Iterator[tuple[Any, Any]]:
        """Yield (key, value) pairs in ascending key order.

        O(n) time and O(h) space (h = height) due to the generator stack.

        Example::

            tree = BTree(t=2)
            tree.insert(3, "three")
            tree.insert(1, "one")
            tree.insert(2, "two")
            list(tree.inorder())
            # → [(1, "one"), (2, "two"), (3, "three")]
        """
        return self._inorder(self._root)

    def height(self) -> int:
        """Return the height of the tree (0 for a single-node tree).

        All paths from root to leaf have exactly this length — a key
        B-tree invariant.

        O(log_t n) time — follows the leftmost path.
        """
        return self._height(self._root)

    def is_valid(self) -> bool:
        """Check all B-tree structural invariants.

        Returns True if valid, False otherwise.  Useful in testing.

        Invariants checked:
          1. Key counts: t-1 ≤ len(node.keys) ≤ 2t-1 for non-root nodes
          2. Root has at least 1 key (unless tree is empty)
          3. Keys within each node are strictly increasing
          4. Keys respect the BST ordering property between parent and children
          5. Child count is len(keys) + 1 for internal nodes
          6. All leaves are at the same depth
        """
        if self._size == 0:
            return True
        expected_leaf_depth: list[int] = [-1]
        return self._validate(
            self._root,
            min_key=None,
            max_key=None,
            depth=0,
            expected_leaf_depth=expected_leaf_depth,
            is_root=True,
        )

    def __repr__(self) -> str:
        """Human-readable summary showing key count and height."""
        return f"BTree(t={self._t}, size={self._size}, height={self.height()})"
