# frozen_string_literal: true

# Cache -- a single configurable level of the cache hierarchy.
#
# This module implements the core cache logic. The same class is used for
# L1, L2, and L3 -- the only difference is the configuration (size,
# associativity, latency). This reflects real hardware: an L1 and an L3
# use the same SRAM cell design, just at different scales.
#
# ## Address Decomposition
#
# When the CPU accesses memory address 0x1A2B3C4D, the cache must figure
# out three things:
#
# 1. **Offset** (lowest bits): Which byte *within* the cache line?
#    - For 64-byte lines: 6 bits (2^6 = 64)
#
# 2. **Set Index** (middle bits): Which set should we look in?
#    - For 256 sets: 8 bits (2^8 = 256)
#
# 3. **Tag** (highest bits): Which memory block is this?
#    - All remaining bits above offset + set_index
#
# Visual for a 64KB, 4-way, 64B-line cache (256 sets):
#
#     Address: | tag (18 bits) | set index (8 bits) | offset (6 bits) |
#              |  31 ... 14    |     13 ... 6       |    5 ... 0      |
#
# This bit-slicing is why cache sizes must be powers of 2 -- it lets the
# hardware extract fields with simple bit masks instead of division.
#
# ## Read Path
#
#     CPU reads address 0x1000
#          |
#          v
#     Decompose: tag=0x4, set=0, offset=0
#          |
#          v
#     Look in Set 0: compare tag 0x4 against all ways
#          |
#     +----+----+
#     |         |
#     HIT      MISS
#     |         |
#     Return   Go to next level (L2/L3/memory)
#     data     Bring data back, allocate in this cache
#              Maybe evict an old line (LRU)

require_relative "cache_set"
require_relative "stats"

