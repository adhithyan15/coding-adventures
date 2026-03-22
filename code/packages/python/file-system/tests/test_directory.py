"""Tests for the DirectoryEntry module."""

import pytest

from file_system.directory import DirectoryEntry


class TestDirectoryEntry:
    """Verify that DirectoryEntry maps names to inodes correctly."""

    def test_creation(self) -> None:
        """A directory entry stores a name and inode number."""
        entry = DirectoryEntry(name="hello.txt", inode_number=5)
        assert entry.name == "hello.txt"
        assert entry.inode_number == 5

    def test_dot_entry(self) -> None:
        """The '.' entry points to the current directory."""
        entry = DirectoryEntry(name=".", inode_number=0)
        assert entry.name == "."
        assert entry.inode_number == 0

    def test_dotdot_entry(self) -> None:
        """The '..' entry points to the parent directory."""
        entry = DirectoryEntry(name="..", inode_number=0)
        assert entry.name == ".."
        assert entry.inode_number == 0

    def test_serialize(self) -> None:
        """Serialization produces 'name:inode_number\\n' format."""
        entry = DirectoryEntry(name="hello.txt", inode_number=5)
        assert entry.serialize() == "hello.txt:5\n"

    def test_serialize_dot(self) -> None:
        """Dot entries serialize correctly."""
        entry = DirectoryEntry(name=".", inode_number=0)
        assert entry.serialize() == ".:0\n"

    def test_deserialize(self) -> None:
        """Deserialization parses 'name:inode_number' back to an entry."""
        entry = DirectoryEntry.deserialize("hello.txt:5")
        assert entry.name == "hello.txt"
        assert entry.inode_number == 5

    def test_deserialize_with_newline(self) -> None:
        """Deserialization handles trailing newlines."""
        entry = DirectoryEntry.deserialize("hello.txt:5\n")
        assert entry.name == "hello.txt"
        assert entry.inode_number == 5

    def test_serialize_deserialize_roundtrip(self) -> None:
        """Serializing then deserializing produces the original entry."""
        original = DirectoryEntry(name="notes.txt", inode_number=23)
        restored = DirectoryEntry.deserialize(original.serialize())
        assert restored.name == original.name
        assert restored.inode_number == original.inode_number

    def test_empty_name_rejected(self) -> None:
        """Empty names are not allowed."""
        with pytest.raises(ValueError, match="cannot be empty"):
            DirectoryEntry(name="", inode_number=0)

    def test_name_with_slash_rejected(self) -> None:
        """Names containing '/' are not allowed."""
        with pytest.raises(ValueError, match="cannot contain '/'"):
            DirectoryEntry(name="a/b", inode_number=0)

    def test_name_with_null_rejected(self) -> None:
        """Names containing null bytes are not allowed."""
        with pytest.raises(ValueError, match="cannot contain null"):
            DirectoryEntry(name="a\0b", inode_number=0)

    def test_name_too_long_rejected(self) -> None:
        """Names longer than MAX_NAME_LENGTH are not allowed."""
        with pytest.raises(ValueError, match="exceeds"):
            DirectoryEntry(name="x" * 256, inode_number=0)

    def test_max_length_name_accepted(self) -> None:
        """A name exactly MAX_NAME_LENGTH chars long is accepted."""
        entry = DirectoryEntry(name="x" * 255, inode_number=42)
        assert len(entry.name) == 255
