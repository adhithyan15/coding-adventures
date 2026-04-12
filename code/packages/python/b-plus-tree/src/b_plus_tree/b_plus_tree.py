"""
B+ Tree — All Data in Leaves, Linked Leaf Layer
=================================================

How does a B+ tree differ from a B-tree?
-----------------------------------------
In a regular B-tree, every node (leaf AND internal) stores (key, value) pairs.
This means a search can terminate early at an internal node when the key is found.

A B+ tree makes a different trade-off:

    **ALL data lives in the leaf nodes.  Internal nodes are a "pure index".**

Internal nodes store only SEPARATOR KEYS — guide signs that tell you which
subtree to descend into — but they carry no associated data values.

This has two important consequences:

  1. **Full leaf layer**: Every key-value pair is in a leaf.  Leaves are
     doubly-linkable (we use a singly-linked `next` pointer here) forming a
     **sorted linked list of all data**.  Walking this list gives you every
     record in sorted order in O(n) time — no tree traversal needed.

  2. **More keys per internal node**: Since internal nodes hold only keys
     (no values), they pack more keys per disk block, reducing tree height.
     A block of 4 KiB holding 8-byte keys and 8-byte pointers can store
     ~340 keys per node — a height-3 B+ tree covers 340^3 ≈ 39 million pages.

Visual B+ tree with t=2 and keys [1, 3, 5, 7, 9]:

    Internal:        [5]
                    /    \\
    Internal:   [3]        [7]
               /   \\      /   \\
    Leaves: [1,3] [4,5] [6,7] [8,9]
              ↓      ↓      ↓      ↓
             next → next → next → None

    (The separator key 5 also appears as the smallest key of the right child's
     subtree.  Unlike a B-tree, it is COPIED, not moved, when a leaf splits.)

The B+ tree leaf split rule
----------------------------
When a leaf overflows (holds 2t keys after an insert):
  - Split into LEFT (first t keys) and RIGHT (remaining t keys).
  - The separator key pushed into the parent is the SMALLEST key of the RIGHT
    leaf — and it stays in the right leaf as a data record.

This is the crucial difference from a B-tree split:
  - B-tree: median key MOVES up to the parent (not in either child).
  - B+ tree leaf split: smallest-of-right key is COPIED up (stays in leaf too).

Why does this matter?  Because every key must be reachable from the leaf layer.
If we moved the separator out of the leaf, that key-value pair would vanish from
the data layer.

Internal node splits (when an internal node overflows) work like B-tree splits:
the median key MOVES up and does NOT stay in either child.

Lookup
------
Search descends from root to the appropriate leaf, then does a linear scan of
the leaf's keys array.

    Search for key k:
      node = root
      while not isinstance(node, BPlusLeafNode):
          i = first index where node.keys[i] > k, or len(node.keys)
          node = node.children[i]
      scan leaf.keys for k

Full scan
---------
    node = first_leaf
    while node is not None:
        yield each (key, value) in node
        node = node.next

Range scan
----------
    Descend to the leaf where low would live.
    Walk the linked list collecting results until key > high.

Delete
------
Deletion in a B+ tree is conceptually similar to deletion in a B-tree, with
two differences:

  1. The actual key-value pair to remove is always in a LEAF.  Even if the key
     appears as a separator in an internal node, the data is in the leaf.

  2. After deleting from a leaf, if the separator key in the parent came from
     that leaf (i.e., it equals the deleted key), the separator may need to be
     updated to the leaf's new minimum key.  However, a common simplification
     is to leave stale separators — they still correctly route searches as long
     as the BST ordering property holds.  We choose this simpler approach here.

  The pre-fill strategy (borrowing from siblings or merging) mirrors the B-tree
  approach, but operates on both internal nodes and leaf nodes.
"""

from __future__ import annotations

import bisect
from collections.abc import Iterator
from dataclasses import dataclass, field
from typing import Any


# ---------------------------------------------------------------------------
# Node types
# ---------------------------------------------------------------------------

