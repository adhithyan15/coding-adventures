"""Tests for CacheHierarchy — multi-level cache system.

Tests cover:
- L1 hit (fastest path)
- L1 miss → L2 hit
- L1+L2 miss → L3 hit
- All miss → main memory
- Inclusive fill policy (data fills back up through all levels)
- Harvard architecture (separate L1I)
- Write through hierarchy
- No-cache configuration (straight to memory)
- Hierarchy utilities (invalidate_all, reset_stats)
"""

from __future__ import annotations

from cache.cache import Cache
from cache.cache_set import CacheConfig
from cache.hierarchy import CacheHierarchy


# ── Helper Factories ──────────────────────────────────────────────────

def make_l1d(size: int = 256) -> Cache:
    """Create a small L1D cache (256B, 2-way, 1-cycle latency)."""
    return Cache(CacheConfig(
        name="L1D", total_size=size, line_size=64,
        associativity=2, access_latency=1,
    ))


def make_l2(size: int = 1024) -> Cache:
    """Create a small L2 cache (1KB, 4-way, 10-cycle latency)."""
    return Cache(CacheConfig(
        name="L2", total_size=size, line_size=64,
        associativity=4, access_latency=10,
    ))


def make_l3(size: int = 4096) -> Cache:
    """Create a small L3 cache (4KB, 8-way, 30-cycle latency)."""
    return Cache(CacheConfig(
        name="L3", total_size=size, line_size=64,
        associativity=8, access_latency=30,
    ))


# ── Read Through Hierarchy ────────────────────────────────────────────

class TestHierarchyRead:
    """Reading through the multi-level hierarchy."""

    def test_first_read_goes_to_memory(self) -> None:
        """On a cold cache, the first read must go all the way to main memory."""
        h = CacheHierarchy(l1d=make_l1d(), l2=make_l2(), main_memory_latency=100)
        result = h.read(0x1000, cycle=0)
        assert result.served_by == "memory"
        # Total: L1 latency (1) + L2 latency (10) + memory (100)
        assert result.total_cycles == 1 + 10 + 100

    def test_second_read_hits_l1(self) -> None:
        """After data is filled into L1, the second read should hit L1."""
        h = CacheHierarchy(l1d=make_l1d(), l2=make_l2(), main_memory_latency=100)
        h.read(0x1000, cycle=0)  # miss → fills L1
        result = h.read(0x1000, cycle=1)
        assert result.served_by == "L1D"
        assert result.total_cycles == 1  # just L1 latency

    def test_l1_miss_l2_hit(self) -> None:
        """If L1 misses but L2 has it, data should be served from L2.

        To set this up: fill L2 directly, then read through the hierarchy.
        """
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        # First, prime L2 by filling it directly
        l2.fill_line(0x1000, data=[0] * 64, cycle=0)

        # Now read — L1 will miss, L2 should hit
        result = h.read(0x1000, cycle=1)
        assert result.served_by == "L2"
        assert result.total_cycles == 1 + 10  # L1 miss + L2 hit

    def test_l1_l2_miss_l3_hit(self) -> None:
        """L1 and L2 miss, but L3 has the data."""
        l1d = make_l1d()
        l2 = make_l2()
        l3 = make_l3()
        h = CacheHierarchy(l1d=l1d, l2=l2, l3=l3, main_memory_latency=100)

        # Prime L3 directly
        l3.fill_line(0x2000, data=[0] * 64, cycle=0)

        result = h.read(0x2000, cycle=1)
        assert result.served_by == "L3"
        assert result.total_cycles == 1 + 10 + 30  # L1 + L2 + L3

    def test_all_levels_miss_goes_to_memory(self) -> None:
        """When all cache levels miss, the request goes to main memory."""
        l1d = make_l1d()
        l2 = make_l2()
        l3 = make_l3()
        h = CacheHierarchy(l1d=l1d, l2=l2, l3=l3, main_memory_latency=100)

        result = h.read(0x3000, cycle=0)
        assert result.served_by == "memory"
        assert result.total_cycles == 1 + 10 + 30 + 100

    def test_inclusive_fill_after_l2_hit(self) -> None:
        """When L2 serves data, L1 should also be filled (inclusive policy).

        After the L2 hit, a subsequent read should hit at L1.
        """
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        l2.fill_line(0x1000, data=[0] * 64, cycle=0)
        h.read(0x1000, cycle=1)  # L1 miss, L2 hit → fills L1

        result = h.read(0x1000, cycle=2)
        assert result.served_by == "L1D"

    def test_inclusive_fill_after_memory(self) -> None:
        """When memory serves data, all levels should be filled."""
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        h.read(0x5000, cycle=0)  # all miss → memory
        # Now L1 should have it
        result = h.read(0x5000, cycle=1)
        assert result.served_by == "L1D"


# ── Instruction Cache (Harvard Architecture) ──────────────────────────

