-- Tests for coding_adventures.virtual_memory
-- ===========================================
-- Comprehensive tests for all virtual memory components.
-- Target: 95%+ coverage.

package.path = "../src/?.lua;" .. "../src/?/init.lua;" .. package.path

local vm = require("coding_adventures.virtual_memory")

-- ============================================================================
-- Module constants
-- ============================================================================

describe("constants", function()
    it("has correct page size", function()
        assert.are.equal(4096, vm.PAGE_SIZE)
    end)
    it("has correct offset bits", function()
        assert.are.equal(12, vm.PAGE_OFFSET_BITS)
    end)
    it("has correct offset mask", function()
        assert.are.equal(0xFFF, vm.PAGE_OFFSET_MASK)
    end)
    it("has default TLB capacity", function()
        assert.are.equal(64, vm.DEFAULT_TLB_CAPACITY)
    end)
end)

-- ============================================================================
-- PageTableEntry
-- ============================================================================

describe("new_pte", function()
    it("creates PTE with default flags", function()
        local pte = vm.new_pte(7)
        assert.are.equal(7, pte.frame_number)
        assert.is_true(pte.present)
        assert.is_false(pte.dirty)
        assert.is_false(pte.accessed)
        assert.is_false(pte.writable)
        assert.is_false(pte.executable)
        assert.is_false(pte.user_accessible)
    end)

    it("accepts optional flags", function()
        local pte = vm.new_pte(3, {writable=true, executable=true, user_accessible=true})
        assert.is_true(pte.writable)
        assert.is_true(pte.executable)
        assert.is_true(pte.user_accessible)
    end)

    it("can be created with present=false", function()
        local pte = vm.new_pte(0, {present=false})
        assert.is_false(pte.present)
    end)
end)

-- ============================================================================
-- Single-Level Page Table
-- ============================================================================

describe("single-level page table", function()
    it("lookup returns nil for missing VPN", function()
        local pt = vm.new_page_table()
        assert.is_nil(vm.pt_lookup(pt, 5))
    end)

    it("pt_map creates a PTE", function()
        local pt = vm.new_page_table()
        vm.pt_map(pt, 5, 10)
        local pte = vm.pt_lookup(pt, 5)
        assert.is_not_nil(pte)
        assert.are.equal(10, pte.frame_number)
    end)

    it("pt_unmap removes the mapping", function()
        local pt = vm.new_page_table()
        vm.pt_map(pt, 3, 7)
        local removed = vm.pt_unmap(pt, 3)
        assert.are.equal(7, removed.frame_number)
        assert.is_nil(vm.pt_lookup(pt, 3))
    end)

    it("pt_unmap on nonexistent VPN returns nil", function()
        local pt = vm.new_page_table()
        assert.is_nil(vm.pt_unmap(pt, 99))
    end)

    it("pt_mapped_count returns correct count", function()
        local pt = vm.new_page_table()
        assert.are.equal(0, vm.pt_mapped_count(pt))
        vm.pt_map(pt, 1, 10)
        vm.pt_map(pt, 2, 11)
        assert.are.equal(2, vm.pt_mapped_count(pt))
    end)

    it("pt_insert puts a PTE directly", function()
        local pt = vm.new_page_table()
        local pte = vm.new_pte(42, {writable=true})
        vm.pt_insert(pt, 10, pte)
        local got = vm.pt_lookup(pt, 10)
        assert.are.equal(42, got.frame_number)
        assert.is_true(got.writable)
    end)
end)

-- ============================================================================
-- split_address
-- ============================================================================

