# Changelog — coding-adventures-b-tree

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of the `BTree` class (DT11)
- `BTreeNode` dataclass with `keys`, `values`, `children`, `is_leaf` fields
- Proactive top-down splitting (CLRS algorithm) — no backtracking on insert
- Full delete implementation: Case 1 (leaf), Case 2a/2b/2c (internal),
  Case 3a/3b (pre-fill before descent)
- Binary search at each node via `bisect_left` for O(log t) key lookup
- Early termination when key found in internal node (unlike B+ tree)
- `search(key)`, `insert(key, value)`, `delete(key)` public methods
- Python protocol support: `__contains__`, `__getitem__`, `__setitem__`,
  `__delitem__`, `__len__`, `__bool__`
- `min_key()`, `max_key()` — O(log_t n) via leftmost/rightmost spine walk
- `range_query(low, high)` — O(log_t n + k) via inorder generator
- `inorder()` generator — yields (key, value) pairs in sorted order
- `height()` — O(log_t n), returns 0 for leaf-only tree
- `is_valid()` — checks all structural invariants (key counts, key ordering,
  BST property, uniform leaf depth)
- Comprehensive test suite with >80% coverage:
  - Tests for all three delete cases and sub-cases
  - Tests with t=2, t=3, t=5
  - 1000-key stress tests for correctness at scale
  - Interleaved insert/delete test with is_valid() after every operation
