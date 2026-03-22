# CodingAdventures::VirtualMemory

A complete virtual memory subsystem implementing page tables, TLB, physical frame allocator, page replacement policies, and an MMU with copy-on-write support.

## What is Virtual Memory?

Virtual memory gives every process the illusion of having the entire memory space to itself. Process A's address `0x1000` and Process B's address `0x1000` map to completely different physical locations in RAM. This provides:

1. **Isolation** -- processes cannot see or corrupt each other's memory.
2. **Abstraction** -- programs don't need to know where in physical RAM they are loaded.
3. **Overcommitment** -- the OS can promise more memory than physically exists.

## Components

### Page Table Entry (PTE)

Describes the mapping for one virtual page: which physical frame it maps to, and what permissions it has (writable, executable, user-accessible, etc.).

### Single-Level Page Table

A hash map from virtual page number to PTE. Simple but memory-inefficient for large address spaces.

### Two-Level Page Table (Sv32)

RISC-V's Sv32 scheme: 10-bit directory index + 10-bit table index + 12-bit offset. Only allocates second-level tables for regions that are actually in use, saving memory.

### TLB (Translation Lookaside Buffer)

A small cache (64 entries) of recent virtual-to-physical translations. Without the TLB, every memory access would require 2-3 extra memory accesses to walk the page table.

### Physical Frame Allocator

Manages which physical frames (4 KB chunks of RAM) are free or in use. Uses a bitmap for tracking and reference counts for copy-on-write support.

### Page Replacement Policies

When physical memory is full, choose which page to evict:

- **FIFO** -- evict the oldest page (simplest, but can evict hot pages)
- **LRU** -- evict the least recently used page (best approximation of optimal)
- **Clock** -- approximate LRU using a use bit and circular sweep (practical compromise)

### MMU (Memory Management Unit)

Ties everything together: per-process page tables, TLB, frame allocator, and replacement policy. Supports:

- Address translation (virtual -> physical)
- Page fault handling (demand paging)
- Copy-on-write fork (efficient process cloning)
- Context switching (TLB flush)

## Usage

```ruby
require "coding_adventures_virtual_memory"

include CodingAdventures::VirtualMemory

# Create an MMU with 256 frames of physical memory (1 MB)
mmu = MMU.new(total_frames: 256)

# Create address space for process 1
mmu.create_address_space(1)

# Map virtual page at address 0x1000
frame = mmu.map_page(1, 0x1000)

# Translate virtual address to physical
physical = mmu.translate(1, 0x1ABC)
# physical == (frame << 12) | 0xABC

# Fork: clone address space with copy-on-write
mmu.clone_address_space(1, 2)

# Both processes read the same physical data (shared frames)
p1 = mmu.translate(1, 0x1000)
p2 = mmu.translate(2, 0x1000)
# p1 == p2 (shared)

# Child writes: COW fault creates a private copy
p2_after = mmu.translate(2, 0x1000, write: true)
# p2_after != p1 (private copy)
```

## Where It Fits

```
Process Manager (D14)        -- fork/exec use MMU
    |
    v
Virtual Memory (D13)         -- THIS PACKAGE
    |
    v
Physical Memory / CPU Core   -- hardware
```

## Testing

```bash
bundle install
bundle exec rake test
```