@dataclass
class BPlusInternalNode:
    """An internal (non-leaf) node in the B+ tree.

    Internal nodes are PURE INDEX: they guide searches but hold no data.
    Think of them as the table of contents pages of a book: they tell you
    which chapter (child subtree) to look in, but the actual text is in
    the chapters (leaves).

    Attributes:
        keys:     Separator keys.  For a node with k keys, there are k+1
                  children.  All keys in children[i] are:
                    - ≥ keys[i-1]  (if i > 0)
                    - < keys[i]    (if i < k)
        children: List of child nodes (BPlusInternalNode or BPlusLeafNode).
    """

    keys: list[Any] = field(default_factory=list)
    children: list["BPlusInternalNode | BPlusLeafNode"] = field(default_factory=list)

    def is_full(self, t: int) -> bool:
        """True if this node has 2t-1 keys (the maximum before splitting)."""
        return len(self.keys) == 2 * t - 1

    def find_child_index(self, key: Any) -> int:
        """Return the index of the child subtree that might contain `key`.

        Uses bisect_right so that a key equal to a separator goes RIGHT
        (into the subtree where all keys are >= the separator).

        Example: separators = [10, 20, 30]
          key=5  → children[0]  (keys < 10)
          key=10 → children[1]  (keys in [10, 20))
          key=15 → children[1]
          key=20 → children[2]  (keys in [20, 30))
          key=35 → children[3]
        """
        return bisect.bisect_right(self.keys, key)


@dataclass
class BPlusLeafNode:
    """A leaf node in the B+ tree.

    Leaf nodes hold ALL the actual data.  They form a sorted singly-linked
    list via the `next` pointer, enabling O(n) full scans and O(log n + k)
    range scans.

    Attributes:
        keys:   Sorted list of keys in this leaf.
        values: Parallel list; values[i] corresponds to keys[i].
        next:   Pointer to the next leaf node in key order, or None if this
                is the last leaf.

    Leaf invariant (for minimum degree t):
        t-1 ≤ len(keys) ≤ 2t-1  (except the root leaf: 0 ≤ len(keys) ≤ 2t-1)
    """

    keys: list[Any] = field(default_factory=list)
    values: list[Any] = field(default_factory=list)
    next: "BPlusLeafNode | None" = field(default=None, repr=False)

    def is_full(self, t: int) -> bool:
        """True if this leaf has 2t-1 keys."""
        return len(self.keys) == 2 * t - 1

    def find_key_index(self, key: Any) -> int:
        """Binary-search for the position of `key` in this leaf."""
        return bisect.bisect_left(self.keys, key)


# ---------------------------------------------------------------------------
# BPlusTree
# ---------------------------------------------------------------------------

