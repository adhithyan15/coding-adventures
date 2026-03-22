# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    class TestPageTable < Minitest::Test
      def setup
        @table = PageTable.new
      end

      # == Empty Table ==
      #
      # A freshly created page table has no mappings. Looking up
      # any VPN returns nil.

      def test_empty_table
        assert_equal 0, @table.mapped_count
        assert_nil @table.lookup(0)
        assert_nil @table.lookup(100)
      end

      # == Basic Mapping ==
      #
      # map_page creates a VPN -> PTE mapping. After mapping,
      # lookup returns the PTE.

      def test_map_and_lookup
        pte = PageTableEntry.new(frame_number: 10, present: true)
        @table.map_page(5, pte)

        result = @table.lookup(5)
        refute_nil result
        assert_equal 10, result.frame_number
        assert result.present?
      end

      # == Multiple Mappings ==
      #
      # A page table can hold many mappings simultaneously.
      # Each VPN maps independently.

      def test_multiple_mappings
        @table.map_page(0, PageTableEntry.new(frame_number: 100, present: true))
        @table.map_page(1, PageTableEntry.new(frame_number: 200, present: true))
        @table.map_page(2, PageTableEntry.new(frame_number: 300, present: true))

        assert_equal 3, @table.mapped_count
        assert_equal 100, @table.lookup(0).frame_number
        assert_equal 200, @table.lookup(1).frame_number
        assert_equal 300, @table.lookup(2).frame_number
      end

      # == Overwrite Mapping ==
      #
      # Mapping a VPN that already has an entry overwrites the old
      # mapping. This happens during page fault resolution (remapping
      # to a different frame).

      def test_overwrite_mapping
        @table.map_page(5, PageTableEntry.new(frame_number: 10, present: true))
        @table.map_page(5, PageTableEntry.new(frame_number: 20, present: true))

        assert_equal 1, @table.mapped_count
        assert_equal 20, @table.lookup(5).frame_number
      end

      # == Unmap Page ==
      #
      # unmap_page removes a mapping. The returned PTE can be used
      # by the caller to free the physical frame.

      def test_unmap_page
        pte = PageTableEntry.new(frame_number: 10, present: true)
        @table.map_page(5, pte)

        removed = @table.unmap_page(5)
        refute_nil removed
        assert_equal 10, removed.frame_number

        # After unmapping, lookup returns nil.
        assert_nil @table.lookup(5)
        assert_equal 0, @table.mapped_count
      end

      # == Unmap Nonexistent ==
      #
      # Unmapping a VPN that isn't mapped returns nil without error.

      def test_unmap_nonexistent
        result = @table.unmap_page(99)
        assert_nil result
      end

      # == Lookup Nonexistent ==
      #
      # Looking up an unmapped VPN returns nil, not an error.
      # This is expected during page table walks for unmapped regions.

      def test_lookup_nonexistent
        assert_nil @table.lookup(42)
      end

      # == Mapped VPNs ==
      #
      # mapped_vpns returns all currently mapped virtual page numbers.

      def test_mapped_vpns
        @table.map_page(3, PageTableEntry.new(frame_number: 1, present: true))
        @table.map_page(7, PageTableEntry.new(frame_number: 2, present: true))

        vpns = @table.mapped_vpns.sort
        assert_equal [3, 7], vpns
      end

      # == Entries Access ==
      #
      # The entries hash is accessible for iteration (used by fork).

      def test_entries_enumeration
        @table.map_page(1, PageTableEntry.new(frame_number: 10, present: true))
        @table.map_page(2, PageTableEntry.new(frame_number: 20, present: true))

        count = 0
        @table.entries.each { |_vpn, _pte| count += 1 }
        assert_equal 2, count
      end
    end
  end
end
