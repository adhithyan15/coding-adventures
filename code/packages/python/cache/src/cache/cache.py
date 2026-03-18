"""Cache — a single configurable level of the cache hierarchy.

This module implements the core cache logic. The same class is used for
L1, L2, and L3 — the only difference is the configuration (size,
associativity, latency). This reflects real hardware: an L1 and an L3
use the same SRAM cell design, just at different scales.

## Address Decomposition

When the CPU accesses memory address 0x1A2B3C4D, the cache must figure
out three things:

1. **Offset** (lowest bits): Which byte *within* the cache line?
   - For 64-byte lines: 6 bits (2^6 = 64)
   - Example: offset = 0x0D = byte 13 of the line

2. **Set Index** (middle bits): Which set should we look in?
   - For 256 sets: 8 bits (2^8 = 256)
   - Example: set_index = 0xF1 = set 241

3. **Tag** (highest bits): Which memory block is this?
   - All remaining bits above offset + set_index
   - Example: tag = 0x1A2B3 (uniquely identifies the block)

Visual for a 64KB, 4-way, 64B-line cache (256 sets):

    Address: | tag (18 bits) | set index (8 bits) | offset (6 bits) |
             |  31 ... 14    |     13 ... 6       |    5 ... 0      |

This bit-slicing is why cache sizes must be powers of 2 — it lets the
hardware extract fields with simple bit masks instead of division.

## Read Path

    CPU reads address 0x1000
         |
         v
    Decompose: tag=0x4, set=0, offset=0
         |
         v
    Look in Set 0: compare tag 0x4 against all ways
         |
    +----+----+
    |         |
    HIT      MISS
    |         |
    Return   Go to next level (L2/L3/memory)
    data     Bring data back, allocate in this cache
             Maybe evict an old line (LRU)
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from cache.cache_line import CacheLine
from cache.cache_set import CacheConfig, CacheSet
from cache.stats import CacheStats


# ── Access Record ─────────────────────────────────────────────────────

@dataclass
class CacheAccess:
    """Record of a single cache access — for debugging and performance analysis.

    Every read() or write() call returns one of these, telling you exactly
    what happened: was it a hit? Which set? Was anything evicted? How many
    cycles did it cost?

    This is like a receipt for each memory transaction.
    """

    address: int
    """The full memory address that was accessed."""

    hit: bool
    """True if the data was found in the cache (no need to go further)."""

    tag: int
    """The tag bits extracted from the address."""

    set_index: int
    """The set index bits — which set in the cache was consulted."""

    offset: int
    """The offset bits — byte position within the cache line."""

    cycles: int
    """Clock cycles this access took (latency)."""

    evicted: CacheLine | None = None
    """If a line was evicted during this access, it's stored here.
    Only set for dirty evictions that need writeback."""


# ── Cache ─────────────────────────────────────────────────────────────

class Cache:
    """A single level of cache — configurable to be L1, L2, or L3.

    This is the workhorse of the cache simulator. Give it a CacheConfig
    and it handles address decomposition, set lookup, LRU replacement,
    and statistics tracking.

    Example:
        >>> config = CacheConfig(name="L1D", total_size=1024, line_size=64,
        ...                      associativity=4, access_latency=1)
        >>> cache = Cache(config)
        >>> access = cache.read(address=0x100, cycle=0)
        >>> access.hit
        False
        >>> access = cache.read(address=0x100, cycle=1)
        >>> access.hit
        True
    """

    def __init__(self, config: CacheConfig) -> None:
        """Initialize the cache with the given configuration.

        Creates all sets, precomputes bit positions for address
        decomposition, and initializes statistics.

        Args:
            config: Cache parameters (size, associativity, latency, etc.)
        """
        self.config = config
        self.stats = CacheStats()

        # Create the set array
        num_sets = config.num_sets
        self.sets: list[CacheSet] = [
            CacheSet(config.associativity, config.line_size)
            for _ in range(num_sets)
        ]

        # Precompute bit positions for address decomposition.
        # These are used as shift amounts and masks in _decompose_address.
        #
        #   offset_bits = log2(line_size)    e.g., log2(64) = 6
        #   set_bits    = log2(num_sets)     e.g., log2(256) = 8
        #
        # For a direct-mapped cache with 1 set per line, set_bits = log2(num_lines).
        # For a 1-set cache (fully associative), set_bits = 0.
        self._offset_bits: int = int(math.log2(config.line_size))
        self._set_bits: int = int(math.log2(num_sets)) if num_sets > 1 else 0
        self._set_mask: int = num_sets - 1  # e.g., 0xFF for 256 sets

    # ── Address Decomposition ─────────────────────────────────────────

    def _decompose_address(self, address: int) -> tuple[int, int, int]:
        """Split a memory address into (tag, set_index, offset).

        This is pure bit manipulation — no division needed because all
        sizes are powers of 2.

        Example for 64KB cache, 64B lines, 256 sets:
            address = 0x1A2B3C4D
            offset     = address & 0x3F              = 0x0D (13)
            set_index  = (address >> 6) & 0xFF        = 0xF1 (241)
            tag        = address >> 14                 = 0x68AC

        Args:
            address: Full memory address (unsigned integer).

        Returns:
            (tag, set_index, offset) tuple.
        """
        offset = address & ((1 << self._offset_bits) - 1)
        set_index = (address >> self._offset_bits) & self._set_mask
        tag = address >> (self._offset_bits + self._set_bits)
        return tag, set_index, offset

    # ── Read ──────────────────────────────────────────────────────────

    def read(self, address: int, size: int = 1, cycle: int = 0) -> CacheAccess:
        """Read data from the cache.

        On a hit, the data is returned immediately with the cache's
        access latency. On a miss, dummy data is allocated (the caller
        — typically the hierarchy — is responsible for actually fetching
        from the next level).

        Args:
            address: Memory address to read.
            size: Number of bytes to read (for stats; actual data is
                  at cache-line granularity).
            cycle: Current clock cycle.

        Returns:
            CacheAccess record describing what happened.
        """
        tag, set_index, offset = self._decompose_address(address)
        cache_set = self.sets[set_index]

        hit, line = cache_set.access(tag, cycle)

        if hit:
            self.stats.record_read(hit=True)
            return CacheAccess(
                address=address,
                hit=True,
                tag=tag,
                set_index=set_index,
                offset=offset,
                cycles=self.config.access_latency,
            )

        # Miss — allocate the line with dummy data.
        # In a real system, the hierarchy fetches from the next level
        # and fills this line. Here we simulate by filling with zeros.
        self.stats.record_read(hit=False)
        evicted = cache_set.allocate(
            tag=tag,
            data=[0] * self.config.line_size,
            cycle=cycle,
        )
        if evicted is not None:
            self.stats.record_eviction(dirty=True)
        elif self._all_ways_were_valid(cache_set, tag):
            # A valid but clean line was evicted
            self.stats.record_eviction(dirty=False)

        return CacheAccess(
            address=address,
            hit=False,
            tag=tag,
            set_index=set_index,
            offset=offset,
            cycles=self.config.access_latency,
            evicted=evicted,
        )

    # ── Write ─────────────────────────────────────────────────────────

    def write(
        self, address: int, data: list[int] | None = None, cycle: int = 0
    ) -> CacheAccess:
        """Write data to the cache.

        **Write-back policy**: Write only to the cache. Mark the line
        as dirty. The data is written to the next level only when the
        line is evicted.

        **Write-through policy**: Write to both the cache and the next
        level simultaneously. The line is never dirty.

        On a write miss, we use **write-allocate**: first bring the
        line into the cache (like a read miss), then perform the write.
        This is the most common policy on modern CPUs.

        Args:
            address: Memory address to write.
            data: Bytes to write (optional; if None, just marks dirty).
            cycle: Current clock cycle.

        Returns:
            CacheAccess record describing what happened.
        """
        tag, set_index, offset = self._decompose_address(address)
        cache_set = self.sets[set_index]

        hit, line = cache_set.access(tag, cycle)

        if hit:
            self.stats.record_write(hit=True)
            # Write the data into the line
            if data is not None:
                for i, byte in enumerate(data):
                    if offset + i < len(line.data):
                        line.data[offset + i] = byte
            # Mark dirty for write-back; write-through stays clean
            if self.config.write_policy == "write-back":
                line.dirty = True
            return CacheAccess(
                address=address,
                hit=True,
                tag=tag,
                set_index=set_index,
                offset=offset,
                cycles=self.config.access_latency,
            )

        # Write miss — allocate (write-allocate policy), then write
        self.stats.record_write(hit=False)
        fill_data = [0] * self.config.line_size
        if data is not None:
            for i, byte in enumerate(data):
                if offset + i < len(fill_data):
                    fill_data[offset + i] = byte

        evicted = cache_set.allocate(tag=tag, data=fill_data, cycle=cycle)
        if evicted is not None:
            self.stats.record_eviction(dirty=True)
        elif self._all_ways_were_valid(cache_set, tag):
            self.stats.record_eviction(dirty=False)

        # For write-back, mark the newly allocated line as dirty
        # (it has new data that isn't in the next level)
        new_hit, new_line = cache_set.access(tag, cycle)
        if new_hit and self.config.write_policy == "write-back":
            new_line.dirty = True

        return CacheAccess(
            address=address,
            hit=False,
            tag=tag,
            set_index=set_index,
            offset=offset,
            cycles=self.config.access_latency,
            evicted=evicted,
        )

    # ── Helpers ───────────────────────────────────────────────────────

    @staticmethod
    def _all_ways_were_valid(cache_set: CacheSet, current_tag: int) -> bool:
        """Check if all ways in a set are valid (meaning an eviction occurred).

        After allocate(), the new line is already in place. We check if
        all ways are now valid — if so, one of them must have been replaced.

        We exclude the just-allocated tag from the check (it's the new line).
        Actually, since allocate always fills an invalid slot first, if all
        are valid now and one has the new tag, then all *were* valid before.
        """
        return all(line.valid for line in cache_set.lines)

    def invalidate(self) -> None:
        """Invalidate all lines in the cache (cache flush).

        This is equivalent to a cold start — after invalidation, every
        access will be a compulsory miss. Used when context-switching
        between processes or when explicitly flushing (e.g., for I/O
        coherence).
        """
        for cache_set in self.sets:
            for line in cache_set.lines:
                line.invalidate()

    def fill_line(
        self, address: int, data: list[int], cycle: int = 0
    ) -> CacheLine | None:
        """Directly fill a cache line with data (used by hierarchy on miss).

        This bypasses the normal read/write path — it's used when the
        hierarchy fetches data from a lower level and wants to install
        it in this cache.

        Args:
            address: The address whose line we're filling.
            data: The full cache line of data from the lower level.
            cycle: Current clock cycle.

        Returns:
            Evicted dirty CacheLine if a writeback is needed, else None.
        """
        tag, set_index, _offset = self._decompose_address(address)
        cache_set = self.sets[set_index]
        return cache_set.allocate(tag=tag, data=data, cycle=cycle)

    def __repr__(self) -> str:
        """Human-readable summary of the cache configuration."""
        return (
            f"Cache({self.config.name}: "
            f"{self.config.total_size // 1024}KB, "
            f"{self.config.associativity}-way, "
            f"{self.config.line_size}B lines, "
            f"{self.config.num_sets} sets)"
        )
