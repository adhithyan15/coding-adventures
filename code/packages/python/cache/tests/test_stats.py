"""Tests for CacheStats — verifying hit rate calculation and counter tracking.

These tests ensure the scorecard is accurate. If the stats are wrong,
every performance analysis built on top will be misleading.
"""

from __future__ import annotations

from cache.stats import CacheStats


class TestCacheStatsBasics:
    """Basic counter operations and initial state."""

    def test_initial_state_is_all_zeros(self) -> None:
        """A fresh CacheStats should have all counters at zero."""
        stats = CacheStats()
        assert stats.reads == 0
        assert stats.writes == 0
        assert stats.hits == 0
        assert stats.misses == 0
        assert stats.evictions == 0
        assert stats.writebacks == 0
        assert stats.total_accesses == 0

    def test_record_read_hit(self) -> None:
        """Recording a read hit should increment reads and hits."""
        stats = CacheStats()
        stats.record_read(hit=True)
        assert stats.reads == 1
        assert stats.hits == 1
        assert stats.misses == 0

    def test_record_read_miss(self) -> None:
        """Recording a read miss should increment reads and misses."""
        stats = CacheStats()
        stats.record_read(hit=False)
        assert stats.reads == 1
        assert stats.hits == 0
        assert stats.misses == 1

    def test_record_write_hit(self) -> None:
        """Recording a write hit should increment writes and hits."""
        stats = CacheStats()
        stats.record_write(hit=True)
        assert stats.writes == 1
        assert stats.hits == 1

    def test_record_write_miss(self) -> None:
        """Recording a write miss should increment writes and misses."""
        stats = CacheStats()
        stats.record_write(hit=False)
        assert stats.writes == 1
        assert stats.misses == 1

    def test_record_eviction_clean(self) -> None:
        """A clean eviction increments evictions but not writebacks."""
        stats = CacheStats()
        stats.record_eviction(dirty=False)
        assert stats.evictions == 1
        assert stats.writebacks == 0

    def test_record_eviction_dirty(self) -> None:
        """A dirty eviction increments both evictions and writebacks."""
        stats = CacheStats()
        stats.record_eviction(dirty=True)
        assert stats.evictions == 1
        assert stats.writebacks == 1


class TestCacheStatsRates:
    """Hit rate and miss rate calculations."""

    def test_hit_rate_no_accesses(self) -> None:
        """Hit rate should be 0.0 when no accesses have been made."""
        stats = CacheStats()
        assert stats.hit_rate == 0.0

    def test_miss_rate_no_accesses(self) -> None:
        """Miss rate should be 0.0 when no accesses have been made."""
        stats = CacheStats()
        assert stats.miss_rate == 0.0

    def test_hit_rate_all_hits(self) -> None:
        """100% hit rate when every access is a hit."""
        stats = CacheStats()
        for _ in range(10):
            stats.record_read(hit=True)
        assert stats.hit_rate == 1.0
        assert stats.miss_rate == 0.0

    def test_hit_rate_all_misses(self) -> None:
        """0% hit rate when every access is a miss."""
        stats = CacheStats()
        for _ in range(10):
            stats.record_read(hit=False)
        assert stats.hit_rate == 0.0
        assert stats.miss_rate == 1.0

    def test_hit_rate_fifty_percent(self) -> None:
        """50% hit rate with equal hits and misses."""
        stats = CacheStats()
        stats.record_read(hit=True)
        stats.record_read(hit=False)
        assert stats.hit_rate == 0.5
        assert stats.miss_rate == 0.5

    def test_hit_rate_mixed_reads_and_writes(self) -> None:
        """Hit rate includes both reads and writes."""
        stats = CacheStats()
        stats.record_read(hit=True)   # hit
        stats.record_write(hit=True)  # hit
        stats.record_read(hit=False)  # miss
        stats.record_write(hit=False) # miss
        assert stats.total_accesses == 4
        assert stats.hit_rate == 0.5

    def test_hit_rate_plus_miss_rate_equals_one(self) -> None:
        """Hit rate + miss rate should always equal 1.0 (with accesses)."""
        stats = CacheStats()
        stats.record_read(hit=True)
        stats.record_read(hit=True)
        stats.record_read(hit=False)
        assert abs(stats.hit_rate + stats.miss_rate - 1.0) < 1e-10


class TestCacheStatsReset:
    """Reset functionality."""

    def test_reset_clears_all_counters(self) -> None:
        """Reset should bring all counters back to zero."""
        stats = CacheStats()
        stats.record_read(hit=True)
        stats.record_write(hit=False)
        stats.record_eviction(dirty=True)
        stats.reset()
        assert stats.reads == 0
        assert stats.writes == 0
        assert stats.hits == 0
        assert stats.misses == 0
        assert stats.evictions == 0
        assert stats.writebacks == 0
        assert stats.total_accesses == 0

    def test_reset_then_record(self) -> None:
        """Stats should work correctly after a reset."""
        stats = CacheStats()
        stats.record_read(hit=True)
        stats.reset()
        stats.record_read(hit=False)
        assert stats.reads == 1
        assert stats.misses == 1
        assert stats.hit_rate == 0.0


class TestCacheStatsRepr:
    """String representation."""

    def test_repr_includes_key_info(self) -> None:
        """repr should show accesses, hits, misses, and hit rate."""
        stats = CacheStats()
        stats.record_read(hit=True)
        stats.record_read(hit=False)
        r = repr(stats)
        assert "accesses=2" in r
        assert "hits=1" in r
        assert "misses=1" in r
        assert "50.0%" in r
