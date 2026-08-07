# Changelog

All notable changes to `static-vector` are documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only `ca::static_vector<T, N>` (CCPP01 PR5): a
  fixed-capacity, no-allocation vector with `push_back`/`pop_back`, unchecked
  `operator[]`, checked `at()` (throws `std::out_of_range`), `size`/`capacity`,
  `empty`/`full`, iterators (range-for), and `clear`.
- Tests (via the shared `iso-harness`) covering fill-to-capacity, overflow
  rejection, checked-access throwing, range-for, and reuse after `pop_back` —
  compiled and run under GCC, Clang, and MSVC with strict ISO-conformance flags.
