# Changelog — CodingAdventures::VirtualMemory

## 0.01 — 2026-03-31

### Added

- Initial Perl port of the virtual memory subsystem (Elixir reference: `elixir/virtual_memory`).
- PageTableEntry with all hardware flags (present, dirty, accessed, writable, executable, user_accessible, COW).
- Single-level page table: new_page_table, pt_map/lookup/unmap/update_pte/mapped_count/all_mappings.
- Two-level page table (Sv32): new_two_level_pt, tpt_map/translate/lookup_pte/unmap/update_pte/all_mappings/mapped_count.
- Address decomposition: split_address (VPN[1]/VPN[0]/offset), vpn_of, make_physical_address.
- TLB: new_tlb with LRU eviction; tlb_lookup/insert/invalidate/flush; hit_rate/size tracking.
- Frame allocator: new_frame_allocator, alloc_frame/free_frame/frame_is_allocated with double-free detection.
- Page replacement policies: FIFO, LRU, Clock (Second Chance) with unified dispatch interface.
- MMU: mmu_create/destroy_address_space, mmu_map_page, mmu_translate (TLB hit/miss/fault/COW), mmu_handle_page_fault, mmu_clone_address_space (COW fork).
- 95%+ test coverage with Test2::V0.
