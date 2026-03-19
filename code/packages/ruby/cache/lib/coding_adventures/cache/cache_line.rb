# frozen_string_literal: true

# Cache line -- the smallest unit of data in a cache.
#
# In a real CPU, data is not moved one byte at a time between memory and the
# cache. Instead, it moves in fixed-size chunks called **cache lines** (also
# called cache blocks). A typical cache line is 64 bytes.
#
# Analogy: Think of a warehouse that ships goods in standard containers.
# You can't order a single screw -- you get the whole container (cache line)
# that includes the screw you need plus 63 other bytes of nearby data.
# This works well because of **spatial locality**: if you accessed byte N,
# you'll likely access bytes N+1, N+2, ... soon.
#
# Each cache line stores:
#
#     +-------+-------+-----+------+---------------------------+
#     | valid | dirty | tag | LRU  |     data (64 bytes)       |
#     +-------+-------+-----+------+---------------------------+
#
# - **valid**: Is this line holding real data? After a reset, all lines are
#   invalid (empty boxes). A line becomes valid when data is loaded into it.
#
# - **dirty**: Has the data been modified since it was loaded from memory?
#   In a write-back cache, writes go only to the cache (not memory). The
#   dirty bit tracks whether the line needs to be written back to memory
#   when evicted.
#
# - **tag**: The high bits of the memory address. Since many addresses map
#   to the same cache set, the tag distinguishes WHICH address is stored here.
#
# - **data**: The actual bytes -- an array of integers, each 0-255.
#
# - **last_access**: A timestamp (cycle count) recording when this line was
#   last read or written. Used by the LRU replacement policy to decide
#   which line to evict when the set is full.

module CodingAdventures
  module Cache
    # Immutable snapshot of a cache line, returned by CacheLine#to_data.
    CacheLineData = Data.define(:valid, :dirty, :tag, :last_access, :data)

    # A single cache line -- one slot in the cache.
    #
    # Example:
    #   line = CodingAdventures::Cache::CacheLine.new(line_size: 64)
    #   line.valid  # => false
    #   line.fill(tag: 42, data: [0xAB] * 64, cycle: 100)
    #   line.valid  # => true
    #   line.tag    # => 42
    #   line.last_access # => 100
    class CacheLine
      attr_accessor :valid, :dirty, :tag, :last_access, :data

      # Create a new invalid cache line with the given size.
      #
      # @param line_size [Integer] Number of bytes per cache line. Defaults
      #   to 64, which is standard on modern x86 and ARM CPUs.
      def initialize(line_size: 64)
        @valid = false
        @dirty = false
        @tag = 0
        @last_access = 0
        @data = Array.new(line_size, 0)
      end

      # -- Operations ------------------------------------------------------

      # Load data into this cache line, marking it valid.
      #
      # This is called when a cache miss brings data from a lower level
      # (L2, L3, or main memory) into this line.
      #
      # @param tag [Integer] The tag bits for the address being cached.
      # @param data [Array<Integer>] The bytes to store (must match line_size).
      # @param cycle [Integer] Current clock cycle (for LRU tracking).
      def fill(tag:, data:, cycle:)
        @valid = true
        @dirty = false # freshly loaded data is clean
        @tag = tag
        @data = data.dup # defensive copy
        @last_access = cycle
      end

      # Update the last access time -- called on every hit.
      #
      # This is the heartbeat of LRU: the most recently used line
      # gets the highest timestamp, so it's the *last* to be evicted.
      def touch(cycle)
        @last_access = cycle
      end

      # Mark this line as invalid (empty).
      #
      # Used during cache flushes or coherence protocol invalidations.
      # The data is not zeroed -- it's just marked as not-present.
      def invalidate
        @valid = false
        @dirty = false
      end

      # Number of bytes in this cache line.
      def line_size
        @data.length
      end

      # Return an immutable snapshot.
      def to_data
        CacheLineData.new(
          valid: @valid,
          dirty: @dirty,
          tag: @tag,
          last_access: @last_access,
          data: @data.dup
        )
      end

      def to_s
        state = @valid ? "V" : "-"
        state += @dirty ? "D" : "-"
        "CacheLine(#{state}, tag=0x#{@tag.to_s(16).upcase}, lru=#{@last_access})"
      end

      alias_method :inspect, :to_s
    end
  end
end
