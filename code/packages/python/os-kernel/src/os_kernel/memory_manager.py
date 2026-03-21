"""Region-based memory manager.

Uses the simplest possible scheme: each process gets a fixed region of
physical memory assigned at creation time. No paging, no virtual memory.

Each region has permissions (read, write, execute) and an owner (PID).
"""

from __future__ import annotations

from dataclasses import dataclass

PERM_READ: int = 0x01
PERM_WRITE: int = 0x02
PERM_EXECUTE: int = 0x04


@dataclass
class MemoryRegion:
    """Describes a contiguous block of memory with permissions.

    Attributes:
        base: Starting address of this region.
        size: Region size in bytes.
        permissions: R/W/X flags.
        owner: PID that owns this region, or -1 for kernel-owned.
        name: Human-readable label.
    """

    base: int
    size: int
    permissions: int
    owner: int
    name: str


class MemoryManager:
    """Tracks all allocated memory regions."""

    def __init__(self: MemoryManager, regions: list[MemoryRegion] | None = None) -> None:
        self.regions: list[MemoryRegion] = list(regions) if regions else []

    def find_region(self: MemoryManager, address: int) -> MemoryRegion | None:
        """Return the memory region containing the given address, or None."""
        for r in self.regions:
            if r.base <= address < r.base + r.size:
                return r
        return None

    def check_access(self: MemoryManager, pid: int, address: int, perm: int) -> bool:
        """Verify that the given PID can access the address with given permissions."""
        region = self.find_region(address)
        if region is None:
            return False
        if region.owner != -1 and region.owner != pid:
            return False
        return (region.permissions & perm) == perm

    def allocate_region(
        self: MemoryManager, pid: int, base: int, size: int, perm: int, name: str
    ) -> None:
        """Add a new memory region for the given PID."""
        self.regions.append(MemoryRegion(
            base=base, size=size, permissions=perm, owner=pid, name=name
        ))

    def region_count(self: MemoryManager) -> int:
        """Return the number of tracked regions."""
        return len(self.regions)
