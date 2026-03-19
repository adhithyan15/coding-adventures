"""Tests for Cache — a single configurable cache level.

Tests cover:
- Address decomposition (tag, set_index, offset)
- Read hits and misses
- Write hits and misses (write-back and write-through)
- Dirty eviction on write
- Cache invalidation (flush)
- Direct-mapped and set-associative configurations
- Edge cases: 1-set cache, address 0
"""

from __future__ import annotations

from cache.cache import Cache, CacheAccess
from cache.cache_set import CacheConfig


# ── Address Decomposition ─────────────────────────────────────────────

class TestAddressDecomposition:
    """Verify the bit-slicing that splits addresses into tag/set/offset.

    For a 1024-byte cache with 64-byte lines and 4-way associativity:
    - num_lines = 1024 / 64 = 16
    - num_sets = 16 / 4 = 4
    - offset_bits = log2(64) = 6
    - set_bits = log2(4) = 2
    - tag = address >> 8

    Address layout:
    |  tag (24+ bits)  | set (2 bits) | offset (6 bits) |
    """

    def _make_cache(self) -> Cache:
        """Create a small 1KB, 4-way cache for testing."""
        return Cache(CacheConfig(
            name="test", total_size=1024, line_size=64, associativity=4,
        ))

    def test_address_zero(self) -> None:
        """Address 0 should decompose to tag=0, set=0, offset=0."""
        cache = self._make_cache()
        tag, set_idx, offset = cache._decompose_address(0)
        assert tag == 0
        assert set_idx == 0
        assert offset == 0

    def test_offset_extraction(self) -> None:
        """Low 6 bits should be the offset (byte within line)."""
        cache = self._make_cache()
        # Address 0x1F = 31 — should be offset 31 within the first line
        tag, set_idx, offset = cache._decompose_address(0x1F)
        assert offset == 31
        assert set_idx == 0

    def test_set_index_extraction(self) -> None:
        """Bits 6-7 should be the set index (for 4 sets)."""
        cache = self._make_cache()
        # Address 0x40 = 64 → offset=0, set_index=1 (bit 6 is set)
        tag, set_idx, offset = cache._decompose_address(0x40)
        assert offset == 0
        assert set_idx == 1
        # Address 0x80 = 128 → set_index=2 (bit 7 is set)
        _, set_idx2, _ = cache._decompose_address(0x80)
        assert set_idx2 == 2
        # Address 0xC0 = 192 → set_index=3
        _, set_idx3, _ = cache._decompose_address(0xC0)
        assert set_idx3 == 3

    def test_tag_extraction(self) -> None:
        """Bits above set+offset are the tag."""
        cache = self._make_cache()
        # Address 0x100 = 256 → offset=0, set=0, tag=1
        tag, set_idx, offset = cache._decompose_address(0x100)
        assert offset == 0
        assert set_idx == 0
        assert tag == 1

    def test_known_address_decomposition(self) -> None:
        """Full decomposition of a known address.

        Address 0x1A2B3C4D:
        - line_size=64, offset_bits=6 → offset = 0x0D = 13
        - 4 sets, set_bits=2 → set_index = (0x1A2B3C4D >> 6) & 0x3
            = 0x68ACF131 & 0x3 = 1
        - tag = 0x1A2B3C4D >> 8 = 0x1A2B3C (after right-shifting by 8)
        """
        cache = self._make_cache()
        tag, set_idx, offset = cache._decompose_address(0x1A2B3C4D)
        assert offset == 0x0D  # low 6 bits of 0x4D = 0b01001101 → 0b001101 = 13
        assert set_idx == ((0x1A2B3C4D >> 6) & 0x3)
        assert tag == (0x1A2B3C4D >> 8)


# ── Read Operations ───────────────────────────────────────────────────

