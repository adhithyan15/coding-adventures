# Changelog — java/red-black-tree

## [0.1.0] — 2026-04-25

### Added
- `RBTree` — purely functional Red-Black tree using Java 21 records
- `Color` enum — RED / BLACK
- `Node` record — immutable node with value, color, left, right
- Okasaki's 4-case balance function for O(log n) insertion
- Sedgewick LLRB deletion with `moveRedLeft`, `moveRedRight`, `fixUp`
- `insert(int)` — returns new tree with value added
- `delete(int)` — returns new tree with value removed (LLRB approach)
- `contains(int)` — O(log n) membership test
- `min()` / `max()` — Optional-returning minimum/maximum
- `predecessor(int)` / `successor(int)` — Optional floor/ceiling
- `kthSmallest(int)` — 1-indexed order statistic via in-order traversal
- `toSortedList()` — in-order traversal returning all elements sorted
- `isValidRB()` — verifies all 5 Red-Black invariants
- `blackHeight()` — black-height of the root
- `size()`, `height()`, `isEmpty()` — structural metrics
- 42 unit tests covering: empty tree, single element, CLRS sequence,
  ascending/descending/alternating inserts, height bound, predecessor/successor,
  kthSmallest, delete (leaf, internal, root, all elements), round-trip,
  immutability, random stress (200 inserts, 100 deletes), parameterized
  height-bound checks
