"""DirectoryEntry --- the name-to-inode mapping.

A directory in Unix is just a special file whose data blocks contain a list
of directory entries. Each entry is a (name, inode_number) pair --- nothing
more. The entry says "the name 'notes.txt' corresponds to inode 23." All
the actual metadata (size, permissions, timestamps) lives in the inode, not
in the directory entry.

This is why renaming a file is nearly instant: you only change the name
field in the directory entry. The inode (and all the file's data on disk)
is untouched.

Every directory must contain at least two entries:
    "."   -> the directory's own inode
    ".."  -> the parent directory's inode

For the root directory (/), both "." and ".." point to inode 0 (root is
its own parent).

Serialization format (for storing entries in data blocks):
    We use a simple text-based format for readability:
        name:inode_number\\n
    For example:
        .:0\\n
        ..:0\\n
        home:5\\n
        etc:3\\n

    Real ext2 uses a binary format with length-prefixed names and padding
    for alignment. Our text format is less space-efficient but much easier
    to debug and understand.
"""

from dataclasses import dataclass

from file_system.constants import MAX_NAME_LENGTH


@dataclass
class DirectoryEntry:
    """A single entry in a directory: maps a name to an inode number.

    Parameters
    ----------
    name : str
        The file or directory name (up to 255 characters). Must not contain
        '/' (the path separator) or '\\0' (null, used as string terminator
        in C). These restrictions match real Unix file systems.

    inode_number : int
        The inode this name refers to. Looking up this inode in the inode
        table gives you all the file's metadata and data block locations.
    """

    name: str
    inode_number: int

    def __post_init__(self) -> None:
        """Validate the entry name after creation.

        Raises ValueError if the name is empty, too long, or contains
        forbidden characters (/ or null byte).
        """
        if not self.name:
            raise ValueError("Directory entry name cannot be empty")
        if len(self.name) > MAX_NAME_LENGTH:
            raise ValueError(
                f"Directory entry name exceeds {MAX_NAME_LENGTH} characters"
            )
        if "/" in self.name:
            raise ValueError("Directory entry name cannot contain '/'")
        if "\0" in self.name:
            raise ValueError("Directory entry name cannot contain null byte")

    def serialize(self) -> str:
        """Convert this entry to its on-disk text representation.

        Format: "name:inode_number\\n"

        Example:
            >>> DirectoryEntry("hello.txt", 5).serialize()
            'hello.txt:5\\n'
        """
        return f"{self.name}:{self.inode_number}\n"

    @staticmethod
    def deserialize(line: str) -> "DirectoryEntry":
        """Parse a directory entry from its on-disk text representation.

        Parameters
        ----------
        line : str
            A string in the format "name:inode_number" (newline optional).

        Returns
        -------
        DirectoryEntry
            The parsed entry.

        Example:
            >>> DirectoryEntry.deserialize("hello.txt:5")
            DirectoryEntry(name='hello.txt', inode_number=5)
        """
        line = line.strip()
        name, inode_str = line.rsplit(":", maxsplit=1)
        return DirectoryEntry(name=name, inode_number=int(inode_str))
