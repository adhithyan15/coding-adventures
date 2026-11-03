# Changelog — swift/b-plus-tree

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of `BPlusTree<K, V>` (DT12) in Swift.
- Internal nodes hold only routing keys; all values live in leaf nodes.
- Leaf nodes linked as a doubly-ended linked list via `next` pointers.
- `firstLeaf` pointer for O(1) start of full scan.
- `search`, `contains`, `insert`, `delete`, `minKey`, `maxKey`.
- `rangeScan(from:to:)` using the leaf linked list — O(log n + k).
- `fullScan()` walking the leaf linked list — O(n).
- `inorder()` (alias for fullScan).
- `count`, `height`, and `isValid()` including linked-list integrity check.
- Comprehensive test suite: 16 test cases covering all operations,
  linked-list integrity, t=2/3/5, 500+ keys, string keys, and re-inserts.
- Literate programming style with ASCII diagrams and detailed comments.
