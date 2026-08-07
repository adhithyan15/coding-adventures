# Changelog

All notable changes to `fenwick-tree` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `fenwick-tree` crate:
  `ca::fenwick_tree` with `update`, `prefix_sum`, `range_sum`, `point_query`,
  `find_kth`, `size`, `empty`. `std::vector<double>` storage, 1-based indexing,
  throwing error paths (`std::out_of_range` / `std::invalid_argument`).
- Tests (via the shared `iso-harness`) covering queries, updates, `find_kth`,
  and the throwing error paths — compiled and run under GCC, Clang, and MSVC
  with strict ISO-conformance flags.
