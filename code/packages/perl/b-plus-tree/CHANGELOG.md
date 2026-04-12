# Changelog — perl/b-plus-tree

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of `CodingAdventures::BPlusTree` (DT12) in Perl.
- Leaf nodes store all (key, value) pairs; internal nodes are routing indexes only.
- Leaf linked list (`next` pointers) for O(log n + k) range scans.
- Proactive top-down splitting on insert (single-pass, no backtracking).
- Leaf split: first key of right half is COPIED up (stays in leaf); B-Tree
  split moves the median — this is the key structural difference.
- Full deletion with borrow-from-left, borrow-from-right, and merge, with
  separator updates that correctly maintain the B+ Tree invariant.
- `search`, `insert`, `delete`, `size`, `height`, `min_key`, `max_key`.
- `range_scan`, `full_scan`, `inorder` (alias for `full_scan`).
- `is_valid` for structural verification + `_is_linked_list_valid` for
  linked-list count and sort-order integrity.
- Comprehensive test suite: 16 sub-tests including linked-list integrity,
  cross-leaf range scans, all deletion sub-cases, t=2/3/5, 500+ keys,
  reverse-order inserts, and re-inserts.
- `is_valid` and linked-list check after every mutation.
- Literate programming style with detailed comments and ASCII diagrams.
