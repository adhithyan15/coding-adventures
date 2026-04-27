# Changelog — kotlin/red-black-tree

## [0.1.0] — 2026-04-25

### Added
- `Color` enum with `toggle()` extension function
- `Node` data class — immutable, uses `copy()` for functional updates
- `Node?.isRed()` null-safe extension function
- `RBTree` class — purely functional LLRB tree
- `RBTree.empty()` factory
- LLRB helpers in companion object: `rotateLeft`, `rotateRight`, `flipColors`,
  `fixUp`, `moveRedLeft`, `moveRedRight`, `deleteMin`, `deleteHelper`
- `insert(Int)` — LLRB bottom-up via fixUp, root always forced BLACK
- `delete(Int)` — LLRB deletion with moveRedLeft/moveRedRight
- `contains(Int)` — iterative BST search, O(log n)
- `min` / `max` — nullable Int computed properties
- `predecessor(Int)` / `successor(Int)` — return nullable Int
- `kthSmallest(Int)` — 1-indexed via in-order traversal
- `toSortedList()` — in-order traversal returning sorted list
- `isValidRB()` — verifies all 5 Red-Black invariants
- `blackHeight` / `size` / `height` / `isEmpty` — computed properties
- 42 unit tests mirroring the Java test suite
