# Changelog — binary-search-tree (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a mutable Binary Search Tree with size-augmented
  nodes for O(log n) order-statistics operations.
- `class BinarySearchTree<T : Comparable<T>>` — generic mutable BST.
- `insert(value)` — O(log n). Ignores duplicates.
- `delete(value)` — O(log n). Uses in-order successor for two-child nodes.
- `search(value)` — O(log n). Returns the matching `Node` or `null`.
- `contains(value)` — O(log n). Boolean membership test.
- `minValue()` / `maxValue()` — O(log n). Returns `T?`.
- `predecessor(value)` / `successor(value)` — O(log n). Returns `T?`.
- `kthSmallest(k)` — O(log n). 1-indexed k-th smallest via size augmentation.
- `rank(value)` — O(log n). Count of elements strictly less than value.
- `toSortedList()` — O(n). In-order traversal returns ascending list.
- `isValid()` — O(n). Validates BST property and size invariant.
- `height()` — O(n). Tree height (-1 for empty).
- `val size: Int` — O(1) via cached root size.
- `val isEmpty: Boolean` — O(1).
- Companion object `fromSortedList(list)` — O(n) balanced BST factory.
- Literate source with algorithm commentary and complexity annotations.
- 41 unit tests covering all operations, edge cases, string keys, and a
  1000-element stress test.

### Design note
The Python implementation is functional/persistent (each mutation returns a
new tree). This Kotlin implementation is mutable (mutations modify in place)
to match JVM idioms and avoid the allocation overhead of persistent trees.
