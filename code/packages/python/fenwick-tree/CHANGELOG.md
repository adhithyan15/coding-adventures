# Changelog

## [0.1.0] — 2026-04-08

### Added

- `FenwickTree` class implementing the Binary Indexed Tree (Fenwick Tree) algorithm
- `FenwickTree(n)` constructor for an empty tree of size n
- `FenwickTree.from_list(values)` class method for O(n) construction from a list
- `update(i, delta)` for O(log n) point updates
- `prefix_sum(i)` for O(log n) prefix sum queries (positions 1..i)
- `range_sum(l, r)` for O(log n) range sum queries using two prefix sums
- `point_query(i)` for O(log n) single-element queries
- `find_kth(k)` for O(log n) order statistics via binary lifting
- `__len__` dunder for O(1) size queries
- `__repr__` dunder showing the internal BIT array for debugging
- `FenwickError` base exception class
- `IndexOutOfRangeError` for out-of-bounds index access
- `EmptyTreeError` for operations on an empty tree
- Full type annotations on all public methods
- 95%+ test coverage with brute-force verification, stress tests, and edge cases
- Literate programming style inline comments explaining every algorithm
