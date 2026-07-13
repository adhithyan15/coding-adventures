# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `cpu-cache` crate: a configurable multi-level
  CPU cache hierarchy simulator (L1I / L1D / L2 / L3 / main memory).
- Full model — cache line (valid/dirty/tag/data/LRU), N-way set-associative set
  with true-LRU replacement and dirty-victim reporting, a configurable cache
  level with power-of-two address decomposition, write-allocate + write-back /
  write-through policies, hit/miss/eviction/writeback statistics, and an
  inclusive multi-level hierarchy with per-level latency accounting.
- `ca_cache_config_new` (validating), `ca_cache_init`/`_free`, `ca_cache_read`/
  `_write`/`_invalidate`/`_fill_line`/`_decompose`, the cache-line, cache-set,
  and stats sub-APIs, and `ca_cache_hierarchy_*` (read/write/invalidate_all/
  reset_stats). Address `log2` is computed as an exact integer bit-count, so the
  port needs no `<math.h>`.
- Allocations guard `size_t` overflow (checked `calloc` multiplies; the config's
  `line_size * associativity` product is overflow-checked). Verified clean under
  ASan + UBSan, the macOS `leaks` tool (0 leaks), and a random-config/
  random-access fuzz sweep.
- Documented divergence: `CaCacheAccess` records an evicted victim's metadata
  inline rather than cloning its data bytes (no consumer ever reads them);
  behavior is identical.
- 258 checks mirroring the crate's unit tests across all five modules run under
  every ISO C compiler via the shared `iso-harness`.
