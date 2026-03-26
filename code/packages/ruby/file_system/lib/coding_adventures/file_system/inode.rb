# frozen_string_literal: true

# = Inode (Index Node)
#
# An inode is the core data structure of a Unix file system. Every file,
# directory, symbolic link, device, pipe, and socket on disk has exactly
# one inode. The inode stores everything about the object *except its name*.
#
# == Why Aren't Names Stored in the Inode?
#
# Because one file can have multiple names! This is called a "hard link."
# If you run `ln original.txt alias.txt`, both names point to the same
# inode. The file's data exists only once on disk. The inode's `link_count`
# tracks how many names reference it. When `link_count` drops to 0, the
# file is truly deleted and its blocks can be reused.
#
# == How Does the Inode Find File Data?
#
# The inode contains an array of *block pointers* — numbers that tell the
# file system which blocks on disk hold this file's data:
#
#   Inode
#   +---------------------------+
#   | direct_blocks[0]  ---------> Data Block (bytes 0..511)
#   | direct_blocks[1]  ---------> Data Block (bytes 512..1023)
#   | ...                       |
#   | direct_blocks[11] ---------> Data Block (bytes 5632..6143)
#   |                           |
#   | indirect_block    ---------> [ptr0, ptr1, ..., ptr127]
#   |                           |     |     |          |
#   +---------------------------+     v     v          v
#                               Data Blk  Data Blk  Data Blk
#
# With 12 direct pointers (each pointing to a 512-byte block), a file can
# be up to 6,144 bytes without needing indirection. The indirect block
# adds 128 more pointers for an additional 65,536 bytes, giving a maximum
# file size of 71,680 bytes.

module CodingAdventures
  module FileSystem
    class Inode
      # Unique identifier for this inode, ranging from 0 to MAX_INODES - 1.
      # Inode 0 is always the root directory "/".
      attr_accessor :inode_number

      # What kind of file system object this inode represents.
      # See the FILE_TYPE_* constants in constants.rb.
      attr_accessor :file_type

      # Size of the file in bytes. For regular files, this is the number of
      # bytes of data. For directories, this is the total size of all
      # directory entries stored in the data blocks.
      attr_accessor :size

      # Permission bits in octal notation (e.g., 0o755 means rwxr-xr-x).
      # Stored as a 16-bit integer.
      #
      # Permission bits truth table (each digit is 3 bits: rwx):
      #   Bit | Meaning  | Octal
      #   ----|----------|------
      #   r   | Read     |  4
      #   w   | Write    |  2
      #   x   | Execute  |  1
      #
      # Example: 0o755 = owner(rwx=7) group(r-x=5) other(r-x=5)
      attr_accessor :permissions

      # PID of the process that created this file. In a real OS, this would
      # be a user ID (UID), but we use PID for simplicity since our OS
      # doesn't have users yet.
      attr_accessor :owner_pid

      # Number of directory entries that point to this inode. Hard links
      # increment this count. When it reaches 0, the inode and all its
      # data blocks are freed.
      attr_accessor :link_count

      # Array of 12 block numbers. direct_blocks[i] is the block number
      # that holds bytes i*BLOCK_SIZE through (i+1)*BLOCK_SIZE - 1 of the
      # file's data. A value of nil means that block slot is unused.
      attr_accessor :direct_blocks

      # Block number of the indirect block. The indirect block itself
      # contains 128 block numbers (4 bytes each in a 512-byte block),
      # each pointing to a data block. nil means no indirect block.
      attr_accessor :indirect_block

      # Timestamps tracking when this inode was created, last modified
      # (data changed), and last accessed (data read).
      attr_accessor :created_at, :modified_at, :accessed_at

      # Creates a new inode with the given number and type.
      #
      # @param inode_number [Integer] The inode's unique ID (0..127)
      # @param file_type [Integer] One of the FILE_TYPE_* constants
      def initialize(inode_number, file_type = FILE_TYPE_NONE)
        @inode_number = inode_number
        @file_type = file_type
        @size = 0
        @permissions = 0o755
        @owner_pid = 0
        @link_count = 0
        @direct_blocks = Array.new(DIRECT_BLOCKS)  # nil-filled
        @indirect_block = nil
        now = Time.now
        @created_at = now
        @modified_at = now
        @accessed_at = now
      end

      # Returns true if this inode has no type assigned, meaning it is
      # free and available for allocation.
      def free?
        @file_type == FILE_TYPE_NONE
      end

      # Returns true if this inode represents a directory.
      def directory?
        @file_type == FILE_TYPE_DIRECTORY
      end

      # Returns true if this inode represents a regular file.
      def regular?
        @file_type == FILE_TYPE_REGULAR
      end
    end
  end
end
