# frozen_string_literal: true

# Entry point for the coding_adventures_cache gem.
#
# This gem simulates a configurable CPU cache hierarchy like those found
# in modern CPUs. The same CacheSimulator class serves as L1, L2, or L3
# by configuring size, associativity, and latency differently.
#
# Modules:
#   CacheLine      - The smallest unit of cached data
#   CacheSet       - Set-associative lookup with LRU replacement
#   CacheConfig    - Configuration (size, associativity, latency, etc.)
#   CacheSimulator - A single configurable cache level
#   CacheHierarchy - L1I/L1D/L2/L3 composition
#   CacheStats     - Hit rate, miss rate, eviction tracking
#
# Usage:
#   require "coding_adventures_cache"
#
#   l1d = CodingAdventures::Cache::CacheSimulator.new(
#     CodingAdventures::Cache::CacheConfig.new(
#       name: "L1D", total_size: 1024, line_size: 64,
#       associativity: 4, access_latency: 1
#     )
#   )

require_relative "coding_adventures/cache/version"
require_relative "coding_adventures/cache/stats"
require_relative "coding_adventures/cache/cache_line"
require_relative "coding_adventures/cache/cache_set"
require_relative "coding_adventures/cache/cache"
require_relative "coding_adventures/cache/hierarchy"
