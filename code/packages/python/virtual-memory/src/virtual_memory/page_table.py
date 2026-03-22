"""PageTable — a single-level page table mapping virtual page numbers to PTEs.

A page table is the data structure the operating system uses to record which
virtual page maps to which physical frame. It is conceptually a dictionary:

    { virtual_page_number: PageTableEntry }

In real hardware, page tables are stored in physical memory and the CPU walks
them automatically during address translation. Our simulation uses a Python
dictionary for clarity, but the semantics are the same.

Why a dictionary instead of an array?
--------------------------------------
A 32-bit address space has 2^20 = 1,048,576 possible virtual pages. A flat
array of that many entries would consume ~4 MB per process even if only a
handful of pages are actually mapped. A dictionary only stores entries for
pages that are actually in use — much more memory-efficient.

Real hardware uses multi-level page tables (see TwoLevelPageTable) to achieve
a similar space savings with array-based structures that the CPU's MMU hardware
can walk efficiently.

This single-level page table is the simpler educational version. It serves as
the building block for the two-level page table.
"""

from virtual_memory.page_table_entry import PageTableEntry


class PageTable:
    """Single-level page table: a dictionary mapping VPN -> PTE.

    This is the simplest possible page table implementation. Every virtual
    page that is mapped has an entry; unmapped pages simply have no entry
    in the dictionary.

    Example:
        >>> pt = PageTable()
        >>> pt.map_page(vpn=5, frame=42)
        >>> pte = pt.lookup(vpn=5)
        >>> pte.frame_number
        42
        >>> pte.present
        True
        >>> pt.lookup(vpn=99) is None  # unmapped page
        True
    """

    def __init__(self) -> None:
        """Initialize an empty page table.

        The internal dictionary starts empty. Pages are added via map_page()
        as the process allocates memory.
        """
        self._entries: dict[int, PageTableEntry] = {}

    def map_page(
        self,
        vpn: int,
        frame: int,
        writable: bool = True,
        executable: bool = False,
        user: bool = True,
    ) -> None:
        """Create a mapping from virtual page number to physical frame.

        This is called when the OS allocates a new page for a process.
        It creates a PTE with the given permissions and marks it as present
        (meaning the page is currently in physical memory).

        Args:
            vpn: Virtual page number to map. This is the upper 20 bits of
                the virtual address, identifying which 4 KB page.
            frame: Physical frame number to map to. This identifies which
                4 KB chunk of physical RAM holds the page's data.
            writable: Whether the process can write to this page.
                Code sections are typically read-only; stack/heap are writable.
            executable: Whether the CPU can fetch instructions from this page.
                Only code pages should be executable (NX bit protection).
            user: Whether user-mode code can access this page.
                Kernel pages set this to False.
        """
        self._entries[vpn] = PageTableEntry(
            frame_number=frame,
            present=True,
            writable=writable,
            executable=executable,
            user_accessible=user,
        )

    def unmap_page(self, vpn: int) -> PageTableEntry | None:
        """Remove a mapping for the given virtual page number.

        Called when a page is freed (e.g., munmap, process exit). Returns
        the removed PTE so the caller can free the physical frame if needed.

        Args:
            vpn: Virtual page number to unmap.

        Returns:
            The removed PageTableEntry, or None if the VPN was not mapped.
        """
        return self._entries.pop(vpn, None)

    def lookup(self, vpn: int) -> PageTableEntry | None:
        """Look up the PTE for a virtual page number.

        This is the core operation during address translation. The MMU
        calls this to find out which physical frame a virtual page maps to
        and what permissions it has.

        Args:
            vpn: Virtual page number to look up.

        Returns:
            The PageTableEntry if the page is mapped, or None if not.
            Note: even if a PTE is returned, the page might not be present
            in memory (pte.present could be False, meaning a page fault
            is needed).
        """
        return self._entries.get(vpn)

    def entries(self) -> dict[int, PageTableEntry]:
        """Return the internal dictionary of all mapped entries.

        Useful for iteration (e.g., during fork() to copy all mappings,
        or during process exit to free all frames).

        Returns:
            Dictionary mapping VPN -> PageTableEntry for all mapped pages.
        """
        return self._entries

    def mapped_count(self) -> int:
        """Return the number of currently mapped pages.

        Returns:
            Count of entries in the page table.
        """
        return len(self._entries)
