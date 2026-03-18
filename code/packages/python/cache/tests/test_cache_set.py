"""Tests for CacheSet and CacheConfig — set-associative lookup with LRU.

Tests cover:
- CacheConfig validation (powers of 2, valid policies)
- CacheSet lookup (hit/miss), access (LRU update), allocation, eviction
- LRU replacement ordering
- Direct-mapped (1-way) behavior
"""

from __future__ import annotations

import pytest

from cache.cache_line import CacheLine
from cache.cache_set import CacheConfig, CacheSet


# ── CacheConfig Validation ────────────────────────────────────────────

class TestCacheConfigValidation:
    """Configuration parameters must follow hardware constraints."""

    def test_valid_config(self) -> None:
        """A typical L1 config should be accepted."""
        config = CacheConfig(
            name="L1D", total_size=65536, line_size=64,
            associativity=4, access_latency=1,
        )
        assert config.num_sets == 256
        assert config.num_lines == 1024

    def test_invalid_total_size(self) -> None:
        """Total size must be positive."""
        with pytest.raises(ValueError, match="total_size must be positive"):
            CacheConfig(name="bad", total_size=0)

    def test_invalid_line_size_not_power_of_2(self) -> None:
        """Line size must be a power of 2."""
        with pytest.raises(ValueError, match="line_size must be a positive power of 2"):
            CacheConfig(name="bad", total_size=256, line_size=48)

    def test_invalid_associativity(self) -> None:
        """Associativity must be positive."""
        with pytest.raises(ValueError, match="associativity must be positive"):
            CacheConfig(name="bad", total_size=256, associativity=0)

    def test_invalid_size_alignment(self) -> None:
        """total_size must be divisible by line_size * associativity."""
        with pytest.raises(ValueError, match="must be divisible"):
            CacheConfig(name="bad", total_size=100, line_size=64, associativity=4)

    def test_invalid_write_policy(self) -> None:
        """Write policy must be 'write-back' or 'write-through'."""
        with pytest.raises(ValueError, match="write_policy must be"):
            CacheConfig(
                name="bad", total_size=256, line_size=64,
                associativity=1, write_policy="write-around",
            )

    def test_negative_latency(self) -> None:
        """Access latency must be non-negative."""
        with pytest.raises(ValueError, match="access_latency must be non-negative"):
            CacheConfig(
                name="bad", total_size=256, line_size=64,
                associativity=1, access_latency=-1,
            )

    def test_write_through_config(self) -> None:
        """Write-through is a valid write policy."""
        config = CacheConfig(
            name="L1D", total_size=256, line_size=64,
            associativity=1, write_policy="write-through",
        )
        assert config.write_policy == "write-through"

    def test_config_is_frozen(self) -> None:
        """CacheConfig is a frozen dataclass — immutable after creation."""
        config = CacheConfig(name="L1D", total_size=256, line_size=64, associativity=1)
        with pytest.raises(AttributeError):
            config.total_size = 512  # type: ignore[misc]


# ── CacheSet Lookup ───────────────────────────────────────────────────

class TestCacheSetLookup:
    """Looking up tags in a cache set."""

    def test_lookup_miss_on_empty_set(self) -> None:
        """An empty set should always miss (all lines invalid)."""
        cs = CacheSet(associativity=4, line_size=64)
        hit, way = cs.lookup(tag=42)
        assert hit is False
        assert way is None

    def test_lookup_hit_after_fill(self) -> None:
        """After filling a line, lookup should find it."""
        cs = CacheSet(associativity=4, line_size=8)
        cs.lines[0].fill(tag=42, data=[0] * 8, cycle=0)
        hit, way = cs.lookup(tag=42)
        assert hit is True
        assert way == 0

    def test_lookup_miss_wrong_tag(self) -> None:
        """Lookup with a different tag should miss."""
        cs = CacheSet(associativity=4, line_size=8)
        cs.lines[0].fill(tag=42, data=[0] * 8, cycle=0)
        hit, way = cs.lookup(tag=99)
        assert hit is False
        assert way is None

    def test_lookup_finds_correct_way(self) -> None:
        """When multiple ways are valid, lookup returns the matching one."""
        cs = CacheSet(associativity=4, line_size=8)
        cs.lines[0].fill(tag=10, data=[0] * 8, cycle=0)
        cs.lines[1].fill(tag=20, data=[0] * 8, cycle=0)
        cs.lines[2].fill(tag=30, data=[0] * 8, cycle=0)
        hit, way = cs.lookup(tag=20)
        assert hit is True
        assert way == 1


# ── CacheSet Access ───────────────────────────────────────────────────

class TestCacheSetAccess:
    """Accessing a set — hit returns line, miss returns LRU victim."""

    def test_access_hit_updates_lru(self) -> None:
        """On a hit, the line's last_access should be updated."""
        cs = CacheSet(associativity=2, line_size=8)
        cs.lines[0].fill(tag=10, data=[0] * 8, cycle=5)
        hit, line = cs.access(tag=10, cycle=100)
        assert hit is True
        assert line.last_access == 100

    def test_access_miss_returns_lru_victim(self) -> None:
        """On a miss with all ways full, return the LRU line."""
        cs = CacheSet(associativity=2, line_size=8)
        cs.lines[0].fill(tag=10, data=[0] * 8, cycle=1)
        cs.lines[1].fill(tag=20, data=[0] * 8, cycle=5)
        hit, victim = cs.access(tag=99, cycle=10)
        assert hit is False
        # Victim should be lines[0] (accessed at cycle 1, older than cycle 5)
        assert victim.tag == 10


