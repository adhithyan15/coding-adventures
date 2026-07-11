# Changelog

All notable changes to `skip-list` (C++) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `skip-list` crate:
  `ca::skip_list<K, V>` (std::map-backed ordered map) with `insert`, `erase`,
  `search`/`contains` (`std::optional`), `rank`/`by_rank`, `min`/`max`, `range`,
  `entries`, `size`/`empty`, and reported `max_level`/`current_max`/`probability`.
- Tests (via the shared `iso-harness`) covering insert/overwrite/search/erase,
  order statistics, min/max, ordered entries, and inclusive/exclusive range
  queries — compiled and run under GCC, Clang, and MSVC with strict
  ISO-conformance flags.
