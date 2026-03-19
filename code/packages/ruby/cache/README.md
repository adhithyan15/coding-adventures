# coding_adventures_cache

Configurable CPU cache hierarchy simulator in Ruby.

## Overview

This gem simulates multi-level cache hierarchies (L1I/L1D/L2/L3) like those found in modern CPUs. The same `CacheSimulator` class serves as L1, L2, or L3 by configuring size, associativity, and latency differently.

Features:
- **Set-associative caches** with configurable associativity (direct-mapped to fully associative)
- **LRU replacement** policy for eviction decisions
- **Write-back and write-through** write policies
- **Multi-level hierarchy** with inclusive fill policy
- **Harvard architecture** support (separate L1I and L1D)
- **Detailed statistics** tracking (hit rate, miss rate, evictions, writebacks)

## Installation

```ruby
gem "coding_adventures_cache"
```

## Usage

```ruby
require "coding_adventures_cache"

# Create a single cache level
config = CodingAdventures::Cache::CacheConfig.new(
  name: "L1D", total_size: 65536, line_size: 64,
  associativity: 4, access_latency: 1
)
cache = CodingAdventures::Cache::CacheSimulator.new(config)

# Read and write
access = cache.read(address: 0x1000, cycle: 0)  # miss
access = cache.read(address: 0x1000, cycle: 1)  # hit!

# Build a hierarchy
l1d = CodingAdventures::Cache::CacheSimulator.new(
  CodingAdventures::Cache::CacheConfig.new(
    name: "L1D", total_size: 1024, line_size: 64,
    associativity: 4, access_latency: 1
  )
)
l2 = CodingAdventures::Cache::CacheSimulator.new(
  CodingAdventures::Cache::CacheConfig.new(
    name: "L2", total_size: 4096, line_size: 64,
    associativity: 8, access_latency: 10
  )
)
hierarchy = CodingAdventures::Cache::CacheHierarchy.new(
  l1d: l1d, l2: l2, main_memory_latency: 100
)
result = hierarchy.read(address: 0x1000, cycle: 0)
puts result.served_by     # => "memory"
puts result.total_cycles  # => 111
```

## How It Fits in the Stack

This package is part of the coding-adventures accelerator stack, building on top of:
- `coding_adventures_clock` (system clock for cycle tracking)
- `coding_adventures_logic_gates` (fundamental digital logic)

It provides the memory subsystem simulation used by higher-level CPU simulator packages.

## License

MIT
