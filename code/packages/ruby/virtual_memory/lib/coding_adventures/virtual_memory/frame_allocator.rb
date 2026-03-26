# frozen_string_literal: true

# = Physical Frame Allocator
#
# The frame allocator manages physical memory frames. Physical memory
# (RAM) is divided into fixed-size frames of PAGE_SIZE (4 KB) bytes.
# The allocator tracks which frames are free and which are in use.
#
# == Bitmap Allocation
#
# We use a bitmap (array of booleans) where each element represents
# one physical frame:
#
#   bitmap[i] = false  →  frame i is FREE
#   bitmap[i] = true   →  frame i is ALLOCATED
#
# Example for 16 frames:
#   [T, T, T, F, F, T, F, F, F, T, T, F, F, F, F, F]
#    ^  ^  ^        ^           ^  ^
#    kernel frames  process     process frames
#
# == Allocation Strategy
#
# We use first-fit: scan the bitmap from the beginning and return the
# first free frame. This is O(n) in the worst case but simple to
# implement and understand.
#
# Real operating systems use more sophisticated allocators:
#   - Free lists: O(1) allocation by maintaining a linked list of free frames
#   - Buddy system: O(log n) allocation that minimizes fragmentation
#   - Per-CPU free lists: avoid lock contention on multiprocessor systems
#
# == Reference Counting
#
# Each frame has a reference count tracking how many page table entries
# point to it. This is essential for copy-on-write: when two processes
# share a frame, the refcount is 2. When one process unmaps the frame,
# the refcount drops to 1. The frame is only truly freed when the
# refcount reaches 0.

module CodingAdventures
  module VirtualMemory
    class FrameAllocator
      attr_reader :total_frames

      # Create a new frame allocator for the given number of frames.
      #
      # @param total_frames [Integer] the total number of physical frames.
      #   For 16 MB of RAM with 4 KB frames: 16 * 1024 * 1024 / 4096 = 4096.
      def initialize(total_frames)
        @total_frames = total_frames

        # The bitmap: false = free, true = allocated.
        @bitmap = Array.new(total_frames, false)

        # Reference counts for copy-on-write support.
        # refcount[frame] tracks how many PTEs point to this frame.
        @refcounts = Array.new(total_frames, 0)

        # Free frame count, maintained for O(1) queries.
        @free_count = total_frames
      end

      # Allocate a physical frame.
      #
      # Scans the bitmap for the first free frame, marks it as allocated,
      # sets its reference count to 1, and returns its frame number.
      #
      # @return [Integer, nil] the allocated frame number, or nil if
      #   all frames are in use (out of memory)
      def allocate
        @bitmap.each_with_index do |allocated, frame|
          unless allocated
            @bitmap[frame] = true
            @refcounts[frame] = 1
            @free_count -= 1
            return frame
          end
        end

        # No free frames available. The caller must either:
        #   - Use page replacement to evict a page and free a frame
        #   - Return an out-of-memory error to the process
        nil
      end

      # Free a physical frame.
      #
      # Marks the frame as free in the bitmap. Raises an error if the
      # frame is already free (double-free is always a bug).
      #
      # @param frame_number [Integer] the frame to free
      # @raise [ArgumentError] if the frame is already free or out of range
      def free(frame_number)
        validate_frame!(frame_number)

        unless @bitmap[frame_number]
          raise ArgumentError, "Double free: frame #{frame_number} is already free"
        end

        @bitmap[frame_number] = false
        @refcounts[frame_number] = 0
        @free_count += 1
      end

      # Check if a frame is currently allocated.
      #
      # @param frame_number [Integer] the frame to check
      # @return [Boolean] true if the frame is in use
      def allocated?(frame_number)
        validate_frame!(frame_number)
        @bitmap[frame_number]
      end

      # How many frames are currently free?
      #
      # @return [Integer] the number of available frames
      def free_count
        @free_count
      end

      # Increment the reference count for a frame.
      #
      # Called when a copy-on-write fork creates a new mapping to
      # an existing frame. Both the parent and child now share the
      # frame, so the refcount goes from 1 to 2.
      #
      # @param frame_number [Integer] the frame to increment
      def increment_refcount(frame_number)
        validate_frame!(frame_number)
        @refcounts[frame_number] += 1
      end

      # Decrement the reference count for a frame.
      #
      # Called when a mapping to a frame is removed. If the refcount
      # drops to 0, the frame is freed.
      #
      # @param frame_number [Integer] the frame to decrement
      # @return [Boolean] true if the frame was freed (refcount hit 0)
      def decrement_refcount(frame_number)
        validate_frame!(frame_number)
        @refcounts[frame_number] -= 1

        if @refcounts[frame_number] <= 0
          @bitmap[frame_number] = false
          @refcounts[frame_number] = 0
          @free_count += 1
          true
        else
          false
        end
      end

      # Get the reference count for a frame.
      #
      # @param frame_number [Integer] the frame to query
      # @return [Integer] the reference count
      def refcount(frame_number)
        validate_frame!(frame_number)
        @refcounts[frame_number]
      end

      private

      # Validate that a frame number is within range.
      #
      # @param frame_number [Integer] the frame to validate
      # @raise [ArgumentError] if out of range
      def validate_frame!(frame_number)
        unless frame_number >= 0 && frame_number < @total_frames
          raise ArgumentError,
            "Frame #{frame_number} out of range (0..#{@total_frames - 1})"
        end
      end
    end
  end
end
