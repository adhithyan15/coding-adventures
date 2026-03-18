# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- `CacheLine`: Cache line data structure with valid/dirty bits, tag, LRU tracking
- `CacheSet`: Set-associative lookup with LRU replacement policy
- `CacheConfig`: Frozen configuration dataclass with validation (power-of-2 sizes, valid policies)
- `Cache`: Single configurable cache level with address decomposition, read/write, invalidation
- `CacheHierarchy`: Multi-level composition (L1I/L1D/L2/L3) with inclusive fill policy
- `CacheStats`: Hit rate, miss rate, eviction, and writeback tracking
- `HierarchyAccess` and `CacheAccess`: Detailed access records for debugging
- Write-back and write-through policies
- Write-allocate on write miss
- Harvard architecture support (separate L1I and L1D)
- Full test suite with 90%+ coverage
