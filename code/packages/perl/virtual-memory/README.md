# CodingAdventures::VirtualMemory

A Pure Perl implementation of a virtual memory subsystem, including page tables, TLB, frame allocator, page replacement policies, and a full MMU.

## Features

- **PageTableEntry** — PTE with present/dirty/accessed/writable/executable/user_accessible/COW flags
- **Single-Level Page Table** — flat VPN → PTE mapping
- **Two-Level Page Table** (Sv32) — directory + leaf tables; VPN[1] selects L1 entry, VPN[0] selects PTE
- **Address Decomposition** — `split_address` extracts VPN[1], VPN[0], offset from a 32-bit virtual address
- **TLB** — 64-entry LRU translation cache with per-process flush and hit-rate tracking
- **Frame Allocator** — free-list allocator for physical frames
- **Page Replacement Policies** — FIFO, LRU, Clock (Second Chance)
- **MMU** — full address space management: map/translate/fault/COW fork

## Usage

```perl
use CodingAdventures::VirtualMemory;

# Create MMU with 64 frames and LRU replacement
my $mmu = CodingAdventures::VirtualMemory::new_mmu(
    total_frames => 64,
    policy_type  => 'lru',   # or 'fifo' or 'clock'
    tlb_capacity => 64,
);

# Process 1 gets its own address space
CodingAdventures::VirtualMemory::mmu_create_address_space($mmu, 1);

# Map virtual page 0x1000 to a new physical frame
my $frame = CodingAdventures::VirtualMemory::mmu_map_page($mmu, 1, 0x1000);

# Translate: first access is TLB miss → page table walk → TLB install
my ($paddr, $type) = CodingAdventures::VirtualMemory::mmu_translate($mmu, 1, 0x1042);
# $type eq 'miss', $paddr = $frame*4096 + 0x42

# Second access hits TLB
my ($paddr2, $type2) = CodingAdventures::VirtualMemory::mmu_translate($mmu, 1, 0x1000);
# $type2 eq 'hit'

# Fork: parent writes trigger COW copy
CodingAdventures::VirtualMemory::mmu_clone_address_space($mmu, 1, 2);
my ($cow_paddr, $cow_type) = CodingAdventures::VirtualMemory::mmu_translate($mmu, 2, 0x1000, write => 1);
# $cow_type eq 'cow' — child gets its own private copy
```

## Installation

```sh
cpanm --installdeps .
```

## Testing

```sh
prove -l -v t/
```