describe("split_address", function()
    it("splits 0x00012ABC into vpn1=0, vpn0=18, offset=0xABC", function()
        -- 0x00012ABC: vpn = 0x12ABC >> 12... wait:
        -- actual address = 0x00012ABC
        -- vpn = floor(0x00012ABC / 4096) = floor(76988 / 4096) = 18
        -- vpn1 = floor(18 / 1024) = 0
        -- vpn0 = 18 % 1024 = 18
        -- offset = 0x00012ABC % 4096 = 0xABC = 2748
        local vpn1, vpn0, offset = vm.split_address(0x00012ABC)
        assert.are.equal(0, vpn1)
        assert.are.equal(18, vpn0)
        assert.are.equal(0xABC, offset)
    end)

    it("address 0x0 → vpn1=0, vpn0=0, offset=0", function()
        local vpn1, vpn0, offset = vm.split_address(0x0)
        assert.are.equal(0, vpn1)
        assert.are.equal(0, vpn0)
        assert.are.equal(0, offset)
    end)

    it("page boundary: 0x1000 → vpn0=1, offset=0", function()
        local vpn1, vpn0, offset = vm.split_address(0x1000)
        assert.are.equal(0, vpn1)
        assert.are.equal(1, vpn0)
        assert.are.equal(0, offset)
    end)

    it("last byte in page 0: vpn=0, offset=0xFFF", function()
        local vpn1, vpn0, offset = vm.split_address(0xFFF)
        assert.are.equal(0, vpn1)
        assert.are.equal(0, vpn0)
        assert.are.equal(0xFFF, offset)
    end)

    it("two-level split: address in vpn1=1 region (>4MB)", function()
        -- vpn1=1 starts at virtual address 0x400000 (1024*1024*4 = 4MB)
        local vpn1, vpn0, offset = vm.split_address(0x400000)
        assert.are.equal(1, vpn1)
        assert.are.equal(0, vpn0)
        assert.are.equal(0, offset)
    end)
end)

-- ============================================================================
-- Two-Level Page Table
-- ============================================================================

