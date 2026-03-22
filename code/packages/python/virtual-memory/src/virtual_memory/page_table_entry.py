"""PageTableEntry — the metadata for a single virtual-to-physical page mapping.

Every virtual page in a process's address space has a corresponding Page Table
Entry (PTE). The PTE answers two questions:

  1. WHERE is this page in physical memory? (frame_number)
  2. WHAT are the rules for accessing it? (permission flags)

Think of a PTE like a library catalog card. The card tells you:
  - Which shelf the book is on (frame_number)
  - Whether you can borrow it (present)
  - Whether it has been read (accessed) or annotated (dirty)
  - Whether you are allowed to write in it (writable)
  - Whether you can execute it as code (executable)
  - Whether regular patrons can access it, or only librarians (user_accessible)

Real hardware PTEs are packed into 32 or 64 bits with each flag occupying
a single bit. Our educational implementation uses a Python dataclass with
boolean fields for clarity.

Bit layout (matching RISC-V Sv32 PTE format):
+--------------------+---+---+---+---+---+---+---+---+
| PPN (frame number) | D | A | G | U | X | W | R | V |
| bits 31-10         | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| (22 bits)          |   |   |   |   |   |   |   |   |
+--------------------+---+---+---+---+---+---+---+---+
V = Valid (present)    R = Readable
W = Writable           X = Executable
U = User-accessible    G = Global (not implemented here)
A = Accessed           D = Dirty
"""

from dataclasses import dataclass

# =============================================================================
# Constants — Page Geometry
# =============================================================================
#
# These three constants define the fundamental page geometry for a 32-bit
# virtual address space with 4 KB pages.
#
# Why 4 KB pages?
# ---------------
# It is a compromise between two competing forces:
#   - Smaller pages reduce internal fragmentation (wasted space within a page)
#     but require larger page tables (more entries to track).
#   - Larger pages mean smaller tables but more wasted space when a program
#     uses only part of a page.
#
# 4 KB (2^12 bytes) has been the standard since the Intel 386 in 1985.
# RISC-V uses it too. It is small enough that most pages are fully utilized,
# yet large enough that the page table overhead is manageable.

PAGE_SIZE: int = 4096
"""The size of each page/frame in bytes: 4 KB = 2^12 = 4096 bytes.

Every page in virtual memory and every frame in physical memory is exactly
this size. When the CPU accesses byte 2748 within a page, it uses the lower
12 bits of the address as the offset within the page.
"""

PAGE_OFFSET_BITS: int = 12
"""Number of bits used for the page offset: 12 bits.

Since 2^12 = 4096, twelve bits can address every byte within a 4 KB page.
Given a 32-bit virtual address:
    - Bits 11-0  (12 bits): page offset (which byte within the page)
    - Bits 31-12 (20 bits): virtual page number (which page)

To extract these:
    vpn    = address >> 12        # shift right to drop the offset
    offset = address & 0xFFF      # mask the lower 12 bits
"""

VPN_BITS: int = 20
"""Number of bits in the Virtual Page Number: 20 bits.

With 20 bits, we can address 2^20 = 1,048,576 distinct virtual pages.
At 4 KB per page, that gives us 4 GB of virtual address space — the full
range of a 32-bit address (2^32 = 4 GB).
"""


# =============================================================================
# PageTableEntry Dataclass
# =============================================================================


@dataclass
class PageTableEntry:
    """A single entry in a page table, describing one virtual-to-physical mapping.

    Each field corresponds to a hardware bit in the PTE. In real CPUs, these
    are packed into a single 32-bit or 64-bit integer. We use named booleans
    for readability.

    Example:
        >>> pte = PageTableEntry(frame_number=42, present=True, writable=True)
        >>> pte.frame_number
        42
        >>> pte.present
        True

    Attributes:
        frame_number: Which physical frame this virtual page maps to.
            Only meaningful when present=True. A frame is a fixed-size chunk
            of physical RAM. If the page is at frame 42 and the page size is
            4 KB, then the page occupies physical bytes 42*4096 through
            42*4096+4095.

        present: Is this page currently resident in physical memory?
            If False, accessing this page triggers a page fault (interrupt 14).
            A page might not be present because:
              - It was never allocated (new mapping, no physical frame yet)
              - It was swapped to disk to make room for other pages
              - It is a lazy allocation (will be allocated on first access)

        dirty: Has this page been written to since it was loaded?
            The hardware sets this bit automatically on any write. When the
            OS needs to evict this page to make room, it checks the dirty bit:
              - dirty=True  -> must write page contents to disk first
              - dirty=False -> page contents are unchanged, just discard

        accessed: Has this page been read or written recently?
            Used by page replacement algorithms (Clock, LRU) to decide which
            page to evict. The Clock algorithm clears this bit periodically;
            if the page is accessed again, hardware sets it back to True.

        writable: Can this page be written to?
            Code pages (containing program instructions) are typically
            read-only. Stack and heap pages are writable. Copy-on-write
            pages start as read-only and become writable after a fault.

        executable: Can instructions be fetched from this page?
            Data pages should NOT be executable — this is the NX (No eXecute)
            bit that prevents code injection attacks. An attacker who manages
            to write shellcode into a data buffer cannot execute it if the
            page is marked non-executable.

        user_accessible: Can user-mode code access this page?
            Kernel pages are NOT user-accessible. This prevents user programs
            from reading or writing kernel memory. The CPU checks this bit
            on every memory access and raises a fault if a user-mode program
            tries to access a kernel page.
    """

    frame_number: int = 0
    present: bool = False
    dirty: bool = False
    accessed: bool = False
    writable: bool = True
    executable: bool = False
    user_accessible: bool = True

    def copy(self) -> "PageTableEntry":
        """Create a deep copy of this PTE.

        Used during copy-on-write (COW) operations when forking a process.
        The parent and child each get their own PTE so that modifying one
        does not affect the other.

        Returns:
            A new PageTableEntry with identical field values.
        """
        return PageTableEntry(
            frame_number=self.frame_number,
            present=self.present,
            dirty=self.dirty,
            accessed=self.accessed,
            writable=self.writable,
            executable=self.executable,
            user_accessible=self.user_accessible,
        )
