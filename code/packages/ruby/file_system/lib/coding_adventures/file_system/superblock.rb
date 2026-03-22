# frozen_string_literal: true

# = Superblock
#
# The superblock is the very first thing on the disk (block 0). It is the
# "table of contents" for the entire file system. Without the superblock,
# the operating system has no idea how the disk is organized — it would just
# see 256 KB of meaningless bytes.
#
# == Analogy
#
# Think of the superblock as the cover page of a book's index. It tells you:
#   - What kind of book this is (magic number = "this is an EXT2 file system")
#   - How many chapters there are (total_blocks, total_inodes)
#   - How many blank pages are left (free_blocks, free_inodes)
#
# == Real-World Note
#
# In real ext2, the superblock is replicated across multiple "block groups"
# for redundancy. If block 0 gets corrupted, the OS can recover from a
# backup copy. We keep just one copy for simplicity.

module CodingAdventures
  module FileSystem
    class Superblock
      # The magic number identifies this disk as containing our file system.
      # If you try to mount a disk and this field doesn't match, the mount
      # fails immediately with "bad magic number."
      attr_accessor :magic

      # Size of each block in bytes. Every block on this disk is exactly
      # this size. Our file system uses 512 bytes.
      attr_accessor :block_size

      # Total number of blocks on the entire disk, including the superblock
      # itself, the inode table, the bitmap, and all data blocks.
      attr_accessor :total_blocks

      # Total number of inodes this file system can hold. This is a fixed
      # limit set at format time — you cannot add more inodes later.
      attr_accessor :total_inodes

      # Number of data blocks currently available for allocation. This
      # decreases as files are written and increases as files are deleted.
      attr_accessor :free_blocks

      # Number of inodes currently available. This decreases as files and
      # directories are created, and increases as they are deleted.
      attr_accessor :free_inodes

      # The inode number of the root directory. Always 0 in our file system.
      attr_accessor :root_inode

      # Creates a new superblock with default values for a freshly formatted
      # disk. The caller (VFS.format) may adjust free_blocks and free_inodes
      # after formatting is complete.
      #
      # @param total_blocks [Integer] Total blocks on disk (default: 512)
      # @param total_inodes [Integer] Maximum inodes (default: 128)
      def initialize(total_blocks: MAX_BLOCKS, total_inodes: MAX_INODES)
        @magic = MAGIC
        @block_size = BLOCK_SIZE
        @total_blocks = total_blocks
        @total_inodes = total_inodes
        @free_blocks = 0
        @free_inodes = total_inodes
        @root_inode = ROOT_INODE
      end

      # Validates that this superblock has the correct magic number.
      # If the magic number is wrong, this disk does not contain our
      # file system and must not be mounted.
      #
      # @return [Boolean] true if the magic number matches
      def valid?
        @magic == MAGIC
      end
    end
  end
end
