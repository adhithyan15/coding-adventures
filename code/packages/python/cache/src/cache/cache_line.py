"""Cache line — the smallest unit of data in a cache.

In a real CPU, data is not moved one byte at a time between memory and the
cache. Instead, it moves in fixed-size chunks called **cache lines** (also
called cache blocks). A typical cache line is 64 bytes.

Analogy: Think of a warehouse that ships goods in standard containers.
You can't order a single screw — you get the whole container (cache line)
that includes the screw you need plus 63 other bytes of nearby data.
This works well because of **spatial locality**: if you accessed byte N,
you'll likely access bytes N+1, N+2, ... soon.

Each cache line stores:

    +-------+-------+-----+------+---------------------------+
    | valid | dirty | tag | LRU  |     data (64 bytes)       |
    +-------+-------+-----+------+---------------------------+

- **valid**: Is this line holding real data? After a reset, all lines are
  invalid (empty boxes). A line becomes valid when data is loaded into it.

- **dirty**: Has the data been modified since it was loaded from memory?
  In a write-back cache, writes go only to the cache (not memory). The
  dirty bit tracks whether the line needs to be written back to memory
  when evicted. (Like editing a document locally — you need to save it
  back to the server before closing.)

- **tag**: The high bits of the memory address. Since many addresses map
  to the same cache set (like many apartments on the same floor), the tag
  distinguishes WHICH address is actually stored here.

- **data**: The actual bytes — a list of integers, each 0-255.

- **last_access**: A timestamp (cycle count) recording when this line was
  last read or written. Used by the LRU replacement policy to decide
  which line to evict when the set is full.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class CacheLine:
    """A single cache line — one slot in the cache.

    Example:
        >>> line = CacheLine(line_size=64)
        >>> line.valid
        False
        >>> line.fill(tag=42, data=[0xAB] * 64, cycle=100)
        >>> line.valid
        True
        >>> line.tag
        42
        >>> line.last_access
        100
    """

    # ── Cache line metadata ───────────────────────────────────────────
    valid: bool = False
    """Is this line holding real data?"""

    dirty: bool = False
    """Has this line been modified? (write-back policy tracking)"""

    tag: int = 0
    """High bits of the address — identifies which memory block is cached."""

    last_access: int = 0
    """Cycle count of last access — used for LRU replacement."""

    # ── Data payload ──────────────────────────────────────────────────
    data: list[int] = field(default_factory=lambda: [0] * 64)
    """The actual bytes stored in this cache line (each 0-255)."""

    def __init__(self, line_size: int = 64) -> None:
        """Create a new invalid cache line with the given size.

        Args:
            line_size: Number of bytes per cache line. Defaults to 64,
                       which is standard on modern x86 and ARM CPUs.
        """
        self.valid = False
        self.dirty = False
        self.tag = 0
        self.last_access = 0
        self.data = [0] * line_size

    # ── Operations ────────────────────────────────────────────────────

    def fill(self, tag: int, data: list[int], cycle: int) -> None:
        """Load data into this cache line, marking it valid.

        This is called when a cache miss brings data from a lower level
        (L2, L3, or main memory) into this line.

        Args:
            tag: The tag bits for the address being cached.
            data: The bytes to store (must match line_size).
            cycle: Current clock cycle (for LRU tracking).
        """
        self.valid = True
        self.dirty = False  # freshly loaded data is clean
        self.tag = tag
        self.data = list(data)  # defensive copy
        self.last_access = cycle

    def touch(self, cycle: int) -> None:
        """Update the last access time — called on every hit.

        This is the heartbeat of LRU: the most recently used line
        gets the highest timestamp, so it's the *last* to be evicted.
        """
        self.last_access = cycle

    def invalidate(self) -> None:
        """Mark this line as invalid (empty).

        Used during cache flushes or coherence protocol invalidations.
        The data is not zeroed — it's just marked as not-present.
        """
        self.valid = False
        self.dirty = False

    @property
    def line_size(self) -> int:
        """Number of bytes in this cache line."""
        return len(self.data)

    def __repr__(self) -> str:
        """Compact representation for debugging."""
        state = "V" if self.valid else "-"
        state += "D" if self.dirty else "-"
        return f"CacheLine({state}, tag=0x{self.tag:X}, lru={self.last_access})"