module CodingAdventures
  module Cache
    # Record of a single cache access -- for debugging and performance analysis.
    #
    # Every read() or write() call returns one of these, telling you exactly
    # what happened: was it a hit? Which set? Was anything evicted? How many
    # cycles did it cost?
    #
    # This is like a receipt for each memory transaction.
    CacheAccess = Data.define(
      :address,    # The full memory address that was accessed.
      :hit,        # True if the data was found in the cache.
      :tag,        # The tag bits extracted from the address.
      :set_index,  # The set index bits.
      :offset,     # The offset bits -- byte position within the cache line.
      :cycles,     # Clock cycles this access took.
      :evicted     # Evicted dirty CacheLine or nil.
    ) do
      def initialize(address:, hit:, tag:, set_index:, offset:, cycles:, evicted: nil)
        super
      end
    end

    # A single level of cache -- configurable to be L1, L2, or L3.
    #
    # This is the workhorse of the cache simulator. Give it a CacheConfig
    # and it handles address decomposition, set lookup, LRU replacement,
    # and statistics tracking.
    #
    # Example:
    #   config = CacheConfig.new(name: "L1D", total_size: 1024, line_size: 64,
    #                            associativity: 4, access_latency: 1)
    #   cache = CodingAdventures::Cache::CacheSimulator.new(config)
    #   access = cache.read(address: 0x100, cycle: 0)
    #   access.hit  # => false
    #   access = cache.read(address: 0x100, cycle: 1)
    #   access.hit  # => true
    class CacheSimulator
      attr_reader :config, :stats, :sets

      # Initialize the cache with the given configuration.
      #
      # Creates all sets, precomputes bit positions for address
      # decomposition, and initializes statistics.
      #
      # @param config [CacheConfig] Cache parameters (size, associativity, etc.)
      def initialize(config)
        @config = config
        @stats = CacheStats.new

        # Create the set array
        num_sets = config.num_sets
        @sets = Array.new(num_sets) do
          CacheSet.new(associativity: config.associativity, line_size: config.line_size)
        end

        # Precompute bit positions for address decomposition.
        #
        #   offset_bits = log2(line_size)    e.g., log2(64) = 6
        #   set_bits    = log2(num_sets)     e.g., log2(256) = 8
        @offset_bits = Math.log2(config.line_size).to_i
        @set_bits = num_sets > 1 ? Math.log2(num_sets).to_i : 0
        @set_mask = num_sets - 1 # e.g., 0xFF for 256 sets
      end

      # -- Address Decomposition -------------------------------------------

      # Split a memory address into [tag, set_index, offset].
      #
      # This is pure bit manipulation -- no division needed because all
      # sizes are powers of 2.
      #
      # @param address [Integer] Full memory address (unsigned integer).
      # @return [Array(Integer, Integer, Integer)] [tag, set_index, offset]
      def decompose_address(address)
        offset = address & ((1 << @offset_bits) - 1)
        set_index = (address >> @offset_bits) & @set_mask
        tag = address >> (@offset_bits + @set_bits)
        [tag, set_index, offset]
      end

      # -- Read ------------------------------------------------------------

      # Read data from the cache.
      #
      # On a hit, the data is returned immediately with the cache's
      # access latency. On a miss, dummy data is allocated (the caller
      # -- typically the hierarchy -- is responsible for actually fetching
      # from the next level).
      #
      # @param address [Integer] Memory address to read.
      # @param size [Integer] Number of bytes to read (for stats).
      # @param cycle [Integer] Current clock cycle.
      # @return [CacheAccess] Record describing what happened.
      def read(address:, size: 1, cycle: 0)
        tag, set_index, offset = decompose_address(address)
        cache_set = @sets[set_index]

        hit, line = cache_set.access(tag, cycle)

        if hit
          @stats.record_read(hit: true)
          return CacheAccess.new(
            address: address, hit: true, tag: tag,
            set_index: set_index, offset: offset,
            cycles: @config.access_latency
          )
        end

        # Miss -- allocate the line with dummy data.
        @stats.record_read(hit: false)
        evicted = cache_set.allocate(
          tag: tag,
          data: Array.new(@config.line_size, 0),
          cycle: cycle
        )
        if evicted
          @stats.record_eviction(dirty: true)
        elsif all_ways_valid?(cache_set)
          @stats.record_eviction(dirty: false)
        end

        CacheAccess.new(
          address: address, hit: false, tag: tag,
          set_index: set_index, offset: offset,
          cycles: @config.access_latency, evicted: evicted
        )
      end

      # -- Write -----------------------------------------------------------

      # Write data to the cache.
      #
      # **Write-back policy**: Write only to the cache. Mark the line
      # as dirty. The data is written to the next level only when the
      # line is evicted.
      #
      # **Write-through policy**: Write to both the cache and the next
      # level simultaneously. The line is never dirty.
      #
      # On a write miss, we use **write-allocate**: first bring the
      # line into the cache (like a read miss), then perform the write.
      #
      # @param address [Integer] Memory address to write.
      # @param data [Array<Integer>, nil] Bytes to write.
      # @param cycle [Integer] Current clock cycle.
      # @return [CacheAccess] Record describing what happened.
      def write(address:, data: nil, cycle: 0)
        tag, set_index, offset = decompose_address(address)
        cache_set = @sets[set_index]

        hit, line = cache_set.access(tag, cycle)

        if hit
          @stats.record_write(hit: true)
          # Write the data into the line
          if data
            data.each_with_index do |byte, i|
              line.data[offset + i] = byte if (offset + i) < line.data.length
            end
          end
          # Mark dirty for write-back; write-through stays clean
          line.dirty = true if @config.write_policy == "write-back"
          return CacheAccess.new(
            address: address, hit: true, tag: tag,
            set_index: set_index, offset: offset,
            cycles: @config.access_latency
          )
        end

        # Write miss -- allocate (write-allocate policy), then write
        @stats.record_write(hit: false)
        fill_data = Array.new(@config.line_size, 0)
        if data
          data.each_with_index do |byte, i|
            fill_data[offset + i] = byte if (offset + i) < fill_data.length
          end
        end

        evicted = cache_set.allocate(tag: tag, data: fill_data, cycle: cycle)
        if evicted
          @stats.record_eviction(dirty: true)
        elsif all_ways_valid?(cache_set)
          @stats.record_eviction(dirty: false)
        end

        # For write-back, mark the newly allocated line as dirty
        new_hit, new_line = cache_set.access(tag, cycle)
        new_line.dirty = true if new_hit && @config.write_policy == "write-back"

        CacheAccess.new(
          address: address, hit: false, tag: tag,
          set_index: set_index, offset: offset,
          cycles: @config.access_latency, evicted: evicted
        )
      end

      # -- Helpers ---------------------------------------------------------

      # Invalidate all lines in the cache (cache flush).
      #
      # This is equivalent to a cold start -- after invalidation, every
      # access will be a compulsory miss.
      def invalidate
        @sets.each do |cache_set|
          cache_set.lines.each(&:invalidate)
        end
      end

      # Directly fill a cache line with data (used by hierarchy on miss).
      #
      # This bypasses the normal read/write path -- it's used when the
      # hierarchy fetches data from a lower level and wants to install
      # it in this cache.
      #
      # @param address [Integer] The address whose line we're filling.
      # @param data [Array<Integer>] The full cache line of data.
      # @param cycle [Integer] Current clock cycle.
      # @return [CacheLine, nil] Evicted dirty CacheLine or nil.
      def fill_line(address:, data:, cycle: 0)
        tag, set_index, _offset = decompose_address(address)
        cache_set = @sets[set_index]
        cache_set.allocate(tag: tag, data: data, cycle: cycle)
      end

      def to_s
        "Cache(#{@config.name}: " \
          "#{@config.total_size / 1024}KB, " \
          "#{@config.associativity}-way, " \
          "#{@config.line_size}B lines, " \
          "#{@config.num_sets} sets)"
      end

      alias_method :inspect, :to_s

      private

      # Check if all ways in a set are valid (meaning an eviction occurred).
      def all_ways_valid?(cache_set)
        cache_set.lines.all?(&:valid)
      end
    end
  end
end
