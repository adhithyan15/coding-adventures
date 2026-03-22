"""Constants --- the fundamental parameters of our file system.

Every file system is defined by a handful of numbers: how big are the blocks?
How many inodes can we have? What do the flag bits mean? These constants are
the DNA of our ext2-inspired file system.

Analogy: Think of these as the architectural blueprints for a library building.
Before you can build shelves (data blocks), catalog cards (inodes), or checkout
desks (file descriptors), you need to decide the dimensions of everything.
"""

# ---------------------------------------------------------------------------
# Block and inode geometry
# ---------------------------------------------------------------------------

BLOCK_SIZE: int = 512
"""Each block on our simulated disk is 512 bytes --- the traditional hard-disk
sector size. Every read and write to the underlying storage operates in units
of exactly this many bytes.

Why 512? Real hard drives have always been organized into 512-byte sectors.
Even modern drives with 4096-byte physical sectors still expose a 512-byte
logical interface for backward compatibility. We use 512 to match reality."""

MAX_BLOCKS: int = 512
"""Total number of blocks on our simulated disk. 512 blocks x 512 bytes =
262,144 bytes = 256 KB. Tiny by modern standards, but large enough to
demonstrate every file system concept (direct blocks, indirect blocks,
directories, bitmaps, etc.)."""

MAX_INODES: int = 128
"""Maximum number of inodes (files + directories + special files) the file
system can hold. Each inode is a fixed-size record that stores metadata about
one file. 128 is enough for our educational purposes --- real ext2 volumes
have millions of inodes."""

DIRECT_BLOCKS: int = 12
"""Each inode has 12 direct block pointers. With 512-byte blocks, this allows
files up to 12 x 512 = 6,144 bytes without needing any indirection. Most
files in a typical Unix system are smaller than this, so direct pointers
handle the common case efficiently."""

ROOT_INODE: int = 0
"""Inode 0 is always the root directory '/'. This is a fixed convention ---
when the OS mounts a file system, it knows to look at inode 0 to find the
top-level directory. Everything else is discovered by walking the directory
tree from this starting point."""

MAX_NAME_LENGTH: int = 255
"""Maximum length of a file or directory name in bytes. This matches the
ext2/ext3/ext4 limit and is also the limit on most modern file systems.
Note: this is the name only (e.g., 'notes.txt'), not the full path."""

# ---------------------------------------------------------------------------
# Open flags --- how a file is opened
# ---------------------------------------------------------------------------
# These mirror the POSIX constants from <fcntl.h>. When a process calls
# open("/path", flags), the flags tell the kernel what the process intends
# to do with the file. They can be combined with bitwise OR.
#
# Example: open("/data/log.txt", O_WRONLY | O_CREAT | O_APPEND)
#   = open for writing, create if it doesn't exist, append to the end

O_RDONLY: int = 0
"""Open for reading only. Any write attempt will fail."""

O_WRONLY: int = 1
"""Open for writing only. Any read attempt will fail."""

O_RDWR: int = 2
"""Open for both reading and writing."""

O_CREAT: int = 64
"""If the file does not exist, create it. Without this flag, opening a
nonexistent file returns an error."""

O_TRUNC: int = 512
"""If the file exists and is opened for writing, truncate it to zero length.
All existing data is discarded."""

O_APPEND: int = 1024
"""Before each write, the file offset is positioned at the end of the file.
This ensures that writes always append, even if multiple processes are
writing to the same file."""

# ---------------------------------------------------------------------------
# Seek whence --- where to measure from when repositioning
# ---------------------------------------------------------------------------
# These are used with lseek(fd, offset, whence) to move the read/write
# cursor within an open file.
#
# Example: lseek(fd, 0, SEEK_SET) = go to the beginning
#          lseek(fd, -10, SEEK_END) = go to 10 bytes before the end

SEEK_SET: int = 0
"""Set the offset to exactly `offset` bytes from the beginning of the file."""

SEEK_CUR: int = 1
"""Set the offset to `offset` bytes relative to the current position.
Positive moves forward, negative moves backward."""

SEEK_END: int = 2
"""Set the offset to `offset` bytes relative to the end of the file.
Typically used with offset=0 to seek to the end, or negative offsets to
seek backward from the end."""
