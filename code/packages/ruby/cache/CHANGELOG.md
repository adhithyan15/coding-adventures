# Changelog

## [0.1.0] - 2026-03-18

### Added
- `CacheLine` — single cache line with valid/dirty/tag/LRU tracking
- `CacheSet` — set-associative lookup with LRU replacement policy
- `CacheConfig` — immutable configuration (Data.define) for cache parameters
- `CacheSimulator` — configurable single-level cache with address decomposition
- `CacheHierarchy` — multi-level hierarchy (L1I/L1D/L2/L3 + main memory)
- `CacheStats` — hit rate, miss rate, eviction, and writeback tracking
- `CacheAccess` and `HierarchyAccess` — immutable access records (Data.define)
- Write-back and write-through write policies
- Write-allocate on miss
- Inclusive fill policy (data fills back up through all levels)
- Harvard architecture support (separate L1I and L1D)
- Full test suite with Minitest and SimpleCov (80%+ coverage)
- Knuth-style literate comments throughout
