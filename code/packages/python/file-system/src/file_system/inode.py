"""Inode --- the heart of the file system.

An inode (short for "index node") is a fixed-size record that stores
everything about a file *except its name*. This is one of the most
important insights in Unix-style file systems: names live in directories,
not in files. A file's inode contains its type, size, permissions, and ---
most importantly --- the block pointers that tell the file system where the
file's data lives on disk.

Why separate names from metadata?
    Because a single file can have *multiple names* (hard links). When you
    run ``ln original.txt alias.txt``, both names point to the same inode.
    The file's data exists only once on disk. The ``link_count`` tracks how
    many directory entries reference this inode; when it drops to zero, the
    inode and its data blocks are freed.

Analogy: An inode is like a library catalog card. The card stores the book's
metadata (author, page count, shelf location) but not the title --- titles
are written on the shelf labels (directory entries). Multiple shelf labels
can point to the same card (hard links), and you can move a label without
touching the card (renaming a file).

Block pointer structure:
    Each inode has 12 direct block pointers and one indirect block pointer.
    Direct pointers point straight to data blocks. The indirect pointer
    points to a block that itself contains more block pointers.

    Inode
    +-------------------+
    | direct_blocks[0]  -----> Data Block (bytes 0-511)
    | direct_blocks[1]  -----> Data Block (bytes 512-1023)
    | ...               |
    | direct_blocks[11] -----> Data Block (bytes 5632-6143)
    |                   |
    | indirect_block    -----> +--------------------+
    |                   |      | ptr[0] -> Data     | (bytes 6144-6655)
    |                   |      | ptr[1] -> Data     | (bytes 6656-7167)
    |                   |      | ...                |
    |                   |      | ptr[127] -> Data   | (bytes 71168-71679)
    +-------------------+      +--------------------+

    Max file size = (12 + 128) * 512 = 71,680 bytes
"""

from dataclasses import dataclass, field
from enum import IntEnum

from file_system.constants import DIRECT_BLOCKS


class FileType(IntEnum):
    """Every inode has a type that determines how the kernel interprets its data.

    In Unix, the mantra is "everything is a file." Directories, devices, pipes,
    and sockets are all represented as inodes with different types. The kernel
    dispatches operations (read, write, etc.) differently based on the type.

    Truth table of file types:
        +---------------+--------------------------------------------------+
        | Type          | What it represents                               |
        +---------------+--------------------------------------------------+
        | REGULAR       | Ordinary file (text, binary, image, etc.)        |
        | DIRECTORY     | Contains directory entries (name -> inode pairs)  |
        | SYMLINK       | Symbolic link (stores a path to another file)    |
        | CHAR_DEVICE   | Character device (e.g., keyboard, serial port)   |
        | BLOCK_DEVICE  | Block device (e.g., hard disk, SSD)              |
        | PIPE          | Named pipe / FIFO for inter-process comm.        |
        | SOCKET        | Unix domain socket for local IPC                 |
        +---------------+--------------------------------------------------+
    """

    REGULAR = 1
    DIRECTORY = 2
    SYMLINK = 3
    CHAR_DEVICE = 4
    BLOCK_DEVICE = 5
    PIPE = 6
    SOCKET = 7


@dataclass
class Inode:
    """A fixed-size record storing all metadata for one file or directory.

    Parameters
    ----------
    inode_number : int
        Unique identifier (0 through MAX_INODES-1). Inode 0 is always the
        root directory ``/``.

    file_type : FileType
        What kind of object this inode represents (regular file, directory,
        symlink, device, pipe, or socket).

    size : int
        File size in bytes. For directories, this is the total serialized
        size of all directory entries.

    permissions : int
        Octal permission bits (e.g., 0o755 = rwxr-xr-x). Stored as an
        integer. In a real OS, the kernel checks these before allowing
        read/write/execute operations.

    owner_pid : int
        The PID of the process that created this file. In a real OS this
        would be a user ID (UID), but we use PID for simplicity since our
        educational OS does not have a user management system.

    link_count : int
        Number of directory entries (hard links) pointing to this inode.
        When this reaches 0 and no file descriptors reference it, the inode
        and all its data blocks are freed.

    direct_blocks : list[int]
        Array of 12 block numbers. ``direct_blocks[i]`` is the block number
        containing bytes ``i*512`` through ``(i+1)*512 - 1`` of the file.
        A value of -1 means "not allocated yet."

    indirect_block : int
        Block number of an indirect block. The indirect block itself contains
        up to 128 four-byte block numbers, each pointing to a data block.
        A value of -1 means "no indirect block allocated."

    created_at : int
        Timestamp (seconds since epoch) when the inode was created.

    modified_at : int
        Timestamp when the file's data was last written.
    """

    inode_number: int
    file_type: FileType = FileType.REGULAR
    size: int = 0
    permissions: int = 0o755
    owner_pid: int = 0
    link_count: int = 1
    direct_blocks: list[int] = field(
        default_factory=lambda: [-1] * DIRECT_BLOCKS
    )
    indirect_block: int = -1
    created_at: int = 0
    modified_at: int = 0
