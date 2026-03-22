"""TwoLevelPageTable — RISC-V Sv32 two-level page table.

A single flat page table for a 32-bit address space would need 2^20 entries
(over one million). Even at 4 bytes each, that is 4 MB per process — wasteful
when most processes use only a tiny fraction of the address space.

The solution is a HIERARCHICAL page table. Instead of one giant table, we use
two levels of smaller tables:

    Level 1 (Page Directory): 1024 entries, each pointing to a Level 2 table.
    Level 2 (Page Table):     1024 entries, each holding a PTE.

Only Level 2 tables that are actually needed get allocated. A process using
just a few pages might need only 2-3 Level 2 tables instead of the full
1024.

Address Splitting (Sv32)
========================

A 32-bit virtual address is split into three fields:

    +------------+------------+----------------+
    | VPN[1]     | VPN[0]     | Page Offset    |
    | bits 31-22 | bits 21-12 | bits 11-0      |
    | (10 bits)  | (10 bits)  | (12 bits)      |
    +------------+------------+----------------+

    VPN[1]: Index into the page directory (Level 1).
            10 bits -> 1024 entries -> each covers 4 MB of address space.

    VPN[0]: Index into the page table (Level 2).
            10 bits -> 1024 entries -> each covers 4 KB (one page).

    Offset: Byte position within the 4 KB page.
            12 bits -> 4096 bytes.

    Total: 10 + 10 + 12 = 32 bits -> 4 GB address space.

Translation Walk
================

To translate virtual address 0x00812ABC:

    Step 1: Extract VPN[1] = (0x00812ABC >> 22) & 0x3FF = 0x002 = 2
    Step 2: Extract VPN[0] = (0x00812ABC >> 12) & 0x3FF = 0x012 = 18
    Step 3: Extract offset = 0x00812ABC & 0xFFF = 0xABC = 2748

    Step 4: Look up directory[2] -> get Level 2 page table
    Step 5: Look up page_table[18] -> get PTE
    Step 6: physical_addr = (PTE.frame_number << 12) | 0xABC

    +------------------+          +------------------+
    | Page Directory   |          | Page Table       |
    | (Level 1)        |          | (Level 2)        |
    |                  |          |                  |
    | entry[0] = None  |          | entry[0] = ...   |
    | entry[1] = None  |          | ...              |
    | entry[2] --------|--------> | entry[18] = PTE  |---> frame 42
    | entry[3] = None  |          | ...              |
    | ...              |          | entry[1023]=...  |
    | entry[1023]=None |          +------------------+
    +------------------+
"""

from virtual_memory.page_table import PageTable
from virtual_memory.page_table_entry import PAGE_OFFSET_BITS, PageTableEntry

# =============================================================================
# Constants for Sv32 address splitting
# =============================================================================

L1_BITS: int = 10
"""Number of bits in the Level 1 (page directory) index.
10 bits -> 1024 entries in the directory."""

L2_BITS: int = 10
"""Number of bits in the Level 2 (page table) index.
10 bits -> 1024 entries per page table."""

L1_ENTRIES: int = 1 << L1_BITS  # 1024
"""Number of entries in the page directory."""

L2_ENTRIES: int = 1 << L2_BITS  # 1024
"""Number of entries in each page table."""

L1_SHIFT: int = PAGE_OFFSET_BITS + L2_BITS  # 22
"""How far to shift a virtual address right to extract the L1 index.
22 = 12 (offset) + 10 (L2 bits)."""

L2_SHIFT: int = PAGE_OFFSET_BITS  # 12
"""How far to shift to extract the L2 index. Same as the page offset width."""

INDEX_MASK: int = 0x3FF
"""Mask for extracting a 10-bit index: 0x3FF = 0b1111111111 = 1023."""


