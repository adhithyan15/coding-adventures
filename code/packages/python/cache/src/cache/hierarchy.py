"""Cache hierarchy — multi-level cache system (L1I + L1D + L2 + L3 + memory).

A modern CPU doesn't have just one cache — it has a **hierarchy** of
progressively larger and slower caches. This is the memory equivalent
of keeping frequently used items close to hand:

    +---------+     +--------+     +--------+     +--------+     +--------+
    |   CPU   | --> |  L1    | --> |   L2   | --> |   L3   | --> |  Main  |
    |  core   |     | 1 cyc  |     | 10 cyc |     | 30 cyc |     | Memory |
    |         |     | 64KB   |     | 256KB  |     | 8MB    |     | 100cyc |
    +---------+     +--------+     +--------+     +--------+     +--------+
                     per-core       per-core       shared         shared

Analogy:
- L1 = the books open on your desk (tiny, instant access)
- L2 = the bookshelf in your office (bigger, a few seconds to grab)
- L3 = the library downstairs (huge, takes a minute to walk there)
- Main memory = the warehouse across town (enormous, takes an hour)

When the CPU reads an address:
1. Check L1D. Hit? Return data (1 cycle). Miss? Continue.
2. Check L2. Hit? Return data (10 cycles), and fill L1D. Miss? Continue.
3. Check L3. Hit? Return data (30 cycles), fill L2 and L1D. Miss? Continue.
4. Go to main memory (100 cycles). Fill L3, L2, and L1D.

The total latency is the sum of all levels that missed:
- L1 hit:         1 cycle
- L1 miss, L2 hit:  1 + 10 = 11 cycles
- L1+L2 miss, L3 hit: 1 + 10 + 30 = 41 cycles
- All miss:       1 + 10 + 30 + 100 = 141 cycles

Harvard vs Unified:
- **Harvard architecture**: Separate L1 for instructions (L1I) and data (L1D).
  This lets the CPU fetch an instruction and load data simultaneously.
- **Unified**: L2 and L3 are typically unified (shared between instructions
  and data) to avoid wasting space.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from cache.cache import Cache, CacheAccess
from cache.cache_line import CacheLine


# ── Hierarchy Access Record ───────────────────────────────────────────

@dataclass
class HierarchyAccess:
    """Record of an access through the full hierarchy.

    Tracks which level served the data and the total latency accumulated
    across all levels that were consulted.
    """

    address: int
    """The memory address that was accessed."""

    served_by: str
    """Name of the level that had the data ("L1D", "L2", "L3", "memory")."""

    total_cycles: int
    """Total clock cycles from start to data delivery."""

    hit_at_level: int
    """Which hierarchy level served the data (0=L1, 1=L2, 2=L3, 3=memory)."""

    level_accesses: list[CacheAccess] = field(default_factory=list)
    """Detailed access records from each cache level consulted."""


# ── Cache Hierarchy ───────────────────────────────────────────────────

class CacheHierarchy:
    """Multi-level cache hierarchy — L1I + L1D + L2 + L3 + main memory.

    Fully configurable: pass any combination of cache levels. You can
    simulate anything from a simple L1-only system to a full 3-level
    hierarchy with separate instruction and data L1 caches.

    Example:
        >>> from cache.cache import Cache
        >>> from cache.cache_set import CacheConfig
        >>> l1d = Cache(CacheConfig("L1D", 1024, 64, 4, 1))
        >>> l2 = Cache(CacheConfig("L2", 4096, 64, 8, 10))
        >>> hierarchy = CacheHierarchy(l1d=l1d, l2=l2)
        >>> result = hierarchy.read(0x1000, cycle=0)
        >>> result.served_by  # first access is always a miss through all levels
        'memory'
    """

    def __init__(
        self,
        l1i: Cache | None = None,
        l1d: Cache | None = None,
        l2: Cache | None = None,
        l3: Cache | None = None,
        main_memory_latency: int = 100,
    ) -> None:
        """Create a cache hierarchy.

        Args:
            l1i: L1 instruction cache (optional, for Harvard architecture).
            l1d: L1 data cache (optional but typical).
            l2: L2 cache (optional).
            l3: L3 cache (optional).
            main_memory_latency: Clock cycles for main memory access.
        """
        self.l1i = l1i
        self.l1d = l1d
        self.l2 = l2
        self.l3 = l3
        self.main_memory_latency = main_memory_latency

        # Build ordered list of (name, cache) for iteration.
        # The hierarchy is walked top-down (fastest to slowest).
        self._data_levels: list[tuple[str, Cache]] = []
        if l1d is not None:
            self._data_levels.append(("L1D", l1d))
        if l2 is not None:
            self._data_levels.append(("L2", l2))
        if l3 is not None:
            self._data_levels.append(("L3", l3))

        self._instr_levels: list[tuple[str, Cache]] = []
        if l1i is not None:
            self._instr_levels.append(("L1I", l1i))
        if l2 is not None:
            self._instr_levels.append(("L2", l2))
        if l3 is not None:
            self._instr_levels.append(("L3", l3))

    # ── Read ──────────────────────────────────────────────────────────

    def read(
        self,
        address: int,
        is_instruction: bool = False,
        cycle: int = 0,
    ) -> HierarchyAccess:
        """Read through the hierarchy. Returns which level served the data.

        Walks the hierarchy top-down. At each level:
        - If hit: stop, fill all higher levels, return.
        - If miss: accumulate latency, continue to next level.
        - If all miss: data comes from main memory.

        The **inclusive** fill policy is used: when L3 serves data, it
        also fills L2 and L1D so subsequent accesses hit at L1.

        Args:
            address: Memory address to read.
            is_instruction: If True, use L1I instead of L1D for the
                           first level. L2 and L3 are unified.
            cycle: Current clock cycle.

        Returns:
            HierarchyAccess with the level that served and total cycles.
        """
        levels = self._instr_levels if is_instruction else self._data_levels

        if not levels:
            # No caches at all — go straight to memory
            return HierarchyAccess(
                address=address,
                served_by="memory",
                total_cycles=self.main_memory_latency,
                hit_at_level=len(levels),
                level_accesses=[],
            )

        total_cycles = 0
        accesses: list[CacheAccess] = []
        served_by = "memory"
        hit_level = len(levels)

        # Walk the hierarchy top-down
        for level_idx, (name, cache) in enumerate(levels):
            access = cache.read(address, cycle=cycle)
            total_cycles += cache.config.access_latency
            accesses.append(access)

            if access.hit:
                served_by = name
                hit_level = level_idx
                break

        if served_by == "memory":
            # Complete miss — add main memory latency
            total_cycles += self.main_memory_latency

        # Fill higher levels (inclusive policy).
        # If L3 served, fill L2 and L1. If L2 served, fill L1.
        # We fill with dummy data (zeros) since we're simulating.
        dummy_data = [0] * self._get_line_size(levels)
        for fill_idx in range(hit_level - 1, -1, -1):
            _fill_name, fill_cache = levels[fill_idx]
            fill_cache.fill_line(address, dummy_data, cycle=cycle)

        return HierarchyAccess(
            address=address,
            served_by=served_by,
            total_cycles=total_cycles,
            hit_at_level=hit_level,
            level_accesses=accesses,
        )

    # ── Write ─────────────────────────────────────────────────────────

    def write(
        self,
        address: int,
        data: list[int] | None = None,
        cycle: int = 0,
    ) -> HierarchyAccess:
        """Write through the hierarchy.

        With write-allocate + write-back (the most common policy):
        1. If L1D hit: write to L1D, mark dirty. Done.
        2. If L1D miss: allocate in L1D (may cause eviction cascade),
           write to L1D. The data comes from the next level that has it
           or from main memory.

        The write is always done at the L1D level. On a miss, we walk
        down to find the data (or go to memory), fill back up, then write.

        Args:
            address: Memory address to write.
            data: Bytes to write.
            cycle: Current clock cycle.

        Returns:
            HierarchyAccess with the level that served and total cycles.
        """
        levels = self._data_levels

        if not levels:
            return HierarchyAccess(
                address=address,
                served_by="memory",
                total_cycles=self.main_memory_latency,
                hit_at_level=0,
                level_accesses=[],
            )

        # Check L1D first (writes always go to the data cache)
        first_name, first_cache = levels[0]
        access = first_cache.write(address, data, cycle=cycle)

        if access.hit:
            return HierarchyAccess(
                address=address,
                served_by=first_name,
                total_cycles=first_cache.config.access_latency,
                hit_at_level=0,
                level_accesses=[access],
            )

        # Write miss at L1 — walk lower levels to find the data
        total_cycles = first_cache.config.access_latency
        accesses: list[CacheAccess] = [access]
        served_by = "memory"
        hit_level = len(levels)

        for level_idx in range(1, len(levels)):
            _name, cache = levels[level_idx]
            level_access = cache.read(address, cycle=cycle)
            total_cycles += cache.config.access_latency
            accesses.append(level_access)

            if level_access.hit:
                served_by = _name
                hit_level = level_idx
                break

        if served_by == "memory":
            total_cycles += self.main_memory_latency

        return HierarchyAccess(
            address=address,
            served_by=served_by,
            total_cycles=total_cycles,
            hit_at_level=hit_level,
            level_accesses=accesses,
        )

    # ── Helpers ───────────────────────────────────────────────────────

    @staticmethod
    def _get_line_size(levels: list[tuple[str, Cache]]) -> int:
        """Get the line size from the first level in the hierarchy."""
        if levels:
            return levels[0][1].config.line_size
        return 64  # default

    def invalidate_all(self) -> None:
        """Invalidate all caches in the hierarchy (full flush)."""
        for cache in [self.l1i, self.l1d, self.l2, self.l3]:
            if cache is not None:
                cache.invalidate()

    def reset_stats(self) -> None:
        """Reset statistics for all cache levels."""
        for cache in [self.l1i, self.l1d, self.l2, self.l3]:
            if cache is not None:
                cache.stats.reset()

    def __repr__(self) -> str:
        """Human-readable summary of the hierarchy."""
        parts = []
        if self.l1i is not None:
            parts.append(f"L1I={self.l1i.config.total_size // 1024}KB")
        if self.l1d is not None:
            parts.append(f"L1D={self.l1d.config.total_size // 1024}KB")
        if self.l2 is not None:
            parts.append(f"L2={self.l2.config.total_size // 1024}KB")
        if self.l3 is not None:
            parts.append(f"L3={self.l3.config.total_size // 1024}KB")
        parts.append(f"mem={self.main_memory_latency}cyc")
        return f"CacheHierarchy({', '.join(parts)})"
