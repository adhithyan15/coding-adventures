# CodingAdventures.VirtualMemory

A complete virtual memory subsystem implemented in Elixir for educational purposes. This package is layer D13 of the computing stack, sitting between physical memory and the process manager.

## What is Virtual Memory?

Virtual memory gives every process the illusion that it has the entire memory space to itself. Without it, every program would need to know exactly where in physical RAM it was loaded, and two programs wanting the same address would overwrite each other.

### The Apartment Building Analogy

Imagine an apartment building. Each tenant thinks their rooms start at "Room 1." But the building manager knows Tenant A's "Room 1" is physical room 401, and Tenant B's "Room 1" is physical room 712. The building manager (the MMU) translates automatically.

## How It Works

### Pages and Frames

Memory is divided into fixed-size 4 KB chunks:
- **Pages**: chunks of virtual memory
- **Frames**: chunks of physical memory

Any virtual page can map to any physical frame via a **page table**.

### Two-Level Page Tables (Sv32)

Instead of a flat table with 1 million entries, Sv32 uses two levels:
- Level 1: Page Directory (1024 entries)
- Level 2: Page Table (1024 entries, each a PTE)

### TLB (Translation Lookaside Buffer)

A small cache (64 entries) for recent translations. Programs exhibit temporal locality, so hit rates above 95% are typical.

### Page Replacement Policies

When physical memory is full:
- **FIFO**: Evict the oldest page
- **LRU**: Evict the least recently accessed page
- **Clock**: Approximate LRU with use bits

### Copy-on-Write (COW)

fork() shares frames (not copies) between parent and child, marked read-only. Writes trigger private copies on demand.

## Elixir Design Notes

This implementation uses immutable data structures throughout, following Elixir conventions:

- All functions return new structs rather than mutating in place.
- The MMU carries all state (page tables, TLB, allocator, policy) as a single struct that is threaded through function calls.
- Atoms are not used for enum-like values to avoid the Elixir reserved word problem; instead, booleans and keyword lists are used for flags.

## Components

| Module | Description |
|---|---|
| `PageTableEntry` | Virtual-to-physical mapping with permission flags |
| `PageTable` | Single-level hash map of VPN to PTE |
| `TwoLevelPageTable` | Sv32-style two-level page table |
| `TLB` | Translation cache with LRU eviction |
| `PhysicalFrameAllocator` | Bitmap-based frame allocator |
| `FIFOPolicy` | First-in, first-out replacement |
| `LRUPolicy` | Least recently used replacement |
| `ClockPolicy` | Second-chance clock replacement |
| `MMU` | Central coordinator |

## Usage

```elixir
alias CodingAdventures.VirtualMemory.MMU

# Create MMU with 256 frames and LRU replacement
mmu = MMU.new(256, :lru)

# Create address space for process 1
mmu = MMU.create_address_space(mmu, 1)

# Map virtual page at 0x5000
{mmu, frame} = MMU.map_page(mmu, 1, 0x5000, writable: true)

# Translate
{mmu, phys} = MMU.translate(mmu, 1, 0x5ABC)

# Fork with COW
mmu = MMU.clone_address_space(mmu, 1, 2)

# Context switch
mmu = MMU.context_switch(mmu, 2)
```

## Development

```bash
mix deps.get
mix test
```
