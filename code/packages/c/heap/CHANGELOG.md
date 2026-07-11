# Changelog

All notable changes to `heap` (C) are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the Rust `heap` crate: an int binary heap with
  `HEAP_MIN`/`HEAP_MAX` ordering (`heap_init`, `heap_push`, `heap_pop`,
  `heap_peek`, `heap_len`, `heap_is_empty`, `heap_free`) plus an ascending
  in-place `heap_sort`.
- Tests (via the shared `iso-harness`) covering min/max ordering, push/pop/peek,
  empty behavior, and heap_sort with duplicates/negatives — compiled and run
  under GCC, Clang, and MSVC with strict ISO-conformance flags.
