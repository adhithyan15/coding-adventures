# Changelog — coding-adventures-b-plus-tree

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of the `BPlusTree` class (DT12)
- Two distinct node types:
  - `BPlusInternalNode`: routing-only nodes with separator keys and children,
    but NO values — internal nodes are a pure index
  - `BPlusLeafNode`: data-bearing nodes with keys, values, and a `next`
    pointer forming a singly-linked sorted leaf list
- `first_leaf` pointer on the tree for O(1) access to the leaf list start
- Leaf split rule: smallest key of right leaf is COPIED into parent (stays in
  leaf too), unlike B-tree where the median MOVES out of the node
- Internal node split rule mirrors B-tree: median MOVES up, does not stay
- Bottom-up insertion with recursive splitting (cleaner for B+ trees than
  the top-down approach used in BTree)
- Full delete with leaf and internal rebalancing:
  - Borrow from left/right sibling for both leaf and internal nodes
  - Merge leaves (no separator pulldown — B+ tree leaf merge is pure concat)
  - Merge internal nodes (pull separator down from parent like B-tree)
- `search(key)`, `insert(key, value)`, `delete(key)` public methods
- Python protocol: `__contains__`, `__getitem__`, `__setitem__`, `__delitem__`,
  `__len__`, `__bool__`, `__iter__` (yields keys via leaf list)
- `items()` / `full_scan()` — yield (key, value) pairs via leaf list in O(n)
- `range_scan(low, high)` — O(log_t n + k) via descent + leaf list walk
- `min_key()` — O(1) via first_leaf pointer
- `max_key()` — O(log_t n) via rightmost spine
- `height()` — O(log_t n)
- `is_valid()` — checks key bounds, sorted order, uniform leaf depth, child
  count, and leaf linked list integrity
- Comprehensive test suite with >80% coverage:
  - Leaf linked list integrity tests (tortoise-and-hare cycle detection)
  - range_scan tests across many boundary conditions
  - Explicit tests for leaf separator COPY behavior
  - Tests with t=2, t=3, t=5
  - 1000-key stress tests for correctness at scale
  - Interleaved insert/delete with is_valid() after every operation
