# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    class TestTwoLevelPageTable < Minitest::Test
      def setup
        @table = TwoLevelPageTable.new
      end

      # == Address Splitting ==
      #
      # A 32-bit virtual address is split into three fields:
      #   VPN[1] (bits 31-22): index into the page directory
      #   VPN[0] (bits 21-12): index into the second-level page table
      #   Offset (bits 11-0):  byte within the page
      #
      # Example: address 0x12ABC
      #   VPN = 0x12ABC >> 12 = 0x12 = 18
      #   VPN[1] = (18 >> 10) & 0x3FF = 0
      #   VPN[0] = 18 & 0x3FF = 18
      #   Offset = 0x12ABC & 0xFFF = 0xABC

      def test_basic_map_and_translate
        # Map virtual address 0x12ABC to frame 7.
        @table.map(0x12ABC, 7)

        result = @table.translate(0x12ABC)
        refute_nil result

        phys_addr, pte = result
        # Physical address = (frame << 12) | offset = (7 << 12) | 0xABC = 0x7ABC
        assert_equal 0x7ABC, phys_addr
        assert_equal 7, pte.frame_number
        assert pte.present?
      end

      # == Directory Creation on Demand ==
      #
      # Second-level page tables are only created when a page in
      # that 4 MB region is first mapped. This saves memory.

      def test_directory_created_on_demand
        # Initially, all directory entries are nil.
        assert @table.directory.all?(&:nil?)

        # Mapping a page creates the corresponding second-level table.
        @table.map(0x1000, 1)

        # VPN[1] for address 0x1000 is 0, so directory[0] should exist.
        refute_nil @table.directory[0]

        # Other directory entries are still nil.
        assert_nil @table.directory[1]
      end

      # == Multiple Mappings in Same Region ==
      #
      # Pages in the same 4 MB region share a second-level table.

      def test_multiple_pages_same_region
        # These addresses all have VPN[1] = 0 (same directory entry).
        @table.map(0x0000, 10)  # VPN = 0
        @table.map(0x1000, 11)  # VPN = 1
        @table.map(0x2000, 12)  # VPN = 2

        assert_equal 3, @table.mapped_count

        assert_equal 0x0000 | (10 << 12), @table.translate(0x0000)[0]
        # Actually let me compute: translate(0x0000) -> frame 10, offset 0 -> 0xA000
        result0 = @table.translate(0x0000)
        assert_equal(10 << 12, result0[0])  # 0xA000

        result1 = @table.translate(0x1000)
        assert_equal(11 << 12, result1[0])  # 0xB000

        result2 = @table.translate(0x2000)
        assert_equal(12 << 12, result2[0])  # 0xC000
      end

      # == Mappings Across Regions ==
      #
      # Pages in different 4 MB regions use different directory entries.

      def test_mappings_across_regions
        # VPN[1] = 0 for address 0x1000
        @table.map(0x1000, 5)

        # VPN[1] = 1 for address 0x400000 (4 MB boundary)
        @table.map(0x400000, 6)

        assert_equal 2, @table.mapped_count

        result1 = @table.translate(0x1000)
        assert_equal((5 << 12), result1[0])

        result2 = @table.translate(0x400000)
        assert_equal((6 << 12), result2[0])
      end

      # == Offset Preservation ==
      #
      # The page offset is carried through to the physical address.
      # Only the page number changes; the offset stays the same.

      def test_offset_preserved_in_translation
        @table.map(0x5000, 3)  # Map VPN 5 to frame 3

        # Access with offset 0x123 within the page.
        result = @table.translate(0x5123)
        assert_equal((3 << 12) | 0x123, result[0])  # 0x3123
      end

      # == Unmapping ==
      #
      # Unmapping a page removes the PTE. If the second-level table
      # becomes empty, it is freed.

      def test_unmap
        @table.map(0x3000, 8)
        assert_equal 1, @table.mapped_count

        result = @table.unmap(0x3000)
        refute_nil result

        assert_equal 0, @table.mapped_count
        assert_nil @table.translate(0x3000)
      end

      def test_unmap_frees_empty_second_level_table
        @table.map(0x1000, 5)
        vpn1 = 0  # For address 0x1000

        refute_nil @table.directory[vpn1]

        @table.unmap(0x1000)

        # The second-level table is empty, so it should be freed.
        assert_nil @table.directory[vpn1]
      end

      def test_unmap_nonexistent
        result = @table.unmap(0x9000)
        assert_nil result
      end

      # == Permission Flags ==
      #
      # The map method accepts permission flags that are stored in the PTE.

      def test_permission_flags
        @table.map(0x1000, 5, writable: false, executable: true, user_accessible: false)

        pte = @table.lookup_pte(0x1000)
        refute pte.writable?
        assert pte.executable?
        refute pte.user_accessible?
      end

      # == Translate Unmapped Address ==
      #
      # Translating an unmapped address returns nil (triggers page fault).

      def test_translate_unmapped_address
        assert_nil @table.translate(0x1000)
      end

      # == Not Present Page ==
      #
      # A page that exists in the table but is not present returns nil
      # from translate (but the PTE is accessible via lookup_pte).

      def test_translate_not_present_page
        @table.map(0x1000, 5)
        pte = @table.lookup_pte(0x1000)
        pte.present = false

        assert_nil @table.translate(0x1000)
      end

      # == Lookup PTE ==
      #
      # lookup_pte returns the PTE without computing a physical address.

      def test_lookup_pte
        @table.map(0x2000, 7, writable: true)

        pte = @table.lookup_pte(0x2000)
        refute_nil pte
        assert_equal 7, pte.frame_number
        assert pte.writable?
      end

      def test_lookup_pte_nonexistent
        assert_nil @table.lookup_pte(0x9000)
      end

      # == Large Address Space ==
      #
      # Verify that high addresses (near 4 GB) work correctly.

      def test_high_address
        # Near the top of the 32-bit address space.
        high_addr = 0xFFFFF000  # VPN = 0xFFFFF
        @table.map(high_addr, 100)

        result = @table.translate(high_addr)
        refute_nil result
        assert_equal(100 << 12, result[0])
      end
    end
  end
end
