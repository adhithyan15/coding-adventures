# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `PageTableEntry` class with all RISC-V Sv32 flags (present, dirty, accessed, writable, executable, user_accessible) and clone support.
- `PageTable` single-level page table (hash map of VPN to PTE) with map, unmap, lookup, and iteration.
- `TwoLevelPageTable` implementing RISC-V Sv32 with 10-bit L1 + 10-bit L2 + 12-bit offset, on-demand second-level table allocation, and full translate/map/unmap/lookup support.
- `TLB` translation lookaside buffer with configurable capacity (default 64), LRU eviction, hit/miss counters, flush, and per-entry invalidation.
- `PhysicalFrameAllocator` bitmap-based allocator with allocate, free, is_allocated, and free_count. Double-free detection.
- `ReplacementPolicy` interface with three implementations:
  - `FIFOPolicy`: first-in, first-out eviction queue.
  - `LRUPolicy`: least recently used eviction using ordered map.
  - `ClockPolicy`: second-chance clock algorithm with use bits and sweeping hand.
- `MMU` central coordinator with:
  - Per-process page tables (create, destroy, clone address spaces).
  - TLB integration (automatic caching and flushing on context switch).
  - Page fault handling with demand paging.
  - Copy-on-write (COW) support for fork() with reference counting.
  - Page eviction when physical memory is full.
  - Context switch with TLB flush.
- Comprehensive test suite covering all components with 90%+ target coverage.
- Knuth-style literate programming with inline explanations, diagrams, and analogies.
