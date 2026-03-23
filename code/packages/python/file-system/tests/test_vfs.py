"""Tests for the VFS (Virtual File System) module.

This is the most comprehensive test file because the VFS integrates all
components (superblock, inode table, block bitmap, open file table) into
a cohesive API. Tests cover the full lifecycle of file operations:
format, create, open, read, write, close, seek, stat, mkdir, readdir,
and unlink.
"""

from file_system.constants import (
    BLOCK_SIZE,
    DIRECT_BLOCKS,
    O_APPEND,
    O_CREAT,
    O_RDONLY,
    O_RDWR,
    O_TRUNC,
    O_WRONLY,
    SEEK_CUR,
    SEEK_END,
    SEEK_SET,
)
from file_system.inode import FileType
from file_system.vfs import VFS


class TestVFSFormat:
    """Verify that formatting creates a valid empty file system."""

    def test_format_creates_root_directory(self) -> None:
        """After format, inode 0 exists and is a directory."""
        vfs = VFS()
        vfs.format()
        root = vfs.stat("/")
        assert root is not None
        assert root.file_type == FileType.DIRECTORY
        assert root.inode_number == 0

    def test_format_root_has_dot_entries(self) -> None:
        """After format, root contains '.' and '..' entries."""
        vfs = VFS()
        vfs.format()
        entries = vfs.readdir("/")
        names = [e.name for e in entries]
        assert "." in names
        assert ".." in names

    def test_format_dot_points_to_root(self) -> None:
        """Both '.' and '..' in root point to inode 0."""
        vfs = VFS()
        vfs.format()
        entries = vfs.readdir("/")
        for entry in entries:
            if entry.name in (".", ".."):
                assert entry.inode_number == 0

    def test_format_superblock_updated(self) -> None:
        """After format, the superblock reflects one used inode and block."""
        vfs = VFS()
        vfs.format()
        sb = vfs.superblock
        assert sb.is_valid()
        assert sb.free_inodes < sb.total_inodes  # At least root used
        assert sb.free_blocks < sb.total_blocks  # At least root block used


