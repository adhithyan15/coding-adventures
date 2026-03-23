# cache

Configurable CPU cache hierarchy simulator. The same `Cache` class serves as L1, L2, or L3 by adjusting size, associativity, and latency parameters.

## Where it fits in the stack

This package sits between the CPU pipeline and main memory. When the CPU reads or writes data, the request passes through the cache hierarchy first. On a hit, data is served quickly; on a miss, it falls through to slower levels.

```
CPU Pipeline -> L1I/L1D Cache -> L2 Cache -> L3 Cache -> Main Memory
```

## Key concepts

- **Cache line**: The smallest unit of data transfer (typically 64 bytes)
- **Set-associative**: Each address maps to a set; within a set, any way can hold it
- **LRU replacement**: When a set is full, the least recently used line is evicted
- **Write-back**: Writes go only to cache; dirty data is written back on eviction
- **Inclusive hierarchy**: When L3 serves data, L2 and L1 are also filled

## Usage

```typescript
import { Cache, CacheConfig, CacheHierarchy } from "@coding-adventures/cache";

// Configure individual cache levels
const l1d = new Cache(new CacheConfig("L1D", 64 * 1024, 64, 4, 1));
const l2 = new Cache(new CacheConfig("L2", 256 * 1024, 64, 8, 10));
const l3 = new Cache(new CacheConfig("L3", 8 * 1024 * 1024, 64, 16, 30));

// Wire them together
const hierarchy = new CacheHierarchy({ l1d, l2, l3, mainMemoryLatency: 100 });

// Read through the hierarchy
const result = hierarchy.read(0x1000, false, 0);
console.log(`Served by: ${result.servedBy}, Cycles: ${result.totalCycles}`);

// Check L1D hit rate
console.log(`L1D hit rate: ${(l1d.stats.hitRate * 100).toFixed(1)}%`);
```

## Real-world configurations

```typescript
// ARM Cortex-A78
const l1d = new Cache(new CacheConfig("L1D", 64 * 1024, 64, 4, 1));
const l2 = new Cache(new CacheConfig("L2", 256 * 1024, 64, 8, 10));

// Apple M4
const l1i = new Cache(new CacheConfig("L1I", 192 * 1024, 64, 6, 1));
const l1d_m4 = new Cache(new CacheConfig("L1D", 128 * 1024, 64, 8, 1));
const l2_m4 = new Cache(new CacheConfig("L2", 16 * 1024 * 1024, 64, 16, 10));
```

## Development

```bash
npm ci
npx vitest run --coverage
```
