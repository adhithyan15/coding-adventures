"""Tests for the InodeTable module."""

import pytest

from file_system.inode import FileType
from file_system.inode_table import InodeTable


class TestInodeTable:
    """Verify that InodeTable manages inode allocation and lookup."""

    def test_allocate_returns_inode(self) -> None:
        """Allocating an inode returns an Inode with the correct number."""
        table = InodeTable(4)
        inode = table.allocate(FileType.REGULAR)
        assert inode is not None
        assert inode.inode_number == 0
        assert inode.file_type == FileType.REGULAR

    def test_allocate_sequential(self) -> None:
        """Successive allocations return sequential inode numbers."""
        table = InodeTable(4)
        i0 = table.allocate(FileType.DIRECTORY)
        i1 = table.allocate(FileType.REGULAR)
        i2 = table.allocate(FileType.REGULAR)
        assert i0 is not None and i0.inode_number == 0
        assert i1 is not None and i1.inode_number == 1
        assert i2 is not None and i2.inode_number == 2

    def test_allocate_exhaustion(self) -> None:
        """Allocating more inodes than the table holds returns None."""
        table = InodeTable(2)
        table.allocate()
        table.allocate()
        assert table.allocate() is None

    def test_get_returns_allocated_inode(self) -> None:
        """get() returns the inode that was allocated at that slot."""
        table = InodeTable(4)
        allocated = table.allocate(FileType.REGULAR)
        assert allocated is not None
        retrieved = table.get(allocated.inode_number)
        assert retrieved is allocated  # Same object

    def test_get_returns_none_for_free_slot(self) -> None:
        """get() returns None for a slot that hasn't been allocated."""
        table = InodeTable(4)
        assert table.get(0) is None

    def test_free_makes_slot_available(self) -> None:
        """After freeing an inode, get() returns None for that slot."""
        table = InodeTable(4)
        inode = table.allocate()
        assert inode is not None
        table.free(inode.inode_number)
        assert table.get(inode.inode_number) is None

    def test_free_slot_is_reused(self) -> None:
        """Freeing slot 0 then allocating again returns inode_number 0."""
        table = InodeTable(4)
        table.allocate()  # 0
        table.allocate()  # 1
        table.free(0)
        reused = table.allocate()
        assert reused is not None
        assert reused.inode_number == 0

    def test_free_count(self) -> None:
        """free_count tracks how many slots are available."""
        table = InodeTable(4)
        assert table.free_count() == 4
        table.allocate()
        assert table.free_count() == 3
        table.allocate()
        assert table.free_count() == 2

    def test_free_out_of_range_raises(self) -> None:
        """Freeing an out-of-range inode number raises ValueError."""
        table = InodeTable(4)
        with pytest.raises(ValueError):
            table.free(4)
        with pytest.raises(ValueError):
            table.free(-1)

    def test_get_out_of_range_raises(self) -> None:
        """Getting an out-of-range inode number raises ValueError."""
        table = InodeTable(4)
        with pytest.raises(ValueError):
            table.get(4)
        with pytest.raises(ValueError):
            table.get(-1)

    def test_max_inodes_property(self) -> None:
        """The max_inodes property returns the configured maximum."""
        table = InodeTable(42)
        assert table.max_inodes == 42

    def test_allocate_directory_type(self) -> None:
        """Allocating with DIRECTORY type sets the file_type correctly."""
        table = InodeTable(4)
        inode = table.allocate(FileType.DIRECTORY)
        assert inode is not None
        assert inode.file_type == FileType.DIRECTORY
