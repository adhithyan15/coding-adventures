# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `PageTableEntry` with RISC-V Sv32 permission bits
- `PageTable` (single-level) with HashMap from VPN to PTE
- `TwoLevelPageTable` implementing Sv32 addressing (10+10+12 bit split)
- `TLB` with LRU eviction, configurable capacity, hit/miss counters
- `PhysicalFrameAllocator` with bitmap allocation and reference counting
- `ReplacementPolicy` trait with FIFO, LRU, and Clock implementations
- `MMU` with per-process page tables, TLB, page fault handling, COW fork, context switching
- Comprehensive inline test modules for all components
- Literate programming style with doc comments explaining concepts
