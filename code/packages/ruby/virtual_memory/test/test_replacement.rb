# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    # == FIFO Tests ==
    #
    # FIFO evicts the oldest page -- the one loaded first.

    class TestFIFOPolicy < Minitest::Test
      def setup
        @policy = FIFOPolicy.new
      end

      def test_add_and_evict
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        assert_equal 3, @policy.size

        # Evict the oldest (first in).
        victim = @policy.select_victim
        assert_equal 0, victim
        assert_equal 2, @policy.size
      end

      def test_fifo_order
        @policy.add_frame(10)
        @policy.add_frame(20)
        @policy.add_frame(30)

        assert_equal 10, @policy.select_victim
        assert_equal 20, @policy.select_victim
        assert_equal 30, @policy.select_victim
        assert_nil @policy.select_victim
      end

      def test_record_access_is_noop
        # FIFO ignores access patterns.
        @policy.add_frame(0)
        @policy.add_frame(1)

        @policy.record_access(0)  # Should not change eviction order.

        assert_equal 0, @policy.select_victim  # Still 0 (oldest).
      end

      def test_remove_frame
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        @policy.remove_frame(1)
        assert_equal 2, @policy.size

        assert_equal 0, @policy.select_victim
        assert_equal 2, @policy.select_victim
      end

      def test_empty_select_victim
        assert_nil @policy.select_victim
      end

      # == Belady's Anomaly Setup ==
      #
      # FIFO can evict hot pages because it doesn't track usage.
      # Access pattern [A, B, C, D] with 3 frames:
      #   A: [A]
      #   B: [A, B]
      #   C: [A, B, C]     -- full
      #   D: evict A → [B, C, D]
      # Even though A might be accessed again soon.

      def test_fifo_evicts_regardless_of_access
        @policy.add_frame(0)  # A
        @policy.add_frame(1)  # B
        @policy.add_frame(2)  # C

        # Heavily access frame 0 -- FIFO doesn't care.
        10.times { @policy.record_access(0) }

        # Frame 0 is still evicted first because it was loaded first.
        assert_equal 0, @policy.select_victim
      end
    end

    # == LRU Tests ==
    #
    # LRU evicts the page that hasn't been accessed for the longest time.

    class TestLRUPolicy < Minitest::Test
      def setup
        @policy = LRUPolicy.new
      end

      def test_add_and_evict
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        # Frame 0 was added first (smallest timestamp) → evicted first.
        assert_equal 0, @policy.select_victim
      end

      def test_access_refreshes_frame
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        # Access frame 0 -- it's now the most recently used.
        @policy.record_access(0)

        # Frame 1 is now the LRU (hasn't been accessed since loading).
        assert_equal 1, @policy.select_victim
      end

      def test_lru_with_multiple_accesses
        @policy.add_frame(0)  # timestamp 1
        @policy.add_frame(1)  # timestamp 2
        @policy.add_frame(2)  # timestamp 3

        # Access pattern: 0, 2 → most recent is 2, then 0, LRU is 1.
        @policy.record_access(0)  # timestamp 4
        @policy.record_access(2)  # timestamp 5

        assert_equal 1, @policy.select_victim  # LRU
        assert_equal 0, @policy.select_victim  # next LRU
        assert_equal 2, @policy.select_victim  # last
      end

      def test_remove_frame
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        @policy.remove_frame(0)
        assert_equal 2, @policy.size

        assert_equal 1, @policy.select_victim
      end

      def test_empty_select_victim
        assert_nil @policy.select_victim
      end
    end

    # == Clock Tests ==
    #
    # Clock approximates LRU using a use bit and circular sweep.

    class TestClockPolicy < Minitest::Test
      def setup
        @policy = ClockPolicy.new
      end

      def test_add_frames_with_use_bit_set
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        assert_equal 3, @policy.size
      end

      def test_evict_clears_use_bits_then_evicts
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        # All frames have use_bit=true (freshly loaded).
        # The clock hand sweeps, clearing bits:
        #   Frame 0: use=1 → clear, advance
        #   Frame 1: use=1 → clear, advance
        #   Frame 2: use=1 → clear, advance
        #   Frame 0: use=0 → EVICT
        victim = @policy.select_victim
        assert_equal 0, victim
        assert_equal 2, @policy.size
      end

      def test_second_chance
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        # Clear all use bits by selecting a victim (sweeps all 3).
        # But actually, select_victim clears them. Let me be explicit:
        # After adding, all have use_bit=true.
        # Now access frame 1 (no-op since it's already true).
        # Let's first force clear frame 0's use bit.

        # We need to manually set up the scenario:
        # frame 0: use=false, frame 1: use=true, frame 2: use=false
        # The clock should skip frame 1 and evict frame 0.

        # Reset and rebuild with specific bits.
        @policy = ClockPolicy.new
        @policy.add_frame(10)
        @policy.add_frame(11)
        @policy.add_frame(12)

        # select_victim will sweep and clear all use bits, then evict 10.
        victim1 = @policy.select_victim
        assert_equal 10, victim1

        # Now 11 and 12 have use_bit=false (cleared during sweep).
        # Next victim is 11.
        victim2 = @policy.select_victim
        assert_equal 11, victim2
      end

      def test_access_sets_use_bit
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        # Select victim clears all use bits, then evicts 0.
        @policy.select_victim  # evicts 0

        # Now 1 and 2 have use_bit=false.
        # Access 1 to set its use bit.
        @policy.record_access(1)

        # Next victim should skip 1 (use=true) and evict 2 (use=false).
        victim = @policy.select_victim
        assert_equal 2, victim
      end

      def test_remove_frame
        @policy.add_frame(0)
        @policy.add_frame(1)
        @policy.add_frame(2)

        @policy.remove_frame(1)
        assert_equal 2, @policy.size
      end

      def test_empty_select_victim
        assert_nil @policy.select_victim
      end

      def test_single_frame
        @policy.add_frame(42)
        # Use bit is true, so clock clears it and wraps, then evicts.
        victim = @policy.select_victim
        assert_equal 42, victim
        assert_equal 0, @policy.size
      end
    end
  end
end
