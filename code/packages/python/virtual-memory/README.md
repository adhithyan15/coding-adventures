# Virtual Memory

A complete virtual memory subsystem simulator, implementing the core abstractions
that allow every process to have its own private address space even though physical
RAM is shared.

## What Is Virtual Memory?

Virtual memory is one of the most important abstractions in computer science. It
gives every process the illusion that it owns the entire memory space — starting
at address 0 and stretching to some large upper limit — even though the physical
machine has limited RAM shared among many processes.

Without virtual memory, every program would need to know exactly where in physical
RAM it was loaded. Two programs wanting the same addresses would overwrite each
other.

**Analogy:** Imagine an apartment building. Each tenant thinks their apartment
starts at "Room 1." But the building manager knows tenant A's "Room 1" is actually
physical room 401, and tenant B's "Room 1" is room 712. The tenants never learn
their real room numbers. They just say "go to my Room 1" and the building manager
(the MMU) translates.

## Components

| Component | File | Purpose |
|-----------|------|---------|
| `PageTableEntry` | `page_table_entry.py` | Metadata for a single page mapping (frame number, flags) |
| `PageTable` | `page_table.py` | Single-level page table: VPN -> PTE dictionary |
| `TwoLevelPageTable` | `multi_level_page_table.py` | Sv32 two-level page table (10+10+12 bit split) |
| `TLB` | `tlb.py` | Translation Lookaside Buffer — caches recent translations |
| `PhysicalFrameAllocator` | `frame_allocator.py` | Bitmap-based physical frame manager |
| `FIFOPolicy` / `LRUPolicy` / `ClockPolicy` | `replacement.py` | Page replacement algorithms |
| `MMU` | `mmu.py` | Memory Management Unit — ties everything together |

## How It Fits in the Stack

```
Process Manager (D14)
    |
    v
Virtual Memory (D13) <-- THIS PACKAGE
    |
    v
Physical Memory (frames)
    |
    v
CPU Core (D05)
```

## Quick Start

```python
from virtual_memory import MMU, FIFOPolicy

# Create an MMU with 256 physical frames (1 MB of RAM)
mmu = MMU(total_frames=256, replacement_policy=FIFOPolicy())

# Create an address space for process 1
mmu.create_address_space(pid=1)

# Map a virtual page and get a physical address
frame = mmu.map_page(pid=1, virtual_addr=0x1000)

# Translate a virtual address to physical
physical = mmu.translate(pid=1, virtual_addr=0x1ABC)
# physical == (frame << 12) | 0xABC
```

## Address Translation

```
32-bit virtual address:
+------------------------+--------------+
| Virtual Page Number    | Page Offset  |
| bits 31-12 (20 bits)   | bits 11-0    |
|                        | (12 bits)    |
+------------------------+--------------+

VPN    = address >> 12
offset = address & 0xFFF

Physical address = (frame_number << 12) | offset
```

## Running Tests

```bash
uv venv && uv pip install -e ".[dev]"
python -m pytest tests/ -v
```
