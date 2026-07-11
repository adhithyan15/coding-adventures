# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `binary-search-tree` crate (DT07).
- Persistent (deep-copy) `bst_insert` / `bst_delete` returning new trees.
- `bst_empty`, `bst_from_sorted_array` (balanced build), `bst_free`.
- Queries: `bst_search`, `bst_contains`, `bst_min_value`, `bst_max_value`,
  `bst_predecessor`, `bst_successor`, `bst_kth_smallest` (1-based order
  statistic), `bst_rank`, `bst_to_sorted_array`, `bst_size`, `bst_height`,
  `bst_is_valid`.
- Per-node cached subtree size for `O(h)` rank / select.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC), mirroring the Rust
  crate's own vectors.
