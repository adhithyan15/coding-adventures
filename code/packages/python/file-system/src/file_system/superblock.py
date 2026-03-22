"""Superblock --- the file system's identity card.

The superblock is always stored at block 0 --- the very first block on disk.
It is the first thing the operating system reads when mounting a file system.
Without a valid superblock, the OS has no idea how to interpret the rest of
the disk: it would just see 262,144 bytes of meaningless data.

Analogy: The superblock is like the cover page of a book. It tells you the
title (magic number proves this is our file system), the number of pages
(total blocks), and the table of contents (where to find inodes and data).

Real-world note: ext2/ext3/ext4 keep *backup copies* of the superblock at
regular intervals across the disk, so that if block 0 is corrupted, a
recovery tool (e2fsck) can find a backup. We skip backups for simplicity.

Layout within block 0 (conceptual):
    +---------+-----------+-------------+-------------+-------------+-------------+
    | magic   | block_size| total_blocks| total_inodes| free_blocks | free_inodes |
    | (4 B)   | (4 B)     | (4 B)       | (4 B)       | (4 B)       | (4 B)       |
    +---------+-----------+-------------+-------------+-------------+-------------+
    byte 0    byte 4      byte 8        byte 12       byte 16       byte 20
"""

from dataclasses import dataclass

from file_system.constants import BLOCK_SIZE, MAX_BLOCKS, MAX_INODES, ROOT_INODE


@dataclass
class Superblock:
    """Stores the essential metadata for the entire file system.

    Fields
    ------
    magic : int
        A "magic number" that identifies this disk as containing our file
        system. We use 0x45585432, which is the ASCII encoding of "EXT2".
        When the OS tries to mount a disk, it reads block 0 and checks
        whether the magic number matches. If it does not, the disk either
        has a different file system or is unformatted.

    block_size : int
        The size of every block on disk, in bytes. Always 512 in our system.

    total_blocks : int
        The total number of blocks on the disk. The file system cannot use
        more blocks than this, no matter what.

    total_inodes : int
        The maximum number of inodes (files/directories) the file system
        supports. Once all inodes are allocated, you cannot create new files
        even if there is free disk space.

    free_blocks : int
        How many data blocks are currently unallocated. This counter is
        updated every time a block is allocated or freed, giving a quick
        answer to "how much disk space is left?" without scanning the
        entire bitmap.

    free_inodes : int
        How many inodes are currently unallocated. Same idea as free_blocks
        but for the inode table.

    root_inode : int
        The inode number of the root directory. Always 0 by convention.
    """

    magic: int = 0x45585432  # ASCII "EXT2"
    block_size: int = BLOCK_SIZE
    total_blocks: int = MAX_BLOCKS
    total_inodes: int = MAX_INODES
    free_blocks: int = MAX_BLOCKS
    free_inodes: int = MAX_INODES
    root_inode: int = ROOT_INODE

    def is_valid(self) -> bool:
        """Check whether this superblock has the correct magic number.

        Returns True if the magic number matches 0x45585432 ("EXT2"), which
        means this disk was formatted with our file system. Returns False
        otherwise, indicating the disk is unformatted or uses a different
        file system.
        """
        return self.magic == 0x45585432
