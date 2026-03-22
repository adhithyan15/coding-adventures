# frozen_string_literal: true

# = CodingAdventures::VirtualMemory
#
# A complete virtual memory subsystem implementing:
#   - Page table entries (PTE) with RISC-V Sv32 permission bits
#   - Single-level page tables (hash map based)
#   - Two-level page tables (Sv32: 10-bit L1 + 10-bit L2 + 12-bit offset)
#   - Translation Lookaside Buffer (TLB) with LRU eviction
#   - Physical frame allocator with bitmap and reference counting
#   - Page replacement policies: FIFO, LRU, and Clock (second-chance)
#   - Memory Management Unit (MMU) with copy-on-write fork support
#
# == Virtual Memory in a Nutshell
#
# Virtual memory gives every process the illusion of having the entire
# memory space to itself. Process A's address 0x1000 and Process B's
# address 0x1000 map to completely different physical locations.
#
# The MMU sits between the CPU and physical memory, translating every
# virtual address to a physical address before the memory access occurs.
#
# == Quick Start
#
#   require "coding_adventures_virtual_memory"
#
#   mmu = CodingAdventures::VirtualMemory::MMU.new(total_frames: 256)
#   mmu.create_address_space(1)
#   frame = mmu.map_page(1, 0x1000)
#   phys = mmu.translate(1, 0x1ABC)
#   # phys == (frame << 12) | 0xABC

module CodingAdventures
  module VirtualMemory
    # Page size in bytes. Every page and frame is exactly this size.
    # 4 KB = 4096 bytes = 2^12 bytes.
    #
    # This has been the standard page size since the Intel 386 (1985).
    # RISC-V also uses 4 KB as the base page size.
    PAGE_SIZE = 4096

    # Number of bits in the page offset (lower bits of an address).
    # 2^12 = 4096, so we need 12 bits to address every byte within a page.
    PAGE_OFFSET_BITS = 12

    # Bitmask for extracting the page offset from an address.
    # 0xFFF = 0b111111111111 = 4095
    # Usage: offset = address & PAGE_OFFSET_MASK
    PAGE_OFFSET_MASK = PAGE_SIZE - 1
  end
end

require_relative "coding_adventures/virtual_memory/version"
require_relative "coding_adventures/virtual_memory/page_table_entry"
require_relative "coding_adventures/virtual_memory/page_table"
require_relative "coding_adventures/virtual_memory/two_level_page_table"
require_relative "coding_adventures/virtual_memory/tlb"
require_relative "coding_adventures/virtual_memory/frame_allocator"
require_relative "coding_adventures/virtual_memory/replacement"
require_relative "coding_adventures/virtual_memory/mmu"
