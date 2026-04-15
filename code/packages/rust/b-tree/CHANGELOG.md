# Changelog — coding-adventures-b-tree

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-11

### Added

- Full from-scratch B-tree implementation (DT11) replacing the earlier
  `std::collections::BTreeMap` stub.
- `BTree<K, V>` with configurable minimum degree `t` (clamped to ≥ 2).
- `insert` — proactive top-down splitting: full nodes are split before
  descending into them, so insertion never requires backtracking.  Root
  splits are handled as a special case that increases tree height by one.
- `delete` — complete CLRS deletion algorithm covering all sub-cases:
  - Case 1: key in leaf with enough keys — direct removal.
  - Case 2a: key in internal node, left child has ≥ t keys — replace with
    predecessor.
  - Case 2b: key in internal node, right child has ≥ t keys — replace with
    successor.
  - Case 2c: both children at minimum — merge, then delete from merged node.
  - Case 3: key not in current node — pre-fill via rotate-left, rotate-right,
    or merge before descending.
- `search`, `contains` — O(log n) key lookup.
- `min_key`, `max_key` — O(height) extremum queries.
- `range_query` — prune subtrees outside the query range for efficiency.
- `inorder` — full sorted traversal.
- `len`, `is_empty`, `height` — O(1) / O(height) metadata.
- `is_valid` — structural invariant checker: leaf depth uniformity, key count
  bounds, key ordering, child count.
- Knuth-style literate doc comments throughout, including ASCII diagrams of
  node structure and split/merge operations.
- 21 unit tests covering: empty tree, single insert, duplicates, delete from
  empty, missing keys, all delete sub-cases (1, 2a, 2b, 2c, 3), range
  queries, inorder ordering, min/max, `is_valid` after every operation, and
  large-scale tests with 10 000 keys at t=2 and t=5.
