# frozen_string_literal: true

# = BlockBitmap
#
# The block bitmap is a simple but elegant data structure that tracks which
# blocks on disk are free (available for storing new data) and which are in
# use. It uses exactly one bit per block:
#
#   0 = free (available for allocation)
#   1 = used (contains data, don't touch)
#
# == Analogy
#
# Imagine a parking garage with numbered spaces. The attendant has a board
# with one light per space: off (green) means the space is open, on (red)
# means it's taken. When a car arrives, the attendant scans for the first
# green light, turns it red, and directs the car there. When a car leaves,
# the light goes back to green.
#
# == Why a Bitmap?
#
# We could use a list of free block numbers, but a bitmap has two advantages:
#   1. Constant-time free/check: just flip/read one bit
#   2. Minimal space: 512 blocks need only 64 bytes (512 bits)
#
# The only operation that's not O(1) is allocation (finding the first free
# bit), which is O(n) in the worst case. Real file systems optimize this
# with hints and caching, but linear scan works fine for our 512 blocks.
#
# == Bitmap Layout
#
#   Bit:   0   1   2   3   4   5   6   7   8   ...
#   Val:   1   1   0   0   1   0   0   0   0   ...
#          ^   ^           ^
#          |   |           |
#        used used       used   (rest are free)

module CodingAdventures
  module FileSystem
    class BlockBitmap
      # Creates a new block bitmap with all blocks initially free.
      #
      # @param total_blocks [Integer] Number of blocks to track
      def initialize(total_blocks)
        @total_blocks = total_blocks
        # Each element is true (used) or false (free).
        # We use an array of booleans for clarity. A real implementation
        # would pack bits into integers for space efficiency.
        @bitmap = Array.new(total_blocks, false)
      end

      # Finds the first free block, marks it as used, and returns its number.
      # Returns nil if all blocks are in use (disk is full).
      #
      # This is the "parking attendant scanning for a green light" operation.
      #
      # @return [Integer, nil] The allocated block number, or nil if full
      def allocate
        @bitmap.each_with_index do |used, index|
          unless used
            @bitmap[index] = true
            return index
          end
        end
        nil  # Disk full — no free blocks
      end

      # Marks a block as free (available for reuse).
      #
      # @param block_number [Integer] The block to free
      # @raise [ArgumentError] If block_number is out of range
      def free(block_number)
        validate_block_number!(block_number)
        @bitmap[block_number] = false
      end

      # Checks whether a specific block is free.
      #
      # @param block_number [Integer] The block to check
      # @return [Boolean] true if the block is free
      def free?(block_number)
        validate_block_number!(block_number)
        !@bitmap[block_number]
      end

      # Returns the number of free blocks.
      #
      # @return [Integer] Count of free (unallocated) blocks
      def free_count
        @bitmap.count(false)
      end

      # Marks a block as used. This is used during formatting to reserve
      # blocks for metadata (root directory, etc.).
      #
      # @param block_number [Integer] The block to mark as used
      def mark_used(block_number)
        validate_block_number!(block_number)
        @bitmap[block_number] = true
      end

      private

      # Validates that a block number is within the valid range.
      #
      # @param block_number [Integer] The block number to validate
      # @raise [ArgumentError] If out of range
      def validate_block_number!(block_number)
        unless block_number >= 0 && block_number < @total_blocks
          raise ArgumentError, "Block number #{block_number} out of range (0..#{@total_blocks - 1})"
        end
      end
    end
  end
end
