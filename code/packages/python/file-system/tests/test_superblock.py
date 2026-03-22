"""Tests for the Superblock module."""

from file_system.superblock import Superblock
from file_system.constants import BLOCK_SIZE, MAX_BLOCKS, MAX_INODES, ROOT_INODE


class TestSuperblock:
    """Verify that the Superblock stores file system metadata correctly."""

    def test_default_values(self) -> None:
        """A fresh superblock should have all the default constants."""
        sb = Superblock()
        assert sb.magic == 0x45585432
        assert sb.block_size == BLOCK_SIZE
        assert sb.total_blocks == MAX_BLOCKS
        assert sb.total_inodes == MAX_INODES
        assert sb.free_blocks == MAX_BLOCKS
        assert sb.free_inodes == MAX_INODES
        assert sb.root_inode == ROOT_INODE

    def test_magic_number_is_ext2(self) -> None:
        """The magic number 0x45585432 encodes the ASCII string 'EXT2'."""
        sb = Superblock()
        # Convert to bytes and verify
        magic_bytes = sb.magic.to_bytes(4, byteorder="big")
        assert magic_bytes == b"EXT2"

    def test_is_valid_with_correct_magic(self) -> None:
        """is_valid() returns True when the magic number is correct."""
        sb = Superblock()
        assert sb.is_valid() is True

    def test_is_valid_with_wrong_magic(self) -> None:
        """is_valid() returns False when the magic number is wrong."""
        sb = Superblock(magic=0xDEADBEEF)
        assert sb.is_valid() is False

    def test_custom_values(self) -> None:
        """Superblock can be created with custom parameters."""
        sb = Superblock(
            total_blocks=1024,
            total_inodes=256,
            free_blocks=900,
            free_inodes=200,
        )
        assert sb.total_blocks == 1024
        assert sb.total_inodes == 256
        assert sb.free_blocks == 900
        assert sb.free_inodes == 200

    def test_free_counts_can_be_updated(self) -> None:
        """Free counts should be mutable (they change as files are created)."""
        sb = Superblock()
        sb.free_blocks -= 10
        sb.free_inodes -= 5
        assert sb.free_blocks == MAX_BLOCKS - 10
        assert sb.free_inodes == MAX_INODES - 5
