# coding-adventures-b-tree

**DT11** — B-Tree: self-balancing multi-way search tree.

## What is a B-Tree?

A B-tree is a generalisation of a binary search tree in which each node can
hold many keys at once.  Invented in 1970 by Rudolf Bayer and Ed McCreight at
Boeing Research Labs, the B-tree was designed to minimise disk I/O.  Because a
hard disk reads one block at a time, the bigger the block the better.  A B-tree
node is sized to fit in one disk block, so a tree of height 4 with 500 keys per
node can index 500^4 = 62.5 billion records in just 4 disk reads per lookup.

SQLite, PostgreSQL, MySQL InnoDB, NTFS, HFS+, ext4, and virtually every
filesystem and database engine in existence use B-trees or their descendants.

## Architecture in this repository

```
Layer DT (Data Structures — Trees)
  DT09  Fenwick Tree
  DT10  Skip List
  DT11  B-Tree          ← this package
  DT12  B+ Tree
  DT13  LSM Tree
```

## How it works

A B-tree is parameterised by its **minimum degree** `t` (≥ 2):

| Parameter | Minimum | Maximum |
|-----------|---------|---------|
| Keys per node (non-root) | t - 1 | 2t - 1 |
| Children per node | t | 2t |
| Keys in root | 1 | 2t - 1 |

All leaves are at the same depth — this is the fundamental invariant that
guarantees O(log_t n) worst-case performance.

**Insert** uses proactive top-down splitting: full nodes are split on the way
DOWN the tree, so no backtracking is ever needed.

**Delete** handles three cases:
- **Case 1**: Key in a leaf → remove directly
- **Case 2a/b/c**: Key in an internal node → use predecessor/successor or merge
- **Case 3a/b**: Child to descend into is thin → rotate from sibling or merge

## Usage

```python
from b_tree import BTree

# Create a tree with minimum degree 3
tree = BTree(t=3)

# Insert key-value pairs
tree.insert(10, "ten")
tree.insert(5,  "five")
tree.insert(20, "twenty")

# Search
tree.search(10)          # → "ten"
tree[5]                  # → "five"
10 in tree               # → True

# Update
tree[10] = "TEN"
tree.search(10)          # → "TEN"

# Min / Max
tree.min_key()           # → 5
tree.max_key()           # → 20

# Range query
tree.range_query(5, 15)  # → [(5, "five"), (10, "TEN")]

# In-order iteration
list(tree.inorder())
# → [(5, "five"), (10, "TEN"), (20, "twenty")]

# Structure info
tree.height()            # → 0 or 1 (depends on how many keys)
tree.is_valid()          # → True

# Delete
del tree[5]
5 in tree                # → False
```

## Installation

```bash
uv pip install -e .
```

## Running Tests

```bash
uv run python -m pytest tests/ -v
```

## Time Complexity

| Operation | Time |
|-----------|------|
| search    | O(t · log_t n) |
| insert    | O(t · log_t n) |
| delete    | O(t · log_t n) |
| min / max | O(log_t n) |
| range_query | O(t · log_t n + k) |
| inorder   | O(n) |
| height    | O(log_t n) |
| is_valid  | O(n) |

## See Also

- [DT12 B+ Tree](../b-plus-tree/) — stores all data in leaves with a sorted linked list of leaves for efficient full scans