class TestVFSMkdir:
    """Verify directory creation."""

    def test_mkdir_creates_directory(self) -> None:
        """mkdir creates a directory that can be stat'd."""
        vfs = VFS()
        vfs.format()
        result = vfs.mkdir("/home")
        assert result == 0
        inode = vfs.stat("/home")
        assert inode is not None
        assert inode.file_type == FileType.DIRECTORY

    def test_mkdir_has_dot_entries(self) -> None:
        """A newly created directory contains '.' and '..'."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/home")
        entries = vfs.readdir("/home")
        names = [e.name for e in entries]
        assert "." in names
        assert ".." in names

    def test_mkdir_dot_points_correctly(self) -> None:
        """'.' points to the directory itself, '..' to the parent."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/home")
        home_inode = vfs.stat("/home")
        assert home_inode is not None
        entries = vfs.readdir("/home")
        for entry in entries:
            if entry.name == ".":
                assert entry.inode_number == home_inode.inode_number
            elif entry.name == "..":
                assert entry.inode_number == 0  # Parent is root

    def test_mkdir_nested(self) -> None:
        """Nested directories can be created: /a/b/c."""
        vfs = VFS()
        vfs.format()
        assert vfs.mkdir("/a") == 0
        assert vfs.mkdir("/a/b") == 0
        assert vfs.mkdir("/a/b/c") == 0
        assert vfs.stat("/a/b/c") is not None

    def test_mkdir_already_exists(self) -> None:
        """mkdir returns -1 if the directory already exists."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/home")
        assert vfs.mkdir("/home") == -1

    def test_mkdir_parent_not_exists(self) -> None:
        """mkdir returns -1 if the parent directory doesn't exist."""
        vfs = VFS()
        vfs.format()
        assert vfs.mkdir("/nonexistent/child") == -1

    def test_mkdir_appears_in_parent_readdir(self) -> None:
        """The new directory appears in the parent's readdir listing."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/home")
        entries = vfs.readdir("/")
        names = [e.name for e in entries]
        assert "home" in names


class TestVFSOpenCloseReadWrite:
    """Verify the core file I/O operations."""

    def test_open_create_write_close_read_roundtrip(self) -> None:
        """Write data to a file, close it, reopen, and read it back."""
        vfs = VFS()
        vfs.format()

        # Create and write
        fd = vfs.open("/hello.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        written = vfs.write(fd, b"Hello, World!")
        assert written == 13
        assert vfs.close(fd) == 0

        # Reopen and read
        fd = vfs.open("/hello.txt", O_RDONLY)
        assert fd >= 3
        data = vfs.read(fd, 100)
        assert data == b"Hello, World!"
        assert vfs.close(fd) == 0

    def test_open_nonexistent_without_creat(self) -> None:
        """Opening a nonexistent file without O_CREAT returns -1."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/nonexistent.txt", O_RDONLY)
        assert fd == -1

    def test_write_advances_offset(self) -> None:
        """Each write advances the file offset."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"aaa")
        vfs.write(fd, b"bbb")
        vfs.lseek(fd, 0, SEEK_SET)
        data = vfs.read(fd, 10)
        assert data == b"aaabbb"
        vfs.close(fd)

    def test_read_at_eof_returns_empty(self) -> None:
        """Reading past the end of a file returns empty bytes."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"hi")
        vfs.lseek(fd, 0, SEEK_SET)
        data = vfs.read(fd, 2)
        assert data == b"hi"
        data = vfs.read(fd, 10)
        assert data == b""  # EOF
        vfs.close(fd)

    def test_write_read_only_fd_returns_error(self) -> None:
        """Writing to a read-only fd returns -1."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"data")
        vfs.close(fd)

        fd = vfs.open("/test.txt", O_RDONLY)
        assert fd >= 3
        assert vfs.write(fd, b"more") == -1
        vfs.close(fd)

    def test_read_write_only_fd_returns_empty(self) -> None:
        """Reading from a write-only fd returns empty bytes."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        data = vfs.read(fd, 10)
        assert data == b""
        vfs.close(fd)

    def test_close_invalid_fd(self) -> None:
        """Closing an invalid fd returns -1."""
        vfs = VFS()
        vfs.format()
        assert vfs.close(99) == -1

    def test_write_invalid_fd(self) -> None:
        """Writing to an invalid fd returns -1."""
        vfs = VFS()
        vfs.format()
        assert vfs.write(99, b"data") == -1

    def test_read_invalid_fd(self) -> None:
        """Reading from an invalid fd returns empty bytes."""
        vfs = VFS()
        vfs.format()
        assert vfs.read(99, 10) == b""

    def test_open_creat_in_subdirectory(self) -> None:
        """O_CREAT creates a file in a subdirectory."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/data")
        fd = vfs.open("/data/log.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"log entry")
        vfs.close(fd)

        fd = vfs.open("/data/log.txt", O_RDONLY)
        assert fd >= 3
        assert vfs.read(fd, 100) == b"log entry"
        vfs.close(fd)


class TestVFSLseek:
    """Verify seek operations."""

    def test_seek_set(self) -> None:
        """SEEK_SET positions the offset at an absolute position."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"Hello, World!")
        new_pos = vfs.lseek(fd, 7, SEEK_SET)
        assert new_pos == 7
        data = vfs.read(fd, 6)
        assert data == b"World!"
        vfs.close(fd)

    def test_seek_cur(self) -> None:
        """SEEK_CUR moves the offset relative to current position."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"ABCDEFGHIJ")
        vfs.lseek(fd, 0, SEEK_SET)
        vfs.read(fd, 3)  # Now at offset 3
        new_pos = vfs.lseek(fd, 2, SEEK_CUR)
        assert new_pos == 5
        data = vfs.read(fd, 5)
        assert data == b"FGHIJ"
        vfs.close(fd)

    def test_seek_end(self) -> None:
        """SEEK_END positions relative to the end of the file."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"Hello!")
        new_pos = vfs.lseek(fd, -3, SEEK_END)
        assert new_pos == 3
        data = vfs.read(fd, 3)
        assert data == b"lo!"
        vfs.close(fd)

    def test_seek_to_beginning(self) -> None:
        """SEEK_SET with offset 0 goes to the very beginning."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"test data")
        assert vfs.lseek(fd, 0, SEEK_SET) == 0
        data = vfs.read(fd, 4)
        assert data == b"test"
        vfs.close(fd)

    def test_seek_invalid_fd(self) -> None:
        """lseek on an invalid fd returns -1."""
        vfs = VFS()
        vfs.format()
        assert vfs.lseek(99, 0, SEEK_SET) == -1

    def test_seek_before_beginning(self) -> None:
        """Seeking before byte 0 returns -1."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        assert vfs.lseek(fd, -1, SEEK_SET) == -1
        vfs.close(fd)

    def test_seek_invalid_whence(self) -> None:
        """An invalid whence value returns -1."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        assert vfs.lseek(fd, 0, 99) == -1
        vfs.close(fd)


class TestVFSStat:
    """Verify stat (metadata lookup)."""

    def test_stat_root(self) -> None:
        """stat('/') returns the root directory inode."""
        vfs = VFS()
        vfs.format()
        inode = vfs.stat("/")
        assert inode is not None
        assert inode.inode_number == 0
        assert inode.file_type == FileType.DIRECTORY

    def test_stat_file(self) -> None:
        """stat on a file returns its inode with correct size."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"12345")
        vfs.close(fd)

        inode = vfs.stat("/test.txt")
        assert inode is not None
        assert inode.file_type == FileType.REGULAR
        assert inode.size == 5

    def test_stat_nonexistent(self) -> None:
        """stat on a nonexistent path returns None."""
        vfs = VFS()
        vfs.format()
        assert vfs.stat("/nonexistent") is None

    def test_stat_empty_path(self) -> None:
        """stat on an empty path returns None."""
        vfs = VFS()
        vfs.format()
        assert vfs.stat("") is None

    def test_stat_relative_path(self) -> None:
        """stat on a relative path (no leading /) returns None."""
        vfs = VFS()
        vfs.format()
        assert vfs.stat("test.txt") is None


