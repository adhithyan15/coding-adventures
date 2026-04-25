# Changelog — binary-search-tree (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a mutable Binary Search Tree with size-augmented
  nodes for O(log n) order-statistics operations.
- `BinarySearchTree<T extends Comparable<T>>` — generic mutable BST.
- `insert(value)` — O(log n). Ignores duplicates.
- `delete(value)` — O(log n). Uses in-order successor for two-child nodes.
- `search(value)` — O(log n). Returns the matching `Node<T>` or `null`.
- `contains(value)` — O(log n). Boolean membership test.
- `minValue()` / `maxValue()` — O(log n). Returns `Optional<T>`.
- `predecessor(value)` / `successor(value)` — O(log n). Largest/smallest
  value strictly less/greater than the given value. Returns `Optional<T>`.
- `kthSmallest(k)` — O(log n). 1-indexed k-th smallest via size augmentation.
- `rank(value)` — O(log n). Count of elements strictly less than value.
- `toSortedList()` — O(n). In-order traversal yields ascending order.
- `isValid()` — O(n). Validates BST property and size invariant throughout.
- `height()` — O(n). Tree height (-1 for empty).
- `size()` / `isEmpty()` — O(1) via the root's cached size field.
- `fromSortedList(list)` — O(n) static factory that builds a balanced BST.
- Literate source with bit-layout diagrams, algorithm commentary, and
  complexity annotations.
- 41 unit tests covering all operations, edge cases, string keys, and a
  1000-element stress test.

### Design note
The Python implementation is functional/persistent (each mutation returns a
new tree). This Java implementation is mutable (mutations modify in place)
to match Java idioms and avoid the allocation overhead of persistent trees.
