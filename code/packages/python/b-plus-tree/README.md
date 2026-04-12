# coding-adventures-b-plus-tree

**DT12** — B+ Tree: all data in leaves with a sorted linked list for range scans.

## What is a B+ Tree?

A B+ tree is a variant of the B-tree where ALL key-value data lives in the leaf
nodes and internal nodes contain only routing (separator) keys.  The leaves are
linked together in a sorted linked list, enabling linear-time full scans and
efficient range queries.

This is the data structure used by virtually every relational database index:
PostgreSQL, MySQL InnoDB, SQLite, Oracle DB.  When you run `SELECT * FROM orders
WHERE date BETWEEN '2024-01-01' AND '2024-12-31'`, the database uses a B+ tree
index to jump to the first matching row and walk the leaf list forward.

## Key differences from B-Tree (DT11)

| Property | B-Tree | B+ Tree |
|----------|--------|---------|
| Data location | Every node | Leaves only |
| Internal nodes | Keys + values | Keys only (routing) |
| Leaf linked list | No | Yes (`next` pointer) |
| Full scan | O(n) tree traversal | O(n) leaf list walk |
| Range scan | O(log n + k) | O(log n + k) |
| Leaf split separator | MOVES to parent | COPIED to parent (stays in leaf) |
| Search termination | May stop at internal | Always reaches leaf |

## Architecture in this repository

```
Layer DT (Data Structures — Trees)
  DT09  Fenwick Tree
  DT10  Skip List
  DT11  B-Tree
  DT12  B+ Tree        ← this package
  DT13  LSM Tree
```

## Node types

**`BPlusInternalNode`** — routing-only:
```
keys:     [10, 20, 30]
children: [c0, c1, c2, c3]
          all in c0 < 10 ≤ c1 < 20 ≤ c2 < 30 ≤ c3
```

**`BPlusLeafNode`** — data-bearing:
```
keys:   [10, 15, 18]
values: ["ten", "fifteen", "eighteen"]
next:   → (next leaf in sorted order)
```

## Usage

```python
from b_plus_tree import BPlusTree

tree = BPlusTree(t=3)

# Insert key-value pairs
tree.insert(10, "ten")
tree.insert(5,  "five")
tree.insert(20, "twenty")

# Search
tree.search(10)          # → "ten"
tree[5]                  # → "five"
10 in tree               # → True

# Min / Max (min is O(1) via first_leaf pointer)
tree.min_key()           # → 5
tree.max_key()           # → 20

# Range scan (O(log n + k) via descent + leaf walk)
tree.range_scan(5, 15)   # → [(5, "five"), (10, "ten")]

# Full scan (O(n) via leaf linked list — no tree traversal)
list(tree.full_scan())
# → [(5, "five"), (10, "ten"), (20, "twenty")]

# Iterate keys only
list(tree)               # → [5, 10, 20]

# Iterate (key, value) pairs
list(tree.items())       # → [(5, "five"), (10, "ten"), (20, "twenty")]

# Structure
tree.height()            # → 0 or 1
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
| min_key   | O(1) via first_leaf |
| max_key   | O(log_t n) |
| range_scan | O(log_t n + k) |
| full_scan / items / iter | O(n) |
| height    | O(log_t n) |
| is_valid  | O(n) |

## See Also

- [DT11 B-Tree](../b-tree/) — stores data at every level (no leaf linked list)
