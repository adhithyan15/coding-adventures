# Changelog

## [0.1.0] - 2026-03-18

### Added
- `CacheLine` — single cache line with valid/dirty/tag/LRU tracking
- `CacheSet` — set-associative lookup with LRU replacement policy
- `CacheConfig` — validated configuration for cache parameters
- `Cache` — configurable single-level cache with address decomposition
- `CacheHierarchy` — multi-level hierarchy (L1I/L1D/L2/L3 + main memory)
- `CacheStats` — hit rate, miss rate, eviction, and writeback tracking
- `CacheAccess` and `HierarchyAccess` — access record structs
- Write-back and write-through write policies
- Write-allocate on miss
- Inclusive fill policy (data fills back up through all levels)
- Harvard architecture support (separate L1I and L1D)
- Comprehensive test suite with 80%+ coverage
