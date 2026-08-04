# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `red-black-tree` crate (DT09), in
  namespace `ca::rb`: a templated left-leaning red-black (LLRB) tree with a
  cached subtree size for O(log n) order statistics.
- Persistent API via value semantics: `insert` / `erase` are `const` and return
  a new tree; `RBTree<T>` deep-copies on copy (`erase` replaces the keyword
  `delete`).
- Queries return `std::optional<T>` where a lookup may miss: `find`, `contains`,
  `min_value`, `max_value`, `predecessor`, `successor`, `kth_smallest`,
  `to_sorted_array`, `size`, `black_height`, `root`, `is_valid_rb`.
- Built on `std::unique_ptr` children (mirroring Rust's `Option<Box<Node>>`);
  the same rotate / fix_up / move-red / delete-min LLRB algorithms. `T` requires
  only less-than comparability and copyability.
- Tests replicate the crate's unit tests and add per-step delete verification,
  neighbour queries, value-semantics independence, and a 0..199 ascending
  insert/delete stress under GCC and Clang via `iso-harness`.
