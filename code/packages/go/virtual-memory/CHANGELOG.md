# Changelog

All notable changes to the virtual-memory package will be documented in this file.

## [0.2.0] - 2026-03-31

### Changed

- Wrapped all public functions and methods across all types (`NewPhysicalFrameAllocator`, `Allocate`, `Free`, `IsAllocated`, `FreeCount`, `AllocatedCount`, `TotalFrames`, `NewMMU`, `CreateAddressSpace`, `DestroyAddressSpace`, `MapPage`, `Translate`, `HandlePageFault`, `CloneAddressSpace`, `ContextSwitch`, `TLB`, `FrameAllocator`, `ActivePID`, `NewPageTable`, `PageTable.MapPage`, `UnmapPage`, `Lookup`, `Entries`, `MappedCount`, `NewPageTableEntry`, `Copy`, `NewFIFOPolicy`, `NewLRUPolicy`, `NewClockPolicy`, and all policy `RecordAccess`/`SelectVictim`/`AddFrame`/`RemoveFrame` methods, `NewTLB`, `TLB.Lookup`, `Insert`, `Invalidate`, `Flush`, `HitRate`, `Size`, `Capacity`, `NewTwoLevelPageTable`, `Map`, `Unmap`, `TwoLevelPageTable.Translate`, `LookupVPN`, `MapVPN`, `Directory`) with the Operations system (`StartNew[T]`), providing automatic timing, structured logging, and panic recovery. Public API signatures unchanged.

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