# ── CacheSet Allocate ─────────────────────────────────────────────────

class TestCacheSetAllocate:
    """Allocation after a miss — fill empty slot or evict LRU."""

    def test_allocate_into_empty_slot(self) -> None:
        """If there's an invalid line, use it (no eviction)."""
        cs = CacheSet(associativity=4, line_size=8)
        evicted = cs.allocate(tag=42, data=[0xAA] * 8, cycle=10)
        assert evicted is None  # no eviction needed
        hit, way = cs.lookup(tag=42)
        assert hit is True

    def test_allocate_evicts_lru_when_full(self) -> None:
        """When all ways are valid, LRU line is evicted."""
        cs = CacheSet(associativity=2, line_size=8)
        # Fill both ways
        cs.allocate(tag=10, data=[0] * 8, cycle=1)
        cs.allocate(tag=20, data=[0] * 8, cycle=2)
        # Now allocate a third — should evict tag=10 (cycle=1 is older)
        evicted = cs.allocate(tag=30, data=[0] * 8, cycle=3)
        # tag=10 was not dirty, so evicted should be None
        assert evicted is None
        # tag=10 should be gone, tag=30 should be present
        hit_10, _ = cs.lookup(tag=10)
        hit_30, _ = cs.lookup(tag=30)
        assert hit_10 is False
        assert hit_30 is True

    def test_allocate_returns_dirty_eviction(self) -> None:
        """If the LRU line is dirty, it should be returned for writeback."""
        cs = CacheSet(associativity=2, line_size=8)
        cs.allocate(tag=10, data=[0xAA] * 8, cycle=1)
        cs.lines[0].dirty = True  # mark first line as dirty
        cs.allocate(tag=20, data=[0] * 8, cycle=2)
        # Now allocate a third — should evict dirty tag=10
        evicted = cs.allocate(tag=30, data=[0] * 8, cycle=3)
        assert evicted is not None
        assert evicted.dirty is True
        assert evicted.tag == 10
        assert evicted.data == [0xAA] * 8

    def test_allocate_fills_all_empty_slots_first(self) -> None:
        """Empty slots should be filled before any eviction occurs."""
        cs = CacheSet(associativity=4, line_size=8)
        for i in range(4):
            evicted = cs.allocate(tag=i, data=[0] * 8, cycle=i)
            assert evicted is None  # still had empty slots
        # 5th allocation must evict
        cs.allocate(tag=99, data=[0] * 8, cycle=10)
        # tag=0 (cycle=0) should have been evicted as LRU
        hit_0, _ = cs.lookup(tag=0)
        assert hit_0 is False


# ── LRU Ordering ──────────────────────────────────────────────────────

class TestLRUReplacement:
    """Verify LRU replacement policy works correctly."""

    def test_lru_prefers_invalid_lines(self) -> None:
        """Invalid lines should always be chosen over valid ones."""
        cs = CacheSet(associativity=4, line_size=8)
        cs.lines[0].fill(tag=1, data=[0] * 8, cycle=100)
        # lines[1], [2], [3] are invalid — should pick one of them
        lru = cs._find_lru()
        assert lru in {1, 2, 3}

    def test_lru_picks_oldest_access(self) -> None:
        """Among valid lines, LRU picks the one with the smallest last_access."""
        cs = CacheSet(associativity=4, line_size=8)
        cs.lines[0].fill(tag=1, data=[0] * 8, cycle=10)
        cs.lines[1].fill(tag=2, data=[0] * 8, cycle=5)   # oldest
        cs.lines[2].fill(tag=3, data=[0] * 8, cycle=20)
        cs.lines[3].fill(tag=4, data=[0] * 8, cycle=15)
        lru = cs._find_lru()
        assert lru == 1  # cycle=5 is the oldest

    def test_accessing_a_line_prevents_its_eviction(self) -> None:
        """Touching a line makes it the most recently used."""
        cs = CacheSet(associativity=2, line_size=8)
        cs.lines[0].fill(tag=10, data=[0] * 8, cycle=1)
        cs.lines[1].fill(tag=20, data=[0] * 8, cycle=2)
        # Access tag=10 at a later cycle — it becomes most recent
        cs.access(tag=10, cycle=100)
        # Now tag=20 (cycle=2) is older than tag=10 (cycle=100)
        lru = cs._find_lru()
        assert lru == 1  # tag=20 is now LRU


# ── Direct-Mapped (1-way) ────────────────────────────────────────────

class TestDirectMappedSet:
    """A set with associativity=1 behaves as direct-mapped."""

    def test_direct_mapped_conflict(self) -> None:
        """Two addresses mapping to the same set cause a conflict miss."""
        cs = CacheSet(associativity=1, line_size=8)
        cs.allocate(tag=10, data=[0] * 8, cycle=1)
        # Allocating a different tag to the same set evicts the first
        cs.allocate(tag=20, data=[0] * 8, cycle=2)
        hit_10, _ = cs.lookup(tag=10)
        hit_20, _ = cs.lookup(tag=20)
        assert hit_10 is False
        assert hit_20 is True
