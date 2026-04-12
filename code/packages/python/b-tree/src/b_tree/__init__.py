"""B-Tree: a self-balancing search tree for disk-based storage systems.

A B-tree generalises the binary search tree by allowing nodes to hold many
keys at once.  This "fat node" design was invented in 1970 by Rudolf Bayer
and Ed McCreight while working at Boeing specifically because hard-disk seek
times dwarf in-memory computation time.  If you can store 1 000 keys in one
disk block, a tree of height 4 can index a BILLION records — and you only need
4 disk reads per lookup.

Example usage::

    from b_tree import BTree

    t = BTree(t=3)          # minimum degree 3 → up to 5 keys per node
    t.insert(10, "ten")
    t.insert(20, "twenty")
    t.insert(5,  "five")

    t.search(10)            # → "ten"
    10 in t                 # → True
    t[20]                   # → "twenty"
    t.min_key()             # → 5
    t.max_key()             # → 20
    t.range_query(5, 15)    # → [(5, "five"), (10, "ten")]
    list(t.inorder())       # → [(5, "five"), (10, "ten"), (20, "twenty")]
    t.height()              # → 1
    t.is_valid()            # → True
    t.delete(10)
    10 in t                 # → False
"""

from b_tree.b_tree import BTree, BTreeNode

__all__ = ["BTree", "BTreeNode"]
