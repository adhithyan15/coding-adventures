# frozen_string_literal: true

# = InodeTable
#
# The inode table is the master index of every file and directory on disk.
# It holds a fixed-size array of inodes (128 in our file system). Each slot
# is either occupied (holding metadata for a file/directory) or free
# (available for a new file/directory).
#
# == Analogy
#
# Think of the inode table as a hotel register. There are 128 rooms
# (inode slots). When a guest checks in (file is created), you assign them
# the first available room and write down their details. When they check
# out (file is deleted), you erase the entry and mark the room as vacant.
#
# == Operations
#
#   allocate(file_type) → inode_number
#     Find the first free inode, initialize it with the given type,
#     and return its number.
#
#   free(inode_number)
#     Mark the inode as free (type = NONE), clearing all its fields.
#
#   get(inode_number) → Inode
#     Return the inode at the given slot for reading or modification.

module CodingAdventures
  module FileSystem
    class InodeTable
      # Creates a new inode table with all inodes initially free.
      #
      # @param total_inodes [Integer] Number of inode slots (default: 128)
      def initialize(total_inodes = MAX_INODES)
        @total_inodes = total_inodes
        # Pre-allocate all inode slots. Each starts as FILE_TYPE_NONE (free).
        @inodes = Array.new(total_inodes) { |i| Inode.new(i) }
      end

      # Finds the first free inode, marks it as the given type, and returns it.
      #
      # @param file_type [Integer] One of the FILE_TYPE_* constants
      # @return [Inode, nil] The newly allocated inode, or nil if table is full
      def allocate(file_type)
        @inodes.each do |inode|
          if inode.free?
            inode.file_type = file_type
            inode.size = 0
            inode.permissions = 0o755
            inode.owner_pid = 0
            inode.link_count = 0
            inode.direct_blocks = Array.new(DIRECT_BLOCKS)
            inode.indirect_block = nil
            now = Time.now
            inode.created_at = now
            inode.modified_at = now
            inode.accessed_at = now
            return inode
          end
        end
        nil  # Inode table full
      end

      # Marks an inode as free, clearing all its fields.
      #
      # @param inode_number [Integer] The inode slot to free
      # @raise [ArgumentError] If inode_number is out of range
      def free(inode_number)
        validate_inode_number!(inode_number)
        inode = @inodes[inode_number]
        inode.file_type = FILE_TYPE_NONE
        inode.size = 0
        inode.permissions = 0
        inode.owner_pid = 0
        inode.link_count = 0
        inode.direct_blocks = Array.new(DIRECT_BLOCKS)
        inode.indirect_block = nil
      end

      # Returns the inode at the given slot.
      #
      # @param inode_number [Integer] The inode slot to retrieve
      # @return [Inode, nil] The inode, or nil if out of range
      def get(inode_number)
        return nil if inode_number < 0 || inode_number >= @total_inodes

        @inodes[inode_number]
      end

      # Returns the number of free (unallocated) inodes.
      #
      # @return [Integer] Count of free inode slots
      def free_count
        @inodes.count(&:free?)
      end

      private

      # Validates that an inode number is within range.
      #
      # @param inode_number [Integer] The inode number to validate
      # @raise [ArgumentError] If out of range
      def validate_inode_number!(inode_number)
        unless inode_number >= 0 && inode_number < @total_inodes
          raise ArgumentError, "Inode number #{inode_number} out of range (0..#{@total_inodes - 1})"
        end
      end
    end
  end
end
