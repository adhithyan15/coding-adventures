# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `PageTableEntry` with RISC-V Sv32 permission bits (present, dirty, accessed, writable, executable, user_accessible)
- `PageTable` (single-level) with hash map from VPN to PTE
- `TwoLevelPageTable` implementing RISC-V Sv32 addressing (10-bit L1 + 10-bit L2 + 12-bit offset)
- `TLB` with LRU eviction, configurable capacity (default 64), hit/miss counters
- `FrameAllocator` with bitmap allocation, reference counting for copy-on-write
- `FIFOPolicy` page replacement (evict oldest page)
- `LRUPolicy` page replacement (evict least recently used)
- `ClockPolicy` page replacement (second-chance algorithm with use bits)
- `MMU` tying all components together with:
  - Per-process address spaces
  - Address translation with TLB caching
  - Page fault handling with demand paging
  - Copy-on-write fork via `clone_address_space`
  - Context switching with TLB flush
- Comprehensive test suite covering all components
- Literate programming style with inline explanations and diagrams
