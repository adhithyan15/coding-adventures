# frozen_string_literal: true

# = Single-Level Page Table
#
# The simplest page table implementation: a hash map from virtual page
# number (VPN) to page table entry (PTE).
#
# == How it works
#
# A process's virtual address space is divided into fixed-size pages
# (4 KB each). Each page has a number: page 0 covers addresses 0x0000-0x0FFF,
# page 1 covers 0x1000-0x1FFF, and so on.
#
# The page table maps each virtual page number to a PTE that says:
#   - Which physical frame does this page live in?
#   - What permissions does it have?
#   - Is it currently in memory?
#
# == Why a hash map?
#
# Real hardware uses arrays (for O(1) indexed lookup by VPN), but a hash
# map is more memory-efficient for sparse address spaces. Most processes
# only use a tiny fraction of their 2^20 possible pages. A hash map only
# stores entries for pages that are actually mapped.
#
# == Example
#
#   table = PageTable.new
#   table.map_page(5, PageTableEntry.new(frame_number: 10, present: true))
#   entry = table.lookup(5)
#   entry.frame_number  # => 10
#
#   table.unmap_page(5)
#   table.lookup(5)  # => nil

module CodingAdventures
  module VirtualMemory
    class PageTable
      # Create a new, empty page table.
      #
      # The entries hash starts empty -- no virtual pages are mapped.
      # Pages are added via map_page as the process allocates memory.
      def initialize
        @entries = {}
      end

      # Map a virtual page number to a page table entry.
      #
      # This creates (or overwrites) the mapping for the given VPN.
      # After this call, looking up the VPN will return the given PTE.
      #
      # @param vpn [Integer] the virtual page number to map
      # @param pte [PageTableEntry] the entry describing the mapping
      def map_page(vpn, pte)
        @entries[vpn] = pte
      end

      # Remove the mapping for a virtual page number.
      #
      # After this call, looking up the VPN will return nil. The physical
      # frame is NOT automatically freed -- that's the caller's
      # responsibility (typically the MMU handles this).
      #
      # @param vpn [Integer] the virtual page number to unmap
      # @return [PageTableEntry, nil] the removed entry, or nil if not mapped
      def unmap_page(vpn)
        @entries.delete(vpn)
      end

      # Look up the PTE for a virtual page number.
      #
      # @param vpn [Integer] the virtual page number to look up
      # @return [PageTableEntry, nil] the entry, or nil if not mapped
      def lookup(vpn)
        @entries[vpn]
      end

      # How many pages are currently mapped in this table?
      #
      # @return [Integer] the number of mapped pages
      def mapped_count
        @entries.size
      end

      # Return all virtual page numbers that are currently mapped.
      #
      # Useful for iterating over a process's entire address space,
      # e.g., during fork (to copy all mappings) or exit (to free
      # all frames).
      #
      # @return [Array<Integer>] the mapped VPNs
      def mapped_vpns
        @entries.keys
      end

      # Return all entries as an enumerable.
      #
      # @return [Hash] vpn => PTE mappings
      def entries
        @entries
      end
    end
  end
end
