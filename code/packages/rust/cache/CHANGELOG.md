# Changelog

## 0.1.0 (2026-03-18)

### Added
- `CacheLine` struct with valid/dirty bits, tag, data, LRU timestamp, fill/touch/invalidate operations
- `CacheSet` with LRU replacement policy, lookup, access, and allocate methods
- `CacheConfig` with validation (power-of-2 constraints, divisibility checks)
- `WritePolicy` enum (WriteBack, WriteThrough)
- `Cache` struct with address decomposition, read/write paths, fill_line, and invalidation
- `CacheAccess` record for per-access diagnostics (hit/miss, cycles, evicted line)
- `CacheHierarchy` supporting L1I + L1D + L2 + L3 + main memory with inclusive fill policy
- `HierarchyAccess` record tracking which level served data and total latency
- `CacheStats` with hit rate, miss rate, eviction, and writeback tracking
- Comprehensive test suite covering address decomposition, LRU eviction, dirty writebacks, write policies, hierarchy walk, inclusive fill, and access patterns
- Knuth-style doc comments throughout explaining cache architecture concepts
