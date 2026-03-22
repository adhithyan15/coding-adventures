# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `PageTableEntry` struct with all RISC-V Sv32 flags (present, dirty, accessed, writable, executable, user_accessible).
- `PageTable` single-level page table (map of VPN to PTE) with map_page, unmap_page, lookup, mapped_count, get_all_vpns, and insert.
- `TwoLevelPageTable` implementing RISC-V Sv32 with 10-bit L1 + 10-bit L2 + 12-bit offset, on-demand second-level table allocation, translate, map, unmap, lookup_pte, update_pte, and all_mappings.
- `TLB` translation lookaside buffer with configurable capacity, LRU eviction, hit/miss counters, flush, and per-entry invalidation.
- `PhysicalFrameAllocator` using MapSet for allocation tracking with allocate, free, is_allocated, and free_count. Double-free detection.
- Three page replacement policies:
  - `FIFOPolicy` using Erlang :queue for O(1) enqueue/dequeue.
  - `LRUPolicy` using ordered list with move-to-end on access.
  - `ClockPolicy` with use bits and sweeping hand.
- `MMU` central coordinator with:
  - Per-process page tables (create, destroy, clone address spaces).
  - TLB integration (automatic caching and flushing on context switch).
  - Page fault handling with demand paging.
  - Copy-on-write (COW) support for fork() with reference counting.
  - Page eviction when physical memory is full.
- All state is immutable — functions return new structs following Elixir conventions.
- Comprehensive ExUnit test suite covering all components.
- Knuth-style literate programming with inline explanations and diagrams.
