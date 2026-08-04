# Changelog

All notable changes to `segment-tree` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `segment-tree` crate: `segment_tree_init`
  (with a caller-supplied combine function + identity), `_init_sum`/`_init_min`/
  `_init_max` convenience builders, `segment_tree_query` (inclusive range),
  `segment_tree_update` (point), `segment_tree_len`, `segment_tree_is_empty`.
- Tests (via the shared `iso-harness`) covering sum/min/max trees, range
  queries, point updates, and safe empty / out-of-range behavior — compiled and
  run under GCC, Clang, and MSVC with strict ISO-conformance flags.
