# frozen_string_literal: true

# Cache hierarchy -- multi-level cache system (L1I + L1D + L2 + L3 + memory).
#
# A modern CPU doesn't have just one cache -- it has a **hierarchy** of
# progressively larger and slower caches. This is the memory equivalent
# of keeping frequently used items close to hand:
#
#     +---------+     +--------+     +--------+     +--------+     +--------+
#     |   CPU   | --> |  L1    | --> |   L2   | --> |   L3   | --> |  Main  |
#     |  core   |     | 1 cyc  |     | 10 cyc |     | 30 cyc |     | Memory |
#     |         |     | 64KB   |     | 256KB  |     | 8MB    |     | 100cyc |
#     +---------+     +--------+     +--------+     +--------+     +--------+
#                      per-core       per-core       shared         shared
#
# Analogy:
# - L1 = the books open on your desk (tiny, instant access)
# - L2 = the bookshelf in your office (bigger, a few seconds to grab)
# - L3 = the library downstairs (huge, takes a minute to walk there)
# - Main memory = the warehouse across town (enormous, takes an hour)
#
# When the CPU reads an address:
# 1. Check L1D. Hit? Return data (1 cycle). Miss? Continue.
# 2. Check L2. Hit? Return data (10 cycles), and fill L1D. Miss? Continue.
# 3. Check L3. Hit? Return data (30 cycles), fill L2 and L1D. Miss? Continue.
# 4. Go to main memory (100 cycles). Fill L3, L2, and L1D.

require_relative "cache"

