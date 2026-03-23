# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    class TestFrameAllocator < Minitest::Test
      def setup
        @allocator = FrameAllocator.new(8)
      end

      # == Fresh Allocator ==
      #
      # All frames start free.

      def test_fresh_allocator
        assert_equal 8, @allocator.total_frames
        assert_equal 8, @allocator.free_count
      end

      # == Sequential Allocation ==
      #
      # First-fit allocation returns frames in order: 0, 1, 2, ...

      def test_sequential_allocation
        assert_equal 0, @allocator.allocate
        assert_equal 1, @allocator.allocate
        assert_equal 2, @allocator.allocate
        assert_equal 5, @allocator.free_count
      end

      # == Allocate All Frames ==
      #
      # When all frames are allocated, allocate returns nil.

      def test_allocate_all_frames
        8.times { @allocator.allocate }
        assert_equal 0, @allocator.free_count
        assert_nil @allocator.allocate
      end

      # == Free and Reallocate ==
      #
      # Freeing a frame makes it available for reallocation.

      def test_free_and_reallocate
        frame0 = @allocator.allocate
        frame1 = @allocator.allocate
        assert_equal 0, frame0
        assert_equal 1, frame1

        @allocator.free(0)
        assert_equal 7, @allocator.free_count

        # Next allocation reuses the freed frame.
        reused = @allocator.allocate
        assert_equal 0, reused
      end

      # == Double Free ==
      #
      # Freeing an already-free frame is a bug. Raise an error.

      def test_double_free_raises_error
        frame = @allocator.allocate
        @allocator.free(frame)

        assert_raises(ArgumentError) { @allocator.free(frame) }
      end

      # == Out of Range ==
      #
      # Frame numbers outside the valid range raise errors.

      def test_out_of_range
        assert_raises(ArgumentError) { @allocator.free(-1) }
        assert_raises(ArgumentError) { @allocator.free(8) }
        assert_raises(ArgumentError) { @allocator.allocated?(8) }
      end

      # == Allocated? ==
      #
      # Check if a specific frame is in use.

      def test_allocated_query
        refute @allocator.allocated?(0)

        @allocator.allocate
        assert @allocator.allocated?(0)
        refute @allocator.allocated?(1)
      end

      # == Reference Counting ==
      #
      # Reference counts track how many page tables point to a frame.
      # This is essential for copy-on-write.

      def test_reference_counting
        frame = @allocator.allocate
        assert_equal 1, @allocator.refcount(frame)

        @allocator.increment_refcount(frame)
        assert_equal 2, @allocator.refcount(frame)

        @allocator.increment_refcount(frame)
        assert_equal 3, @allocator.refcount(frame)
      end

      def test_decrement_refcount
        frame = @allocator.allocate
        @allocator.increment_refcount(frame)
        # refcount is now 2.

        freed = @allocator.decrement_refcount(frame)
        refute freed
        assert_equal 1, @allocator.refcount(frame)

        freed = @allocator.decrement_refcount(frame)
        assert freed
        assert_equal 0, @allocator.refcount(frame)
        refute @allocator.allocated?(frame)
      end

      def test_decrement_refcount_frees_frame
        frame = @allocator.allocate
        assert_equal 7, @allocator.free_count

        freed = @allocator.decrement_refcount(frame)
        assert freed
        assert_equal 8, @allocator.free_count
      end

      # == Refcount Out of Range ==

      def test_refcount_out_of_range
        assert_raises(ArgumentError) { @allocator.refcount(-1) }
        assert_raises(ArgumentError) { @allocator.increment_refcount(8) }
        assert_raises(ArgumentError) { @allocator.decrement_refcount(8) }
      end
    end
  end
end
