# Changelog — swift/b-tree

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of `BTree<K, V>` (DT11) in Swift.
- Proactive top-down splitting on insert (single-pass, no backtracking).
- Full deletion with all sub-cases (1, 2a, 2b, 2c, 3a, 3b, 3c).
- `search`, `contains`, `insert`, `delete`, `minKey`, `maxKey`.
- `rangeQuery(from:to:)` and `inorder()` traversal.
- `count`, `height`, and `isValid()` for structural inspection.
- Comprehensive test suite: 19 test cases covering all deletion cases,
  t=2/3/5, 500+ keys, string keys, reverse-order inserts, and re-inserts.
- Literate programming style with ASCII diagrams and detailed comments.
