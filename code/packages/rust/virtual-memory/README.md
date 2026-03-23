# virtual-memory

A complete virtual memory subsystem implementing page tables, TLB, physical frame allocator, page replacement policies, and an MMU with copy-on-write support.

## What is Virtual Memory?

Virtual memory gives every process the illusion of having the entire memory space to itself. Process A's address `0x1000` and Process B's address `0x1000` map to completely different physical locations in RAM. This provides:

1. **Isolation** -- processes cannot see or corrupt each other's memory.
2. **Abstraction** -- programs don't need to know where in physical RAM they are loaded.
3. **Overcommitment** -- the OS can promise more memory than physically exists.

## Components

- **PageTableEntry** -- permission bits and frame mapping for one virtual page
- **PageTable** -- single-level hash map from VPN to PTE
- **TwoLevelPageTable** -- RISC-V Sv32 (10+10+12 bit) hierarchical page table
- **TLB** -- translation cache with LRU eviction
- **PhysicalFrameAllocator** -- bitmap allocator with reference counting
- **ReplacementPolicy** trait with FIFO, LRU, and Clock implementations
- **MMU** -- ties everything together with COW fork support

## Usage

```rust
use virtual_memory::*;

let mut mmu = MMU::new(256, Box::new(FIFOPolicy::new()));
mmu.create_address_space(1);

let frame = mmu.map_page(1, 0x1000, PagePermissions::default());
let phys = mmu.translate(1, 0x1ABC, false).unwrap();
assert_eq!(phys, (frame << 12) | 0xABC);
```

## Testing

```bash
cargo test -p virtual-memory
```
