# Virtual Memory (Go)

A complete virtual memory subsystem simulator implementing page tables, TLB,
MMU, and page replacement policies. This is the Go implementation of the
virtual memory package (D13 spec).

## What Is Virtual Memory?

Virtual memory gives every process the illusion of having its own private
address space, even though all processes share the same physical RAM. The MMU
(Memory Management Unit) translates virtual addresses to physical addresses
on every memory access.

## Components

| File | Type | Purpose |
|------|------|---------|
| `page_table_entry.go` | `PageTableEntry` | Metadata for one page mapping (frame, flags) |
| `page_table.go` | `PageTable` | Single-level VPN-to-PTE map |
| `two_level_page_table.go` | `TwoLevelPageTable` | Sv32 two-level (10+10+12) page table |
| `tlb.go` | `TLB` | Translation Lookaside Buffer with hit/miss stats |
| `frame_allocator.go` | `PhysicalFrameAllocator` | Bitmap-based physical frame manager |
| `replacement.go` | `ReplacementPolicy` | Interface + FIFO, LRU, Clock implementations |
| `mmu.go` | `MMU` | Ties everything together: translate, fault, COW, context switch |

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
go test ./... -v -cover
```