class BPlusTree:
    """B+ Tree: a self-balancing search tree where all data lives in leaves.

    The B+ tree is the data structure of choice for database indexes.
    Unlike a B-tree, internal nodes store only routing keys — no values.
    All key-value pairs are in the leaf layer, which forms a sorted linked
    list enabling efficient full and range scans.

    Parameters:
        t: Minimum degree (≥ 2).  Each leaf and internal node holds between
           t-1 and 2t-1 keys (root may hold fewer).

    Usage::

        tree = BPlusTree(t=3)
        tree.insert(5, "five")
        tree.insert(3, "three")
        tree.insert(7, "seven")

        tree.search(3)              # → "three"
        tree[5]                     # → "five"
        tree.min_key()              # → 3
        tree.max_key()              # → 7
        tree.range_scan(3, 6)       # → [(3, "three"), (5, "five")]
        list(tree.full_scan())      # → [(3, "three"), (5, "five"), (7, "seven")]
        list(tree)                  # → [3, 5, 7]
        list(tree.items())          # → [(3, "three"), (5, "five"), (7, "seven")]
        tree.height()               # → 1
        tree.is_valid()             # → True

        del tree[3]
        3 in tree                   # → False
    """

    def __init__(self, t: int = 2) -> None:
        if t < 2:
            raise ValueError(f"Minimum degree t must be >= 2, got {t}")
        self._t = t
        # The tree starts as a single empty leaf (the root IS a leaf)
        self._root: BPlusInternalNode | BPlusLeafNode = BPlusLeafNode()
        # first_leaf gives O(1) access to the start of the linked leaf list
        self._first_leaf: BPlusLeafNode = self._root  # type: ignore[assignment]
        self._size: int = 0

    # -----------------------------------------------------------------------
    # Internal helpers: traversal
    # -----------------------------------------------------------------------

    def _find_leaf(
        self,
        key: Any,
        track_path: bool = False,
    ) -> tuple[BPlusLeafNode, list[tuple[BPlusInternalNode, int]]]:
        """Descend from root to the leaf where `key` belongs.

        If track_path is True, also return the path (list of (parent, child_idx)
        tuples) from root to the leaf.  This is needed for delete operations that
        may need to merge/rebalance along the path.

        Returns:
            (leaf, path)
            path is empty if track_path is False.
        """
        path: list[tuple[BPlusInternalNode, int]] = []
        node: BPlusInternalNode | BPlusLeafNode = self._root

        while isinstance(node, BPlusInternalNode):
            i = node.find_child_index(key)
            if track_path:
                path.append((node, i))
            node = node.children[i]

        return node, path  # type: ignore[return-value]

    # -----------------------------------------------------------------------
    # Internal helpers: insert
    # -----------------------------------------------------------------------

    def _split_leaf(
        self, parent: BPlusInternalNode, child_idx: int
    ) -> None:
        """Split the leaf at parent.children[child_idx].

        B+ TREE LEAF SPLIT RULE (different from B-tree!):
          - LEFT leaf keeps the first t keys.
          - RIGHT leaf keeps the remaining t keys (including the split point).
          - The FIRST key of the right leaf is COPIED into the parent.
            It stays in the right leaf as a data record.

        Before:
            parent: [..., sep_left, sep_right, ...]
                                   |
                       leaf: [k0, k1, ..., k_{t-1}, k_t, ..., k_{2t-2}]

        After:
            parent: [..., sep_left, k_t, sep_right, ...]
                                   /           \\
                   left: [k0,..,k_{t-1}]   right: [k_t,..,k_{2t-2}]

        Note: k_t is the SEPARATOR.  It appears in the parent AND as the
        first key of the right leaf.  This ensures every key remains
        accessible from the leaf layer.

        The linked list is updated: left.next = right, right.next = old left.next.
        """
        t = self._t
        leaf = parent.children[child_idx]
        assert isinstance(leaf, BPlusLeafNode)

        # Create the right sibling
        right = BPlusLeafNode()

        # Split point: right gets keys[t:], left keeps keys[:t]
        right.keys = leaf.keys[t:]
        right.values = leaf.values[t:]
        leaf.keys = leaf.keys[:t]
        leaf.values = leaf.values[:t]

        # Update the linked list: left → right → (old right)
        right.next = leaf.next
        leaf.next = right

        # The separator pushed into the parent is the FIRST key of the right leaf
        separator = right.keys[0]

        # Insert separator into parent
        parent.keys.insert(child_idx, separator)
        parent.children.insert(child_idx + 1, right)

    def _split_internal(
        self, parent: BPlusInternalNode, child_idx: int
    ) -> None:
        """Split the internal node at parent.children[child_idx].

        Internal nodes split LIKE A B-TREE: the median key MOVES up to the
        parent.  It does NOT stay in either child.

        Before:
            child (2t-1 keys): [k0, ..., k_{t-2}, k_{t-1}, k_t, ..., k_{2t-2}]

        After:
            parent gets k_{t-1} inserted.
            left child: [k0, ..., k_{t-2}]           (t-1 keys)
            right child: [k_t, ..., k_{2t-2}]         (t-1 keys)
            Children are split: left gets first t, right gets last t.
        """
        t = self._t
        child = parent.children[child_idx]
        assert isinstance(child, BPlusInternalNode)

        right = BPlusInternalNode()
        mid = t - 1  # index of the median key

        # Median moves to parent
        median = child.keys[mid]

        # Right internal node gets upper half of keys (excluding median) and children
        right.keys = child.keys[mid + 1:]
        right.children = child.children[t:]

        # Trim child
        child.keys = child.keys[:mid]
        child.children = child.children[:t]

        # Insert median and right node into parent
        parent.keys.insert(child_idx, median)
        parent.children.insert(child_idx + 1, right)

    def _insert_recursive(
        self, node: BPlusInternalNode | BPlusLeafNode, key: Any, value: Any
    ) -> tuple[bool, Any | None, BPlusInternalNode | BPlusLeafNode | None]:
        """Recursively insert key into the subtree rooted at node.

        Returns:
            (is_new, split_key, split_node)
            - is_new:    True if a new key was inserted (not just updated).
            - split_key: If node was split, the separator key to push into
                         the parent.  None if no split occurred.
            - split_node: The new right node if a split occurred.

        This is a BOTTOM-UP approach: we descend first, then handle splits on
        the way back up (unlike the top-down proactive splitting in BTree).
        The bottom-up approach is easier to implement correctly for B+ trees
        because leaf splits need to update the linked list BEFORE reporting
        the split key.
        """
        t = self._t

        if isinstance(node, BPlusLeafNode):
            # ---- BASE CASE: insert into leaf ----
            i = node.find_key_index(key)
            if i < len(node.keys) and node.keys[i] == key:
                node.values[i] = value
                return False, None, None  # updated, no split

            node.keys.insert(i, key)
            node.values.insert(i, value)

            if len(node.keys) > 2 * t - 1:
                # Leaf overflowed — split it
                right = BPlusLeafNode()
                right.keys = node.keys[t:]
                right.values = node.values[t:]
                node.keys = node.keys[:t]
                node.values = node.values[:t]
                right.next = node.next
                node.next = right
                split_key = right.keys[0]  # smallest key of right → goes to parent
                return True, split_key, right

            return True, None, None  # inserted, no split

        else:
            # ---- RECURSIVE CASE: internal node ----
            assert isinstance(node, BPlusInternalNode)
            i = node.find_child_index(key)
            is_new, split_key, split_child = self._insert_recursive(
                node.children[i], key, value
            )

            if split_key is not None:
                # Child split — insert the separator into this internal node
                node.keys.insert(i, split_key)
                node.children.insert(i + 1, split_child)

                if len(node.keys) > 2 * t - 1:
                    # This internal node also overflowed — split it too
                    mid = t - 1
                    median = node.keys[mid]
                    right_internal = BPlusInternalNode()
                    right_internal.keys = node.keys[mid + 1:]
                    right_internal.children = node.children[t:]
                    node.keys = node.keys[:mid]
                    node.children = node.children[:t]
                    return is_new, median, right_internal

            return is_new, None, None

    # -----------------------------------------------------------------------
    # Internal helpers: delete
    # -----------------------------------------------------------------------

    def _borrow_from_left_leaf(
        self,
        parent: BPlusInternalNode,
        child_idx: int,
    ) -> None:
        """Rotate: move the rightmost key from left sibling to child.

        The separator in the parent is updated to the child's new minimum key
        (after borrowing).

        Leaf rotate right:
            parent.keys[child_idx-1] ← left.keys[-1]
            child.keys.insert(0, left.keys.pop())
        """
        left_sib: BPlusLeafNode = parent.children[child_idx - 1]  # type: ignore
        child: BPlusLeafNode = parent.children[child_idx]  # type: ignore

        # Move rightmost key-value from left to front of child
        child.keys.insert(0, left_sib.keys.pop())
        child.values.insert(0, left_sib.values.pop())

        # Update parent separator to child's new minimum
        parent.keys[child_idx - 1] = child.keys[0]

    def _borrow_from_right_leaf(
        self,
        parent: BPlusInternalNode,
        child_idx: int,
    ) -> None:
        """Rotate: move the leftmost key from right sibling to child.

        The separator between child and right sibling is updated.

        Leaf rotate left:
            child.keys.append(right.keys.pop(0))
            parent.keys[child_idx] ← right.keys[0]  (new minimum of right)
        """
        child: BPlusLeafNode = parent.children[child_idx]  # type: ignore
        right_sib: BPlusLeafNode = parent.children[child_idx + 1]  # type: ignore

        child.keys.append(right_sib.keys.pop(0))
        child.values.append(right_sib.values.pop(0))
        parent.keys[child_idx] = right_sib.keys[0]

    def _merge_leaves(
        self,
        parent: BPlusInternalNode,
        left_idx: int,
    ) -> None:
        """Merge parent.children[left_idx] with parent.children[left_idx+1].

        Unlike B-tree merge (which pulls the separator DOWN into the merged node),
        B+ tree leaf merge simply concatenates the two leaves.  The separator in
        the parent is discarded (it was a routing copy, not real data).

        After merging:
          - left.keys = left.keys + right.keys
          - left.next = right.next
          - parent loses keys[left_idx] and children[left_idx+1]
        """
        left: BPlusLeafNode = parent.children[left_idx]  # type: ignore
        right: BPlusLeafNode = parent.children[left_idx + 1]  # type: ignore

        left.keys.extend(right.keys)
        left.values.extend(right.values)
        left.next = right.next  # maintain linked list

        parent.keys.pop(left_idx)
        parent.children.pop(left_idx + 1)

    def _borrow_from_left_internal(
        self,
        parent: BPlusInternalNode,
        child_idx: int,
    ) -> None:
        """Rotate: internal node borrows one key from its left sibling.

        The parent separator key at keys[child_idx-1] comes DOWN into child,
        and the last key of the left sibling goes UP to the parent.

        This is identical to the B-tree Case 3a rotation.
        """
        left_sib: BPlusInternalNode = parent.children[child_idx - 1]  # type: ignore
        child: BPlusInternalNode = parent.children[child_idx]  # type: ignore

        # Parent key comes down to front of child
        child.keys.insert(0, parent.keys[child_idx - 1])
        # Last child of left sibling becomes first child of child
        child.children.insert(0, left_sib.children.pop())
        # Last key of left sibling goes up to parent
        parent.keys[child_idx - 1] = left_sib.keys.pop()

    def _borrow_from_right_internal(
        self,
        parent: BPlusInternalNode,
        child_idx: int,
    ) -> None:
        """Rotate: internal node borrows one key from its right sibling."""
        child: BPlusInternalNode = parent.children[child_idx]  # type: ignore
        right_sib: BPlusInternalNode = parent.children[child_idx + 1]  # type: ignore

        # Parent key comes down to end of child
        child.keys.append(parent.keys[child_idx])
        # First child of right sibling moves to end of child
        child.children.append(right_sib.children.pop(0))
        # First key of right sibling goes up to parent
        parent.keys[child_idx] = right_sib.keys.pop(0)

    def _merge_internals(
        self,
        parent: BPlusInternalNode,
        left_idx: int,
    ) -> None:
        """Merge two internal nodes, pulling the parent separator key down.

        This mirrors the B-tree Case 2c/3b merge exactly.
        """
        left: BPlusInternalNode = parent.children[left_idx]  # type: ignore
        right: BPlusInternalNode = parent.children[left_idx + 1]  # type: ignore

        sep_key = parent.keys.pop(left_idx)
        parent.children.pop(left_idx + 1)

        left.keys.append(sep_key)
        left.keys.extend(right.keys)
        left.children.extend(right.children)

    def _delete_recursive(
        self,
        node: BPlusInternalNode | BPlusLeafNode,
        key: Any,
        parent: BPlusInternalNode | None,
        child_idx: int,
    ) -> bool:
        """Recursively delete key from the subtree rooted at node.

        Returns True if deleted, False if not found.

        parent and child_idx are used to rebalance after deletion.
        """
        t = self._t

        if isinstance(node, BPlusLeafNode):
            # ---- BASE CASE: find and delete from leaf ----
            i = node.find_key_index(key)
            if i >= len(node.keys) or node.keys[i] != key:
                return False  # key not in tree

            node.keys.pop(i)
            node.values.pop(i)

            # Rebalance if this leaf is now too thin (and it's not the root)
            if parent is not None and len(node.keys) < t - 1:
                self._rebalance_leaf(parent, child_idx)

            return True

        else:
            # ---- RECURSIVE CASE ----
            assert isinstance(node, BPlusInternalNode)
            i = node.find_child_index(key)
            deleted = self._delete_recursive(node.children[i], key, node, i)

            if deleted and parent is not None and len(node.keys) < t - 1:
                self._rebalance_internal(parent, child_idx)

            return deleted

    def _rebalance_leaf(
        self,
        parent: BPlusInternalNode,
        child_idx: int,
    ) -> None:
        """Restore the leaf at parent.children[child_idx] to minimum size.

        Tries (in order):
          1. Borrow from left sibling (if it has ≥ t keys)
          2. Borrow from right sibling (if it has ≥ t keys)
          3. Merge with left sibling
          4. Merge with right sibling
        """
        t = self._t

        # Try left borrow
        if child_idx > 0:
            left_sib: BPlusLeafNode = parent.children[child_idx - 1]  # type: ignore
            if len(left_sib.keys) >= t:
                self._borrow_from_left_leaf(parent, child_idx)
                return

        # Try right borrow
        if child_idx < len(parent.children) - 1:
            right_sib: BPlusLeafNode = parent.children[child_idx + 1]  # type: ignore
            if len(right_sib.keys) >= t:
                self._borrow_from_right_leaf(parent, child_idx)
                return

        # Merge
        if child_idx > 0:
            self._merge_leaves(parent, child_idx - 1)
        else:
            self._merge_leaves(parent, child_idx)

    def _rebalance_internal(
        self,
        parent: BPlusInternalNode,
        child_idx: int,
    ) -> None:
        """Restore the internal node at parent.children[child_idx]."""
        t = self._t

        if child_idx > 0:
            left_sib = parent.children[child_idx - 1]
            assert isinstance(left_sib, BPlusInternalNode)
            if len(left_sib.keys) >= t:
                self._borrow_from_left_internal(parent, child_idx)
                return

        if child_idx < len(parent.children) - 1:
            right_sib = parent.children[child_idx + 1]
            assert isinstance(right_sib, BPlusInternalNode)
            if len(right_sib.keys) >= t:
                self._borrow_from_right_internal(parent, child_idx)
                return

        if child_idx > 0:
            self._merge_internals(parent, child_idx - 1)
        else:
            self._merge_internals(parent, child_idx)

    # -----------------------------------------------------------------------
    # Validation helpers
    # -----------------------------------------------------------------------

    def _validate(
        self,
        node: BPlusInternalNode | BPlusLeafNode,
        min_key: Any,
        max_key: Any,
        depth: int,
        expected_leaf_depth: list[int],
        is_root: bool,
    ) -> bool:
        """Recursively check all B+ tree structural invariants."""
        t = self._t

        if isinstance(node, BPlusLeafNode):
            n = len(node.keys)

            # Key count bounds
            if not is_root and n < t - 1:
                return False
            if n > 2 * t - 1:
                return False

            # Keys sorted and within bounds
            for j, k in enumerate(node.keys):
                if min_key is not None and k < min_key:
                    return False
                if max_key is not None and k >= max_key:
                    return False
                if j > 0 and node.keys[j] <= node.keys[j - 1]:
                    return False

            # Leaf depth consistency
            if expected_leaf_depth[0] == -1:
                expected_leaf_depth[0] = depth
            elif expected_leaf_depth[0] != depth:
                return False

            return True

        else:
            assert isinstance(node, BPlusInternalNode)
            n = len(node.keys)

            # Key count bounds (root can have 1 key; others need >= t-1)
            if not is_root and n < t - 1:
                return False
            if n > 2 * t - 1:
                return False
            if len(node.children) != n + 1:
                return False

            # Keys sorted and within bounds
            for j, k in enumerate(node.keys):
                if min_key is not None and k < min_key:
                    return False
                if max_key is not None and k >= max_key:
                    return False
                if j > 0 and node.keys[j] <= node.keys[j - 1]:
                    return False

            # Recurse into children
            for j, child in enumerate(node.children):
                lo = node.keys[j - 1] if j > 0 else min_key
                hi = node.keys[j] if j < n else max_key
                if not self._validate(child, lo, hi, depth + 1, expected_leaf_depth, False):
                    return False

            return True

    def _validate_leaf_list(self) -> bool:
        """Verify the leaf linked list is sorted and contains all keys."""
        prev_key = None
        node = self._first_leaf
        count = 0

        while node is not None:
            for k in node.keys:
                count += 1
                if prev_key is not None and k <= prev_key:
                    return False  # not strictly increasing across leaves
                prev_key = k
            node = node.next

        return count == self._size

    # -----------------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------------

    def insert(self, key: Any, value: Any) -> None:
        """Insert key with associated value into the B+ tree.

        If the key already exists, its value is updated in place.
        O(t · log_t n) time.

        Example::

            tree = BPlusTree(t=2)
            tree.insert(10, "ten")
            tree.insert(5,  "five")
            tree.search(10)  # → "ten"
        """
        is_new, split_key, split_node = self._insert_recursive(
            self._root, key, value
        )

        if split_key is not None:
            # Root split — create a new root
            new_root = BPlusInternalNode()
            new_root.keys = [split_key]
            new_root.children = [self._root, split_node]
            self._root = new_root
            # first_leaf is unchanged (still the leftmost leaf)

        if is_new:
            self._size += 1

    def delete(self, key: Any) -> None:
        """Remove key from the B+ tree.

        Raises KeyError if the key is not found.
        O(t · log_t n) time.

        Example::

            tree = BPlusTree(t=2)
            tree.insert(5, "five")
            tree.delete(5)
            5 in tree  # → False
        """
        leaf, _ = self._find_leaf(key)
        i = leaf.find_key_index(key)
        if i >= len(leaf.keys) or leaf.keys[i] != key:
            raise KeyError(key)

        deleted = self._delete_recursive(self._root, key, None, 0)

        if not deleted:
            raise KeyError(key)  # shouldn't happen given check above

        self._size -= 1

        # If root is an internal node with no keys but one child, shrink
        if isinstance(self._root, BPlusInternalNode) and not self._root.keys:
            self._root = self._root.children[0]

        # Update first_leaf if necessary (the leftmost leaf may have changed)
        # Walk down the leftmost spine to find the new first leaf
        node: BPlusInternalNode | BPlusLeafNode = self._root
        while isinstance(node, BPlusInternalNode):
            node = node.children[0]
        self._first_leaf = node  # type: ignore[assignment]

    def search(self, key: Any) -> Any | None:
        """Return the value for key, or None if not found.

        O(t · log_t n) time.

        Example::

            tree = BPlusTree(t=2)
            tree.insert(7, "seven")
            tree.search(7)   # → "seven"
            tree.search(99)  # → None
        """
        leaf, _ = self._find_leaf(key)
        i = leaf.find_key_index(key)
        if i < len(leaf.keys) and leaf.keys[i] == key:
            return leaf.values[i]
        return None

    def __contains__(self, key: Any) -> bool:
        """Support ``key in tree`` syntax."""
        leaf, _ = self._find_leaf(key)
        i = leaf.find_key_index(key)
        return i < len(leaf.keys) and leaf.keys[i] == key

    def __getitem__(self, key: Any) -> Any:
        """Return tree[key]. Raises KeyError if not found."""
        leaf, _ = self._find_leaf(key)
        i = leaf.find_key_index(key)
        if i < len(leaf.keys) and leaf.keys[i] == key:
            return leaf.values[i]
        raise KeyError(key)

    def __setitem__(self, key: Any, value: Any) -> None:
        """Support tree[key] = value (insert or update)."""
        self.insert(key, value)

    def __delitem__(self, key: Any) -> None:
        """Support del tree[key]. Raises KeyError if not found."""
        self.delete(key)

    def __len__(self) -> int:
        """Return the number of (key, value) pairs. O(1)."""
        return self._size

    def __bool__(self) -> bool:
        """Return True if the tree is non-empty."""
        return self._size > 0

    def __iter__(self) -> Iterator[Any]:
        """Yield keys in ascending sorted order by walking the leaf list.

        This is an O(n) full scan — no tree traversal needed!

        Example::

            tree = BPlusTree(t=2)
            for k in [3, 1, 2]:
                tree.insert(k, k)
            list(tree)  # → [1, 2, 3]
        """
        node = self._first_leaf
        while node is not None:
            yield from node.keys
            node = node.next

    def items(self) -> Iterator[tuple[Any, Any]]:
        """Yield (key, value) pairs in ascending key order via leaf list.

        O(n) time — no tree traversal needed.

        Example::

            tree = BPlusTree(t=2)
            tree.insert(1, "a")
            tree.insert(2, "b")
            list(tree.items())  # → [(1, "a"), (2, "b")]
        """
        node = self._first_leaf
        while node is not None:
            yield from zip(node.keys, node.values)
            node = node.next

    def full_scan(self) -> Iterator[tuple[Any, Any]]:
        """Yield all (key, value) pairs in sorted order.

        Identical to items() — provided for naming clarity.
        O(n) via the leaf linked list.

        This is the operation that makes B+ trees so valuable for databases:
        a full table scan needs only to walk the leaf layer, never touching
        the internal index nodes.
        """
        return self.items()

    def range_scan(self, low: Any, high: Any) -> list[tuple[Any, Any]]:
        """Return all (key, value) pairs where low <= key <= high, sorted.

        O(log_t n + k) time where k is the number of results.

        Algorithm:
          1. Descend to the leaf where `low` would live: O(log_t n)
          2. Walk the leaf linked list forward, collecting results: O(k)
          3. Stop when key > high.

        Example::

            tree = BPlusTree(t=2)
            for k, v in [(1,"a"), (3,"c"), (5,"e"), (7,"g"), (9,"i")]:
                tree.insert(k, v)
            tree.range_scan(3, 7)
            # → [(3, "c"), (5, "e"), (7, "g")]
        """
        result = []
        leaf, _ = self._find_leaf(low)

        # Start scanning from the first key in the leaf that is >= low
        i = leaf.find_key_index(low)

        node: BPlusLeafNode | None = leaf
        while node is not None:
            for j in range(i, len(node.keys)):
                k = node.keys[j]
                if k > high:
                    return result
                result.append((k, node.values[j]))
            node = node.next
            i = 0  # reset index for subsequent leaves

        return result

    def min_key(self) -> Any:
        """Return the smallest key in the tree.

        Raises ValueError if the tree is empty.
        O(1) via first_leaf pointer.
        """
        if not self._size:
            raise ValueError("Tree is empty")
        return self._first_leaf.keys[0]

    def max_key(self) -> Any:
        """Return the largest key in the tree.

        Raises ValueError if the tree is empty.
        O(log_t n) — walk the rightmost spine.
        """
        if not self._size:
            raise ValueError("Tree is empty")
        node: BPlusInternalNode | BPlusLeafNode = self._root
        while isinstance(node, BPlusInternalNode):
            node = node.children[-1]
        return node.keys[-1]  # type: ignore[union-attr]

    def height(self) -> int:
        """Return the height of the tree (0 = leaf-only tree).

        O(log_t n) — follows the leftmost spine.
        """
        node: BPlusInternalNode | BPlusLeafNode = self._root
        h = 0
        while isinstance(node, BPlusInternalNode):
            node = node.children[0]
            h += 1
        return h

    def is_valid(self) -> bool:
        """Check all B+ tree structural invariants.

        Returns True if valid, False otherwise.

        Checks:
          1. Key count bounds for internal and leaf nodes
          2. Keys strictly increasing within each node
          3. BST ordering property between parent separators and children
          4. All leaves at the same depth
          5. Child count = key count + 1 for internal nodes
          6. Leaf linked list is sorted and contains exactly size keys
        """
        if self._size == 0:
            return True
        expected_leaf_depth: list[int] = [-1]
        if not self._validate(
            self._root,
            min_key=None,
            max_key=None,
            depth=0,
            expected_leaf_depth=expected_leaf_depth,
            is_root=True,
        ):
            return False
        return self._validate_leaf_list()

    def __repr__(self) -> str:
        """Human-readable summary."""
        return f"BPlusTree(t={self._t}, size={self._size}, height={self.height()})"
