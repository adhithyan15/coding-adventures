# Changelog — @coding-adventures/b-plus-tree

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `BPlusTree<K, V>` — a generic B+ tree with minimum degree `t`.
- `BPlusInternalNode<K, V>` — routing-only nodes holding separator keys and child pointers.
- `BPlusLeafNode<K, V>` — data nodes with key-value pairs and a `next` linked-list pointer.
- `firstLeaf` field for O(1) full-scan start.
- `insert(key, value)` — proactive top-down splitting; leaf splits COPY the separator to parent while keeping it in the right leaf. O(t·log_t n).
- `delete(key)` — borrow-from-sibling and merge strategy, updating separator keys in ancestors. O(t·log_t n).
- `search(key)` — descends to leaf, O(t·log_t n).
- `contains(key)` — boolean existence check.
- `minKey()` — O(1) via `firstLeaf`.
- `maxKey()` — O(h) via rightmost path.
- `rangeScan(low, high)` — descends to first leaf then follows `next` pointers. O(log_t n + m).
- `fullScan()` — O(n) sequential scan via leaf linked list, no tree traversal needed.
- `height()` — O(h).
- `isValid()` — checks all B+ tree invariants including linked list integrity and `firstLeaf`. O(n).
- `[Symbol.iterator]()` — enables `for...of` and spread operator, iterates via leaf list.
- Upsert semantics: inserting a duplicate key updates its value.
- Knuth-style literate programming comments with ASCII diagrams explaining leaf split vs internal split.
- Vitest test suite with 95%+ coverage: leaf linked list integrity, rangeScan, fullScan, iterator, all delete cases, t=2/3/5, 5000+ key tests.
