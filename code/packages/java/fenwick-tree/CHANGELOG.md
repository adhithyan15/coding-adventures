# Changelog — fenwick-tree (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a Fenwick Tree (Binary Indexed Tree) for O(log n)
  prefix sums and point updates over a 1-indexed `long[]` array.
- `FenwickTree(int n)` — constructs an all-zero tree of capacity `n`.
- `FenwickTree(long[] values)` — O(n) construction from an existing array via
  the "copy then propagate to parent" technique.
- `update(i, delta)` — O(log n). Walks upward adding `delta` to all cells
  covering position `i` via `i += i & -i`.
- `prefixSum(i)` — O(log n). Walks downward summing cells via `i -= i & -i`.
- `rangeSum(l, r)` — O(log n). Returns `prefixSum(r) - prefixSum(l-1)`.
- `capacity()` — returns the tree's maximum 1-based index.
- Bounds checking on all public methods; throws `IllegalArgumentException`.
- Literate source with bit-trick diagrams, worked examples, and complexity table.
- 28 unit tests covering construction (empty and from array), point updates,
  prefix sums, range sums, edge cases, bounds validation, negative deltas,
  and a 1000-element smoke test.
