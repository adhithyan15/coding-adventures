"""Virtual Memory — page tables, TLB, MMU, and page replacement policies.

This package simulates the virtual memory subsystem found in every modern
operating system. Virtual memory gives each process the illusion of having
its own private address space, even though all processes share the same
physical RAM.

Modules:
    page_table_entry       - PageTableEntry: flags and frame number for one page
    page_table             - PageTable: single-level VPN -> PTE mapping
    multi_level_page_table - TwoLevelPageTable: Sv32 two-level page table
    tlb                    - TLB: translation lookaside buffer (cache)
    frame_allocator        - PhysicalFrameAllocator: bitmap-based frame management
    replacement            - FIFO, LRU, Clock page replacement policies
    mmu                    - MMU: the central translation + fault handling unit

Quick start:
    >>> from virtual_memory import MMU, FIFOPolicy
    >>> mmu = MMU(total_frames=256)
    >>> mmu.create_address_space(pid=1)
    >>> frame = mmu.map_page(pid=1, virtual_addr=0x1000)
    >>> physical = mmu.translate(pid=1, virtual_addr=0x1ABC)
"""

from virtual_memory.frame_allocator import PhysicalFrameAllocator
from virtual_memory.mmu import MMU
from virtual_memory.multi_level_page_table import TwoLevelPageTable
from virtual_memory.page_table import PageTable
from virtual_memory.page_table_entry import (
    PAGE_OFFSET_BITS,
    PAGE_SIZE,
    VPN_BITS,
    PageTableEntry,
)
from virtual_memory.replacement import ClockPolicy, FIFOPolicy, LRUPolicy
from virtual_memory.tlb import TLB

__all__ = [
    "PAGE_SIZE",
    "PAGE_OFFSET_BITS",
    "VPN_BITS",
    "PageTableEntry",
    "PageTable",
    "TwoLevelPageTable",
    "TLB",
    "PhysicalFrameAllocator",
    "FIFOPolicy",
    "LRUPolicy",
    "ClockPolicy",
    "MMU",
]
