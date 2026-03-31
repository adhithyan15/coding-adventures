# coding-adventures-virtual-memory

A Lua implementation of a virtual memory subsystem with paging, TLB, and page replacement policies.

## What it does

Simulates the core components of an operating system's virtual memory manager:

- **PageTableEntry** — maps one virtual page to a physical frame with permission bits
- **Single-level page table** — simple VPN → PTE hash map
- **Two-level page table (Sv32)** — RISC-V style, memory-efficient
- **TLB** — translation cache with LRU eviction and hit/miss statistics
- **PhysicalFrameAllocator** — bitmap-based frame management
- **Page replacement policies** — FIFO, LRU, Clock (second-chance)
- **MMU** — ties it all together: translate(), map_page(), clone_address_space() (COW fork)

## Address translation

```
32-bit virtual address:
┌────────────┬────────────┬────────────────┐
│ VPN[1]     │ VPN[0]     │ Page Offset    │
│ bits 31-22 │ bits 21-12 │ bits 11-0      │
└────────────┴────────────┴────────────────┘

physical = (frame_number << 12) | offset
```

## Usage

```lua
local vm = require("coding_adventures.virtual_memory")

-- Create an MMU with 64 physical frames and LRU replacement
local mmu = vm.new_mmu(64, "lru")

-- Create a process address space
vm.mmu_create_address_space(mmu, 1)

-- Map virtual address 0x5000 to a physical frame
local frame = vm.mmu_map_page(mmu, 1, 0x5000, {writable=true})

-- Translate virtual to physical
local phys = vm.mmu_translate(mmu, 1, 0x5ABC)
-- phys = frame * 4096 + 0xABC

-- TLB statistics
print(vm.tlb_hit_rate(mmu.tlb))  -- 0.0 on first translate (miss)
vm.mmu_translate(mmu, 1, 0x5ABC) -- second time is a TLB hit
print(vm.tlb_hit_rate(mmu.tlb))  -- 0.5

-- COW fork
vm.mmu_clone_address_space(mmu, 1, 2)  -- child PID=2 shares frames with parent PID=1
```

## Installation

```sh
luarocks make --local coding-adventures-virtual-memory-0.1.0-1.rockspec
```

## Testing

```sh
cd tests && busted . --verbose --pattern=test_
```
