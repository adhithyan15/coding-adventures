"""B+ Tree: all data in leaves with a sorted linked list for full-table scans.

A B+ tree is a variant of the B-tree where:
  - Internal nodes store ONLY separator keys (no associated values).
  - ALL data (key, value pairs) lives in the leaf nodes.
  - Leaf nodes are linked together in a sorted doubly-linked list (here: singly),
    enabling O(n) full scans without touching internal nodes at all.

This design is used in virtually every relational database index because:
  1. Full table scans are O(n) — just walk the leaf list.
  2. Range queries are O(log n + k) — find the start leaf, walk forward.
  3. Internal nodes hold more keys (no values) → shallower trees → fewer I/Os.

Example usage::

    from b_plus_tree import BPlusTree

    t = BPlusTree(t=3)
    t.insert(10, "ten")
    t.insert(20, "twenty")
    t.insert(5,  "five")

    t.search(10)                  # → "ten"
    10 in t                       # → True
    t[20]                         # → "twenty"
    t.min_key()                   # → 5
    t.max_key()                   # → 20
    t.range_scan(5, 15)           # → [(5, "five"), (10, "ten")]
    list(t.full_scan())           # → [(5, "five"), (10, "ten"), (20, "twenty")]
    list(t)                       # → [5, 10, 20]   (keys only)
    list(t.items())               # → [(5, "five"), (10, "ten"), (20, "twenty")]
    t.height()                    # → 1
    t.is_valid()                  # → True
    t.delete(10)
    10 in t                       # → False
"""

from b_plus_tree.b_plus_tree import BPlusInternalNode, BPlusLeafNode, BPlusTree

__all__ = ["BPlusTree", "BPlusInternalNode", "BPlusLeafNode"]
