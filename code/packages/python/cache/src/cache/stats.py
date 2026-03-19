"""Cache statistics tracking — measuring how well the cache is performing.

Every cache keeps a scorecard. Just like a baseball player tracks batting
average (hits / at-bats), a cache tracks its **hit rate** (cache hits /
total accesses). A high hit rate means the cache is doing its job well —
most memory requests are being served quickly from the cache rather than
going to slower main memory.

Key metrics:
- **Reads/Writes**: How many times the CPU asked for data or stored data.
- **Hits**: How many times the requested data was already in the cache.
- **Misses**: How many times we had to go to a slower level to get the data.
- **Evictions**: How many times we had to kick out old data to make room.
- **Writebacks**: How many evictions involved dirty data that needed to be
  written back to the next level (only relevant for write-back caches).

Analogy: Think of a library desk (L1 cache). If you keep the right books
on your desk, you rarely need to walk to the shelf (L2). Your "hit rate"
is how often the book you need is already on your desk.
"""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class CacheStats:
    """Tracks performance statistics for a single cache level.

    Every read or write to the cache updates these counters. After running
    a simulation, you can inspect hit_rate and miss_rate to see how
    effective the cache configuration is for a given workload.

    Example:
        >>> stats = CacheStats()
        >>> stats.record_read(hit=True)
        >>> stats.record_read(hit=False)
        >>> stats.hit_rate
        0.5
        >>> stats.miss_rate
        0.5
    """

    # ── Counters ──────────────────────────────────────────────────────
    reads: int = 0
    writes: int = 0
    hits: int = 0
    misses: int = 0
    evictions: int = 0
    writebacks: int = 0  # dirty evictions that needed writeback

    # ── Derived metrics ───────────────────────────────────────────────

    @property
    def total_accesses(self) -> int:
        """Total number of read + write operations."""
        return self.reads + self.writes

    @property
    def hit_rate(self) -> float:
        """Fraction of accesses that were cache hits (0.0 to 1.0).

        Returns 0.0 if no accesses have been made (avoid division by zero).

        A hit rate of 0.95 means 95% of memory requests were served from
        this cache level — excellent for an L1 cache.
        """
        if self.total_accesses == 0:
            return 0.0
        return self.hits / self.total_accesses

    @property
    def miss_rate(self) -> float:
        """Fraction of accesses that were cache misses (0.0 to 1.0).

        Always equals 1.0 - hit_rate. Provided for convenience since
        miss rate is the more commonly discussed metric in architecture
        papers ("this workload has a 5% L1 miss rate").
        """
        if self.total_accesses == 0:
            return 0.0
        return self.misses / self.total_accesses

    # ── Recording methods ─────────────────────────────────────────────

    def record_read(self, *, hit: bool) -> None:
        """Record a read access. Pass hit=True for a cache hit."""
        self.reads += 1
        if hit:
            self.hits += 1
        else:
            self.misses += 1

    def record_write(self, *, hit: bool) -> None:
        """Record a write access. Pass hit=True for a cache hit."""
        self.writes += 1
        if hit:
            self.hits += 1
        else:
            self.misses += 1

    def record_eviction(self, *, dirty: bool) -> None:
        """Record an eviction. Pass dirty=True if the evicted line was dirty.

        A dirty eviction means the data was modified in the cache but not
        yet written to the next level. The cache controller must "write back"
        the dirty data before discarding it — this is the extra cost of a
        write-back policy.
        """
        self.evictions += 1
        if dirty:
            self.writebacks += 1

    def reset(self) -> None:
        """Reset all counters to zero.

        Useful when you want to measure stats for a specific phase of
        execution (e.g., "what's the hit rate during matrix multiply?"
        without counting the initial data loading phase).
        """
        self.reads = 0
        self.writes = 0
        self.hits = 0
        self.misses = 0
        self.evictions = 0
        self.writebacks = 0

    def __repr__(self) -> str:
        """Human-readable summary of cache statistics."""
        return (
            f"CacheStats(accesses={self.total_accesses}, "
            f"hits={self.hits}, misses={self.misses}, "
            f"hit_rate={self.hit_rate:.1%}, "
            f"evictions={self.evictions}, writebacks={self.writebacks})"
        )