describe("two-level page table", function()
    it("tpt_translate returns nil for unmapped address", function()
        local tpt = vm.new_two_level_pt()
        assert.is_nil(vm.tpt_translate(tpt, 0x5000))
    end)

    it("tpt_map and tpt_translate work", function()
        local tpt = vm.new_two_level_pt()
        -- Map virtual page at 0x5000 → frame 10
        vm.tpt_map(tpt, 0x5000, 10)
        local phys, pte = vm.tpt_translate(tpt, 0x5ABC)
        -- Frame 10: physical = 10*4096 + 0xABC = 40960 + 2748 = 43708
        assert.are.equal(10 * 4096 + 0xABC, phys)
        assert.are.equal(10, pte.frame_number)
    end)

    it("tpt_translate returns nil when present=false", function()
        local tpt = vm.new_two_level_pt()
        vm.tpt_map(tpt, 0x2000, 5, {present=false})
        assert.is_nil(vm.tpt_translate(tpt, 0x2000))
    end)

    it("tpt_unmap removes the mapping", function()
        local tpt = vm.new_two_level_pt()
        vm.tpt_map(tpt, 0x3000, 7)
        local removed = vm.tpt_unmap(tpt, 0x3000)
        assert.are.equal(7, removed.frame_number)
        assert.is_nil(vm.tpt_translate(tpt, 0x3000))
    end)

    it("tpt_unmap for unmapped address returns nil", function()
        local tpt = vm.new_two_level_pt()
        assert.is_nil(vm.tpt_unmap(tpt, 0x9999))
    end)

    it("tpt_lookup_pte returns PTE without computing physical address", function()
        local tpt = vm.new_two_level_pt()
        vm.tpt_map(tpt, 0x4000, 3, {writable=true})
        local pte = vm.tpt_lookup_pte(tpt, 0x4ABC)
        assert.are.equal(3, pte.frame_number)
        assert.is_true(pte.writable)
    end)

    it("tpt_lookup_pte returns nil for missing address", function()
        local tpt = vm.new_two_level_pt()
        assert.is_nil(vm.tpt_lookup_pte(tpt, 0x8000))
    end)

    it("tpt_update_pte modifies a PTE in-place", function()
        local tpt = vm.new_two_level_pt()
        vm.tpt_map(tpt, 0x1000, 2)
        vm.tpt_update_pte(tpt, 0x1000, function(p)
            p.accessed = true
            p.dirty = true
            return p
        end)
        local pte = vm.tpt_lookup_pte(tpt, 0x1000)
        assert.is_true(pte.accessed)
        assert.is_true(pte.dirty)
    end)

    it("tpt_update_pte on missing address does nothing", function()
        local tpt = vm.new_two_level_pt()
        -- Should not error
        vm.tpt_update_pte(tpt, 0x9999, function(p) return p end)
    end)

    it("tpt_update_pte on missing vpn1 does nothing", function()
        local tpt = vm.new_two_level_pt()
        vm.tpt_update_pte(tpt, 0x400000, function(p) return p end)
    end)

    it("tpt_all_mappings returns all mapped entries", function()
        local tpt = vm.new_two_level_pt()
        vm.tpt_map(tpt, 0x0000, 1)
        vm.tpt_map(tpt, 0x1000, 2)
        vm.tpt_map(tpt, 0x2000, 3)
        local mappings = vm.tpt_all_mappings(tpt)
        assert.are.equal(3, #mappings)
    end)
end)

-- ============================================================================
-- TLB
-- ============================================================================

describe("TLB", function()
    it("new TLB has zero hits and misses", function()
        local tlb = vm.new_tlb(64)
        assert.are.equal(0, tlb.hits)
        assert.are.equal(0, tlb.misses)
        assert.are.equal(0, vm.tlb_size(tlb))
    end)

    it("lookup miss increments misses", function()
        local tlb = vm.new_tlb(64)
        local result = vm.tlb_lookup(tlb, 1, 100)
        assert.is_nil(result)
        assert.are.equal(0, tlb.hits)
        assert.are.equal(1, tlb.misses)
    end)

    it("insert then lookup is a hit", function()
        local tlb = vm.new_tlb(64)
        local pte = vm.new_pte(5)
        vm.tlb_insert(tlb, 1, 100, 5, pte)
        local result = vm.tlb_lookup(tlb, 1, 100)
        assert.is_not_nil(result)
        assert.are.equal(5, result.frame)
        assert.are.equal(1, tlb.hits)
        assert.are.equal(0, tlb.misses)
    end)

    it("different processes have separate entries", function()
        local tlb = vm.new_tlb(64)
        local pte1 = vm.new_pte(10)
        local pte2 = vm.new_pte(20)
        vm.tlb_insert(tlb, 1, 5, 10, pte1)
        vm.tlb_insert(tlb, 2, 5, 20, pte2)
        local r1 = vm.tlb_lookup(tlb, 1, 5)
        local r2 = vm.tlb_lookup(tlb, 2, 5)
        assert.are.equal(10, r1.frame)
        assert.are.equal(20, r2.frame)
    end)

    it("flush clears all entries", function()
        local tlb = vm.new_tlb(64)
        local pte = vm.new_pte(1)
        vm.tlb_insert(tlb, 1, 0, 1, pte)
        vm.tlb_flush(tlb)
        assert.are.equal(0, vm.tlb_size(tlb))
        local result = vm.tlb_lookup(tlb, 1, 0)
        assert.is_nil(result)
    end)

    it("invalidate removes specific entry", function()
        local tlb = vm.new_tlb(64)
        local pte = vm.new_pte(3)
        vm.tlb_insert(tlb, 1, 7, 3, pte)
        vm.tlb_invalidate(tlb, 1, 7)
        assert.is_nil(vm.tlb_lookup(tlb, 1, 7))
    end)

    it("hit rate is 0.0 when no lookups", function()
        local tlb = vm.new_tlb(64)
        assert.are.equal(0.0, vm.tlb_hit_rate(tlb))
    end)

    it("hit rate is 0.5 for one hit one miss", function()
        local tlb = vm.new_tlb(64)
        local pte = vm.new_pte(1)
        vm.tlb_insert(tlb, 1, 0, 1, pte)
        vm.tlb_lookup(tlb, 1, 0)   -- hit
        vm.tlb_lookup(tlb, 1, 99)  -- miss
        assert.are.equal(0.5, vm.tlb_hit_rate(tlb))
    end)

    it("evicts LRU when at capacity", function()
        local tlb = vm.new_tlb(3)  -- tiny TLB
        local pte = vm.new_pte(1)
        vm.tlb_insert(tlb, 1, 0, 0, pte)
        vm.tlb_insert(tlb, 1, 1, 1, pte)
        vm.tlb_insert(tlb, 1, 2, 2, pte)
        -- TLB is full. Insert a 4th entry — evicts entry (pid=1, vpn=0) (oldest)
        vm.tlb_insert(tlb, 1, 3, 3, pte)
        assert.are.equal(3, vm.tlb_size(tlb))
        assert.is_nil(vm.tlb_lookup(tlb, 1, 0))  -- evicted (but this increments misses)
    end)

    it("re-inserting existing key updates it", function()
        local tlb = vm.new_tlb(64)
        local pte1 = vm.new_pte(1)
        local pte2 = vm.new_pte(2)
        vm.tlb_insert(tlb, 1, 5, 1, pte1)
        vm.tlb_insert(tlb, 1, 5, 2, pte2)  -- update same key
        local result = vm.tlb_lookup(tlb, 1, 5)
        assert.are.equal(2, result.frame)
        assert.are.equal(1, vm.tlb_size(tlb))
    end)
end)

-- ============================================================================
-- Physical Frame Allocator
-- ============================================================================

describe("PhysicalFrameAllocator", function()
    it("all frames free initially", function()
        local alloc = vm.new_frame_allocator(8)
        assert.are.equal(8, alloc.free_count)
    end)

    it("allocate returns sequential frames", function()
        local alloc = vm.new_frame_allocator(8)
        assert.are.equal(0, vm.alloc_frame(alloc))
        assert.are.equal(1, vm.alloc_frame(alloc))
        assert.are.equal(2, vm.alloc_frame(alloc))
        assert.are.equal(5, alloc.free_count)
    end)

    it("allocate returns nil when out of memory", function()
        local alloc = vm.new_frame_allocator(2)
        vm.alloc_frame(alloc)
        vm.alloc_frame(alloc)
        assert.is_nil(vm.alloc_frame(alloc))
    end)

    it("free makes frame available for re-allocation", function()
        local alloc = vm.new_frame_allocator(4)
        local f1 = vm.alloc_frame(alloc)
        local f2 = vm.alloc_frame(alloc)
        vm.free_frame(alloc, f1)
        local f3 = vm.alloc_frame(alloc)
        assert.are.equal(f1, f3)  -- reuses freed frame
        assert.are.equal(2, alloc.free_count)  -- wait: 4-2 alloc + 1 free + 1 alloc = 2 free
    end)

    it("double-free raises error", function()
        local alloc = vm.new_frame_allocator(4)
        vm.alloc_frame(alloc)
        vm.free_frame(alloc, 0)
        assert.has_error(function() vm.free_frame(alloc, 0) end)
    end)

    it("out-of-range free raises error", function()
        local alloc = vm.new_frame_allocator(4)
        assert.has_error(function() vm.free_frame(alloc, 99) end)
    end)

    it("frame_is_allocated returns correct state", function()
        local alloc = vm.new_frame_allocator(4)
        assert.is_false(vm.frame_is_allocated(alloc, 0))
        vm.alloc_frame(alloc)
        assert.is_true(vm.frame_is_allocated(alloc, 0))
    end)

    it("frame_is_allocated raises on out-of-range", function()
        local alloc = vm.new_frame_allocator(4)
        assert.has_error(function() vm.frame_is_allocated(alloc, 100) end)
    end)
end)

-- ============================================================================
-- FIFO Policy
-- ============================================================================

describe("FIFO policy", function()
    it("select_victim returns nil when empty", function()
        local p = vm.new_fifo_policy()
        assert.is_nil(vm.policy_select_victim(p))
    end)

    it("evicts the oldest frame (FIFO order)", function()
        local p = vm.new_fifo_policy()
        vm.policy_add_frame(p, 10)
        vm.policy_add_frame(p, 20)
        vm.policy_add_frame(p, 30)
        -- Oldest = 10
        assert.are.equal(10, vm.policy_select_victim(p))
        assert.are.equal(20, vm.policy_select_victim(p))
        assert.are.equal(30, vm.policy_select_victim(p))
    end)

    it("record_access does nothing (FIFO ignores access)", function()
        local p = vm.new_fifo_policy()
        vm.policy_add_frame(p, 10)
        vm.policy_add_frame(p, 20)
        vm.policy_record_access(p, 20)  -- should be no-op
        assert.are.equal(10, vm.policy_select_victim(p))  -- still evicts oldest
    end)

    it("remove_frame removes a specific frame", function()
        local p = vm.new_fifo_policy()
        vm.policy_add_frame(p, 1)
        vm.policy_add_frame(p, 2)
        vm.policy_add_frame(p, 3)
        vm.policy_remove_frame(p, 2)
        assert.are.equal(1, vm.policy_select_victim(p))
        assert.are.equal(3, vm.policy_select_victim(p))
    end)
end)

-- ============================================================================
-- LRU Policy
-- ============================================================================

describe("LRU policy", function()
    it("select_victim returns nil when empty", function()
        local p = vm.new_lru_policy()
        assert.is_nil(vm.policy_select_victim(p))
    end)

    it("evicts least recently used", function()
        local p = vm.new_lru_policy()
        vm.policy_add_frame(p, 1)
        vm.policy_add_frame(p, 2)
        vm.policy_add_frame(p, 3)
        -- Access 1 → it becomes MRU. LRU is now 2.
        vm.policy_record_access(p, 1)
        assert.are.equal(2, vm.policy_select_victim(p))
    end)

    it("record_access moves frame to MRU position", function()
        local p = vm.new_lru_policy()
        vm.policy_add_frame(p, 10)
        vm.policy_add_frame(p, 20)
        vm.policy_add_frame(p, 30)
        -- Access 10 (oldest) → 20 becomes LRU
        vm.policy_record_access(p, 10)
        assert.are.equal(20, vm.policy_select_victim(p))
    end)

    it("remove_frame works", function()
        local p = vm.new_lru_policy()
        vm.policy_add_frame(p, 5)
        vm.policy_add_frame(p, 6)
        vm.policy_remove_frame(p, 5)
        assert.are.equal(6, vm.policy_select_victim(p))
        assert.is_nil(vm.policy_select_victim(p))
    end)
end)

-- ============================================================================
-- Clock Policy
-- ============================================================================

describe("Clock policy", function()
    it("select_victim returns nil when empty", function()
        local p = vm.new_clock_policy()
        assert.is_nil(vm.policy_select_victim(p))
    end)

    it("evicts frame with use_bit=false", function()
        local p = vm.new_clock_policy()
        vm.policy_add_frame(p, 1)  -- use_bit=true
        vm.policy_add_frame(p, 2)  -- use_bit=true
        -- Clear frame 1's use bit manually
        p.use_bits[1] = false
        -- Clock should evict frame 1 (first one with use_bit=false)
        local victim = vm.policy_select_victim(p)
        assert.are.equal(1, victim)
    end)

    it("gives second chance to recently accessed frames", function()
        local p = vm.new_clock_policy()
        vm.policy_add_frame(p, 10)  -- use_bit=true (from add)
        vm.policy_add_frame(p, 20)
        -- Both have use_bit=true. Clock will clear 10, advance, clear 20, advance,
        -- then come back to 10 (now false) and evict it.
        local victim = vm.policy_select_victim(p)
        assert.are.equal(10, victim)
    end)

    it("record_access sets use bit", function()
        local p = vm.new_clock_policy()
        vm.policy_add_frame(p, 5)
        p.use_bits[5] = false
        vm.policy_record_access(p, 5)
        assert.is_true(p.use_bits[5])
    end)

    it("remove_frame removes frame from tracking", function()
        local p = vm.new_clock_policy()
        vm.policy_add_frame(p, 1)
        vm.policy_add_frame(p, 2)
        vm.policy_remove_frame(p, 1)
        -- Only frame 2 remains; clear its bit and evict
        p.use_bits[2] = false
        assert.are.equal(2, vm.policy_select_victim(p))
    end)
end)

-- ============================================================================
-- MMU — basic operations
-- ============================================================================

describe("MMU create/destroy address spaces", function()
    it("create_address_space initializes empty page table", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        assert.is_not_nil(mmu.page_tables[1])
    end)

    it("mmu_map_page allocates a frame", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x1000)
        assert.is_not_nil(frame)
        assert.is_true(vm.frame_is_allocated(mmu.frame_allocator, frame))
    end)

    it("mmu_map_page raises for unknown PID", function()
        local mmu = vm.new_mmu(16, "lru")
        assert.has_error(function() vm.mmu_map_page(mmu, 99, 0x1000) end)
    end)

    it("mmu_destroy_address_space frees frames", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x0000)
        vm.mmu_destroy_address_space(mmu, 1)
        assert.is_nil(mmu.page_tables[1])
        assert.is_false(vm.frame_is_allocated(mmu.frame_allocator, frame))
    end)

    it("destroy nonexistent address space is a no-op", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_destroy_address_space(mmu, 99)  -- should not error
    end)
