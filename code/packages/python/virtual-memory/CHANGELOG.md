# Changelog

All notable changes to the virtual-memory package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `PageTableEntry` dataclass with frame number and permission flags (present, dirty, accessed, writable, executable, user_accessible)
- `PageTable` single-level page table with VPN-to-PTE dictionary mapping
- `TwoLevelPageTable` implementing RISC-V Sv32 scheme (10-bit L1 + 10-bit L2 + 12-bit offset)
- `TLB` translation lookaside buffer with LRU eviction and hit/miss statistics
- `PhysicalFrameAllocator` bitmap-based frame allocator with sequential allocation and free/reuse
- `FIFOPolicy` page replacement (evict oldest page)
- `LRUPolicy` page replacement (evict least recently used)
- `ClockPolicy` page replacement (second chance with use bits)
- `MMU` memory management unit with per-process page tables, TLB integration, page fault handling, copy-on-write cloning, and context switching
- Comprehensive test suite targeting 90%+ coverage
