# Changelog

All notable changes to the virtual-memory package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `PageTableEntry` struct with frame number and permission flags
- `PageTable` single-level page table with VPN-to-PTE map
- `TwoLevelPageTable` implementing RISC-V Sv32 scheme (10+10+12 split)
- `TLB` translation lookaside buffer with LRU eviction and hit/miss stats
- `PhysicalFrameAllocator` bitmap-based frame allocator
- `ReplacementPolicy` interface with FIFO, LRU, and Clock implementations
- `MMU` memory management unit with per-process page tables, TLB, page faults, COW cloning, and context switching
- Comprehensive test suite with 80%+ coverage