class TwoLevelPageTable:
    """Sv32 two-level page table with a 10-bit directory and 10-bit page tables.

    The directory is a fixed-size list of 1024 slots. Each slot is either None
    (meaning that 4 MB region of address space is unmapped) or a PageTable
    containing up to 1024 PTEs.

    This is how real RISC-V hardware organizes page tables. The CPU has a
    special register (satp) that points to the base of the page directory.
    On every memory access, the CPU hardware walks the two levels to find
    the PTE.

    Example:
        >>> pt = TwoLevelPageTable()
        >>> pt.map(0x1000, physical_frame=42)
        >>> result = pt.translate(0x1ABC)
        >>> result is not None
        True
        >>> physical_addr, pte = result
        >>> physical_addr
        0x2AABC  # (42 << 12) | 0xABC
    """

    def __init__(self) -> None:
        """Initialize the two-level page table with an empty directory.

        All 1024 directory entries start as None, meaning no regions of the
        address space are mapped yet. Level 2 tables are created on demand
        when the first page in a 4 MB region is mapped.
        """
        self._directory: list[PageTable | None] = [None] * L1_ENTRIES

    @staticmethod
    def _split_address(virtual_addr: int) -> tuple[int, int, int]:
        """Split a 32-bit virtual address into L1 index, L2 index, and offset.

        This is the fundamental operation of two-level page tables. Every
        virtual address encodes three pieces of information:

            +------------+------------+--------+
            | L1 index   | L2 index   | offset |
            | bits 31-22 | bits 21-12 | 11-0   |
            +------------+------------+--------+

        Args:
            virtual_addr: A 32-bit virtual address.

        Returns:
            Tuple of (l1_index, l2_index, offset).

        Example:
            >>> TwoLevelPageTable._split_address(0x00812ABC)
            (2, 18, 2748)
            # L1 = 2, L2 = 18 (0x12), offset = 0xABC = 2748
        """
        l1_index = (virtual_addr >> L1_SHIFT) & INDEX_MASK
        l2_index = (virtual_addr >> L2_SHIFT) & INDEX_MASK
        offset = virtual_addr & 0xFFF
        return (l1_index, l2_index, offset)

    def map(
        self,
        virtual_addr: int,
        physical_frame: int,
        writable: bool = True,
        executable: bool = False,
        user: bool = True,
    ) -> None:
        """Map a virtual address to a physical frame.

        Creates the Level 2 page table if it does not exist yet (lazy
        allocation of page table structures). Then inserts a PTE into
        the appropriate Level 2 table.

        Args:
            virtual_addr: The virtual address to map. The offset portion
                is ignored — the entire 4 KB page containing this address
                is mapped.
            physical_frame: The physical frame number to map to.
            writable: Whether writes are allowed.
            executable: Whether instruction fetch is allowed.
            user: Whether user-mode access is allowed.
        """
        l1_index, l2_index, _ = self._split_address(virtual_addr)

        # Create the Level 2 table if this is the first mapping in this
        # 4 MB region. This is "lazy allocation" — we only create page
        # table structures when they are actually needed.
        if self._directory[l1_index] is None:
            self._directory[l1_index] = PageTable()

        self._directory[l1_index].map_page(  # type: ignore[union-attr]
            vpn=l2_index,
            frame=physical_frame,
            writable=writable,
            executable=executable,
            user=user,
        )

    def unmap(self, virtual_addr: int) -> PageTableEntry | None:
        """Remove the mapping for the page containing the given virtual address.

        Args:
            virtual_addr: The virtual address whose page mapping to remove.

        Returns:
            The removed PageTableEntry, or None if the page was not mapped.
        """
        l1_index, l2_index, _ = self._split_address(virtual_addr)
        page_table = self._directory[l1_index]

        if page_table is None:
            return None

        return page_table.unmap_page(l2_index)

    def translate(self, virtual_addr: int) -> tuple[int, PageTableEntry] | None:
        """Translate a virtual address to a physical address.

        Walks both levels of the page table:
            1. Use VPN[1] to index into the directory -> find Level 2 table
            2. Use VPN[0] to index into the Level 2 table -> find PTE
            3. Combine PTE.frame_number with the page offset

        Args:
            virtual_addr: The 32-bit virtual address to translate.

        Returns:
            A tuple of (physical_address, PageTableEntry) if the mapping
            exists, or None if the virtual address is not mapped.
        """
        l1_index, l2_index, offset = self._split_address(virtual_addr)

        # Step 1: Look up the Level 2 table in the directory.
        page_table = self._directory[l1_index]
        if page_table is None:
            return None  # This 4 MB region is completely unmapped.

        # Step 2: Look up the PTE in the Level 2 table.
        pte = page_table.lookup(l2_index)
        if pte is None:
            return None  # This specific page is not mapped.

        # Step 3: Compute the physical address.
        # physical_addr = (frame_number << 12) | offset
        #
        # The frame number tells us which 4 KB chunk of physical RAM.
        # The offset tells us which byte within that chunk.
        physical_addr = (pte.frame_number << PAGE_OFFSET_BITS) | offset
        return (physical_addr, pte)

    def lookup_vpn(self, vpn: int) -> PageTableEntry | None:
        """Look up a PTE by virtual page number (as opposed to full address).

        Convenience method for when we already have the VPN extracted.

        Args:
            vpn: The 20-bit virtual page number (address >> 12).

        Returns:
            The PageTableEntry if mapped, or None.
        """
        l1_index = (vpn >> L2_BITS) & INDEX_MASK
        l2_index = vpn & INDEX_MASK

        page_table = self._directory[l1_index]
        if page_table is None:
            return None

        return page_table.lookup(l2_index)

    def map_vpn(
        self,
        vpn: int,
        physical_frame: int,
        writable: bool = True,
        executable: bool = False,
        user: bool = True,
    ) -> None:
        """Map a virtual page number to a physical frame.

        Convenience method for when we already have the VPN extracted.

        Args:
            vpn: The 20-bit virtual page number.
            physical_frame: The physical frame number.
            writable: Whether writes are allowed.
            executable: Whether instruction fetch is allowed.
            user: Whether user-mode access is allowed.
        """
        # Convert VPN back to a virtual address (offset = 0) and delegate.
        virtual_addr = vpn << PAGE_OFFSET_BITS
        self.map(virtual_addr, physical_frame, writable, executable, user)

    @property
    def directory(self) -> list[PageTable | None]:
        """Access the raw directory for iteration (used during fork/clone)."""
        return self._directory
