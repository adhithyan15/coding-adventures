# Changelog — kotlin/avl-tree

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-25

### Added

- `AVLTree<T : Comparable<T>>` — mutable, size-augmented AVL tree
- `insert(T)` — O(log n) insert with four-case rebalancing; duplicate is no-op
- `delete(T)` — O(log n) delete using in-order successor for two-child nodes;
  throws `NoSuchElementException` if absent
- `contains(T)` — O(log n) iterative membership test
- `min()` / `max()` — O(log n) leftmost/rightmost walk; throw on empty tree
- `predecessor(T)` / `successor(T)` — O(log n) floor/ceiling search
- `kthSmallest(Int)` — O(log n) order statistic using size augmentation (1-based)
- `rank(T)` — O(log n) count of elements strictly less than value (0-based)
- `toSortedList()` — O(n) in-order traversal
- `val height: Int` — O(1) cached height; -1 for empty tree
- `val size: Int` / `val isEmpty: Boolean` — O(1) cardinality helpers
- `val balanceFactor: Int` — balance factor of root node
- `isValid()` — validates all 4 AVL invariants: BST ordering, |BF| ≤ 1, correct
  cached height, correct cached size
- `isValidBST()` — validates BST ordering only
- `toString()` — summary of size and height
- `inner class Node` — `inner` allows access to outer type parameter `T`; holds
  `value`, `left`, `right`, `height`, `size`
- `rotateLeft()` / `rotateRight()` helpers
- `rebalance()` — dispatch to the four rotation cases
- `update()` — recalculates height and size from children
- `validateAVL()` — recursive validation returning `IntArray(height, size)` or null
- 34 unit tests covering all operations and a 1000-operation stress test