class TestVFSReaddir:
    """Verify directory listing."""

    def test_readdir_root(self) -> None:
        """readdir on root after format shows '.' and '..'."""
        vfs = VFS()
        vfs.format()
        entries = vfs.readdir("/")
        names = [e.name for e in entries]
        assert "." in names
        assert ".." in names

    def test_readdir_with_files(self) -> None:
        """readdir lists all files in a directory."""
        vfs = VFS()
        vfs.format()
        vfs.open("/a.txt", O_WRONLY | O_CREAT)
        vfs.open("/b.txt", O_WRONLY | O_CREAT)
        entries = vfs.readdir("/")
        names = [e.name for e in entries]
        assert "a.txt" in names
        assert "b.txt" in names

    def test_readdir_nonexistent(self) -> None:
        """readdir on a nonexistent path returns empty list."""
        vfs = VFS()
        vfs.format()
        assert vfs.readdir("/nonexistent") == []

    def test_readdir_on_file(self) -> None:
        """readdir on a regular file returns empty list."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        assert vfs.readdir("/test.txt") == []


class TestVFSUnlink:
    """Verify file deletion."""

    def test_unlink_removes_file(self) -> None:
        """unlink removes the file from its parent directory."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"data")
        vfs.close(fd)

        assert vfs.unlink("/test.txt") == 0
        assert vfs.stat("/test.txt") is None

    def test_unlink_frees_blocks(self) -> None:
        """unlink frees the data blocks used by the file."""
        vfs = VFS()
        vfs.format()
        free_before = vfs.superblock.free_blocks

        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"x" * 1024)  # Uses 2 blocks
        vfs.close(fd)
        free_after_write = vfs.superblock.free_blocks

        vfs.unlink("/test.txt")
        free_after_unlink = vfs.superblock.free_blocks

        assert free_after_write < free_before
        assert free_after_unlink > free_after_write

    def test_unlink_frees_inode(self) -> None:
        """unlink frees the inode when link_count drops to 0."""
        vfs = VFS()
        vfs.format()
        free_inodes_before = vfs.superblock.free_inodes

        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        free_inodes_after_create = vfs.superblock.free_inodes

        vfs.unlink("/test.txt")
        free_inodes_after_unlink = vfs.superblock.free_inodes

        assert free_inodes_after_create < free_inodes_before
        assert free_inodes_after_unlink > free_inodes_after_create

    def test_unlink_nonexistent(self) -> None:
        """unlink returns -1 for a nonexistent file."""
        vfs = VFS()
        vfs.format()
        assert vfs.unlink("/nonexistent") == -1

    def test_unlink_directory_fails(self) -> None:
        """unlink returns -1 for directories (use rmdir instead)."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/dir")
        assert vfs.unlink("/dir") == -1

    def test_unlink_root_fails(self) -> None:
        """unlink returns -1 for root (cannot unlink root)."""
        vfs = VFS()
        vfs.format()
        assert vfs.unlink("/") == -1


class TestVFSAppend:
    """Verify O_APPEND flag behavior."""

    def test_append_writes_at_end(self) -> None:
        """O_APPEND causes writes to go to the end of the file."""
        vfs = VFS()
        vfs.format()

        fd = vfs.open("/log.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"first ")
        vfs.close(fd)

        fd = vfs.open("/log.txt", O_WRONLY | O_APPEND)
        assert fd >= 3
        vfs.write(fd, b"second")
        vfs.close(fd)

        fd = vfs.open("/log.txt", O_RDONLY)
        assert fd >= 3
        data = vfs.read(fd, 100)
        assert data == b"first second"
        vfs.close(fd)


class TestVFSTruncate:
    """Verify O_TRUNC flag behavior."""

    def test_trunc_clears_file(self) -> None:
        """O_TRUNC truncates the file to zero length on open."""
        vfs = VFS()
        vfs.format()

        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"old data")
        vfs.close(fd)

        fd = vfs.open("/test.txt", O_WRONLY | O_TRUNC)
        assert fd >= 3
        vfs.write(fd, b"new")
        vfs.close(fd)

        fd = vfs.open("/test.txt", O_RDONLY)
        assert fd >= 3
        data = vfs.read(fd, 100)
        assert data == b"new"
        vfs.close(fd)


class TestVFSMultiBlock:
    """Verify that files spanning multiple blocks work correctly."""

    def test_write_read_spanning_blocks(self) -> None:
        """Data larger than BLOCK_SIZE is correctly split across blocks."""
        vfs = VFS()
        vfs.format()

        # Write more than one block (512 bytes)
        data = bytes(range(256)) * 4  # 1024 bytes = 2 blocks
        fd = vfs.open("/big.bin", O_RDWR | O_CREAT)
        assert fd >= 3
        written = vfs.write(fd, data)
        assert written == 1024
        vfs.lseek(fd, 0, SEEK_SET)
        read_back = vfs.read(fd, 1024)
        assert read_back == data
        vfs.close(fd)

    def test_write_all_direct_blocks(self) -> None:
        """A file using all 12 direct blocks (6144 bytes) works."""
        vfs = VFS()
        vfs.format()

        data = b"X" * (DIRECT_BLOCKS * BLOCK_SIZE)  # 6144 bytes
        fd = vfs.open("/full_direct.bin", O_RDWR | O_CREAT)
        assert fd >= 3
        written = vfs.write(fd, data)
        assert written == DIRECT_BLOCKS * BLOCK_SIZE

        vfs.lseek(fd, 0, SEEK_SET)
        read_back = vfs.read(fd, DIRECT_BLOCKS * BLOCK_SIZE)
        assert read_back == data
        vfs.close(fd)

    def test_write_uses_indirect_block(self) -> None:
        """A file larger than 12 blocks uses the indirect block pointer."""
        vfs = VFS(total_blocks=256, total_inodes=32)
        vfs.format()

        # Write just past the direct block limit
        size = (DIRECT_BLOCKS + 2) * BLOCK_SIZE  # 14 * 512 = 7168 bytes
        data = b"Y" * size
        fd = vfs.open("/indirect.bin", O_RDWR | O_CREAT)
        assert fd >= 3
        written = vfs.write(fd, data)
        assert written == size

        # Verify the inode has an indirect block
        inode = vfs.stat("/indirect.bin")
        assert inode is not None
        assert inode.indirect_block != -1

        # Read back and verify
        vfs.lseek(fd, 0, SEEK_SET)
        read_back = vfs.read(fd, size)
        assert read_back == data
        vfs.close(fd)


class TestVFSPathResolution:
    """Verify path resolution through nested directories."""

    def test_resolve_root(self) -> None:
        """Resolving '/' returns inode 0."""
        vfs = VFS()
        vfs.format()
        inode = vfs.resolve_path("/")
        assert inode is not None
        assert inode.inode_number == 0

    def test_resolve_nested_path(self) -> None:
        """Resolving a deeply nested path works correctly."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/a")
        vfs.mkdir("/a/b")
        vfs.mkdir("/a/b/c")

        fd = vfs.open("/a/b/c/file.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)

        inode = vfs.resolve_path("/a/b/c/file.txt")
        assert inode is not None
        assert inode.file_type == FileType.REGULAR

    def test_resolve_nonexistent_path(self) -> None:
        """Resolving a nonexistent path returns None."""
        vfs = VFS()
        vfs.format()
        assert vfs.resolve_path("/nonexistent/path") is None

    def test_resolve_through_file_fails(self) -> None:
        """Resolving a path through a regular file (not directory) fails."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/file.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        # Cannot traverse through a regular file
        assert vfs.resolve_path("/file.txt/child") is None

    def test_resolve_trailing_slash(self) -> None:
        """Paths with trailing slashes resolve correctly."""
        vfs = VFS()
        vfs.format()
        vfs.mkdir("/home")
        inode = vfs.resolve_path("/home/")
        assert inode is not None
        assert inode.file_type == FileType.DIRECTORY


class TestVFSEdgeCases:
    """Edge cases and error conditions."""

    def test_open_creat_parent_not_directory(self) -> None:
        """O_CREAT fails if parent path component is not a directory."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/file.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        # Try to create a file under a regular file
        fd = vfs.open("/file.txt/child.txt", O_WRONLY | O_CREAT)
        assert fd == -1

    def test_open_creat_no_parent(self) -> None:
        """O_CREAT fails if the parent directory doesn't exist."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/nonexistent/file.txt", O_WRONLY | O_CREAT)
        assert fd == -1

    def test_multiple_files_in_same_directory(self) -> None:
        """Multiple files can be created in the same directory."""
        vfs = VFS()
        vfs.format()
        for i in range(5):
            fd = vfs.open(f"/file{i}.txt", O_WRONLY | O_CREAT)
            assert fd >= 3
            vfs.write(fd, f"content {i}".encode())
            vfs.close(fd)

        for i in range(5):
            fd = vfs.open(f"/file{i}.txt", O_RDONLY)
            assert fd >= 3
            data = vfs.read(fd, 100)
            assert data == f"content {i}".encode()
            vfs.close(fd)

    def test_full_workflow(self) -> None:
        """Full workflow: format -> mkdir -> create -> write -> close ->
        open -> read -> verify."""
        vfs = VFS()
        vfs.format()

        # Create nested structure
        vfs.mkdir("/home")
        vfs.mkdir("/home/alice")

        # Create and write a file
        fd = vfs.open("/home/alice/notes.txt", O_RDWR | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"These are Alice's notes.\n")
        vfs.write(fd, b"Second line.\n")
        vfs.close(fd)

        # Verify directory structure
        entries = vfs.readdir("/home/alice")
        names = [e.name for e in entries]
        assert "notes.txt" in names

        # Read back the file
        fd = vfs.open("/home/alice/notes.txt", O_RDONLY)
        assert fd >= 3
        data = vfs.read(fd, 1000)
        assert data == b"These are Alice's notes.\nSecond line.\n"
        vfs.close(fd)

        # Check stat
        inode = vfs.stat("/home/alice/notes.txt")
        assert inode is not None
        assert inode.file_type == FileType.REGULAR
        assert inode.size == len(b"These are Alice's notes.\nSecond line.\n")

    def test_inode_exhaustion(self) -> None:
        """Creating files until inodes run out returns -1."""
        vfs = VFS(total_blocks=64, total_inodes=4)
        vfs.format()
        # inode 0 = root, so we can create 3 more files
        fd = vfs.open("/a.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        fd = vfs.open("/b.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        fd = vfs.open("/c.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.close(fd)
        # Fourth file should fail (all 4 inodes used)
        fd = vfs.open("/d.txt", O_WRONLY | O_CREAT)
        assert fd == -1

    def test_open_existing_file_without_creat(self) -> None:
        """Opening an existing file without O_CREAT works fine."""
        vfs = VFS()
        vfs.format()
        fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)
        assert fd >= 3
        vfs.write(fd, b"data")
        vfs.close(fd)

        fd = vfs.open("/test.txt", O_RDONLY)
        assert fd >= 3
        data = vfs.read(fd, 10)
        assert data == b"data"
        vfs.close(fd)
