defmodule CodingAdventures.VirtualMemoryTest do
  use ExUnit.Case, async: true
  import Bitwise

  alias CodingAdventures.VirtualMemory.{
    PageTableEntry,
    PageTable,
    TwoLevelPageTable,
    TLB,
    PhysicalFrameAllocator,
    FIFOPolicy,
    LRUPolicy,
    ClockPolicy,
    MMU
  }

  alias CodingAdventures.VirtualMemory

  # ============================================================================
  # PageTableEntry Tests
  # ============================================================================

  describe "PageTableEntry" do
    test "creates a PTE with default flags" do
      pte = PageTableEntry.new(42)
      assert pte.frame_number == 42
      assert pte.present == true
      assert pte.dirty == false
      assert pte.accessed == false
      assert pte.writable == false
      assert pte.executable == false
      assert pte.user_accessible == false
    end

    test "creates a PTE with custom flags" do
      pte = PageTableEntry.new(7,
        present: true,
        dirty: true,
        accessed: true,
        writable: true,
        executable: true,
        user_accessible: true
      )

      assert pte.frame_number == 7
      assert pte.present == true
      assert pte.dirty == true
      assert pte.accessed == true
      assert pte.writable == true
      assert pte.executable == true
      assert pte.user_accessible == true
    end

    test "allows partial flag overrides" do
      pte = PageTableEntry.new(5, writable: true)
      assert pte.writable == true
      assert pte.executable == false
      assert pte.present == true
    end
  end

  # ============================================================================
  # Constants Tests
  # ============================================================================

  describe "Constants" do
    test "PAGE_SIZE is 4096" do
      assert VirtualMemory.page_size() == 4096
    end

    test "PAGE_OFFSET_BITS is 12" do
      assert VirtualMemory.page_offset_bits() == 12
    end

    test "PAGE_OFFSET_MASK is 0xFFF" do
      assert VirtualMemory.page_offset_mask() == 0xFFF
    end

    test "2^PAGE_OFFSET_BITS equals PAGE_SIZE" do

      assert (1 <<< VirtualMemory.page_offset_bits()) == VirtualMemory.page_size()
    end
  end

  # ============================================================================
  # PageTable (Single-Level) Tests
  # ============================================================================

  describe "PageTable" do
    test "starts empty" do
      pt = PageTable.new()
      assert PageTable.mapped_count(pt) == 0
      assert PageTable.lookup(pt, 0) == nil
    end

    test "maps and looks up a page" do
      pt = PageTable.new()
      pt = PageTable.map_page(pt, 5, 10, writable: true)

      pte = PageTable.lookup(pt, 5)
      assert pte != nil
      assert pte.frame_number == 10
      assert pte.present == true
      assert pte.writable == true
      assert PageTable.mapped_count(pt) == 1
    end

    test "returns nil for unmapped pages" do
      pt = PageTable.new() |> PageTable.map_page(0, 1)
      assert PageTable.lookup(pt, 999) == nil
    end

    test "unmaps a page" do
      pt = PageTable.new() |> PageTable.map_page(3, 7)
      {pt, removed} = PageTable.unmap_page(pt, 3)

      assert removed != nil
      assert removed.frame_number == 7
      assert PageTable.lookup(pt, 3) == nil
      assert PageTable.mapped_count(pt) == 0
    end

    test "returns nil when unmapping non-existent page" do
      pt = PageTable.new()
      {_pt, removed} = PageTable.unmap_page(pt, 42)
      assert removed == nil
    end

    test "handles multiple mappings" do
      pt =
        PageTable.new()
        |> PageTable.map_page(0, 100)
        |> PageTable.map_page(1, 200)
        |> PageTable.map_page(2, 300)

      assert PageTable.mapped_count(pt) == 3
      assert PageTable.lookup(pt, 0).frame_number == 100
      assert PageTable.lookup(pt, 1).frame_number == 200
      assert PageTable.lookup(pt, 2).frame_number == 300
    end

    test "overwrites an existing mapping" do
      pt =
        PageTable.new()
        |> PageTable.map_page(5, 10)
        |> PageTable.map_page(5, 20, writable: true)

      assert PageTable.mapped_count(pt) == 1
      assert PageTable.lookup(pt, 5).frame_number == 20
      assert PageTable.lookup(pt, 5).writable == true
    end

    test "gets all VPNs" do
      pt =
        PageTable.new()
        |> PageTable.map_page(1, 10)
        |> PageTable.map_page(5, 50)
        |> PageTable.map_page(3, 30)

      vpns = PageTable.get_all_vpns(pt) |> Enum.sort()
      assert vpns == [1, 3, 5]
    end

    test "inserts PTE directly" do
      pt = PageTable.new()
      pte = PageTableEntry.new(42, writable: true, dirty: true)
      pt = PageTable.insert(pt, 7, pte)

      assert PageTable.lookup(pt, 7) == pte
      assert PageTable.lookup(pt, 7).dirty == true
    end
  end

  # ============================================================================
  # TwoLevelPageTable Tests
  # ============================================================================

  describe "TwoLevelPageTable" do
    test "splits address 0x00012ABC correctly" do
      {vpn1, vpn0, page_offset} = TwoLevelPageTable.split_address(0x00012ABC)
      assert vpn1 == 0
      assert vpn0 == 18
      assert page_offset == 0xABC
    end

    test "splits address 0x0 correctly" do
      {vpn1, vpn0, page_offset} = TwoLevelPageTable.split_address(0)
      assert vpn1 == 0
      assert vpn0 == 0
      assert page_offset == 0
    end

    test "splits address 0xFFFFFFFF correctly" do
      {vpn1, vpn0, page_offset} = TwoLevelPageTable.split_address(0xFFFFFFFF)
      assert vpn1 == 1023
      assert vpn0 == 1023
      assert page_offset == 4095
    end

    test "splits address 0x80000000 correctly" do
      {vpn1, vpn0, page_offset} = TwoLevelPageTable.split_address(0x80000000)
      assert vpn1 == 512
      assert vpn0 == 0
      assert page_offset == 0
    end

    test "maps and translates a single page" do
      pt = TwoLevelPageTable.new()
      pt = TwoLevelPageTable.map(pt, 0x5000, 10, writable: true)

      result = TwoLevelPageTable.translate(pt, 0x5ABC)
      assert result != nil
      {phys_addr, pte} = result
      assert phys_addr == 0xAABC
      assert pte.frame_number == 10
      assert pte.writable == true
    end

    test "returns nil for unmapped addresses" do
      pt = TwoLevelPageTable.new()
      assert TwoLevelPageTable.translate(pt, 0x1000) == nil
    end

    test "handles pages in different directory entries" do
      pt =
        TwoLevelPageTable.new()
        |> TwoLevelPageTable.map(0x00001000, 1)
        |> TwoLevelPageTable.map(0x00400000, 2)

      {_phys1, pte1} = TwoLevelPageTable.translate(pt, 0x00001000)
      {_phys2, pte2} = TwoLevelPageTable.translate(pt, 0x00400000)
      assert pte1.frame_number == 1
      assert pte2.frame_number == 2
    end

    test "handles pages within the same directory entry" do
      pt =
        TwoLevelPageTable.new()
        |> TwoLevelPageTable.map(0x00001000, 10)
        |> TwoLevelPageTable.map(0x00002000, 20)

      {_p1, pte1} = TwoLevelPageTable.translate(pt, 0x00001000)
      {_p2, pte2} = TwoLevelPageTable.translate(pt, 0x00002000)
      assert pte1.frame_number == 10
      assert pte2.frame_number == 20
    end

    test "unmaps a mapped page" do
      pt = TwoLevelPageTable.new() |> TwoLevelPageTable.map(0x3000, 5)
      {pt, removed} = TwoLevelPageTable.unmap(pt, 0x3000)

      assert removed != nil
      assert removed.frame_number == 5
      assert TwoLevelPageTable.translate(pt, 0x3000) == nil
    end

    test "returns nil when unmapping non-existent page" do
      pt = TwoLevelPageTable.new()
      {_pt, removed} = TwoLevelPageTable.unmap(pt, 0x9000)
      assert removed == nil
    end

    test "lookup_pte returns PTE for mapped address" do
      pt = TwoLevelPageTable.new() |> TwoLevelPageTable.map(0x5000, 10, writable: true)
      pte = TwoLevelPageTable.lookup_pte(pt, 0x5000)
      assert pte != nil
      assert pte.frame_number == 10
      assert pte.writable == true
    end

    test "lookup_pte returns nil for unmapped address" do
      pt = TwoLevelPageTable.new()
      assert TwoLevelPageTable.lookup_pte(pt, 0xDEAD0000) == nil
    end

    test "all_mappings returns all mappings" do
      pt =
        TwoLevelPageTable.new()
        |> TwoLevelPageTable.map(0x1000, 1)
        |> TwoLevelPageTable.map(0x2000, 2)
        |> TwoLevelPageTable.map(0x3000, 3)

      mappings = TwoLevelPageTable.all_mappings(pt)
      assert length(mappings) == 3
      frames = Enum.map(mappings, fn {_vaddr, pte} -> pte.frame_number end) |> Enum.sort()
      assert frames == [1, 2, 3]
    end

    test "all_mappings returns empty list for empty table" do
      pt = TwoLevelPageTable.new()
      assert TwoLevelPageTable.all_mappings(pt) == []
    end

    test "update_pte modifies a PTE in place" do
      pt = TwoLevelPageTable.new() |> TwoLevelPageTable.map(0x1000, 5)
      pt = TwoLevelPageTable.update_pte(pt, 0x1000, fn pte -> %{pte | dirty: true} end)
      pte = TwoLevelPageTable.lookup_pte(pt, 0x1000)
      assert pte.dirty == true
    end

    test "update_pte on unmapped address returns unchanged table" do
      pt = TwoLevelPageTable.new()
      pt2 = TwoLevelPageTable.update_pte(pt, 0x1000, fn pte -> %{pte | dirty: true} end)
      assert pt2 == pt
    end
  end

  # ============================================================================
  # TLB Tests
  # ============================================================================

  describe "TLB" do
    test "misses on empty TLB" do
      tlb = TLB.new(4)
      {tlb, result} = TLB.lookup(tlb, 1, 0)
      assert result == nil
      assert tlb.misses == 1
      assert tlb.hits == 0
    end

    test "hits after inserting an entry" do
      pte = PageTableEntry.new(10)
      tlb = TLB.new(4) |> TLB.insert(1, 5, 10, pte)

      {tlb, result} = TLB.lookup(tlb, 1, 5)
      assert result != nil
      {frame, _} = result
      assert frame == 10
      assert tlb.hits == 1
    end

    test "misses for wrong pid" do
      pte = PageTableEntry.new(10)
      tlb = TLB.new(4) |> TLB.insert(1, 5, 10, pte)

      {tlb, result} = TLB.lookup(tlb, 2, 5)
      assert result == nil
      assert tlb.misses == 1
    end

    test "misses for wrong vpn" do
      pte = PageTableEntry.new(10)
      tlb = TLB.new(4) |> TLB.insert(1, 5, 10, pte)

      {tlb, result} = TLB.lookup(tlb, 1, 6)
      assert result == nil
      assert tlb.misses == 1
    end

    test "evicts LRU entry when full" do
      tlb = TLB.new(4)

      tlb =
        Enum.reduce(0..3, tlb, fn i, acc ->
          TLB.insert(acc, 1, i, i * 10, PageTableEntry.new(i * 10))
        end)

      assert TLB.size(tlb) == 4

      # Insert 5th — evicts VPN 0
      tlb = TLB.insert(tlb, 1, 99, 990, PageTableEntry.new(990))
      assert TLB.size(tlb) == 4

      {_tlb, result} = TLB.lookup(tlb, 1, 0)
      assert result == nil

      {_tlb, result} = TLB.lookup(tlb, 1, 99)
      assert result != nil
    end

    test "updates LRU order on lookup" do
      tlb = TLB.new(4)

      tlb =
        Enum.reduce(0..3, tlb, fn i, acc ->
          TLB.insert(acc, 1, i, i * 10, PageTableEntry.new(i * 10))
        end)

      # Access VPN 0 — moves to most recent
      {tlb, _} = TLB.lookup(tlb, 1, 0)

      # Insert 5th — should evict VPN 1 (now LRU)
      tlb = TLB.insert(tlb, 1, 99, 990, PageTableEntry.new(990))

      {_tlb, result0} = TLB.lookup(tlb, 1, 0)
      assert result0 != nil

      {_tlb, result1} = TLB.lookup(tlb, 1, 1)
      assert result1 == nil
    end

    test "flushes all entries" do
      tlb =
        TLB.new(4)
        |> TLB.insert(1, 0, 0, PageTableEntry.new(0))
        |> TLB.insert(1, 1, 10, PageTableEntry.new(10))
        |> TLB.insert(2, 0, 20, PageTableEntry.new(20))

      tlb = TLB.flush(tlb)
      assert TLB.size(tlb) == 0
      {_tlb, r1} = TLB.lookup(tlb, 1, 0)
      assert r1 == nil
    end

    test "invalidates a single entry" do
      tlb =
        TLB.new(4)
        |> TLB.insert(1, 0, 0, PageTableEntry.new(0))
        |> TLB.insert(1, 1, 10, PageTableEntry.new(10))

      tlb = TLB.invalidate(tlb, 1, 0)

      {_tlb, r0} = TLB.lookup(tlb, 1, 0)
      assert r0 == nil

      {_tlb, r1} = TLB.lookup(tlb, 1, 1)
      assert r1 != nil
    end

    test "calculates hit rate correctly" do
      tlb = TLB.new(4)
      assert TLB.hit_rate(tlb) == 0.0

      tlb = TLB.insert(tlb, 1, 0, 0, PageTableEntry.new(0))
      {tlb, _} = TLB.lookup(tlb, 1, 0)  # hit
      {tlb, _} = TLB.lookup(tlb, 1, 0)  # hit
      {tlb, _} = TLB.lookup(tlb, 1, 1)  # miss

      assert tlb.hits == 2
      assert tlb.misses == 1
      assert_in_delta TLB.hit_rate(tlb), 2 / 3, 0.0001
    end

    test "handles re-insertion of existing key" do
      pte1 = PageTableEntry.new(10)
      pte2 = PageTableEntry.new(20)

      tlb = TLB.new(4) |> TLB.insert(1, 5, 10, pte1) |> TLB.insert(1, 5, 20, pte2)
      assert TLB.size(tlb) == 1

      {_tlb, result} = TLB.lookup(tlb, 1, 5)
      {frame, _} = result
      assert frame == 20
    end
  end

  # ============================================================================
  # PhysicalFrameAllocator Tests
  # ============================================================================

  describe "PhysicalFrameAllocator" do
    test "starts with all frames free" do
      alloc = PhysicalFrameAllocator.new(8)
      assert alloc.free_count == 8
    end

    test "allocates sequential frame numbers" do
      alloc = PhysicalFrameAllocator.new(8)
      {alloc, f0} = PhysicalFrameAllocator.allocate(alloc)
      {alloc, f1} = PhysicalFrameAllocator.allocate(alloc)
      {_alloc, f2} = PhysicalFrameAllocator.allocate(alloc)

      assert f0 == 0
      assert f1 == 1
      assert f2 == 2
    end

    test "returns nil when all frames allocated" do
      alloc = PhysicalFrameAllocator.new(2)
      {alloc, _} = PhysicalFrameAllocator.allocate(alloc)
      {alloc, _} = PhysicalFrameAllocator.allocate(alloc)
      {_alloc, frame} = PhysicalFrameAllocator.allocate(alloc)
      assert frame == nil
    end

    test "frees a frame and makes it available again" do
      alloc = PhysicalFrameAllocator.new(8)
      {alloc, frame} = PhysicalFrameAllocator.allocate(alloc)
      assert alloc.free_count == 7

      alloc = PhysicalFrameAllocator.free(alloc, frame)
      assert alloc.free_count == 8
      assert PhysicalFrameAllocator.is_allocated(alloc, frame) == false
    end

    test "reuses freed frames" do
      alloc = PhysicalFrameAllocator.new(8)
      {alloc, _} = PhysicalFrameAllocator.allocate(alloc)  # 0
      {alloc, _} = PhysicalFrameAllocator.allocate(alloc)  # 1
      {alloc, _} = PhysicalFrameAllocator.allocate(alloc)  # 2

      alloc = PhysicalFrameAllocator.free(alloc, 1)
      {_alloc, reused} = PhysicalFrameAllocator.allocate(alloc)
      assert reused == 1
    end

    test "raises on double-free" do
      alloc = PhysicalFrameAllocator.new(8)
      {alloc, frame} = PhysicalFrameAllocator.allocate(alloc)
      alloc = PhysicalFrameAllocator.free(alloc, frame)

      assert_raise RuntimeError, ~r/Double-free/, fn ->
        PhysicalFrameAllocator.free(alloc, frame)
      end
    end

    test "raises on out-of-range frame number" do
      alloc = PhysicalFrameAllocator.new(8)

      assert_raise RuntimeError, ~r/out of range/, fn ->
        PhysicalFrameAllocator.free(alloc, 99)
      end

      assert_raise RuntimeError, ~r/out of range/, fn ->
        PhysicalFrameAllocator.is_allocated(alloc, 99)
      end
    end

    test "reports allocation status correctly" do
      alloc = PhysicalFrameAllocator.new(8)
      {alloc, _} = PhysicalFrameAllocator.allocate(alloc)

      assert PhysicalFrameAllocator.is_allocated(alloc, 0) == true
      assert PhysicalFrameAllocator.is_allocated(alloc, 1) == false
    end
  end

  # ============================================================================
  # FIFO Policy Tests
  # ============================================================================

  describe "FIFOPolicy" do
    test "returns nil when empty" do
      policy = FIFOPolicy.new()
      {_policy, victim} = FIFOPolicy.select_victim(policy)
      assert victim == nil
    end

    test "evicts in FIFO order" do
      policy =
        FIFOPolicy.new()
        |> FIFOPolicy.add_frame(10)
        |> FIFOPolicy.add_frame(20)
        |> FIFOPolicy.add_frame(30)

      {policy, v1} = FIFOPolicy.select_victim(policy)
      assert v1 == 10

      {policy, v2} = FIFOPolicy.select_victim(policy)
      assert v2 == 20

      {policy, v3} = FIFOPolicy.select_victim(policy)
      assert v3 == 30

      {_policy, v4} = FIFOPolicy.select_victim(policy)
      assert v4 == nil
    end

    test "ignores access patterns" do
      policy =
        FIFOPolicy.new()
        |> FIFOPolicy.add_frame(10)
        |> FIFOPolicy.add_frame(20)
        |> FIFOPolicy.add_frame(30)
        |> FIFOPolicy.record_access(10)
        |> FIFOPolicy.record_access(10)

      {_policy, victim} = FIFOPolicy.select_victim(policy)
      assert victim == 10
    end

    test "removes a specific frame" do
      policy =
        FIFOPolicy.new()
        |> FIFOPolicy.add_frame(10)
        |> FIFOPolicy.add_frame(20)
        |> FIFOPolicy.add_frame(30)
        |> FIFOPolicy.remove_frame(20)

      {policy, v1} = FIFOPolicy.select_victim(policy)
      assert v1 == 10

      {_policy, v2} = FIFOPolicy.select_victim(policy)
      assert v2 == 30
    end
  end

  # ============================================================================
  # LRU Policy Tests
  # ============================================================================

  describe "LRUPolicy" do
    test "returns nil when empty" do
      policy = LRUPolicy.new()
      {_policy, victim} = LRUPolicy.select_victim(policy)
      assert victim == nil
    end

    test "evicts the least recently used frame" do
      policy =
        LRUPolicy.new()
        |> LRUPolicy.add_frame(10)
        |> LRUPolicy.add_frame(20)
        |> LRUPolicy.add_frame(30)

      {_policy, victim} = LRUPolicy.select_victim(policy)
      assert victim == 10
    end

    test "updates order on access" do
      policy =
        LRUPolicy.new()
        |> LRUPolicy.add_frame(10)
        |> LRUPolicy.add_frame(20)
        |> LRUPolicy.add_frame(30)
        |> LRUPolicy.record_access(10)

      {policy, v1} = LRUPolicy.select_victim(policy)
      assert v1 == 20

      {policy, v2} = LRUPolicy.select_victim(policy)
      assert v2 == 30

      {_policy, v3} = LRUPolicy.select_victim(policy)
      assert v3 == 10
    end

    test "handles repeated access correctly" do
      policy =
        LRUPolicy.new()
        |> LRUPolicy.add_frame(10)
        |> LRUPolicy.add_frame(20)
        |> LRUPolicy.add_frame(30)
        |> LRUPolicy.record_access(20)
        |> LRUPolicy.record_access(10)
        |> LRUPolicy.record_access(30)
        |> LRUPolicy.record_access(10)

      {_policy, victim} = LRUPolicy.select_victim(policy)
      assert victim == 20
    end

    test "removes a specific frame" do
      policy =
        LRUPolicy.new()
        |> LRUPolicy.add_frame(10)
        |> LRUPolicy.add_frame(20)
        |> LRUPolicy.add_frame(30)
        |> LRUPolicy.remove_frame(10)

      {policy, v1} = LRUPolicy.select_victim(policy)
      assert v1 == 20

      {_policy, v2} = LRUPolicy.select_victim(policy)
      assert v2 == 30
    end
  end

  # ============================================================================
  # Clock Policy Tests
  # ============================================================================

  describe "ClockPolicy" do
    test "returns nil when empty" do
      policy = ClockPolicy.new()
      {_policy, victim} = ClockPolicy.select_victim(policy)
      assert victim == nil
    end

    test "evicts a frame after clearing use bits" do
      policy =
        ClockPolicy.new()
        |> ClockPolicy.add_frame(10)
        |> ClockPolicy.add_frame(20)
        |> ClockPolicy.add_frame(30)

      # All use bits set. Clock sweeps: clear 10, clear 20, clear 30, evict 10
      {_policy, victim} = ClockPolicy.select_victim(policy)
      assert victim == 10
    end

    test "respects use bits" do
      policy =
        ClockPolicy.new()
        |> ClockPolicy.add_frame(10)
        |> ClockPolicy.add_frame(20)
        |> ClockPolicy.add_frame(30)

      # Force a sweep to clear bits and evict 10
      {policy, _} = ClockPolicy.select_victim(policy)

      # 20 and 30 remain with use=0. Access 20 to set its use bit.
      policy = ClockPolicy.record_access(policy, 20)

      # select_victim: 20 (use=1, clear), 30 (use=0, evict)
      {_policy, victim} = ClockPolicy.select_victim(policy)
      assert victim == 30
    end

    test "removes a specific frame" do
      policy =
        ClockPolicy.new()
        |> ClockPolicy.add_frame(10)
        |> ClockPolicy.add_frame(20)
        |> ClockPolicy.add_frame(30)
        |> ClockPolicy.remove_frame(20)

      # 10 and 30 remain. Sweep: clear 10, clear 30, evict 10.
      {policy, v1} = ClockPolicy.select_victim(policy)
      assert v1 == 10

      {_policy, v2} = ClockPolicy.select_victim(policy)
      assert v2 == 30
    end

    test "handles single frame" do
      policy = ClockPolicy.new() |> ClockPolicy.add_frame(42)

      {policy, victim} = ClockPolicy.select_victim(policy)
      assert victim == 42

      {_policy, victim2} = ClockPolicy.select_victim(policy)
      assert victim2 == nil
    end
  end

  # ============================================================================
  # MMU Tests
  # ============================================================================

  describe "MMU" do
    test "creates an address space" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      assert MMU.get_page_table(mmu, 1) != nil
    end

    test "destroys an address space" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      mmu = MMU.destroy_address_space(mmu, 1)
      assert MMU.get_page_table(mmu, 1) == nil
    end

    test "frees frames when destroying address space" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      initial_free = mmu.frame_allocator.free_count

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000)
      {mmu, _} = MMU.map_page(mmu, 1, 0x2000)
      assert mmu.frame_allocator.free_count == initial_free - 2

      mmu = MMU.destroy_address_space(mmu, 1)
      assert mmu.frame_allocator.free_count == initial_free
    end

    test "handles destroying non-existent address space" do
      mmu = MMU.new(16, :lru, 4)
      mmu2 = MMU.destroy_address_space(mmu, 999)
      assert mmu2 == mmu
    end

    test "maps a page and returns frame number" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {_mmu, frame} = MMU.map_page(mmu, 1, 0x5000, writable: true)
      assert frame != nil
      assert frame >= 0
    end

    test "raises when mapping without address space" do
      mmu = MMU.new(16, :lru, 4)
      assert_raise RuntimeError, ~r/No address space/, fn ->
        MMU.map_page(mmu, 999, 0x1000)
      end
    end

    test "translates a mapped address" do

      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, frame} = MMU.map_page(mmu, 1, 0x5000, writable: true)

      {_mmu, phys} = MMU.translate(mmu, 1, 0x5ABC)
      assert phys == ((frame <<< 12) ||| 0xABC)
    end

    test "uses TLB cache on second access" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)

      {mmu, _} = MMU.translate(mmu, 1, 0x1000)
      assert mmu.tlb.misses == 1
      assert mmu.tlb.hits == 0

      {mmu, _} = MMU.translate(mmu, 1, 0x1000)
      assert mmu.tlb.hits == 1
    end

    test "handles page fault for unmapped address" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {_mmu, phys} = MMU.translate(mmu, 1, 0x9000)
      assert is_integer(phys)
    end

    test "sets dirty bit on write" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, _} = MMU.map_page(mmu, 1, 0x3000, writable: true)

      {mmu, _} = MMU.translate(mmu, 1, 0x3000, true)

      pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x3000)
      assert pte.dirty == true
      assert pte.accessed == true
    end

    test "sets accessed bit on read" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, _} = MMU.map_page(mmu, 1, 0x4000, writable: true)

      {mmu, _} = MMU.translate(mmu, 1, 0x4000, false)

      pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x4000)
      assert pte.accessed == true
    end

    test "raises for non-existent address space on translate" do
      mmu = MMU.new(16, :lru, 4)
      assert_raise RuntimeError, ~r/No address space/, fn ->
        MMU.translate(mmu, 999, 0x1000)
      end
    end

    test "clones all mappings with COW" do
      mmu =
        MMU.new(16, :lru, 4)
        |> MMU.create_address_space(1)

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      {mmu, _} = MMU.map_page(mmu, 1, 0x2000, writable: true)

      mmu = MMU.clone_address_space(mmu, 1, 2)

      child_pt = MMU.get_page_table(mmu, 2)
      assert child_pt != nil

      parent_pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x1000)
      child_pte = TwoLevelPageTable.lookup_pte(child_pt, 0x1000)

      assert child_pte != nil
      assert child_pte.frame_number == parent_pte.frame_number
    end

    test "marks shared pages as read-only after clone" do
      mmu =
        MMU.new(16, :lru, 4)
        |> MMU.create_address_space(1)

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      mmu = MMU.clone_address_space(mmu, 1, 2)

      parent_pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x1000)
      child_pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 2), 0x1000)

      assert parent_pte.writable == false
      assert child_pte.writable == false
    end

    test "performs COW copy on write after fork" do
      mmu =
        MMU.new(16, :lru, 4)
        |> MMU.create_address_space(1)

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      original_frame = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x1000).frame_number

      mmu = MMU.clone_address_space(mmu, 1, 2)

      # Write to child — triggers COW
      {mmu, _} = MMU.translate(mmu, 2, 0x1000, true)

      child_pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 2), 0x1000)
      assert child_pte.frame_number != original_frame
      assert child_pte.writable == true
      assert child_pte.dirty == true

      parent_pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x1000)
      assert parent_pte.frame_number == original_frame
    end

    test "raises when cloning from non-existent address space" do
      mmu = MMU.new(16, :lru, 4)
      assert_raise RuntimeError, ~r/No address space/, fn ->
        MMU.clone_address_space(mmu, 999, 2)
      end
    end

    test "flushes TLB on context switch" do
      mmu =
        MMU.new(16, :lru, 4)
        |> MMU.create_address_space(1)

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      {mmu, _} = MMU.translate(mmu, 1, 0x1000)
      assert TLB.size(mmu.tlb) > 0

      mmu = MMU.context_switch(mmu, 2)
      assert TLB.size(mmu.tlb) == 0
    end

    test "causes TLB misses after context switch" do
      mmu =
        MMU.new(16, :lru, 4)
        |> MMU.create_address_space(1)

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      {mmu, _} = MMU.translate(mmu, 1, 0x1000)  # miss
      {mmu, _} = MMU.translate(mmu, 1, 0x1000)  # hit

      mmu = MMU.context_switch(mmu, 1)

      {mmu, _} = MMU.translate(mmu, 1, 0x1000)  # miss again
      assert mmu.tlb.misses == 2
    end

    test "allocates a frame on page fault" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      initial_free = mmu.frame_allocator.free_count

      {mmu, _} = MMU.translate(mmu, 1, 0x7000)
      assert mmu.frame_allocator.free_count == initial_free - 1
    end

    test "maps the page after fault" do
      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, _} = MMU.translate(mmu, 1, 0x8000)

      pte = TwoLevelPageTable.lookup_pte(MMU.get_page_table(mmu, 1), 0x8000)
      assert pte != nil
      assert pte.present == true
    end

    test "evicts pages when out of frames" do
      mmu = MMU.new(4, :fifo, 4) |> MMU.create_address_space(1)

      {mmu, _} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      {mmu, _} = MMU.map_page(mmu, 1, 0x2000, writable: true)
      {mmu, _} = MMU.map_page(mmu, 1, 0x3000, writable: true)
      {mmu, _} = MMU.map_page(mmu, 1, 0x4000, writable: true)

      assert mmu.frame_allocator.free_count == 0

      {_mmu, frame} = MMU.map_page(mmu, 1, 0x5000, writable: true)
      assert frame != nil
    end

    test "translates address 0x5ABC to correct physical address" do

      mmu = MMU.new(16, :lru, 4) |> MMU.create_address_space(1)
      {mmu, frame} = MMU.map_page(mmu, 1, 0x5000)

      {_mmu, phys} = MMU.translate(mmu, 1, 0x5ABC)
      assert phys == ((frame <<< 12) ||| 0xABC)
    end

    test "handles multiple processes independently" do
      mmu =
        MMU.new(16, :lru, 4)
        |> MMU.create_address_space(1)
        |> MMU.create_address_space(2)

      {mmu, frame1} = MMU.map_page(mmu, 1, 0x1000, writable: true)
      {mmu, frame2} = MMU.map_page(mmu, 2, 0x1000, writable: true)

      assert frame1 != frame2

      {mmu, phys1} = MMU.translate(mmu, 1, 0x1000)
      {_mmu, phys2} = MMU.translate(mmu, 2, 0x1000)

      assert phys1 != phys2
    end
  end
end
