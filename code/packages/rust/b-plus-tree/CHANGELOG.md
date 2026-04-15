# Changelog — coding-adventures-b-plus-tree

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-11

### Added

- Full from-scratch B+ tree implementation (DT12) replacing the earlier
  `std::collections::BTreeMap` stub.
- Enum-based node representation: `BPlusNode::Internal` and `BPlusNode::Leaf`.
- `BPlusLeaf` with a `*mut BPlusLeaf` raw pointer forming a sorted singly-
  linked list across all leaves.  Safety invariant documented: all leaves are
  owned by the tree through `root`; raw pointers are only dereferenced behind
  `&self` / `&mut self`.
- `BPlusTree<K, V>` with configurable minimum degree `t` and `first_leaf`
  pointer for O(1) full-scan entry.
- `insert` — recursive top-down insertion with leaf splitting (separator key
  is *copied* to parent and retained in the right leaf) and internal node
  splitting when needed.  Root splits handled by promoting a new root.
- `delete` — recursive deletion with post-order rebalancing: borrow from left
  sibling, borrow from right sibling, or merge (for both leaf and internal
  nodes).  Leaf linked list pointers updated on merge.
- `search`, `contains` — O(log n) lookup that finds the leaf then binary-
  searches within it.
- `range_scan` — walks the leaf linked list from the start point for O(log n
  + k) range queries.
- `full_scan` — walks the entire leaf list in O(n), ideal for sequential
  access patterns.
- `min_key`, `max_key` — O(height) extremum via leftmost/rightmost leaf.
- `len`, `is_empty`, `height` — O(1) / O(height) metadata.
- `is_valid` — structural validator checking: node key counts, key ordering,
  child counts, leaf depth uniformity, leaf linked list sorted order, and
  `size` counter consistency.
- `iter()` — zero-copy reference iterator walking the leaf linked list.
- `IntoIterator` — consuming iterator (materialises all entries then yields
  them in order).
- Knuth-style literate doc comments with ASCII diagrams comparing B-tree and
  B+ tree layouts, node memory layout, and the leaf linked list structure.
- 24 unit tests covering: empty tree, single insert, duplicate update, delete
  from empty, missing key delete, single-key delete, leaf linked list sorted
  invariant after inserts and after deletes, leaf list each-key-exactly-once
  after mixed operations, full scan, range scan (basic, empty result, full
  range), iterator and IntoIterator, t=2/3/5, min/max after ops, is_valid
  after every operation, and 10 000-key scale tests at t=2 and t=5.
