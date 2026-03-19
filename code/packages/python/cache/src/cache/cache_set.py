"""Cache set — a group of cache lines that share the same set index.

A cache set is like a row of labeled boxes on a shelf. When the CPU
accesses memory, the address tells us *which shelf* (set) to look at.
Within that shelf, we check each box (way) to see if our data is there.

In a **4-way set-associative** cache, each set has 4 lines (ways).
When all 4 are full and we need to bring in new data, we must **evict**
one. The LRU (Least Recently Used) policy picks the line that hasn't
been accessed for the longest time — the logic being "if you haven't
used it lately, you probably won't need it soon."

Associativity is a key design tradeoff:
- **Direct-mapped** (1-way): Fast lookup, but high conflict misses.
  Like a parking lot where each car is assigned exactly one spot — if
  two cars map to the same spot, one must leave even if other spots
  are empty.
- **Fully associative** (N-way = total lines): No conflicts, but
  expensive to search every line on every access.
- **Set-associative** (2/4/8/16-way): The sweet spot. Each address
  maps to a set, and within that set, any way can hold it.

    Set 0: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
    Set 1: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
    Set 2: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
    ...

"""

from __future__ import annotations

from dataclasses import dataclass

from cache.cache_line import CacheLine


# ── Configuration ─────────────────────────────────────────────────────

@dataclass(frozen=True)
class CacheConfig:
    """Configuration for a cache level — the knobs you turn to get L1/L2/L3.

    By adjusting these parameters, the exact same Cache class can simulate
    anything from a tiny 1KB direct-mapped L1 to a massive 32MB 16-way L3.

    Real-world examples:
        ARM Cortex-A78: L1D = 64KB, 4-way, 64B lines, 1 cycle
        Intel Alder Lake: L1D = 48KB, 12-way, 64B lines, 5 cycles
        Apple M4: L1D = 128KB, 8-way, 64B lines, ~3 cycles

    Args:
        name: Human-readable name for this cache level ("L1D", "L2", etc.)
        total_size: Total capacity in bytes (e.g., 65536 for 64KB).
        line_size: Bytes per cache line. Must be a power of 2.
        associativity: Number of ways per set. 1 = direct-mapped.
        access_latency: Clock cycles to access this level on a hit.
        write_policy: "write-back" (defer writes) or "write-through" (immediate).
    """

    name: str
    total_size: int
    line_size: int = 64
    associativity: int = 4
    access_latency: int = 1
    write_policy: str = "write-back"

    def __post_init__(self) -> None:
        """Validate configuration parameters.

        Cache sizes and line sizes must be powers of 2 — this is a
        hardware constraint because address bit-slicing only works
        cleanly with power-of-2 sizes.
        """
        if self.total_size <= 0:
            msg = f"total_size must be positive, got {self.total_size}"
            raise ValueError(msg)
        if self.line_size <= 0 or (self.line_size & (self.line_size - 1)) != 0:
            msg = f"line_size must be a positive power of 2, got {self.line_size}"
            raise ValueError(msg)
        if self.associativity <= 0:
            msg = f"associativity must be positive, got {self.associativity}"
            raise ValueError(msg)
        if self.total_size % (self.line_size * self.associativity) != 0:
            msg = (
                f"total_size ({self.total_size}) must be divisible by "
                f"line_size * associativity ({self.line_size * self.associativity})"
            )
            raise ValueError(msg)
        if self.write_policy not in ("write-back", "write-through"):
            msg = f"write_policy must be 'write-back' or 'write-through', got '{self.write_policy}'"
            raise ValueError(msg)
        if self.access_latency < 0:
            msg = f"access_latency must be non-negative, got {self.access_latency}"
            raise ValueError(msg)

    @property
    def num_lines(self) -> int:
        """Total number of cache lines = total_size / line_size."""
        return self.total_size // self.line_size

    @property
    def num_sets(self) -> int:
        """Number of sets = num_lines / associativity."""
        return self.num_lines // self.associativity


# ── Cache Set ─────────────────────────────────────────────────────────

