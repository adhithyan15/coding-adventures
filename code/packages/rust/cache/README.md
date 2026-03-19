# cache

Configurable CPU cache hierarchy simulator in Rust.

## What it does

Simulates a multi-level cache hierarchy (L1I, L1D, L2, L3, main memory) with configurable parameters. The same `Cache` struct models any level -- L1, L2, or L3 -- by adjusting size, associativity, line size, and latency.

## How it fits in the stack

This is the Rust port of the Python `cache` package. It sits at layer 8 of the accelerator stack, above the arithmetic units and below the CPU pipeline. The cache hierarchy connects the CPU core to main memory, providing fast access to recently used data.

## Key types

- `CacheConfig` -- configuration knobs (size, associativity, latency, write policy)
- `CacheLine` -- one slot in the cache (valid/dirty bits, tag, data, LRU timestamp)
- `CacheSet` -- a group of lines with LRU replacement
- `Cache` -- a single cache level with address decomposition and statistics
- `CacheHierarchy` -- L1I + L1D + L2 + L3 + main memory composition
- `CacheStats` -- hit rate, miss rate, eviction tracking

## Usage

```rust
use cache::{Cache, CacheConfig, CacheHierarchy};

// Configure a 64KB L1D and 256KB L2
let l1d = Cache::new(CacheConfig::new("L1D", 65536, 64, 4, 1));
let l2 = Cache::new(CacheConfig::new("L2", 262144, 64, 8, 10));
let mut hierarchy = CacheHierarchy::new(None, Some(l1d), Some(l2), None, 100);

// Read an address -- first access misses through to memory
let result = hierarchy.read(0x1000, false, 0);
assert_eq!(result.served_by, "memory");

// Second access hits in L1D
let result = hierarchy.read(0x1000, false, 1);
assert_eq!(result.served_by, "L1D");
```

## Running tests

```bash
cargo test -p cache
```
