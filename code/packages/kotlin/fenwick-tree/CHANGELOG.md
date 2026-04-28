# Changelog — fenwick-tree (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a Fenwick Tree (Binary Indexed Tree) for O(log n)
  prefix sums and point updates over a 1-indexed `LongArray`.
- `FenwickTree(n: Int)` — constructs an all-zero tree of capacity `n`.
- `FenwickTree(values: LongArray)` — O(n) construction from an existing array via
  the "copy then propagate to parent" technique.
- `update(i: Int, delta: Long)` — O(log n). Walks upward adding `delta` to all
  cells covering position `i` via `i += i and -i`.
- `prefixSum(i: Int): Long` — O(log n). Walks downward summing cells via
  `i -= i and -i`.
- `rangeSum(l: Int, r: Int): Long` — O(log n). Returns `prefixSum(r) - prefixSum(l-1)`.
- `capacity: Int` — Kotlin property returning the tree's maximum 1-based index.
- Bounds checking on all public methods; throws `IllegalArgumentException`.
- Literate source with bit-trick diagrams, worked examples, and complexity table.
- 28 unit tests covering construction (empty and from array), point updates,
  prefix sums, range sums, edge cases, bounds validation, negative deltas,
  and a 1000-element smoke test.
