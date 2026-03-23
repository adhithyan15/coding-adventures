"""Sparse Memory -- simulating a 32-bit address space without 4 GB.

=== Why sparse? ===

A real 32-bit CPU can address 4 GB of memory (2^32 bytes). But most of
that address space is empty -- a typical embedded system might have:

    0x00000000 - 0x000FFFFF: 1 MB of RAM (for code and data)
    0xFFFB0000 - 0xFFFFFFFF: 320 KB of I/O registers (for peripherals)

Everything in between is unmapped -- accessing it would trigger a bus fault
on real hardware. Allocating a contiguous 4 GB byte array to simulate this
would be wasteful and impractical.

SparseMemory solves this by mapping only the regions that actually exist.
Each region is a named bytearray at a specific base address. Reads and
writes are dispatched to the correct region by checking address ranges.

=== How it works ===

Think of SparseMemory as a building with multiple floors, where each floor
has a different purpose:

    Floor 0 (0x00000000): RAM      -- read/write, for code and data
    Floor N (0xFFFB0000): I/O Regs -- some read-only, some read/write

When the CPU reads address 0x00001234, we find which "floor" contains that
address (RAM, at base 0x00000000), compute the offset within the floor
(0x1234), and read from that floor's backing bytearray at that offset.

=== Memory-mapped I/O ===

In embedded systems and operating systems, hardware devices (UART, timers,
interrupt controllers) appear as memory addresses. Writing to address
0xFFFF0000 might send a byte over a serial port. SparseMemory naturally
supports this pattern -- each device gets its own MemoryRegion.

=== Read-only regions ===

Some regions should never be written to (ROM, read-only status registers).
When a region is marked read_only, writes are silently ignored. This matches
real hardware where writing to ROM has no effect.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class MemoryRegion:
    """A contiguous block of addressable memory.

    Each region has a base address, a size, and a backing bytearray.
    The region occupies addresses [base, base + size). Any access within
    this range is translated to an offset into the data array:

        offset = address - base
        value  = data[offset]

    Example::

        MemoryRegion(base=0x1000, size=256, name="SRAM")

        read_byte(0x1000) -> data[0]
        read_byte(0x10FF) -> data[255]
        read_byte(0x1100) -> ERROR: outside this region
    """

    # Starting address of this region in the 32-bit address space.
    base: int

    # Number of bytes in this region. Covers addresses [base, base + size).
    size: int

    # Human-readable label for debugging (e.g., "RAM", "ROM", "UART").
    name: str = ""

    # When True, WriteByte and WriteWord silently discard values.
    # Models ROM, flash memory, and read-only status registers.
    read_only: bool = False

    # Backing storage. Allocated automatically if not provided.
    data: bytearray = field(default=None)  # type: ignore[assignment]

    def __post_init__(self) -> None:
        if self.data is None:
            self.data = bytearray(self.size)


class SparseMemory:
    """Maps address ranges to backing bytearrays, enabling a full 32-bit
    address space without allocating 4 GB.

    === Region lookup ===

    On every access, SparseMemory searches through its regions to find one
    that contains the target address. This is a linear scan -- O(N) where N
    is the number of regions. For the small number of regions in a typical
    system (2-10), this is negligible.

    === Unmapped addresses ===

    If no region contains the target address, the access raises a
    RuntimeError. On real hardware this would be a bus fault. Raising an
    exception makes bugs immediately visible.

    Example -- a simple embedded system memory map::

        regions = [
            MemoryRegion(base=0x00000000, size=0x100000, name="RAM"),
            MemoryRegion(base=0xFFFB0000, size=0x50000, name="I/O", read_only=True),
        ]
        mem = SparseMemory(regions)
    """

    def __init__(self, regions: list[MemoryRegion]) -> None:
        # Copy regions so the caller cannot mutate our list.
        self._regions: list[MemoryRegion] = []
        for r in regions:
            # Make a copy with its own data bytearray.
            copied = MemoryRegion(
                base=r.base,
                size=r.size,
                name=r.name,
                read_only=r.read_only,
                data=bytearray(r.data) if r.data is not None else bytearray(r.size),
            )
            self._regions.append(copied)

    # -- Internal helpers ---------------------------------------------------

    def _find_region(self, address: int, num_bytes: int) -> tuple[MemoryRegion, int]:
        """Locate the MemoryRegion containing [address, address + num_bytes).

        Returns (region, offset_within_region).
        Raises RuntimeError if unmapped (models a bus fault).
        """
        end = address + num_bytes
        for r in self._regions:
            region_end = r.base + r.size
            if address >= r.base and end <= region_end:
                offset = address - r.base
                return r, offset
        msg = (
            f"SparseMemory: unmapped address 0x{address:08X} "
            f"(accessing {num_bytes} bytes)"
        )
        raise RuntimeError(msg)

    # -- Byte operations ----------------------------------------------------

    def read_byte(self, address: int) -> int:
        """Read a single byte from the sparse address space.

        Raises RuntimeError if the address is unmapped.
        """
        region, offset = self._find_region(address, 1)
        return region.data[offset]

    def write_byte(self, address: int, value: int) -> None:
        """Write a single byte to the sparse address space.

        If the target region is read-only, the write is silently ignored.
        This matches real hardware behavior.
        """
        region, offset = self._find_region(address, 1)
        if region.read_only:
            return
        region.data[offset] = value & 0xFF

    # -- Word operations (32-bit, little-endian) ----------------------------

    def read_word(self, address: int) -> int:
        """Read a 32-bit word (4 bytes) in little-endian byte order.

        Little-endian means the least significant byte is stored at the lowest
        address. For the value 0xDEADBEEF stored at address 0x1000:

            Address  Byte
            0x1000   0xEF  (least significant)
            0x1001   0xBE
            0x1002   0xAD
            0x1003   0xDE  (most significant)
        """
        region, offset = self._find_region(address, 4)
        return (
            region.data[offset]
            | (region.data[offset + 1] << 8)
            | (region.data[offset + 2] << 16)
            | (region.data[offset + 3] << 24)
        )

    def write_word(self, address: int, value: int) -> None:
        """Write a 32-bit word (4 bytes) in little-endian byte order.

        If the target region is read-only, the write is silently ignored.
        """
        region, offset = self._find_region(address, 4)
        if region.read_only:
            return
        value = value & 0xFFFFFFFF
        region.data[offset] = value & 0xFF
        region.data[offset + 1] = (value >> 8) & 0xFF
        region.data[offset + 2] = (value >> 16) & 0xFF
        region.data[offset + 3] = (value >> 24) & 0xFF

    # -- Bulk operations ----------------------------------------------------

    def load_bytes(self, address: int, data: bytes | bytearray) -> None:
        """Copy bytes into the sparse address space starting at ``address``.

        Typically used to load a program binary into simulated RAM or to
        initialize ROM contents. The entire range must fall within a single
        region.

        Note: load_bytes bypasses the read_only check. This allows pre-loading
        ROM contents during system initialization before the CPU starts.
        """
        region, offset = self._find_region(address, len(data))
        for i, b in enumerate(data):
            region.data[offset + i] = b

    def dump(self, start: int, length: int) -> list[int]:
        """Return a copy of bytes from the sparse address space.

        The entire range [start, start + length) must fall within a single
        memory region. The returned list is a copy -- modifying it does not
        affect the simulated memory.
        """
        region, offset = self._find_region(start, length)
        return list(region.data[offset : offset + length])

    # -- Diagnostics --------------------------------------------------------

    def region_count(self) -> int:
        """Return the number of mapped regions."""
        return len(self._regions)

    @property
    def regions(self) -> list[MemoryRegion]:
        """Direct access to regions for testing/inspection."""
        return self._regions
