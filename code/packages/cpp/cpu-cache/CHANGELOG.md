# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `cpu-cache` crate in namespace
  `ca::cpu_cache`: a configurable multi-level CPU cache hierarchy simulator
  (L1I / L1D / L2 / L3 / main memory).
- Full model — `CacheLine`, N-way set-associative `CacheSet` with true-LRU
  replacement (dirty victim via `std::optional<CacheLine>`), a configurable
  `Cache` level with power-of-two address decomposition, write-allocate +
  write-back / write-through policies, `CacheStats`, and an inclusive
  `CacheHierarchy` (levels held as `std::optional<Cache>`) with per-level
  latency accounting.
- `CacheConfig::create` validates and throws `std::invalid_argument` where the
  Rust panics; `with_write_policy` is the builder. Address `log2` is an exact
  integer bit-count, so the header needs no `<cmath>`. Verified clean under
  ASan + UBSan.
- 202 checks mirroring the crate's unit tests across all five modules run under
  every ISO C++ compiler via the shared `iso-harness`.
