# Changelog

All notable changes to `segment-tree` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `segment-tree` crate:
  `ca::segment_tree<T>` with a `std::function` combine + identity, `sum_tree`/
  `min_tree`/`max_tree` factories, inclusive-range `query`, point `update`,
  `size`, `empty`.
- Tests (via the shared `iso-harness`) covering sum/min/max factories, a custom
  gcd combine, queries, updates, and safe empty / out-of-range behavior —
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
