# Changelog — @coding-adventures/b-tree

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `BTree<K, V>` — a generic B-tree with minimum degree `t`.
- `BTreeNode<K, V>` interface with `keys`, `values`, `children`, and `isLeaf` fields.
- `insert(key, value)` — proactive top-down splitting (CLRS §18.3), O(t·log_t n).
- `delete(key)` — full CLRS deletion with cases 1, 2a, 2b, 2c, 3a, 3b, O(t·log_t n).
- `search(key)` — returns associated value or `undefined`, O(t·log_t n).
- `contains(key)` — boolean existence check.
- `minKey()` / `maxKey()` — O(h) leftmost/rightmost traversal.
- `rangeQuery(low, high)` — returns sorted `[K, V]` pairs in range, O(t·log_t n + m).
- `inorder()` — returns all pairs sorted, O(n).
- `height()` — follows leftmost path, O(h).
- `isValid()` — checks all five B-tree invariants plus size counter, O(n).
- Upsert semantics: inserting a duplicate key updates its value.
- Knuth-style literate programming comments with ASCII diagrams throughout.
- Vitest test suite with 95%+ coverage covering all delete cases, t=2/3/5, 5000+ key tests.