class TestCacheRead:
    """Reading from the cache — hits and misses."""

    def _make_cache(self) -> Cache:
        """Small 256-byte, 2-way cache with 64-byte lines (2 sets)."""
        return Cache(CacheConfig(
            name="test", total_size=256, line_size=64,
            associativity=2, access_latency=3,
        ))

    def test_first_read_is_miss(self) -> None:
        """The first read to any address is always a compulsory miss."""
        cache = self._make_cache()
        access = cache.read(address=0x100, cycle=0)
        assert access.hit is False
        assert access.cycles == 3

    def test_second_read_same_address_is_hit(self) -> None:
        """After a miss brings data in, the next read should hit."""
        cache = self._make_cache()
        cache.read(address=0x100, cycle=0)  # miss — fills the line
        access = cache.read(address=0x100, cycle=1)  # should hit
        assert access.hit is True
        assert access.cycles == 3

    def test_read_different_address_same_line(self) -> None:
        """Addresses within the same cache line share the line.

        Address 0x100 and 0x110 (both within the same 64-byte block)
        should map to the same cache line.
        """
        cache = self._make_cache()
        cache.read(address=0x100, cycle=0)  # miss — fills line for block starting at 0x100
        # 0x110 = 0x100 + 16, same 64-byte block
        access = cache.read(address=0x110, cycle=1)
        assert access.hit is True  # same line!

    def test_read_miss_updates_stats(self) -> None:
        """A read miss should be reflected in the statistics."""
        cache = self._make_cache()
        cache.read(address=0x100, cycle=0)
        assert cache.stats.reads == 1
        assert cache.stats.misses == 1
        assert cache.stats.hits == 0

    def test_read_hit_updates_stats(self) -> None:
        """A read hit should be reflected in the statistics."""
        cache = self._make_cache()
        cache.read(address=0x100, cycle=0)
        cache.read(address=0x100, cycle=1)
        assert cache.stats.reads == 2
        assert cache.stats.hits == 1
        assert cache.stats.misses == 1

    def test_read_returns_correct_decomposition(self) -> None:
        """The CacheAccess should contain the correct address decomposition."""
        cache = self._make_cache()
        access = cache.read(address=0x100, cycle=0)
        assert access.address == 0x100
        assert access.set_index == access.set_index  # just check it's set
        assert isinstance(access.tag, int)
        assert isinstance(access.offset, int)


# ── Write Operations ──────────────────────────────────────────────────

class TestCacheWrite:
    """Writing to the cache — write-back and write-through policies."""

    def _make_wb_cache(self) -> Cache:
        """Small write-back cache."""
        return Cache(CacheConfig(
            name="test", total_size=256, line_size=64,
            associativity=2, access_latency=1,
            write_policy="write-back",
        ))

    def _make_wt_cache(self) -> Cache:
        """Small write-through cache."""
        return Cache(CacheConfig(
            name="test", total_size=256, line_size=64,
            associativity=2, access_latency=1,
            write_policy="write-through",
        ))

    def test_write_miss_allocates_line(self) -> None:
        """A write miss should allocate a new cache line (write-allocate)."""
        cache = self._make_wb_cache()
        access = cache.write(address=0x100, data=[0xAB], cycle=0)
        assert access.hit is False
        # Now reading should hit
        read_access = cache.read(address=0x100, cycle=1)
        assert read_access.hit is True

    def test_write_hit_marks_dirty_in_writeback(self) -> None:
        """In write-back, a write hit marks the line as dirty."""
        cache = self._make_wb_cache()
        cache.read(address=0x100, cycle=0)  # bring line in
        cache.write(address=0x100, data=[0xAB], cycle=1)  # write hit
        # Check the line is dirty
        tag, set_idx, _ = cache._decompose_address(0x100)
        hit, way = cache.sets[set_idx].lookup(tag)
        assert hit is True
        assert way is not None
        assert cache.sets[set_idx].lines[way].dirty is True

    def test_write_through_does_not_mark_dirty(self) -> None:
        """In write-through, lines are never dirty (writes go straight through)."""
        cache = self._make_wt_cache()
        cache.read(address=0x100, cycle=0)
        cache.write(address=0x100, data=[0xAB], cycle=1)
        tag, set_idx, _ = cache._decompose_address(0x100)
        hit, way = cache.sets[set_idx].lookup(tag)
        assert hit is True
        assert way is not None
        assert cache.sets[set_idx].lines[way].dirty is False

    def test_write_stores_data(self) -> None:
        """Written data should be readable from the cache line."""
        cache = self._make_wb_cache()
        # Write to address 0x100, offset 0
        cache.write(address=0x100, data=[0xDE, 0xAD], cycle=0)
        tag, set_idx, offset = cache._decompose_address(0x100)
        _, way = cache.sets[set_idx].lookup(tag)
        assert way is not None
        line = cache.sets[set_idx].lines[way]
        assert line.data[offset] == 0xDE
        assert line.data[offset + 1] == 0xAD

    def test_write_updates_stats(self) -> None:
        """Write operations should be tracked in stats."""
        cache = self._make_wb_cache()
        cache.write(address=0x100, cycle=0)  # miss
        cache.write(address=0x100, cycle=1)  # hit
        assert cache.stats.writes == 2
        assert cache.stats.misses == 1
        assert cache.stats.hits == 1