end)

-- ============================================================================
-- MMU — translate
-- ============================================================================

describe("MMU translate", function()
    it("full translation: VPN 5 → frame 10, translate 0x5ABC → 0xAABC", function()
        local mmu = vm.new_mmu(32, "lru")
        vm.mmu_create_address_space(mmu, 1)
        -- Map virtual address 0x5000 (VPN=5) → get first free frame (0)
        -- Then we need to verify the formula. Let's map to a specific frame.
        -- Easier: just map and translate and check frame*4096 + offset.
        local frame = vm.mmu_map_page(mmu, 1, 0x5000)
        local phys = vm.mmu_translate(mmu, 1, 0x5ABC)
        assert.are.equal(frame * 4096 + 0xABC, phys)
    end)

    it("second translation hits TLB", function()
        local mmu = vm.new_mmu(32, "lru")
        vm.mmu_create_address_space(mmu, 1)
        vm.mmu_map_page(mmu, 1, 0x1000)
        -- First translate: TLB miss
        vm.mmu_translate(mmu, 1, 0x1000)
        local misses_after_first = mmu.tlb.misses
        -- Second translate: TLB hit
        vm.mmu_translate(mmu, 1, 0x1000)
        assert.are.equal(1, mmu.tlb.hits)
        assert.are.equal(misses_after_first, mmu.tlb.misses)
    end)

    it("translate raises for unknown PID", function()
        local mmu = vm.new_mmu(8, "lru")
        assert.has_error(function() vm.mmu_translate(mmu, 99, 0x0) end)
    end)

    it("translate returns nil for unmapped (segfault)", function()
        local mmu = vm.new_mmu(8, "lru")
        vm.mmu_create_address_space(mmu, 1)
        -- No pages mapped — segfault
        local result = vm.mmu_translate(mmu, 1, 0x5000)
        assert.is_nil(result)
    end)

    it("write sets dirty bit", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        vm.mmu_map_page(mmu, 1, 0x0000, {writable=true})
        vm.mmu_translate(mmu, 1, 0x0000, true)  -- write access
        local pte = vm.tpt_lookup_pte(mmu.page_tables[1], 0x0000)
        assert.is_true(pte.dirty)
    end)
end)

