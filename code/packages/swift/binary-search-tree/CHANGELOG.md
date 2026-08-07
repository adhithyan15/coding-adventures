# Changelog — BinarySearchTree (Swift)

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Swift port of the `binary-search-tree` reference package.
- Immutable `BST<Element>` — `insert` / `delete` return a new tree; `search`,
  `contains`, `minValue` / `maxValue`, `predecessor` / `successor`,
  `kthSmallest`, `rank`, `toSortedArray`, `isValid`, `height`, `count`,
  `isEmpty`, a `fromSorted` balanced builder, and a description.
- Backed by an `indirect enum` (value type) with cached subtree sizes for O(h)
  order-statistic queries; unchanged subtrees are shared structurally between
  versions.
- 11 XCTest cases: reference insert/search/delete/rank/kthSmallest example with
  immutability check, balanced `fromSorted`, empty-tree behaviour, duplicate
  no-op, in-order output, all delete node-shapes + absent-value no-op, repeated-
  delete validity/sortedness, predecessor/successor (present/absent/edge),
  full-range kthSmallest/rank, String elements, and description.