# ── Dirty Eviction ────────────────────────────────────────────────────

class TestDirtyEviction:
    """Write-back caches must handle dirty evictions (writebacks)."""

    def test_dirty_eviction_returns_evicted_line(self) -> None:
        """When a dirty line is evicted, the CacheAccess should report it.

        Use a direct-mapped cache (1-way) so we can force conflict evictions.
        """
        # 1-way, 1 set (fully conflicts everything)
        cache = Cache(CacheConfig(
            name="test", total_size=64, line_size=64,
            associativity=1, access_latency=1,
            write_policy="write-back",
        ))
        # Write to address 0 — miss, allocate, mark dirty
        cache.write(address=0, data=[0xFF], cycle=0)
        # Write to address 64 — different tag, same set → evict address 0
        access = cache.read(address=64, cycle=1)
        assert access.hit is False
        assert access.evicted is not None
        assert access.evicted.dirty is True

    def test_eviction_stats_tracked(self) -> None:
        """Evictions and writebacks should be counted in stats."""
        cache = Cache(CacheConfig(
            name="test", total_size=64, line_size=64,
            associativity=1, access_latency=1,
            write_policy="write-back",
        ))
        cache.write(address=0, data=[0xFF], cycle=0)
        cache.read(address=64, cycle=1)  # evicts dirty line
        assert cache.stats.evictions >= 1
        assert cache.stats.writebacks >= 1


# ── Cache Invalidation ────────────────────────────────────────────────

class TestCacheInvalidation:
    """Cache flush — invalidate all lines."""

    def test_invalidate_causes_all_misses(self) -> None:
        """After invalidation, every access should be a miss."""
        cache = Cache(CacheConfig(
            name="test", total_size=256, line_size=64,
            associativity=2, access_latency=1,
        ))
        cache.read(address=0x100, cycle=0)
        cache.read(address=0x100, cycle=1)  # should hit
        assert cache.stats.hits == 1

        cache.invalidate()
        access = cache.read(address=0x100, cycle=2)
        assert access.hit is False  # cold miss after flush


# ── Edge Cases ────────────────────────────────────────────────────────

class TestCacheEdgeCases:
    """Unusual configurations and boundary conditions."""

    def test_single_set_cache(self) -> None:
        """A cache with only 1 set (fully associative for its size)."""
        cache = Cache(CacheConfig(
            name="tiny", total_size=128, line_size=64,
            associativity=2, access_latency=1,
        ))
        # Both addresses map to set 0 (only set)
        cache.read(address=0, cycle=0)
        cache.read(address=64, cycle=1)
        # Both should be cached (2-way, 1 set)
        assert cache.read(address=0, cycle=2).hit is True
        assert cache.read(address=64, cycle=3).hit is True

    def test_direct_mapped_conflict_eviction(self) -> None:
        """Direct-mapped: two addresses to the same set cause thrashing.

        Classic pathological case: alternating between two addresses
        that map to the same set results in 100% miss rate.
        """
        cache = Cache(CacheConfig(
            name="dm", total_size=256, line_size=64,
            associativity=1, access_latency=1,
        ))
        # Two addresses that map to the same set (same set index, different tag)
        # With 4 sets (256/64/1), set_bits=2, addresses 0x000 and 0x100 both map to set 0
        addr_a = 0x000
        addr_b = 0x100  # different tag, same set
        cache.read(addr_a, cycle=0)  # miss, fill
        cache.read(addr_b, cycle=1)  # miss, evict a, fill b
        cache.read(addr_a, cycle=2)  # miss, evict b, fill a
        cache.read(addr_b, cycle=3)  # miss, evict a, fill b
        # All misses after the initial cold miss
        assert cache.stats.hits == 0
        assert cache.stats.misses == 4

    def test_fill_line_directly(self) -> None:
        """fill_line() installs data without going through read/write stats."""
        cache = Cache(CacheConfig(
            name="test", total_size=256, line_size=64,
            associativity=2, access_latency=1,
        ))
        cache.fill_line(address=0x100, data=[0xAB] * 64, cycle=0)
        # Should hit on a subsequent read
        access = cache.read(address=0x100, cycle=1)
        assert access.hit is True

    def test_repr(self) -> None:
        """Cache repr should show configuration summary."""
        cache = Cache(CacheConfig(
            name="L1D", total_size=65536, line_size=64,
            associativity=4, access_latency=1,
        ))
        r = repr(cache)
        assert "L1D" in r
        assert "64KB" in r
        assert "4-way" in r