-- ============================================================================
-- MMU — page fault handling
-- ============================================================================

describe("page fault handling", function()
    it("handle_page_fault allocates a frame for an unmapped PTE", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        -- Manually create a present=false PTE
        local tpt = mmu.page_tables[1]
        vm.tpt_map(tpt, 0x3000, 0, {present=false})
        local frame = vm.mmu_handle_page_fault(mmu, 1, 0x3000)
        assert.is_not_nil(frame)
        assert.is_true(vm.frame_is_allocated(mmu.frame_allocator, frame))
    end)

    it("handle_page_fault returns nil for address with no PTE (segfault)", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        local result = vm.mmu_handle_page_fault(mmu, 1, 0x9999)
        assert.is_nil(result)
    end)

    it("handle_page_fault returns frame for already-present page (no-op)", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x1000)
        local result = vm.mmu_handle_page_fault(mmu, 1, 0x1000)
        assert.are.equal(frame, result)
    end)

    it("handle_page_fault returns nil when no address space", function()
        local mmu = vm.new_mmu(16, "lru")
        local result = vm.mmu_handle_page_fault(mmu, 99, 0x0)
        assert.is_nil(result)
    end)
end)

-- ============================================================================
-- MMU — clone_address_space (COW fork)
-- ============================================================================

describe("clone_address_space (COW)", function()
    it("child has the same mappings as parent", function()
        local mmu = vm.new_mmu(32, "lru")
        vm.mmu_create_address_space(mmu, 1)
        vm.mmu_map_page(mmu, 1, 0x0000, {writable=true})
        vm.mmu_map_page(mmu, 1, 0x1000, {writable=true})
        vm.mmu_clone_address_space(mmu, 1, 2)
        assert.is_not_nil(mmu.page_tables[2])
        -- Child page table should have same number of mappings
        local parent_count = #vm.tpt_all_mappings(mmu.page_tables[1])
        local child_count  = #vm.tpt_all_mappings(mmu.page_tables[2])
        assert.are.equal(parent_count, child_count)
    end)

    it("both parent and child are marked read-only after clone", function()
        local mmu = vm.new_mmu(32, "lru")
        vm.mmu_create_address_space(mmu, 1)
        vm.mmu_map_page(mmu, 1, 0x0000, {writable=true})
        vm.mmu_clone_address_space(mmu, 1, 2)
        local parent_pte = vm.tpt_lookup_pte(mmu.page_tables[1], 0x0000)
        local child_pte  = vm.tpt_lookup_pte(mmu.page_tables[2], 0x0000)
        assert.is_false(parent_pte.writable)
        assert.is_false(child_pte.writable)
    end)

    it("frame refcount is incremented for shared frames", function()
        local mmu = vm.new_mmu(32, "lru")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x0000, {writable=true})
        vm.mmu_clone_address_space(mmu, 1, 2)
        assert.are.equal(2, mmu.frame_refcounts[frame])
    end)

    it("clone of nonexistent source is a no-op", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_clone_address_space(mmu, 99, 2)  -- should not error
    end)
