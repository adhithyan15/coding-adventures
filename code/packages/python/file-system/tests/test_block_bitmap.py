"""Tests for the BlockBitmap module."""

import pytest

from file_system.block_bitmap import BlockBitmap


class TestBlockBitmap:
    """Verify that BlockBitmap tracks free/used blocks correctly."""

    def test_all_blocks_free_initially(self) -> None:
        """A new bitmap starts with all blocks free."""
        bm = BlockBitmap(10)
        assert bm.free_count() == 10
        for i in range(10):
            assert bm.is_free(i) is True

    def test_allocate_returns_sequential_blocks(self) -> None:
        """Allocating blocks returns them in order (first-fit)."""
        bm = BlockBitmap(5)
        assert bm.allocate() == 0
        assert bm.allocate() == 1
        assert bm.allocate() == 2

    def test_allocate_marks_block_as_used(self) -> None:
        """After allocation, is_free returns False."""
        bm = BlockBitmap(5)
        block = bm.allocate()
        assert block is not None
        assert bm.is_free(block) is False

    def test_free_makes_block_available(self) -> None:
        """After freeing, is_free returns True and the block can be reused."""
        bm = BlockBitmap(5)
        block = bm.allocate()
        assert block is not None
        bm.free(block)
        assert bm.is_free(block) is True

    def test_free_block_is_reused(self) -> None:
        """Freeing block 0 then allocating again returns block 0."""
        bm = BlockBitmap(5)
        bm.allocate()  # 0
        bm.allocate()  # 1
        bm.free(0)
        assert bm.allocate() == 0  # Reuses freed block

    def test_exhaustion(self) -> None:
        """Allocating all blocks, then trying again returns None."""
        bm = BlockBitmap(3)
        bm.allocate()  # 0
        bm.allocate()  # 1
        bm.allocate()  # 2
        assert bm.allocate() is None  # Full
        assert bm.free_count() == 0

    def test_free_count_decreases_on_allocate(self) -> None:
        """Each allocation reduces the free count by one."""
        bm = BlockBitmap(10)
        assert bm.free_count() == 10
        bm.allocate()
        assert bm.free_count() == 9
        bm.allocate()
        assert bm.free_count() == 8

    def test_free_count_increases_on_free(self) -> None:
        """Freeing a block increases the free count."""
        bm = BlockBitmap(10)
        block = bm.allocate()
        assert block is not None
        assert bm.free_count() == 9
        bm.free(block)
        assert bm.free_count() == 10

    def test_free_out_of_range_raises(self) -> None:
        """Freeing a block number beyond the range raises ValueError."""
        bm = BlockBitmap(5)
        with pytest.raises(ValueError):
            bm.free(5)
        with pytest.raises(ValueError):
            bm.free(-1)

    def test_is_free_out_of_range_raises(self) -> None:
        """Checking a block number beyond the range raises ValueError."""
        bm = BlockBitmap(5)
        with pytest.raises(ValueError):
            bm.is_free(5)
        with pytest.raises(ValueError):
            bm.is_free(-1)

    def test_total_blocks_property(self) -> None:
        """The total_blocks property returns the configured total."""
        bm = BlockBitmap(42)
        assert bm.total_blocks == 42
