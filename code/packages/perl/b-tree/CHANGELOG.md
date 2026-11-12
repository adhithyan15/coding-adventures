# Changelog — perl/b-tree

## [Unreleased]

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

## [0.1.0] — 2026-04-11

### Added
- Initial implementation of `CodingAdventures::BTree` (DT11) in Perl.
- Proactive top-down splitting on insert (single-pass, no backtracking).
- Full deletion with all CLRS sub-cases (A, B1, B2, B3, C with rotate and merge).
- `search`, `insert`, `delete`, `size`, `height`, `min_key`, `max_key`.
- `inorder` and `range_query` traversal methods.
- `is_valid` for structural verification.
- Comprehensive test suite: 15 sub-tests covering all deletion cases,
  t=2/3/5, 500+ keys, reverse-order inserts, and re-inserts.
- `is_valid` checked after every mutation.
- Literate programming style with detailed comments.
