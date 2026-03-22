# @coding-adventures/virtual-memory

A complete virtual memory subsystem implemented in TypeScript for educational purposes. This package is layer D13 of the computing stack, sitting between physical memory and the process manager.

## What is Virtual Memory?

Virtual memory gives every process the illusion that it has the entire memory space to itself. Without it, every program would need to know exactly where in physical RAM it was loaded, and two programs wanting the same address would overwrite each other.

### The Apartment Building Analogy

Imagine an apartment building. Each tenant thinks their rooms start at "Room 1." But the building manager knows Tenant A's "Room 1" is physical room 401, and Tenant B's "Room 1" is physical room 712. The tenants never learn their real room numbers -- the building manager (the MMU) translates automatically.

## How It Works

### Pages and Frames

Memory is divided into fixed-size chunks:
- **Pages**: chunks of virtual memory (4 KB each)
- **Frames**: chunks of physical memory (4 KB each)

Any virtual page can map to any physical frame. The mapping is stored in a **page table**.

### Address Translation

A 32-bit virtual address is split:
- **Upper 20 bits**: Virtual Page Number (VPN)
- **Lower 12 bits**: Page offset (byte within the page)

The physical address is: `(frame_number << 12) | offset`

### Two-Level Page Tables (Sv32)

Instead of a flat table with 1 million entries, Sv32 uses two levels:
- Level 1: Page Directory (1024 entries, each pointing to a page table)
- Level 2: Page Table (1024 entries, each a PTE)

This saves memory because only used regions have second-level tables allocated.

### TLB (Translation Lookaside Buffer)

A small, fast cache (64 entries) that remembers recent translations. Without it, every memory access would require 2-3 extra memory accesses to walk the page table. Hit rates above 95% are typical due to temporal locality.

### Page Replacement Policies

When physical memory is full, the system must evict a page:
- **FIFO**: Evict the oldest page (simple but can be pathological)
- **LRU**: Evict the least recently accessed page (near-optimal but expensive)
- **Clock**: Approximates LRU using use bits and a sweeping clock hand (what real OSes use)

### Copy-on-Write (COW)

When fork() clones a process, physical frames are shared (not copied). Both processes see the same data, marked read-only. When either writes, a page fault triggers a copy of just that one page. This makes fork() nearly free.

## Components

| Component | Description |
|---|---|
| `PageTableEntry` | Describes one virtual-to-physical mapping with permission flags |
| `PageTable` | Single-level hash map of VPN to PTE |
| `TwoLevelPageTable` | Sv32-style two-level page table |
| `TLB` | Translation cache with LRU eviction |
| `PhysicalFrameAllocator` | Bitmap-based frame allocator |
| `FIFOPolicy` | First-in, first-out page replacement |
| `LRUPolicy` | Least recently used page replacement |
| `ClockPolicy` | Second-chance clock page replacement |
| `MMU` | Central coordinator tying all components together |

## Usage

```typescript
import {
  MMU, LRUPolicy, PAGE_OFFSET_BITS
} from "@coding-adventures/virtual-memory";

// Create an MMU with 256 physical frames and LRU replacement
const mmu = new MMU(256, new LRUPolicy());

// Create an address space for process 1
mmu.create_address_space(1);

// Map virtual page at 0x5000 to a physical frame
const frame = mmu.map_page(1, 0x5000, { writable: true });

// Translate a virtual address
const phys = mmu.translate(1, 0x5ABC);
// phys = (frame << 12) | 0xABC

// Fork: clone address space with COW
mmu.clone_address_space(1, 2);

// Context switch
mmu.context_switch(2);
```

## Where It Fits

```
Process Manager (D14)     -- uses clone/create/destroy address space
    |
Virtual Memory (D13)      -- YOU ARE HERE
    |
Physical Memory / CPU Core (D05)
```

## Development

```bash
npm install
npm test
npm run test:coverage
```
