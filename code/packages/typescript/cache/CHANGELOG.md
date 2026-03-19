# Changelog

## 0.1.0 — 2026-03-19

### Added
- `CacheLine` — the smallest unit of cached data, with fill/touch/invalidate operations
- `CacheConfig` — immutable configuration for a cache level (size, associativity, latency, write policy) with validation
- `CacheSet` — set-associative lookup with LRU replacement policy
- `Cache` — a single configurable cache level with address decomposition, read/write, and statistics
- `CacheHierarchy` — multi-level cache system (L1I + L1D + L2 + L3 + main memory) with inclusive fill policy
- `CacheStats` — hit rate, miss rate, eviction, and writeback tracking
- `HierarchyAccess` and `CacheAccess` interfaces for access result records
- Full test suite covering all modules (stats, cache-line, cache-set, cache, hierarchy)
- Knuth-style literate programming comments throughout all source files
- TypeScript port from the Python implementation with identical behavior
