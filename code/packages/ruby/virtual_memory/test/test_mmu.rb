# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    class TestMMU < Minitest::Test
      def setup
        @mmu = MMU.new(total_frames: 16, replacement_policy: FIFOPolicy.new)
      end

      # == Create Address Space ==
      #
      # Each process gets its own page table.

      def test_create_address_space
        @mmu.create_address_space(1)
        assert @mmu.address_space?(1)
        refute @mmu.address_space?(2)
      end

      # == Map and Translate ==
      #
      # The core workflow: map a page, then translate an address on it.

      def test_map_and_translate
        @mmu.create_address_space(1)
        frame = @mmu.map_page(1, 0x5000)

        # Translate address 0x5ABC → (frame << 12) | 0xABC
        phys = @mmu.translate(1, 0x5ABC)
        assert_equal((frame << 12) | 0xABC, phys)
      end

      # == Address Isolation ==
      #
      # Two processes can use the same virtual address but get
      # different physical addresses.

      def test_address_isolation
        @mmu.create_address_space(1)
        @mmu.create_address_space(2)

        frame1 = @mmu.map_page(1, 0x1000)
        frame2 = @mmu.map_page(2, 0x1000)

        phys1 = @mmu.translate(1, 0x1000)
        phys2 = @mmu.translate(2, 0x1000)

        # Same virtual address, different physical addresses.
        refute_equal phys1, phys2
        assert_equal(frame1 << 12, phys1)
        assert_equal(frame2 << 12, phys2)
      end

      # == TLB Caching ==
      #
      # The second translation of the same address should hit the TLB.

      def test_tlb_caching
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000)

        # First translate: TLB miss → page table walk → TLB insert.
        @mmu.translate(1, 0x1000)
        assert_equal 1, @mmu.tlb.misses

        # Second translate: TLB hit.
        @mmu.translate(1, 0x1000)
        assert_equal 1, @mmu.tlb.hits
      end

      # == Page Fault Handling ==
      #
      # Accessing an unmapped page triggers a page fault, which
      # allocates a frame and maps the page.

      def test_page_fault_allocates_frame
        @mmu.create_address_space(1)

        # Don't map the page first -- let the page fault handle it.
        phys = @mmu.handle_page_fault(1, 0x3000)
        refute_nil phys

        # Now the page is mapped and translatable.
        phys2 = @mmu.translate(1, 0x3000)
        assert_equal phys, phys2
      end

      # == Destroy Address Space ==
      #
      # Destroying an address space frees all frames.

      def test_destroy_address_space
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000)
        @mmu.map_page(1, 0x2000)

        free_before = @mmu.frame_allocator.free_count

        @mmu.destroy_address_space(1)

        refute @mmu.address_space?(1)
        assert_equal free_before + 2, @mmu.frame_allocator.free_count
      end

      # == Copy-on-Write Fork ==
      #
      # clone_address_space shares frames between parent and child.

      def test_clone_address_space
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000)
        @mmu.map_page(1, 0x2000)

        @mmu.clone_address_space(1, 2)

        assert @mmu.address_space?(2)

        # Both processes should translate to the same physical addresses
        # (frames are shared).
        phys1 = @mmu.translate(1, 0x1000)
        phys2 = @mmu.translate(2, 0x1000)
        assert_equal phys1, phys2
      end

      def test_cow_write_creates_private_copy
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000)

        # Translate to establish the initial mapping.
        @mmu.translate(1, 0x1000)

        @mmu.clone_address_space(1, 2)

        # Both read the same physical address (shared frame).
        phys_parent = @mmu.translate(1, 0x1000)
        phys_child = @mmu.translate(2, 0x1000)
        assert_equal phys_parent, phys_child

        # Child writes → COW fault → private copy.
        phys_child_after = @mmu.translate(2, 0x1000, write: true)

        # After COW, child has a different physical frame.
        phys_parent_after = @mmu.translate(1, 0x1000)
        refute_equal phys_child_after, phys_parent_after
      end

      # == Context Switch ==
      #
      # Context switch flushes the TLB to prevent cross-process leaks.

      def test_context_switch
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000)
        @mmu.translate(1, 0x1000)

        assert_equal 1, @mmu.tlb.size

        @mmu.context_switch(2)

        assert_equal 2, @mmu.current_pid
        assert_equal 0, @mmu.tlb.size
      end

      # == No Address Space Error ==
      #
      # Operating on a nonexistent PID raises an error.

      def test_no_address_space_error
        assert_raises(ArgumentError) { @mmu.translate(99, 0x1000) }
        assert_raises(ArgumentError) { @mmu.map_page(99, 0x1000) }
      end

      # == Destroy Nonexistent ==
      #
      # Destroying a nonexistent address space is a no-op.

      def test_destroy_nonexistent
        @mmu.destroy_address_space(99)
        # Should not raise.
      end

      # == Page Replacement Under Pressure ==
      #
      # When all frames are allocated, mapping a new page evicts a victim.

      def test_page_replacement
        mmu = MMU.new(total_frames: 4, replacement_policy: FIFOPolicy.new)
        mmu.create_address_space(1)

        # Allocate all 4 frames.
        mmu.map_page(1, 0x1000)
        mmu.map_page(1, 0x2000)
        mmu.map_page(1, 0x3000)
        mmu.map_page(1, 0x4000)
        assert_equal 0, mmu.frame_allocator.free_count

        # Mapping a 5th page should evict the oldest frame (FIFO).
        mmu.map_page(1, 0x5000)

        # The new page should be translatable.
        phys = mmu.translate(1, 0x5000)
        refute_nil phys
      end

      # == Multiple Mappings ==
      #
      # A process can have many mapped pages simultaneously.

      def test_multiple_mappings
        @mmu.create_address_space(1)

        frames = []
        5.times do |i|
          frames << @mmu.map_page(1, i * PAGE_SIZE)
        end

        5.times do |i|
          phys = @mmu.translate(1, i * PAGE_SIZE)
          assert_equal(frames[i] << PAGE_OFFSET_BITS, phys)
        end
      end

      # == Write Marks Dirty ==
      #
      # A write access should set the dirty bit on the PTE.

      def test_write_sets_dirty_bit
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000)

        @mmu.translate(1, 0x1000, write: true)

        pte = @mmu.page_table_for(1).lookup_pte(0x1000)
        assert pte.dirty?
        assert pte.accessed?
      end

      # == Permission Flags ==
      #
      # map_page accepts permission flags.

      def test_permission_flags
        @mmu.create_address_space(1)
        @mmu.map_page(1, 0x1000, writable: false, executable: true)

        pte = @mmu.page_table_for(1).lookup_pte(0x1000)
        refute pte.writable?
        assert pte.executable?
      end

      # == LRU Replacement Policy ==

      def test_lru_replacement
        mmu = MMU.new(total_frames: 3, replacement_policy: LRUPolicy.new)
        mmu.create_address_space(1)

        mmu.map_page(1, 0x1000)  # frame 0
        mmu.map_page(1, 0x2000)  # frame 1
        mmu.map_page(1, 0x3000)  # frame 2

        # Access page 0x1000 to make it recently used.
        mmu.translate(1, 0x1000)

        # Map a 4th page -- should evict frame 1 (LRU, 0x2000).
        mmu.map_page(1, 0x4000)

        # 0x1000 should still be accessible (was recently used).
        phys = mmu.translate(1, 0x1000)
        refute_nil phys
      end

      # == Clock Replacement Policy ==

      def test_clock_replacement
        mmu = MMU.new(total_frames: 3, replacement_policy: ClockPolicy.new)
        mmu.create_address_space(1)

        mmu.map_page(1, 0x1000)
        mmu.map_page(1, 0x2000)
        mmu.map_page(1, 0x3000)

        # Map a 4th page -- clock should evict one of the existing.
        mmu.map_page(1, 0x4000)

        # The new page should be translatable.
        phys = mmu.translate(1, 0x4000)
        refute_nil phys
      end

      # == Page Table For ==

      def test_page_table_for
        @mmu.create_address_space(1)
        pt = @mmu.page_table_for(1)
        assert_instance_of TwoLevelPageTable, pt
      end

      def test_page_table_for_nonexistent
        assert_nil @mmu.page_table_for(99)
      end
    end
  end
end
