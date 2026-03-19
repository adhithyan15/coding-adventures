# frozen_string_literal: true

# Cache set -- a group of cache lines that share the same set index.
#
# A cache set is like a row of labeled boxes on a shelf. When the CPU
# accesses memory, the address tells us *which shelf* (set) to look at.
# Within that shelf, we check each box (way) to see if our data is there.
#
# In a **4-way set-associative** cache, each set has 4 lines (ways).
# When all 4 are full and we need to bring in new data, we must **evict**
# one. The LRU (Least Recently Used) policy picks the line that hasn't
# been accessed for the longest time -- the logic being "if you haven't
# used it lately, you probably won't need it soon."
#
# Associativity is a key design tradeoff:
# - **Direct-mapped** (1-way): Fast lookup, but high conflict misses.
# - **Fully associative** (N-way = total lines): No conflicts, but
#   expensive to search every line on every access.
# - **Set-associative** (2/4/8/16-way): The sweet spot. Each address
#   maps to a set, and within that set, any way can hold it.
#
#     Set 0: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
#     Set 1: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
#     Set 2: [ Way 0 ] [ Way 1 ] [ Way 2 ] [ Way 3 ]
#     ...

require_relative "cache_line"

module CodingAdventures
  module Cache
    # Configuration for a cache level -- the knobs you turn to get L1/L2/L3.
    #
    # By adjusting these parameters, the exact same Cache class can simulate
    # anything from a tiny 1KB direct-mapped L1 to a massive 32MB 16-way L3.
    #
    # Real-world examples:
    #   ARM Cortex-A78: L1D = 64KB, 4-way, 64B lines, 1 cycle
    #   Intel Alder Lake: L1D = 48KB, 12-way, 64B lines, 5 cycles
    #   Apple M4: L1D = 128KB, 8-way, 64B lines, ~3 cycles
    CacheConfig = Data.define(
      :name,           # Human-readable name ("L1D", "L2", etc.)
      :total_size,     # Total capacity in bytes (e.g., 65536 for 64KB)
      :line_size,      # Bytes per cache line. Must be a power of 2.
      :associativity,  # Number of ways per set. 1 = direct-mapped.
      :access_latency, # Clock cycles to access this level on a hit.
      :write_policy    # "write-back" or "write-through"
    ) do
      # @param name [String]
      # @param total_size [Integer]
      # @param line_size [Integer]
      # @param associativity [Integer]
      # @param access_latency [Integer]
      # @param write_policy [String]
      def initialize(name:, total_size:, line_size: 64, associativity: 4, access_latency: 1, write_policy: "write-back")
        # Validate configuration parameters.
        # Cache sizes and line sizes must be powers of 2 -- this is a
        # hardware constraint because address bit-slicing only works
        # cleanly with power-of-2 sizes.
        raise ArgumentError, "total_size must be positive, got #{total_size}" if total_size <= 0
        if line_size <= 0 || (line_size & (line_size - 1)) != 0
          raise ArgumentError, "line_size must be a positive power of 2, got #{line_size}"
        end
        raise ArgumentError, "associativity must be positive, got #{associativity}" if associativity <= 0
        if total_size % (line_size * associativity) != 0
          raise ArgumentError,
            "total_size (#{total_size}) must be divisible by " \
            "line_size * associativity (#{line_size * associativity})"
        end
        unless %w[write-back write-through].include?(write_policy)
          raise ArgumentError, "write_policy must be 'write-back' or 'write-through', got '#{write_policy}'"
        end
        raise ArgumentError, "access_latency must be non-negative, got #{access_latency}" if access_latency < 0

        super
      end

      # Total number of cache lines = total_size / line_size.
      def num_lines = total_size / line_size

      # Number of sets = num_lines / associativity.
      def num_sets = num_lines / associativity
    end

    # One set in the cache -- contains N ways (lines).
    #
    # Implements LRU (Least Recently Used) replacement: when all ways are
    # full and we need to bring in new data, evict the line that was
    # accessed least recently.
    #
    # Think of it like a desk with N book slots. When all slots are full
    # and you need a new book, you put away the one you haven't read in
    # the longest time.
    class CacheSet
      attr_reader :lines

      # Create a cache set with the given number of ways.
      #
      # @param associativity [Integer] Number of ways (lines) in this set.
      # @param line_size [Integer] Bytes per cache line.
      def initialize(associativity:, line_size:)
        @lines = Array.new(associativity) { CacheLine.new(line_size: line_size) }
      end

      # -- Lookup ----------------------------------------------------------

      # Check if a tag is present in this set.
      #
      # Searches all ways for a valid line with a matching tag. This is
      # what happens in hardware with a parallel tag comparator -- all
      # ways are checked simultaneously.
      #
      # @param tag [Integer] The tag bits from the address.
      # @return [Array(Boolean, Integer|nil)] [hit, way_index]
      def lookup(tag)
        @lines.each_with_index do |line, i|
          return [true, i] if line.valid && line.tag == tag
        end
        [false, nil]
      end

      # -- Access ----------------------------------------------------------

      # Access this set for a given tag. Returns [hit, line].
      #
      # On a hit, updates the line's LRU timestamp so it becomes the
      # most recently used. On a miss, returns the LRU victim line
      # (the caller decides what to do -- typically allocate new data).
      #
      # @param tag [Integer] The tag bits from the address.
      # @param cycle [Integer] Current clock cycle for LRU tracking.
      # @return [Array(Boolean, CacheLine)] [hit, line]
      def access(tag, cycle)
        hit, way_index = lookup(tag)
        if hit
          line = @lines[way_index]
          line.touch(cycle)
          [true, line]
        else
          # Miss -- return the LRU line (candidate for eviction)
          lru_index = find_lru
          [false, @lines[lru_index]]
        end
      end

      # -- Allocation (filling after a miss) --------------------------------

      # Bring new data into this set after a cache miss.
      #
      # First tries to find an invalid (empty) way. If all ways are
      # valid, evicts the LRU line. Returns the evicted line if it was
      # dirty (the caller must write it back to the next level).
      #
      # @param tag [Integer] Tag for the new data.
      # @param data [Array<Integer>] The bytes to store.
      # @param cycle [Integer] Current clock cycle.
      # @return [CacheLine, nil] Evicted dirty CacheLine or nil.
      #
      # Think of it like clearing a desk slot for a new book:
      # 1. If there's an empty slot, use it (no eviction needed).
      # 2. If all slots are full, pick the least-recently-read book.
      # 3. If that book had notes scribbled in it (dirty), you need
      #    to save those notes before putting the book away.
      def allocate(tag:, data:, cycle:)
        # Step 1: Look for an invalid (empty) way
        @lines.each do |line|
          unless line.valid
            line.fill(tag: tag, data: data, cycle: cycle)
            return nil # no eviction needed
          end
        end

        # Step 2: All ways full -- evict the LRU line
        lru_index = find_lru
        victim = @lines[lru_index]

        # Step 3: Check if the victim is dirty (needs writeback)
        evicted = nil
        if victim.dirty
          # Create a copy of the evicted line for writeback
          evicted = CacheLine.new(line_size: victim.data.length)
          evicted.valid = true
          evicted.dirty = true
          evicted.tag = victim.tag
          evicted.data = victim.data.dup
          evicted.last_access = victim.last_access
        end

        # Step 4: Overwrite the victim with new data
        victim.fill(tag: tag, data: data, cycle: cycle)

        evicted
      end

      private

      # -- LRU Selection ---------------------------------------------------

      # Find the least recently used way index.
      #
      # LRU replacement is simple: each line records its last access
      # time (cycle count). The line with the smallest timestamp is
      # the one that hasn't been touched for the longest time.
      #
      # Special case: invalid lines are always preferred over valid ones
      # (an empty slot is "older" than any real data).
      #
      # @return [Integer] Index of the LRU way in @lines.
      def find_lru
        best_index = 0
        best_time = Float::INFINITY
        @lines.each_with_index do |line, i|
          # Invalid lines are always the best candidates
          return i unless line.valid

          if line.last_access < best_time
            best_time = line.last_access
            best_index = i
          end
        end
        best_index
      end
    end
  end
end