end)

-- ============================================================================
-- MMU — with different replacement policies
-- ============================================================================

describe("MMU with FIFO policy", function()
    it("creates and translates correctly", function()
        local mmu = vm.new_mmu(8, "fifo")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x0000)
        local phys = vm.mmu_translate(mmu, 1, 0x0100)
        assert.are.equal(frame * 4096 + 0x100, phys)
    end)
end)

describe("MMU with Clock policy", function()
    it("creates and translates correctly", function()
        local mmu = vm.new_mmu(8, "clock")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x0000)
        local phys = vm.mmu_translate(mmu, 1, 0x0200)
        assert.are.equal(frame * 4096 + 0x200, phys)
    end)
end)

-- ============================================================================
-- Integration: full translation round-trip
-- ============================================================================

describe("integration: full translation", function()
    it("map VPN 5 → frame, translate 0x5ABC, verify physical", function()
        local mmu = vm.new_mmu(64, "lru")
        vm.mmu_create_address_space(mmu, 1)
        local frame = vm.mmu_map_page(mmu, 1, 0x5000)
        local phys = vm.mmu_translate(mmu, 1, 0x5ABC)
        assert.are.equal(frame * 4096 + 0xABC, phys)
    end)

    it("TLB integration: miss then hit then flush then miss again", function()
        local mmu = vm.new_mmu(16, "lru")
        vm.mmu_create_address_space(mmu, 1)
        vm.mmu_map_page(mmu, 1, 0x0000)
        -- First: TLB miss (miss=1)
        vm.mmu_translate(mmu, 1, 0x0000)
        assert.are.equal(1, mmu.tlb.misses)
        -- Second: TLB hit (hit=1)
        vm.mmu_translate(mmu, 1, 0x0000)
        assert.are.equal(1, mmu.tlb.hits)
        -- Flush
        vm.tlb_flush(mmu.tlb)
        -- Third: TLB miss again (miss=2)
        vm.mmu_translate(mmu, 1, 0x0000)
        assert.are.equal(2, mmu.tlb.misses)
    end)
end)
