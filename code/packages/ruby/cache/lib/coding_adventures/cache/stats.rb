# frozen_string_literal: true

# Cache statistics tracking -- measuring how well the cache is performing.
#
# Every cache keeps a scorecard. Just like a baseball player tracks batting
# average (hits / at-bats), a cache tracks its **hit rate** (cache hits /
# total accesses). A high hit rate means the cache is doing its job well --
# most memory requests are being served quickly from the cache rather than
# going to slower main memory.
#
# Key metrics:
# - **Reads/Writes**: How many times the CPU asked for data or stored data.
# - **Hits**: How many times the requested data was already in the cache.
# - **Misses**: How many times we had to go to a slower level to get the data.
# - **Evictions**: How many times we had to kick out old data to make room.
# - **Writebacks**: How many evictions involved dirty data that needed to be
#   written back to the next level (only relevant for write-back caches).
#
# Analogy: Think of a library desk (L1 cache). If you keep the right books
# on your desk, you rarely need to walk to the shelf (L2). Your "hit rate"
# is how often the book you need is already on your desk.

module CodingAdventures
  module Cache
    # Immutable snapshot of cache statistics, returned by CacheStats#to_data.
    #
    # This is a frozen value object -- useful for capturing stats at a point
    # in time without worrying about later mutations.
    CacheStatsData = Data.define(
      :reads, :writes, :hits, :misses,
      :evictions, :writebacks,
      :total_accesses, :hit_rate, :miss_rate
    )

    # Tracks performance statistics for a single cache level.
    #
    # Every read or write to the cache updates these counters. After running
    # a simulation, you can inspect hit_rate and miss_rate to see how
    # effective the cache configuration is for a given workload.
    #
    # Example:
    #   stats = CodingAdventures::Cache::CacheStats.new
    #   stats.record_read(hit: true)
    #   stats.record_read(hit: false)
    #   stats.hit_rate  # => 0.5
    #   stats.miss_rate # => 0.5
    class CacheStats
      attr_reader :reads, :writes, :hits, :misses, :evictions, :writebacks

      def initialize
        @reads = 0
        @writes = 0
        @hits = 0
        @misses = 0
        @evictions = 0
        @writebacks = 0
      end

      # -- Derived metrics ------------------------------------------------

      # Total number of read + write operations.
      def total_accesses
        @reads + @writes
      end

      # Fraction of accesses that were cache hits (0.0 to 1.0).
      #
      # Returns 0.0 if no accesses have been made (avoid division by zero).
      # A hit rate of 0.95 means 95% of memory requests were served from
      # this cache level -- excellent for an L1 cache.
      def hit_rate
        return 0.0 if total_accesses.zero?

        @hits.to_f / total_accesses
      end

      # Fraction of accesses that were cache misses (0.0 to 1.0).
      #
      # Always equals 1.0 - hit_rate. Provided for convenience since
      # miss rate is the more commonly discussed metric in architecture
      # papers ("this workload has a 5% L1 miss rate").
      def miss_rate
        return 0.0 if total_accesses.zero?

        @misses.to_f / total_accesses
      end

      # -- Recording methods -----------------------------------------------

      # Record a read access. Pass hit: true for a cache hit.
      def record_read(hit:)
        @reads += 1
        if hit
          @hits += 1
        else
          @misses += 1
        end
      end

      # Record a write access. Pass hit: true for a cache hit.
      def record_write(hit:)
        @writes += 1
        if hit
          @hits += 1
        else
          @misses += 1
        end
      end

      # Record an eviction. Pass dirty: true if the evicted line was dirty.
      #
      # A dirty eviction means the data was modified in the cache but not
      # yet written to the next level. The cache controller must "write back"
      # the dirty data before discarding it -- this is the extra cost of a
      # write-back policy.
      def record_eviction(dirty:)
        @evictions += 1
        @writebacks += 1 if dirty
      end

      # Reset all counters to zero.
      #
      # Useful when you want to measure stats for a specific phase of
      # execution (e.g., "what's the hit rate during matrix multiply?"
      # without counting the initial data loading phase).
      def reset
        @reads = 0
        @writes = 0
        @hits = 0
        @misses = 0
        @evictions = 0
        @writebacks = 0
      end

      # Return an immutable snapshot of the current statistics.
      def to_data
        CacheStatsData.new(
          reads: @reads,
          writes: @writes,
          hits: @hits,
          misses: @misses,
          evictions: @evictions,
          writebacks: @writebacks,
          total_accesses: total_accesses,
          hit_rate: hit_rate,
          miss_rate: miss_rate
        )
      end

      def to_s
        "CacheStats(accesses=#{total_accesses}, " \
          "hits=#{@hits}, misses=#{@misses}, " \
          "hit_rate=#{"%.1f%%" % (hit_rate * 100)}, " \
          "evictions=#{@evictions}, writebacks=#{@writebacks})"
      end

      alias_method :inspect, :to_s
    end
  end
end
