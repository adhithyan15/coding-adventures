# cache

Configurable CPU cache hierarchy simulation for Lua 5.4.

The package models:

- cache lines with valid/dirty/tag metadata
- set-associative lookup with LRU replacement
- configurable cache levels (`L1`, `L2`, `L3`)
- inclusive hierarchies with `L1I`, `L1D`, `L2`, `L3`
- cache performance statistics

## Example

```lua
local cache = require("coding_adventures.cache")

local l1d = cache.Cache.new(cache.CacheConfig.new("L1D", 1024, 64, 4, 1))
local l2 = cache.Cache.new(cache.CacheConfig.new("L2", 4096, 64, 8, 10))
local hierarchy = cache.CacheHierarchy.new({
    l1d = l1d,
    l2 = l2,
    main_memory_latency = 100,
})

local first = hierarchy:read(0x1000, false, 0)
local second = hierarchy:read(0x1000, false, 1)

assert(first.served_by == "memory")
assert(second.served_by == "L1D")
```
