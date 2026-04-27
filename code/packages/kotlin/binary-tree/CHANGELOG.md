# Changelog — binary-tree (Kotlin)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a generic binary tree with traversal and shape helpers.
- `class BinaryTree<T>` — generic class with public `inner class Node`.
- `fromLevelOrder(List<T?>)` — companion object factory. Builds from a level-order
  list; null entries represent absent nodes.
- `find(value)` — O(n) pre-order search; returns the matching `Node?` or null.
- `leftChild(value)` / `rightChild(value)` — convenience wrappers over `find`.
- `isFull()` — true iff every node has 0 or 2 children.
- `isComplete()` — true iff all levels filled left-to-right except the last.
  Uses null-sentinel BFS with `LinkedList` (Kotlin `ArrayDeque` forbids nulls).
- `isPerfect()` — true iff `size == 2^(h+1) - 1`.
- `inorder()` / `preorder()` / `postorder()` — O(n) DFS traversals.
- `levelOrder()` — O(n) BFS traversal.
- `toArray()` — O(n) level-order array projection with null for absent nodes.
- `toAscii()` — ASCII tree renderer with box-drawing connectors.
- `val height: Int` computed via `height()` method.
- `val size: Int` — O(n) recursive count.
- `val isEmpty: Boolean` — O(1).
- Literate source with traversal diagrams, shape predicate explanations, and
  Kotlin idiom commentary.
- 42 unit tests covering all operations, edge cases, string values, and
  manual tree construction.
