# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `red-black-tree` crate (DT09): a left-leaning
  red-black (LLRB) tree with a cached subtree size for O(log n) order
  statistics.
- Persistent API mirroring the Rust crate: `rb_insert` / `rb_delete` return a
  new tree (deep copy) and leave the input untouched. `rb_empty` / `rb_free`
  manage ownership; updates return `NULL` only on allocation failure.
- Queries: `rb_search`, `rb_contains`, `rb_min_value`, `rb_max_value`,
  `rb_predecessor`, `rb_successor`, `rb_kth_smallest`, `rb_to_sorted_array`,
  `rb_size`, `rb_black_height`, `rb_is_valid_rb`.
- Same rotate / fix_up / move-red / delete-min LLRB algorithms as the crate,
  including the colour-preserving rotations; recursion depth bounded by tree
  height (balanced ⇒ O(log n)).
- Tests replicate the crate's unit tests and add per-step delete verification,
  neighbour queries, persistence, and a 0..199 ascending insert/delete stress
  under GCC and Clang via `iso-harness`.