class CacheSet:
    """One set in the cache — contains N ways (lines).

    Implements LRU (Least Recently Used) replacement: when all ways are
    full and we need to bring in new data, evict the line that was
    accessed least recently.

    Think of it like a desk with N book slots. When all slots are full
    and you need a new book, you put away the one you haven't read in
    the longest time.
    """

    def __init__(self, associativity: int, line_size: int) -> None:
        """Create a cache set with the given number of ways.

        Args:
            associativity: Number of ways (lines) in this set.
            line_size: Bytes per cache line.
        """
        self.lines: list[CacheLine] = [
            CacheLine(line_size=line_size) for _ in range(associativity)
        ]

    # ── Lookup ────────────────────────────────────────────────────────

    def lookup(self, tag: int) -> tuple[bool, int | None]:
        """Check if a tag is present in this set.

        Searches all ways for a valid line with a matching tag. This is
        what happens in hardware with a parallel tag comparator — all
        ways are checked simultaneously.

        Args:
            tag: The tag bits from the address.

        Returns:
            (hit, way_index): hit is True if found; way_index is the
            index of the matching line (or None if miss).
        """
        for i, line in enumerate(self.lines):
            if line.valid and line.tag == tag:
                return True, i
        return False, None

    # ── Access ────────────────────────────────────────────────────────

    def access(self, tag: int, cycle: int) -> tuple[bool, CacheLine]:
        """Access this set for a given tag. Returns (hit, line).

        On a hit, updates the line's LRU timestamp so it becomes the
        most recently used. On a miss, returns the LRU victim line
        (the caller decides what to do — typically allocate new data).

        Args:
            tag: The tag bits from the address.
            cycle: Current clock cycle for LRU tracking.

        Returns:
            (hit, line): If hit, the matching line. If miss, the LRU
            victim line (which may need writeback if dirty).
        """
        hit, way_index = self.lookup(tag)
        if hit:
            assert way_index is not None
            line = self.lines[way_index]
            line.touch(cycle)
            return True, line
        # Miss — return the LRU line (candidate for eviction)
        lru_index = self._find_lru()
        return False, self.lines[lru_index]

    # ── Allocation (filling after a miss) ─────────────────────────────

    def allocate(
        self, tag: int, data: list[int], cycle: int
    ) -> CacheLine | None:
        """Bring new data into this set after a cache miss.

        First tries to find an invalid (empty) way. If all ways are
        valid, evicts the LRU line. Returns the evicted line if it was
        dirty (the caller must write it back to the next level).

        Args:
            tag: Tag for the new data.
            data: The bytes to store.
            cycle: Current clock cycle.

        Returns:
            The evicted CacheLine if it was dirty (needs writeback),
            or None if no dirty writeback is needed.

        Think of it like clearing a desk slot for a new book:
        1. If there's an empty slot, use it (no eviction needed).
        2. If all slots are full, pick the least-recently-read book.
        3. If that book had notes scribbled in it (dirty), you need
           to save those notes before putting the book away.
        """
        # Step 1: Look for an invalid (empty) way
        for line in self.lines:
            if not line.valid:
                line.fill(tag=tag, data=data, cycle=cycle)
                return None  # no eviction needed

        # Step 2: All ways full — evict the LRU line
        lru_index = self._find_lru()
        victim = self.lines[lru_index]

        # Step 3: Check if the victim is dirty (needs writeback)
        evicted: CacheLine | None = None
        if victim.dirty:
            # Create a copy of the evicted line for writeback
            evicted = CacheLine(line_size=len(victim.data))
            evicted.valid = True
            evicted.dirty = True
            evicted.tag = victim.tag
            evicted.data = list(victim.data)
            evicted.last_access = victim.last_access

        # Step 4: Overwrite the victim with new data
        victim.fill(tag=tag, data=data, cycle=cycle)

        return evicted

    # ── LRU Selection ─────────────────────────────────────────────────

    def _find_lru(self) -> int:
        """Find the least recently used way index.

        LRU replacement is simple: each line records its last access
        time (cycle count). The line with the smallest timestamp is
        the one that hasn't been touched for the longest time.

        In real hardware, true LRU is expensive for high associativity
        (tracking N! orderings). CPUs often use pseudo-LRU (tree-PLRU)
        or RRIP as approximations. For simulation, true LRU is fine.

        Special case: invalid lines are always preferred over valid ones
        (an empty slot is "older" than any real data).

        Returns:
            Index of the LRU way in self.lines.
        """
        best_index = 0
        best_time = float("inf")
        for i, line in enumerate(self.lines):
            # Invalid lines are always the best candidates
            if not line.valid:
                return i
            if line.last_access < best_time:
                best_time = line.last_access
                best_index = i
        return best_index
