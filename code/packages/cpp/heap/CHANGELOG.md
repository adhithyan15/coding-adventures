# Changelog

All notable changes to `heap` (C++) are documented in this file.

## [0.2.0] - Unreleased

### Added

- Added bounded min/max priority queues with checked capacity, one-pass range
  construction, explicit move/ownership and comparator-exception guarantees,
  and an installable relocatable CMake package.
- Added independent model histories, ownership/error-path tests, comparison
  bounds, and a relocated installed-consumer smoke.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the Rust `heap` crate:
  `ca::binary_heap<T, Compare>` with `ca::min_heap<T>` / `ca::max_heap<T>`
  aliases (`push`, `pop`, `peek` returning `std::optional`, `size`, `empty`,
  O(n) heapify construction) plus free functions `heap_sort`, `nlargest`,
  `nsmallest`.
- Tests (via the shared `iso-harness`) covering min/max heaps, push/pop/peek,
  heapify, heap_sort, and nlargest/nsmallest — compiled and run under GCC,
  Clang, and MSVC with strict ISO-conformance flags.
