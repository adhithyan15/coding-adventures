"""Tests for CacheLine — the smallest unit of data in a cache.

Verifies the lifecycle of a cache line: creation (invalid), filling
(valid + data), touching (LRU update), modification (dirty), and
invalidation.
"""

from __future__ import annotations

from cache.cache_line import CacheLine


class TestCacheLineCreation:
    """Initial state of a newly created cache line."""

    def test_default_line_is_invalid(self) -> None:
        """A new cache line should be invalid (empty box)."""
        line = CacheLine()
        assert line.valid is False
        assert line.dirty is False
        assert line.tag == 0
        assert line.last_access == 0

    def test_default_line_size_is_64(self) -> None:
        """Default line size is 64 bytes (standard on modern CPUs)."""
        line = CacheLine()
        assert len(line.data) == 64
        assert line.line_size == 64

    def test_custom_line_size(self) -> None:
        """Lines can be created with non-standard sizes (e.g., 32 bytes)."""
        line = CacheLine(line_size=32)
        assert len(line.data) == 32
        assert line.line_size == 32

    def test_data_initialized_to_zeros(self) -> None:
        """All bytes in a new line should be zero."""
        line = CacheLine(line_size=8)
        assert line.data == [0, 0, 0, 0, 0, 0, 0, 0]


class TestCacheLineFill:
    """Filling a cache line with data from memory."""

    def test_fill_makes_line_valid(self) -> None:
        """After fill, the line should be valid with the correct tag."""
        line = CacheLine(line_size=8)
        line.fill(tag=42, data=[1, 2, 3, 4, 5, 6, 7, 8], cycle=100)
        assert line.valid is True
        assert line.tag == 42
        assert line.last_access == 100

    def test_fill_sets_data(self) -> None:
        """Fill should store the provided data bytes."""
        line = CacheLine(line_size=4)
        line.fill(tag=7, data=[0xAA, 0xBB, 0xCC, 0xDD], cycle=0)
        assert line.data == [0xAA, 0xBB, 0xCC, 0xDD]

    def test_fill_clears_dirty_bit(self) -> None:
        """Freshly loaded data is always clean (not modified)."""
        line = CacheLine(line_size=4)
        line.dirty = True  # simulate a prior dirty state
        line.fill(tag=1, data=[0] * 4, cycle=0)
        assert line.dirty is False

    def test_fill_makes_defensive_copy(self) -> None:
        """Fill should copy the data, not hold a reference to the original."""
        line = CacheLine(line_size=4)
        original = [1, 2, 3, 4]
        line.fill(tag=1, data=original, cycle=0)
        original[0] = 99  # mutate the original
        assert line.data[0] == 1  # line's data should be unchanged


class TestCacheLineTouch:
    """LRU tracking via touch()."""

    def test_touch_updates_last_access(self) -> None:
        """touch() should update the LRU timestamp."""
        line = CacheLine()
        line.fill(tag=1, data=[0] * 64, cycle=10)
        assert line.last_access == 10
        line.touch(cycle=50)
        assert line.last_access == 50


class TestCacheLineInvalidate:
    """Invalidation (cache flush / coherence)."""

    def test_invalidate_clears_valid_and_dirty(self) -> None:
        """Invalidation marks the line as not present."""
        line = CacheLine(line_size=4)
        line.fill(tag=5, data=[1, 2, 3, 4], cycle=10)
        line.dirty = True
        line.invalidate()
        assert line.valid is False
        assert line.dirty is False

    def test_invalidate_does_not_zero_data(self) -> None:
        """Data is not erased on invalidation (just marked invalid)."""
        line = CacheLine(line_size=4)
        line.fill(tag=5, data=[0xAA, 0xBB, 0xCC, 0xDD], cycle=0)
        line.invalidate()
        # Data still physically present (like a file in a recycle bin)
        assert line.data == [0xAA, 0xBB, 0xCC, 0xDD]


class TestCacheLineRepr:
    """String representation for debugging."""

    def test_repr_invalid_line(self) -> None:
        """Invalid lines show '--' for valid/dirty flags."""
        line = CacheLine()
        r = repr(line)
        assert "--" in r

    def test_repr_valid_clean_line(self) -> None:
        """Valid clean lines show 'V-'."""
        line = CacheLine(line_size=4)
        line.fill(tag=0xFF, data=[0] * 4, cycle=0)
        r = repr(line)
        assert "V-" in r
        assert "0xff" in r.lower()

    def test_repr_valid_dirty_line(self) -> None:
        """Valid dirty lines show 'VD'."""
        line = CacheLine(line_size=4)
        line.fill(tag=1, data=[0] * 4, cycle=0)
        line.dirty = True
        r = repr(line)
        assert "VD" in r