class TestHarvardArchitecture:
    """Separate L1 instruction and data caches."""

    def test_instruction_read_uses_l1i(self) -> None:
        """Instruction reads should go through L1I, not L1D."""
        l1i = Cache(CacheConfig(
            name="L1I", total_size=256, line_size=64,
            associativity=2, access_latency=1,
        ))
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1i=l1i, l1d=l1d, l2=l2, main_memory_latency=100)

        # Prime L1I directly
        l1i.fill_line(0x1000, data=[0] * 64, cycle=0)

        result = h.read(0x1000, is_instruction=True, cycle=1)
        assert result.served_by == "L1I"
        assert result.total_cycles == 1

    def test_data_read_does_not_use_l1i(self) -> None:
        """Data reads should use L1D, even if L1I has the data."""
        l1i = Cache(CacheConfig(
            name="L1I", total_size=256, line_size=64,
            associativity=2, access_latency=1,
        ))
        l1d = make_l1d()
        h = CacheHierarchy(l1i=l1i, l1d=l1d, main_memory_latency=100)

        l1i.fill_line(0x1000, data=[0] * 64, cycle=0)

        result = h.read(0x1000, is_instruction=False, cycle=1)
        # L1D doesn't have it — goes to memory
        assert result.served_by == "memory"


# ── Write Through Hierarchy ───────────────────────────────────────────

class TestHierarchyWrite:
    """Writing through the hierarchy."""

    def test_write_hit_at_l1(self) -> None:
        """If L1D has the data, write hits there."""
        l1d = make_l1d()
        h = CacheHierarchy(l1d=l1d, main_memory_latency=100)

        h.read(0x1000, cycle=0)  # fill L1
        result = h.write(0x1000, data=[0xAB], cycle=1)
        assert result.served_by == "L1D"
        assert result.total_cycles == 1

    def test_write_miss_goes_to_lower_levels(self) -> None:
        """A write miss at L1 walks down to find the data."""
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        result = h.write(0x2000, data=[0xFF], cycle=0)
        # L1 misses, L2 misses → memory
        assert result.served_by == "memory"

    def test_write_miss_l2_hit(self) -> None:
        """Write miss at L1, but L2 has the data."""
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        l2.fill_line(0x1000, data=[0] * 64, cycle=0)
        result = h.write(0x1000, data=[0xAB], cycle=1)
        assert result.served_by == "L2"


# ── No-Cache Configuration ───────────────────────────────────────────

class TestNoCacheHierarchy:
    """Edge case: hierarchy with no caches at all."""

    def test_read_goes_straight_to_memory(self) -> None:
        """With no caches, every read costs main memory latency."""
        h = CacheHierarchy(main_memory_latency=200)
        result = h.read(0x1000, cycle=0)
        assert result.served_by == "memory"
        assert result.total_cycles == 200

    def test_write_goes_straight_to_memory(self) -> None:
        """With no caches, every write costs main memory latency."""
        h = CacheHierarchy(main_memory_latency=200)
        result = h.write(0x1000, data=[0xAB], cycle=0)
        assert result.served_by == "memory"
        assert result.total_cycles == 200


# ── Utilities ─────────────────────────────────────────────────────────

class TestHierarchyUtilities:
    """Helper methods: invalidate_all, reset_stats, repr."""

    def test_invalidate_all(self) -> None:
        """invalidate_all() should cause all subsequent reads to miss."""
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        h.read(0x1000, cycle=0)
        h.read(0x1000, cycle=1)  # L1 hit
        h.invalidate_all()
        result = h.read(0x1000, cycle=2)
        assert result.served_by == "memory"  # cold miss after flush

    def test_reset_stats(self) -> None:
        """reset_stats() should zero all cache level stats."""
        l1d = make_l1d()
        l2 = make_l2()
        h = CacheHierarchy(l1d=l1d, l2=l2, main_memory_latency=100)

        h.read(0x1000, cycle=0)
        h.reset_stats()
        assert l1d.stats.total_accesses == 0
        assert l2.stats.total_accesses == 0

    def test_repr(self) -> None:
        """repr should summarize the hierarchy configuration."""
        h = CacheHierarchy(
            l1d=make_l1d(), l2=make_l2(), l3=make_l3(),
            main_memory_latency=100,
        )
        r = repr(h)
        assert "L1D" in r
        assert "L2" in r
        assert "L3" in r
        assert "mem=100cyc" in r

    def test_hit_at_level_tracking(self) -> None:
        """HierarchyAccess should report which level index served the data."""
        h = CacheHierarchy(l1d=make_l1d(), l2=make_l2(), main_memory_latency=100)
        # First read → memory (level index = 2, beyond all caches)
        result = h.read(0x1000, cycle=0)
        assert result.hit_at_level == 2  # past L1D (0) and L2 (1)
        # Second read → L1D (level index = 0)
        result = h.read(0x1000, cycle=1)
        assert result.hit_at_level == 0
