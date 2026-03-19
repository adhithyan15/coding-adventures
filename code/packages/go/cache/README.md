# cache

Configurable CPU cache hierarchy simulator in Go.

## Overview

This package simulates multi-level cache hierarchies (L1I/L1D/L2/L3) like those found in modern CPUs. The same `Cache` struct serves as L1, L2, or L3 by configuring size, associativity, and latency differently.

Features:
- **Set-associative caches** with configurable associativity (direct-mapped to fully associative)
- **LRU replacement** policy for eviction decisions
- **Write-back and write-through** write policies
- **Multi-level hierarchy** with inclusive fill policy
- **Harvard architecture** support (separate L1I and L1D)
- **Detailed statistics** tracking (hit rate, miss rate, evictions, writebacks)

## Usage

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/cache"

// Create a single cache level
cfg, _ := cache.NewCacheConfig("L1D", 65536, 64, 4, 1, "write-back")
c := cache.NewCache(cfg)

// Read and write
access := c.Read(0x1000, 0)  // miss
access = c.Read(0x1000, 1)   // hit!

// Build a hierarchy
l1dCfg, _ := cache.NewCacheConfig("L1D", 1024, 64, 4, 1, "write-back")
l2Cfg, _ := cache.NewCacheConfig("L2", 4096, 64, 8, 10, "write-back")
l1d := cache.NewCache(l1dCfg)
l2 := cache.NewCache(l2Cfg)
h := cache.NewCacheHierarchy(nil, l1d, l2, nil, 100)

result := h.Read(0x1000, false, 0)
fmt.Println(result.ServedBy)    // "memory"
fmt.Println(result.TotalCycles) // 111
```

## How It Fits in the Stack

This package is part of the coding-adventures accelerator stack, building on:
- `clock` (system clock for cycle tracking)
- `logic-gates` (fundamental digital logic)

It provides the memory subsystem simulation used by higher-level CPU simulator packages.

## License

MIT
