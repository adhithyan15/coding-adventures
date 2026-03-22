"""Tests for OpenFile, OpenFileTable, and FileDescriptorTable."""

from file_system.file_descriptor import (
    FileDescriptorTable,
    OpenFile,
    OpenFileTable,
)
from file_system.constants import O_RDONLY, O_RDWR, O_WRONLY


class TestOpenFile:
    """Verify that OpenFile stores per-opening state correctly."""

    def test_default_values(self) -> None:
        """A fresh OpenFile has offset 0, RDONLY flags, ref_count 1."""
        of = OpenFile(inode_number=5)
        assert of.inode_number == 5
        assert of.offset == 0
        assert of.flags == O_RDONLY
        assert of.ref_count == 1

    def test_custom_values(self) -> None:
        """OpenFile accepts custom flags and offset."""
        of = OpenFile(inode_number=10, offset=100, flags=O_RDWR, ref_count=3)
        assert of.inode_number == 10
        assert of.offset == 100
        assert of.flags == O_RDWR
        assert of.ref_count == 3


class TestOpenFileTable:
    """Verify the system-wide open file table."""

    def test_open_returns_fd_starting_at_3(self) -> None:
        """File descriptors start at 3 (0-2 reserved for stdio)."""
        table = OpenFileTable()
        fd = table.open(inode_number=5, flags=O_RDONLY)
        assert fd == 3

    def test_open_returns_sequential_fds(self) -> None:
        """Each open call returns the next available fd."""
        table = OpenFileTable()
        assert table.open(5, O_RDONLY) == 3
        assert table.open(6, O_RDONLY) == 4
        assert table.open(7, O_RDONLY) == 5

    def test_get_returns_open_file(self) -> None:
        """get() retrieves the OpenFile entry for a valid fd."""
        table = OpenFileTable()
        fd = table.open(5, O_RDWR)
        entry = table.get(fd)
        assert entry is not None
        assert entry.inode_number == 5
        assert entry.flags == O_RDWR

    def test_get_returns_none_for_invalid_fd(self) -> None:
        """get() returns None for an fd that doesn't exist."""
        table = OpenFileTable()
        assert table.get(99) is None

    def test_close_removes_entry(self) -> None:
        """Closing an fd (with ref_count 1) removes the entry."""
        table = OpenFileTable()
        fd = table.open(5, O_RDONLY)
        assert table.close(fd) is True
        assert table.get(fd) is None

    def test_close_invalid_fd_returns_false(self) -> None:
        """Closing a nonexistent fd returns False."""
        table = OpenFileTable()
        assert table.close(99) is False

    def test_dup_creates_new_fd(self) -> None:
        """dup() creates a new fd pointing to the same OpenFile."""
        table = OpenFileTable()
        fd1 = table.open(5, O_RDONLY)
        fd2 = table.dup(fd1)
        assert fd2 is not None
        assert fd2 != fd1
        # Both fds point to the same entry
        assert table.get(fd1) is table.get(fd2)

    def test_dup_increments_ref_count(self) -> None:
        """dup() increments the ref_count on the OpenFile entry."""
        table = OpenFileTable()
        fd1 = table.open(5, O_RDONLY)
        table.dup(fd1)
        entry = table.get(fd1)
        assert entry is not None
        assert entry.ref_count == 2

    def test_dup_invalid_fd_returns_none(self) -> None:
        """dup() returns None for an invalid fd."""
        table = OpenFileTable()
        assert table.dup(99) is None

    def test_dup_shared_offset(self) -> None:
        """Two fds from dup() share the same offset (writing advances both)."""
        table = OpenFileTable()
        fd1 = table.open(5, O_RDWR)
        fd2 = table.dup(fd1)
        assert fd2 is not None
        entry = table.get(fd1)
        assert entry is not None
        entry.offset = 42
        entry2 = table.get(fd2)
        assert entry2 is not None
        assert entry2.offset == 42  # Same object, same offset

    def test_dup2_redirects_fd(self) -> None:
        """dup2() makes new_fd point to the same OpenFile as old_fd."""
        table = OpenFileTable()
        fd1 = table.open(5, O_RDONLY)
        result = table.dup2(fd1, 1)  # Redirect fd 1 (stdout) to our file
        assert result == 1
        assert table.get(1) is table.get(fd1)

    def test_dup2_closes_existing_fd(self) -> None:
        """dup2() closes new_fd if it was already open."""
        table = OpenFileTable()
        fd1 = table.open(5, O_RDONLY)
        fd2 = table.open(6, O_RDONLY)
        # dup2 fd1 onto fd2 --- fd2's original entry should be closed
        table.dup2(fd1, fd2)
        # fd2 now points to inode 5, not inode 6
        entry = table.get(fd2)
        assert entry is not None
        assert entry.inode_number == 5

    def test_dup2_invalid_old_fd_returns_none(self) -> None:
        """dup2() returns None if old_fd is invalid."""
        table = OpenFileTable()
        assert table.dup2(99, 1) is None

    def test_close_with_ref_count_gt_1(self) -> None:
        """Closing one of two fds decrements ref_count but keeps entry."""
        table = OpenFileTable()
        fd1 = table.open(5, O_RDONLY)
        fd2 = table.dup(fd1)
        assert fd2 is not None
        table.close(fd1)
        # fd2 should still work (ref_count was 2, now 1)
        entry = table.get(fd2)
        assert entry is not None
        assert entry.ref_count == 1


class TestFileDescriptorTable:
    """Verify the per-process fd mapping table."""

    def test_add_and_get(self) -> None:
        """add() creates a mapping, get_global() retrieves it."""
        fdt = FileDescriptorTable()
        fdt.add(3, 100)
        assert fdt.get_global(3) == 100

    def test_get_nonexistent_returns_none(self) -> None:
        """get_global() returns None for unmapped local fds."""
        fdt = FileDescriptorTable()
        assert fdt.get_global(99) is None

    def test_remove_returns_global_fd(self) -> None:
        """remove() returns the global fd and removes the mapping."""
        fdt = FileDescriptorTable()
        fdt.add(3, 100)
        assert fdt.remove(3) == 100
        assert fdt.get_global(3) is None

    def test_remove_nonexistent_returns_none(self) -> None:
        """remove() returns None for unmapped local fds."""
        fdt = FileDescriptorTable()
        assert fdt.remove(99) is None

    def test_clone_creates_independent_copy(self) -> None:
        """clone() creates a copy that can be modified independently."""
        fdt = FileDescriptorTable()
        fdt.add(3, 100)
        fdt.add(4, 200)

        cloned = fdt.clone()
        # Cloned table has same mappings
        assert cloned.get_global(3) == 100
        assert cloned.get_global(4) == 200

        # Modifying the clone doesn't affect the original
        cloned.remove(3)
        assert cloned.get_global(3) is None
        assert fdt.get_global(3) == 100  # Original unchanged
