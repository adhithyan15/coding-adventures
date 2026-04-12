# Changelog

All notable changes to `go/b-plus-tree` are documented here.

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of a generic B+ tree (`BPlusTree[K, V]`) — DT12.
- Minimum degree `t` configurable at construction; any `t ≥ 2` is valid.
- Key ordering via user-supplied `less func(K, K) bool`.
- **Operations**: `Insert`, `Delete`, `Search`, `Contains`, `MinKey`, `MaxKey`,
  `RangeScan`, `FullScan`, `Len`, `Height`, `IsValid`.
- Leaf linked list (`firstLeaf` → `next` chain):
  - `firstLeaf` maintained at all times for O(1) start of `FullScan`.
  - `RangeScan(low, high)` uses the linked list for O(log n + k) range scan.
  - `FullScan()` walks the full chain in O(n).
- Correct split semantics:
  - Leaf split: separator is COPIED into parent AND kept in right leaf.
  - Internal split: median is MOVED to parent (not kept in children).
- Deletion with all fill sub-cases: rotate-right, rotate-left, and merge,
  for both leaf nodes and internal nodes.
- `IsValid()` validates all B+ tree invariants:
  - Uniform leaf depth.
  - Key count bounds per node.
  - Sorted keys within each node.
  - Correct child count in internal nodes.
  - Leaf linked list order consistency.
  - `firstLeaf` pointer correctness.
  - `size` counter accuracy.
- Literate programming style: ASCII diagrams, invariant tables, and
  step-by-step explanations throughout the source.
- Test suite with 95%+ coverage:
  - Leaf linked list integrity checked after every operation.
  - `RangeScan` correctness verified against linear scan.
  - All delete sub-cases tested individually.
  - Random insert/delete stress test with `t ∈ {2, 3, 5}`.
  - Bulk test with 10,000+ keys.
