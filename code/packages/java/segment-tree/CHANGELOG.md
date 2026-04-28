# Changelog — java/segment-tree

## [0.1.0] — 2026-04-25

### Added
- `SegmentTree<T>` class — generic, array-backed, 1-indexed
- Constructor `SegmentTree(T[] array, BinaryOperator<T> combine, T identity)`
- `query(int ql, int qr): T` — range aggregate in O(log n)
- `update(int index, T value): void` — point update in O(log n)
- `toList(): List<T>` — reconstruct array from leaves in O(n)
- `size(): int`, `isEmpty(): boolean` — metadata
- Static factories: `sumTree(int[])`, `minTree(int[])`, `maxTree(int[])`, `gcdTree(int[])`
- Private `gcd(int, int)` using Euclidean algorithm
- Literate comments: array-backed storage diagram, three-case query algorithm,
  O(log n) complexity analysis, monoid requirement table
- 40 unit tests covering: empty tree, single element, sum/min/max/GCD trees,
  brute-force correctness for all query ranges, point updates, toList,
  edge cases (negative values, non-power-of-2 sizes, mixed signs),
  exception paths (invalid ranges and indices), multiple updates,
  random stress test (200 elements, 200 updates), large array (100k elements),
  custom combines (product, bitwise OR), parameterized sizes 1–20
