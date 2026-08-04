# Changelog

All notable changes to `skip-list` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `skip-list` crate (an ordered map):
  `skiplist_init`(`_with_params`), `insert`, `delete`, `search`, `contains`,
  `rank`, `by_rank`, `min`, `max`, `len`, `is_empty`, `foreach`, `range`, and the
  reported `max_level`/`current_max`/`probability`. Sorted-array backed;
  `current_max` computed libm-free.
- Tests (via the shared `iso-harness`) covering insert/overwrite/search/delete,
  order statistics, min/max, ordered enumeration, and inclusive/exclusive range
  queries — compiled and run under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
