# frozen_string_literal: true

# = Two-Level Page Table (Sv32)
#
# RISC-V's Sv32 addressing scheme uses a two-level page table to efficiently
# manage a 32-bit (4 GB) virtual address space without wasting memory on
# unmapped regions.
#
# == The Problem with Single-Level Tables
#
# A single flat page table for 32-bit addresses needs 2^20 = 1,048,576
# entries. Even at 4 bytes each, that's 4 MB per process -- wasteful when
# most processes only use a few MB of their 4 GB address space.
#
# == The Two-Level Solution
#
# Split the 20-bit VPN into two 10-bit indices:
#
#   32-bit virtual address:
#   +------------+------------+----------------+
#   | VPN[1]     | VPN[0]     | Page Offset    |
#   | bits 31-22 | bits 21-12 | bits 11-0      |
#   | (10 bits)  | (10 bits)  | (12 bits)      |
#   +------------+------------+----------------+
#
#   VPN[1] selects one of 1024 entries in the PAGE DIRECTORY.
#   Each directory entry points to a second-level PAGE TABLE.
#   VPN[0] selects one of 1024 entries in that page table.
#
# == Memory Savings
#
# The directory always exists (1024 entries = 4 KB). But second-level
# tables are only created when needed. A process using 8 MB of memory
# needs only 2 second-level tables (each covers 4 MB = 1024 pages * 4 KB).
# Total overhead: 4 KB (directory) + 2 * 4 KB (tables) = 12 KB.
# Compare to 4 MB for a flat table!
#
# == Translation Walk
#
#   1. Extract VPN[1] = (vpn >> 10) & 0x3FF
#   2. Look up directory[VPN[1]] -- is there a second-level table?
#   3. If no table exists, the page is unmapped (page fault).
#   4. Extract VPN[0] = vpn & 0x3FF
#   5. Look up table[VPN[0]] -- get the PTE.

module CodingAdventures
  module VirtualMemory
    # Number of entries in the page directory (2^10 = 1024).
    # Each entry covers 4 MB of virtual address space (1024 pages * 4 KB).
    DIRECTORY_ENTRIES = 1024

    class TwoLevelPageTable
      # The page directory: an array of 1024 slots, each either nil
      # (that 4 MB region is unmapped) or a PageTable (second-level table).
      attr_reader :directory

      def initialize
        # Initialize all 1024 directory entries to nil.
        # Second-level tables are created on demand when a page in
        # that region is first mapped.
        @directory = Array.new(DIRECTORY_ENTRIES)
      end

      # Map a virtual address to a physical frame with the given flags.
      #
      # This creates a PTE in the appropriate second-level page table.
      # If the second-level table doesn't exist yet, it is created.
      #
      # @param vaddr [Integer] the 32-bit virtual address
      # @param frame_number [Integer] the physical frame to map to
      # @param writable [Boolean] whether the page is writable
      # @param executable [Boolean] whether the page is executable
      # @param user_accessible [Boolean] whether user-mode can access it
      def map(vaddr, frame_number, writable: true, executable: false, user_accessible: true)
        vpn = vaddr >> PAGE_OFFSET_BITS
        vpn1 = (vpn >> 10) & 0x3FF
        vpn0 = vpn & 0x3FF

        # Create the second-level table if it doesn't exist yet.
        # This is "lazy allocation" of page table structures --
        # we only pay for the regions actually in use.
        @directory[vpn1] ||= PageTable.new

        pte = PageTableEntry.new(
          frame_number: frame_number,
          present: true,
          writable: writable,
          executable: executable,
          user_accessible: user_accessible
        )

        @directory[vpn1].map_page(vpn0, pte)
      end

      # Remove the mapping for a virtual address.
      #
      # @param vaddr [Integer] the 32-bit virtual address to unmap
      # @return [PageTableEntry, nil] the removed PTE, or nil if not mapped
      def unmap(vaddr)
        vpn = vaddr >> PAGE_OFFSET_BITS
        vpn1 = (vpn >> 10) & 0x3FF
        vpn0 = vpn & 0x3FF

        table = @directory[vpn1]
        return nil if table.nil?

        result = table.unmap_page(vpn0)

        # If the second-level table is now empty, free it.
        # This reclaims memory for regions that are completely unmapped.
        @directory[vpn1] = nil if table.mapped_count == 0

        result
      end

      # Translate a virtual address to a physical address.
      #
      # Walks the two-level page table structure:
      #   1. Use VPN[1] to find the second-level table in the directory.
      #   2. Use VPN[0] to find the PTE in that table.
      #   3. Combine the frame number with the page offset.
      #
      # @param vaddr [Integer] the 32-bit virtual address
      # @return [Array(Integer, PageTableEntry), nil] the physical address
      #   and the PTE, or nil if the page is not mapped
      def translate(vaddr)
        vpn = vaddr >> PAGE_OFFSET_BITS
        offset = vaddr & PAGE_OFFSET_MASK
        vpn1 = (vpn >> 10) & 0x3FF
        vpn0 = vpn & 0x3FF

        # Step 1: Look up the directory entry
        table = @directory[vpn1]
        return nil if table.nil?

        # Step 2: Look up the page table entry
        pte = table.lookup(vpn0)
        return nil if pte.nil?
        return nil unless pte.present?

        # Step 3: Compute the physical address
        phys_addr = (pte.frame_number << PAGE_OFFSET_BITS) | offset
        [phys_addr, pte]
      end

      # Look up the PTE for a virtual address without computing the
      # physical address. Useful for checking permissions or modifying
      # PTE flags.
      #
      # @param vaddr [Integer] the 32-bit virtual address
      # @return [PageTableEntry, nil] the PTE, or nil if not mapped
      def lookup_pte(vaddr)
        vpn = vaddr >> PAGE_OFFSET_BITS
        vpn1 = (vpn >> 10) & 0x3FF
        vpn0 = vpn & 0x3FF

        table = @directory[vpn1]
        return nil if table.nil?

        table.lookup(vpn0)
      end

      # Count the total number of mapped pages across all second-level tables.
      #
      # @return [Integer] total mapped page count
      def mapped_count
        @directory.compact.sum(&:mapped_count)
      end
    end
  end
end
