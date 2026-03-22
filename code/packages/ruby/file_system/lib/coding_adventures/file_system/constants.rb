# frozen_string_literal: true

# = File System Constants
#
# These constants define the fundamental parameters of our ext2-inspired file
# system. They control the disk geometry (how many blocks, how big each block
# is), the inode capacity (how many files we can store), and the open flags
# (how processes specify what they want to do with a file).
#
# == Why These Particular Values?
#
# - BLOCK_SIZE = 512 bytes: This is the traditional hard disk sector size.
#   Every disk I/O operation reads or writes exactly one sector. Modern SSDs
#   use 4096-byte sectors, but 512 keeps our examples small and readable.
#
# - MAX_BLOCKS = 512: With 512-byte blocks, 512 blocks gives us a 256 KB
#   disk. Tiny by modern standards, but large enough to demonstrate every
#   file system concept including indirect blocks.
#
# - MAX_INODES = 128: Each inode represents one file or directory. 128 inodes
#   means we can have at most 128 files/directories on our disk. Real ext2
#   file systems have millions of inodes.
#
# - DIRECT_BLOCKS = 12: Each inode has 12 direct block pointers. This means
#   a file can be up to 12 * 512 = 6,144 bytes before needing an indirect
#   block. Most files in a Unix system are small enough to fit in direct
#   blocks alone.
#
# - ROOT_INODE = 0: The root directory "/" always lives at inode 0. This is
#   a fixed convention — the OS always knows where to start path resolution.

module CodingAdventures
  module FileSystem
    # === Disk Geometry Constants ===

    # Size of each block in bytes. Every read and write operates on
    # exactly one block. This mirrors the traditional 512-byte disk sector.
    BLOCK_SIZE = 512

    # Total number of blocks on the disk. 512 blocks * 512 bytes = 256 KB.
    MAX_BLOCKS = 512

    # Maximum number of inodes (files + directories) the file system supports.
    MAX_INODES = 128

    # Number of direct block pointers in each inode. Files up to
    # DIRECT_BLOCKS * BLOCK_SIZE = 6,144 bytes need only direct pointers.
    DIRECT_BLOCKS = 12

    # The inode number for the root directory "/". This is always 0 by
    # convention — the kernel hardcodes this when mounting the file system.
    ROOT_INODE = 0

    # Maximum length of a file or directory name in bytes. Matches the
    # traditional Unix NAME_MAX of 255 characters.
    MAX_NAME_LENGTH = 255

    # === File Types ===
    #
    # Every inode has a file_type field that tells the kernel what kind of
    # object this inode represents. This matters because the kernel treats
    # each type differently:
    #
    #   REGULAR files: data blocks contain file contents
    #   DIRECTORIES: data blocks contain DirectoryEntry records
    #   SYMLINKS: data blocks contain the target path string
    #   Devices, pipes, sockets: special kernel behavior, no data blocks
    #
    # These numeric values match the ext2 convention.

    # No type assigned — the inode is free and available for allocation.
    FILE_TYPE_NONE = 0

    # A regular file: text, binary, image, executable, etc.
    FILE_TYPE_REGULAR = 1

    # A directory: a file whose blocks contain (name, inode) pairs.
    FILE_TYPE_DIRECTORY = 2

    # A symbolic link: a file whose blocks contain a path string.
    FILE_TYPE_SYMLINK = 3

    # A character device (e.g., /dev/tty, /dev/null).
    FILE_TYPE_CHAR_DEVICE = 4

    # A block device (e.g., /dev/sda).
    FILE_TYPE_BLOCK_DEVICE = 5

    # A named pipe (FIFO) for inter-process communication.
    FILE_TYPE_PIPE = 6

    # A Unix domain socket for local network communication.
    FILE_TYPE_SOCKET = 7

    # === Open Flags ===
    #
    # When a process calls open(), it passes flags to specify:
    #   1. Access mode: read-only, write-only, or both
    #   2. Behavior modifiers: create if missing, truncate, append
    #
    # These values match the Linux kernel's flag definitions.
    #
    # Flags can be combined with bitwise OR. For example:
    #   O_WRONLY | O_CREAT | O_TRUNC  means "open for writing, create if
    #   it doesn't exist, and truncate to zero length if it does."
    #
    # Truth table for access mode check:
    #   flags & 0x3  | Can read? | Can write?
    #   -------------|-----------|----------
    #   0 (O_RDONLY) |    yes    |    no
    #   1 (O_WRONLY) |    no     |    yes
    #   2 (O_RDWR)   |    yes    |    yes

    # Open for reading only. The most restrictive mode.
    O_RDONLY = 0

    # Open for writing only.
    O_WRONLY = 1

    # Open for both reading and writing.
    O_RDWR = 2

    # Create the file if it does not exist. Without this flag, opening a
    # nonexistent file returns an error.
    O_CREAT = 64

    # Truncate the file to zero length when opening. Useful for overwriting
    # a file's contents completely.
    O_TRUNC = 512

    # Set the file offset to the end before each write. This ensures that
    # writes always append to the file, even if another process is also
    # writing to it.
    O_APPEND = 1024

    # === Seek Whence Constants ===
    #
    # These control how lseek() interprets the offset argument:
    #
    #   SEEK_SET: offset is absolute (from the beginning of the file)
    #   SEEK_CUR: offset is relative to the current position
    #   SEEK_END: offset is relative to the end of the file
    #
    # Example: a 100-byte file, current position at byte 50
    #   lseek(fd, 10, SEEK_SET) → position becomes 10
    #   lseek(fd, 10, SEEK_CUR) → position becomes 60
    #   lseek(fd, -10, SEEK_END) → position becomes 90

    # Set position to exactly the given offset.
    SEEK_SET = 0

    # Move position forward (or backward if negative) from current.
    SEEK_CUR = 1

    # Set position relative to end of file.
    SEEK_END = 2

    # === Magic Number ===
    #
    # The superblock's first field is a "magic number" — a fixed value that
    # lets the kernel verify this disk actually contains our file system.
    # Without it, mounting a random disk could corrupt data.
    #
    # 0x45585432 is the ASCII encoding of "EXT2".
    MAGIC = 0x45585432
  end
end
