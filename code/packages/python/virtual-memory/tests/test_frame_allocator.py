"""Tests for PhysicalFrameAllocator — bitmap-based frame manager."""

import pytest

from virtual_memory.frame_allocator import PhysicalFrameAllocator


class TestFrameAllocatorInit:
    """Test allocator initialization."""

    def test_all_frames_free_initially(self) -> None:
        """A fresh allocator has all frames free."""
        alloc = PhysicalFrameAllocator(total_frames=16)
        assert alloc.free_count() == 16
        assert alloc.allocated_count() == 0

    def test_total_frames_property(self) -> None:
        """total_frames reports the configured total."""
        alloc = PhysicalFrameAllocator(total_frames=256)
        assert alloc.total_frames == 256

    def test_invalid_total_frames(self) -> None:
        """Zero or negative total_frames raises ValueError."""
        with pytest.raises(ValueError):
            PhysicalFrameAllocator(total_frames=0)
        with pytest.raises(ValueError):
            PhysicalFrameAllocator(total_frames=-1)


class TestFrameAllocation:
    """Test allocate and free operations."""

    def test_allocate_sequential(self) -> None:
        """Frames are allocated sequentially starting from 0."""
        alloc = PhysicalFrameAllocator(total_frames=8)
        assert alloc.allocate() == 0
        assert alloc.allocate() == 1
        assert alloc.allocate() == 2

    def test_allocate_updates_counts(self) -> None:
        """Allocation updates free and allocated counts."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        alloc.allocate()
        assert alloc.free_count() == 3
        assert alloc.allocated_count() == 1

    def test_allocate_all_frames(self) -> None:
        """All frames can be allocated."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        frames = [alloc.allocate() for _ in range(4)]
        assert frames == [0, 1, 2, 3]
        assert alloc.free_count() == 0

    def test_allocate_when_full_returns_none(self) -> None:
        """Allocating when all frames are used returns None."""
        alloc = PhysicalFrameAllocator(total_frames=2)
        alloc.allocate()
        alloc.allocate()
        assert alloc.allocate() is None

    def test_free_and_reallocate(self) -> None:
        """Freeing a frame makes it available for reallocation."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        alloc.allocate()  # 0
        alloc.allocate()  # 1
        alloc.allocate()  # 2
        alloc.allocate()  # 3

        alloc.free(1)
        assert alloc.free_count() == 1

        # Re-allocate: should get frame 1 back (first free frame).
        frame = alloc.allocate()
        assert frame == 1

    def test_is_allocated(self) -> None:
        """is_allocated() reports frame status correctly."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        assert alloc.is_allocated(0) is False

        alloc.allocate()
        assert alloc.is_allocated(0) is True

        alloc.free(0)
        assert alloc.is_allocated(0) is False


class TestFrameAllocatorErrors:
    """Test error handling."""

    def test_free_out_of_range(self) -> None:
        """Freeing an out-of-range frame raises ValueError."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        with pytest.raises(ValueError):
            alloc.free(10)
        with pytest.raises(ValueError):
            alloc.free(-1)

    def test_double_free(self) -> None:
        """Freeing an already-free frame raises RuntimeError."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        alloc.allocate()  # frame 0
        alloc.free(0)

        with pytest.raises(RuntimeError, match="double-free"):
            alloc.free(0)

    def test_is_allocated_out_of_range(self) -> None:
        """Checking an out-of-range frame raises ValueError."""
        alloc = PhysicalFrameAllocator(total_frames=4)
        with pytest.raises(ValueError):
            alloc.is_allocated(100)
        with pytest.raises(ValueError):
            alloc.is_allocated(-1)
