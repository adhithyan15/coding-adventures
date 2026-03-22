# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    class TestTLB < Minitest::Test
      def setup
        @tlb = TLB.new(capacity: 4)
      end

      # == Empty TLB ==
      #
      # A fresh TLB has no entries and zero hits/misses.

      def test_empty_tlb
        assert_equal 0, @tlb.size
        assert_equal 0, @tlb.hits
        assert_equal 0, @tlb.misses
      end

      # == Insert and Lookup ==
      #
      # After inserting a translation, lookup should return the frame.

      def test_insert_and_lookup
        @tlb.insert(1, 5, 10)

        result = @tlb.lookup(1, 5)
        assert_equal 10, result
        assert_equal 1, @tlb.hits
        assert_equal 0, @tlb.misses
      end

      # == Lookup Miss ==
      #
      # Looking up a nonexistent entry returns nil and increments misses.

      def test_lookup_miss
        result = @tlb.lookup(1, 5)
        assert_nil result
        assert_equal 0, @tlb.hits
        assert_equal 1, @tlb.misses
      end

      # == Process Isolation ==
      #
      # The same VPN in different processes maps to different frames.

      def test_process_isolation
        @tlb.insert(1, 5, 10)
        @tlb.insert(2, 5, 20)

        assert_equal 10, @tlb.lookup(1, 5)
        assert_equal 20, @tlb.lookup(2, 5)
      end

      # == LRU Eviction ==
      #
      # When the TLB is full, the least recently used entry is evicted.

      def test_lru_eviction
        # Fill TLB to capacity (4 entries).
        @tlb.insert(1, 0, 100)
        @tlb.insert(1, 1, 101)
        @tlb.insert(1, 2, 102)
        @tlb.insert(1, 3, 103)
        assert_equal 4, @tlb.size

        # Access entry 0 to make it recently used.
        @tlb.lookup(1, 0)

        # Insert a 5th entry -- should evict entry 1 (LRU).
        @tlb.insert(1, 4, 104)
        assert_equal 4, @tlb.size

        # Entry 1 should be evicted.
        assert_nil @tlb.lookup(1, 1)

        # Entry 0 should still be present (was accessed).
        assert_equal 100, @tlb.lookup(1, 0)
      end

      # == Update Existing Entry ==
      #
      # Inserting with an existing key updates the frame number.

      def test_update_existing_entry
        @tlb.insert(1, 5, 10)
        @tlb.insert(1, 5, 20)

        assert_equal 20, @tlb.lookup(1, 5)
        assert_equal 1, @tlb.size  # No duplicate entries.
      end

      # == Invalidate ==
      #
      # invalidate removes a single entry (e.g., after a page fault).

      def test_invalidate
        @tlb.insert(1, 5, 10)
        @tlb.insert(1, 6, 11)

        @tlb.invalidate(1, 5)
        assert_nil @tlb.lookup(1, 5)
        assert_equal 11, @tlb.lookup(1, 6)
        assert_equal 1, @tlb.size
      end

      def test_invalidate_nonexistent
        # Should not raise an error.
        @tlb.invalidate(1, 99)
      end

      # == Flush ==
      #
      # flush clears all entries (called on context switch).

      def test_flush
        @tlb.insert(1, 0, 100)
        @tlb.insert(1, 1, 101)
        @tlb.insert(2, 0, 200)

        @tlb.flush
        assert_equal 0, @tlb.size
        assert_nil @tlb.lookup(1, 0)
        assert_nil @tlb.lookup(2, 0)
      end

      # == Hit Rate ==
      #
      # hit_rate = hits / (hits + misses).

      def test_hit_rate
        assert_equal 0.0, @tlb.hit_rate  # No lookups yet.

        @tlb.insert(1, 0, 100)
        @tlb.lookup(1, 0)   # hit
        @tlb.lookup(1, 0)   # hit
        @tlb.lookup(1, 0)   # hit
        @tlb.lookup(1, 99)  # miss

        # 3 hits, 1 miss = 75% hit rate
        assert_in_delta 0.75, @tlb.hit_rate, 0.001
      end

      # == Reset Stats ==
      #
      # Reset counters for a fresh measurement window.

      def test_reset_stats
        @tlb.insert(1, 0, 100)
        @tlb.lookup(1, 0)
        @tlb.lookup(1, 99)

        @tlb.reset_stats
        assert_equal 0, @tlb.hits
        assert_equal 0, @tlb.misses
        assert_equal 0.0, @tlb.hit_rate
      end

      # == Capacity ==
      #
      # The default TLB capacity is 64.

      def test_default_capacity
        tlb = TLB.new
        assert_equal DEFAULT_TLB_CAPACITY, tlb.capacity
      end

      # == Eviction Under Pressure ==
      #
      # Verifies correct eviction order when continuously inserting.

      def test_continuous_eviction
        # TLB capacity is 4.
        # Insert 6 entries -- first 2 should be evicted.
        6.times { |i| @tlb.insert(1, i, i * 10) }

        assert_equal 4, @tlb.size

        # Entries 0 and 1 should be evicted (LRU -- oldest).
        assert_nil @tlb.lookup(1, 0)
        assert_nil @tlb.lookup(1, 1)

        # Entries 2-5 should still be present.
        assert_equal 20, @tlb.lookup(1, 2)
        assert_equal 30, @tlb.lookup(1, 3)
        assert_equal 40, @tlb.lookup(1, 4)
        assert_equal 50, @tlb.lookup(1, 5)
      end
    end
  end
end
