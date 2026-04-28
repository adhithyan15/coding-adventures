# Changelog — binary-tree (Java)

## [0.1.0] — 2026-04-25

### Added
- Initial implementation of a generic binary tree with traversal and shape helpers.
- `BinaryTree<T>` — generic class with public `BinaryTreeNode<T>` inner class.
- `fromLevelOrder(List<T>)` — builds the tree from a level-order (BFS) list;
  null entries represent absent nodes. Uses heap-array index arithmetic.
- `find(value)` — O(n) pre-order search; returns the matching node or null.
- `leftChild(value)` / `rightChild(value)` — convenience wrappers over `find`.
- `isFull()` — true iff every node has 0 or 2 children.
- `isComplete()` — true iff all levels are filled left-to-right except the last.
  Uses a null-sentinel BFS with `LinkedList` (ArrayDeque forbids null elements).
- `isPerfect()` — true iff all leaves are at the same depth (size == 2^(h+1) - 1).
- `inorder()` / `preorder()` / `postorder()` — O(n) recursive DFS traversals.
- `levelOrder()` — O(n) iterative BFS traversal via `ArrayDeque`.
- `toArray()` — O(n) level-order array projection with null for absent nodes.
- `toAscii()` — ASCII tree renderer with box-drawing connectors.
- `height()` — O(n). Empty tree → -1.
- `size()` — O(n) recursive count.
- `isEmpty()` — O(1).
- Literate source with traversal diagrams, shape predicate explanations, and
  algorithm commentary.
- 42 unit tests covering all operations, edge cases, string values, and
  manual tree construction.
