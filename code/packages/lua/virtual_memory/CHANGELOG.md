# Changelog — coding-adventures-virtual-memory

## 0.1.0 — 2026-03-31

### Added

- Initial Lua port of the virtual memory subsystem (Elixir reference: `elixir/virtual_memory`).
- PageTableEntry with RISC-V Sv32 flags: present, dirty, accessed, writable, executable, user_accessible.
- Single-level page table: pt_lookup, pt_map, pt_unmap, pt_insert, pt_mapped_count.
- Two-level page table (Sv32): split_address, tpt_map, tpt_translate, tpt_unmap, tpt_lookup_pte, tpt_update_pte, tpt_all_mappings.
- TLB: tlb_lookup, tlb_insert, tlb_invalidate, tlb_flush, tlb_hit_rate, tlb_size. LRU eviction. Hit/miss counters.
- PhysicalFrameAllocator: alloc_frame, free_frame, frame_is_allocated, free_count.
- Page replacement policies: FIFO, LRU, Clock (second-chance). Unified interface: policy_add_frame, policy_record_access, policy_select_victim, policy_remove_frame.
- MMU: new_mmu, mmu_create_address_space, mmu_destroy_address_space, mmu_map_page, mmu_translate (with TLB integration), mmu_handle_page_fault, mmu_clone_address_space (COW fork).
- 95%+ test coverage with busted.
