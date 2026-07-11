# Changelog

All notable changes to `fenwick-tree` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `fenwick-tree` crate: `fenwick_init`,
  `fenwick_init_from_slice`, `fenwick_update`, `fenwick_prefix_sum`,
  `fenwick_range_sum`, `fenwick_point_query`, `fenwick_find_kth`, `fenwick_len`,
  `fenwick_is_empty`. Status-code error handling; 1-based indexing matching the
  crate.
- Tests (via the shared `iso-harness`) covering prefix/range/point queries,
  updates, the `find_kth` cumulative search, and all error paths — compiled and
  run under GCC, Clang, and MSVC with strict ISO-conformance flags.