module CodingAdventures
  module Cache
    # Record of an access through the full hierarchy.
    #
    # Tracks which level served the data and the total latency accumulated
    # across all levels that were consulted.
    HierarchyAccess = Data.define(
      :address,         # The memory address that was accessed.
      :served_by,       # Name of the level that had the data ("L1D", "L2", etc.)
      :total_cycles,    # Total clock cycles from start to data delivery.
      :hit_at_level,    # Which hierarchy level served the data (0=L1, 1=L2, etc.)
      :level_accesses   # Detailed access records from each cache level consulted.
    ) do
      def initialize(address:, served_by:, total_cycles:, hit_at_level:, level_accesses: [])
        super
      end
    end

    # Multi-level cache hierarchy -- L1I + L1D + L2 + L3 + main memory.
    #
    # Fully configurable: pass any combination of cache levels. You can
    # simulate anything from a simple L1-only system to a full 3-level
    # hierarchy with separate instruction and data L1 caches.
    #
    # Example:
    #   l1d = CacheSimulator.new(CacheConfig.new(name: "L1D", total_size: 1024,
    #                            line_size: 64, associativity: 4, access_latency: 1))
    #   l2 = CacheSimulator.new(CacheConfig.new(name: "L2", total_size: 4096,
    #                           line_size: 64, associativity: 8, access_latency: 10))
    #   hierarchy = CacheHierarchy.new(l1d: l1d, l2: l2)
    #   result = hierarchy.read(address: 0x1000, cycle: 0)
    #   result.served_by  # => "memory"
    class CacheHierarchy
      attr_reader :l1i, :l1d, :l2, :l3, :main_memory_latency

      # Create a cache hierarchy.
      #
      # @param l1i [CacheSimulator, nil] L1 instruction cache (optional).
      # @param l1d [CacheSimulator, nil] L1 data cache (optional but typical).
      # @param l2 [CacheSimulator, nil] L2 cache (optional).
      # @param l3 [CacheSimulator, nil] L3 cache (optional).
      # @param main_memory_latency [Integer] Clock cycles for main memory access.
      def initialize(l1i: nil, l1d: nil, l2: nil, l3: nil, main_memory_latency: 100)
        @l1i = l1i
        @l1d = l1d
        @l2 = l2
        @l3 = l3
        @main_memory_latency = main_memory_latency

        # Build ordered list of [name, cache] for iteration.
        # The hierarchy is walked top-down (fastest to slowest).
        @data_levels = []
        @data_levels << ["L1D", l1d] if l1d
        @data_levels << ["L2", l2] if l2
        @data_levels << ["L3", l3] if l3

        @instr_levels = []
        @instr_levels << ["L1I", l1i] if l1i
        @instr_levels << ["L2", l2] if l2
        @instr_levels << ["L3", l3] if l3
      end

      # -- Read ------------------------------------------------------------

      # Read through the hierarchy. Returns which level served the data.
      #
      # Walks the hierarchy top-down. At each level:
      # - If hit: stop, fill all higher levels, return.
      # - If miss: accumulate latency, continue to next level.
      # - If all miss: data comes from main memory.
      #
      # The **inclusive** fill policy is used: when L3 serves data, it
      # also fills L2 and L1D so subsequent accesses hit at L1.
      #
      # @param address [Integer] Memory address to read.
      # @param is_instruction [Boolean] If true, use L1I instead of L1D.
      # @param cycle [Integer] Current clock cycle.
      # @return [HierarchyAccess]
      def read(address:, is_instruction: false, cycle: 0)
        levels = is_instruction ? @instr_levels : @data_levels

        if levels.empty?
          return HierarchyAccess.new(
            address: address,
            served_by: "memory",
            total_cycles: @main_memory_latency,
            hit_at_level: levels.length,
            level_accesses: []
          )
        end

        total_cycles = 0
        accesses = []
        served_by = "memory"
        hit_level = levels.length

        # Walk the hierarchy top-down
        levels.each_with_index do |(name, cache), level_idx|
          access = cache.read(address: address, cycle: cycle)
          total_cycles += cache.config.access_latency
          accesses << access

          if access.hit
            served_by = name
            hit_level = level_idx
            break
          end
        end

        # Complete miss -- add main memory latency
        total_cycles += @main_memory_latency if served_by == "memory"

        # Fill higher levels (inclusive policy).
        # If L3 served, fill L2 and L1. If L2 served, fill L1.
        dummy_data = Array.new(get_line_size(levels), 0)
        (hit_level - 1).downto(0) do |fill_idx|
          _fill_name, fill_cache = levels[fill_idx]
          fill_cache.fill_line(address: address, data: dummy_data, cycle: cycle)
        end

        HierarchyAccess.new(
          address: address,
          served_by: served_by,
          total_cycles: total_cycles,
          hit_at_level: hit_level,
          level_accesses: accesses
        )
      end

      # -- Write -----------------------------------------------------------

      # Write through the hierarchy.
      #
      # With write-allocate + write-back (the most common policy):
      # 1. If L1D hit: write to L1D, mark dirty. Done.
      # 2. If L1D miss: allocate in L1D, walk down to find data, fill back up.
      #
      # @param address [Integer] Memory address to write.
      # @param data [Array<Integer>, nil] Bytes to write.
      # @param cycle [Integer] Current clock cycle.
      # @return [HierarchyAccess]
      def write(address:, data: nil, cycle: 0)
        levels = @data_levels

        if levels.empty?
          return HierarchyAccess.new(
            address: address,
            served_by: "memory",
            total_cycles: @main_memory_latency,
            hit_at_level: 0,
            level_accesses: []
          )
        end

        # Check L1D first (writes always go to the data cache)
        first_name, first_cache = levels[0]
        access = first_cache.write(address: address, data: data, cycle: cycle)

        if access.hit
          return HierarchyAccess.new(
            address: address,
            served_by: first_name,
            total_cycles: first_cache.config.access_latency,
            hit_at_level: 0,
            level_accesses: [access]
          )
        end

        # Write miss at L1 -- walk lower levels to find the data
        total_cycles = first_cache.config.access_latency
        accesses = [access]
        served_by = "memory"
        hit_level = levels.length

        (1...levels.length).each do |level_idx|
          _name, cache = levels[level_idx]
          level_access = cache.read(address: address, cycle: cycle)
          total_cycles += cache.config.access_latency
          accesses << level_access

          if level_access.hit
            served_by = _name
            hit_level = level_idx
            break
          end
        end

        total_cycles += @main_memory_latency if served_by == "memory"

        HierarchyAccess.new(
          address: address,
          served_by: served_by,
          total_cycles: total_cycles,
          hit_at_level: hit_level,
          level_accesses: accesses
        )
      end

      # -- Helpers ---------------------------------------------------------

      # Invalidate all caches in the hierarchy (full flush).
      def invalidate_all
        [l1i, l1d, l2, l3].compact.each(&:invalidate)
      end

      # Reset statistics for all cache levels.
      def reset_stats
        [l1i, l1d, l2, l3].compact.each { |c| c.stats.reset }
      end

      def to_s
        parts = []
        parts << "L1I=#{l1i.config.total_size / 1024}KB" if l1i
        parts << "L1D=#{l1d.config.total_size / 1024}KB" if l1d
        parts << "L2=#{l2.config.total_size / 1024}KB" if l2
        parts << "L3=#{l3.config.total_size / 1024}KB" if l3
        parts << "mem=#{@main_memory_latency}cyc"
        "CacheHierarchy(#{parts.join(", ")})"
      end

      alias_method :inspect, :to_s

      private

      # Get the line size from the first level in the hierarchy.
      def get_line_size(levels)
        levels.empty? ? 64 : levels[0][1].config.line_size
      end
    end
  end
end
