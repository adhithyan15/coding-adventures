"""Tests for the Inode and FileType modules."""

from file_system.inode import FileType, Inode
from file_system.constants import DIRECT_BLOCKS


class TestFileType:
    """Verify that FileType enum has all expected members."""

    def test_all_file_types_exist(self) -> None:
        """The enum should have 7 types matching the ext2 spec."""
        assert FileType.REGULAR == 1
        assert FileType.DIRECTORY == 2
        assert FileType.SYMLINK == 3
        assert FileType.CHAR_DEVICE == 4
        assert FileType.BLOCK_DEVICE == 5
        assert FileType.PIPE == 6
        assert FileType.SOCKET == 7

    def test_file_type_is_int(self) -> None:
        """FileType is an IntEnum, so it can be used in integer comparisons."""
        assert FileType.REGULAR == 1
        assert FileType.DIRECTORY > FileType.REGULAR


class TestInode:
    """Verify that Inode dataclass stores file metadata correctly."""

    def test_default_inode(self) -> None:
        """A freshly created inode should have sensible defaults."""
        inode = Inode(inode_number=0)
        assert inode.inode_number == 0
        assert inode.file_type == FileType.REGULAR
        assert inode.size == 0
        assert inode.permissions == 0o755
        assert inode.owner_pid == 0
        assert inode.link_count == 1
        assert inode.indirect_block == -1
        assert inode.created_at == 0
        assert inode.modified_at == 0

    def test_direct_blocks_initialized_to_minus_one(self) -> None:
        """All 12 direct block pointers should start as -1 (unallocated)."""
        inode = Inode(inode_number=5)
        assert len(inode.direct_blocks) == DIRECT_BLOCKS
        assert all(b == -1 for b in inode.direct_blocks)

    def test_direct_blocks_are_independent(self) -> None:
        """Each inode should have its own copy of direct_blocks."""
        inode1 = Inode(inode_number=0)
        inode2 = Inode(inode_number=1)
        inode1.direct_blocks[0] = 42
        assert inode2.direct_blocks[0] == -1

    def test_directory_inode(self) -> None:
        """An inode can represent a directory."""
        inode = Inode(inode_number=0, file_type=FileType.DIRECTORY)
        assert inode.file_type == FileType.DIRECTORY

    def test_custom_permissions(self) -> None:
        """Permissions can be set to any octal value."""
        inode = Inode(inode_number=3, permissions=0o644)
        assert inode.permissions == 0o644

    def test_block_pointers_can_be_set(self) -> None:
        """Direct and indirect block pointers can be assigned."""
        inode = Inode(inode_number=7)
        inode.direct_blocks[0] = 100
        inode.direct_blocks[11] = 200
        inode.indirect_block = 300
        assert inode.direct_blocks[0] == 100
        assert inode.direct_blocks[11] == 200
        assert inode.indirect_block == 300

    def test_size_and_link_count_mutable(self) -> None:
        """Size and link_count change as the file grows and gets linked."""
        inode = Inode(inode_number=0)
        inode.size = 1024
        inode.link_count = 3
        assert inode.size == 1024
        assert inode.link_count == 3
