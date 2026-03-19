# cache

Configurable CPU cache hierarchy simulator. The same `Cache` class serves as L1, L2, or L3 by adjusting size, associativity, and latency parameters.

## Where it fits in the stack

This package sits between the CPU pipeline and main memory. When the CPU reads or writes data, the request passes through the cache hierarchy first. On a hit, data is served quickly; on a miss, it falls through to slower levels.

```
CPU Pipeline → L1I/L1D Cache → L2 Cache → L3 Cache → Main Memory
```

## Key concepts

- **Cache line**: The smallest unit of data transfer (typically 64 bytes)
- **Set-associative**: Each address maps to a set; within a set, any way can hold it
- **LRU replacement**: When a set is full, the least recently used line is evicted
- **Write-back**: Writes go only to cache; dirty data is written back on eviction
- **Inclusive hierarchy**: When L3 serves data, L2 and L1 are also filled

## Usage

```python
from cache import Cache, CacheConfig, CacheHierarchy

# Configure individual cache levels
l1d = Cache(CacheConfig(name="L1D", total_size=64*1024, line_size=64, associativity=4, access_latency=1))
l2 = Cache(CacheConfig(name="L2", total_size=256*1024, line_size=64, associativity=8, access_latency=10))
l3 = Cache(CacheConfig(name="L3", total_size=8*1024*1024, line_size=64, associativity=16, access_latency=30))

# Wire them together
hierarchy = CacheHierarchy(l1d=l1d, l2=l2, l3=l3, main_memory_latency=100)

# Read through the hierarchy
result = hierarchy.read(0x1000, cycle=0)
print(f"Served by: {result.served_by}, Cycles: {result.total_cycles}")

# Check L1D hit rate
print(f"L1D hit rate: {l1d.stats.hit_rate:.1%}")
```

## Real-world configurations

```python
# ARM Cortex-A78
l1d = Cache(CacheConfig("L1D", 64*1024, 64, 4, 1))
l2  = Cache(CacheConfig("L2",  256*1024, 64, 8, 10))

# Apple M4
l1i = Cache(CacheConfig("L1I", 192*1024, 64, 6, 1))
l1d = Cache(CacheConfig("L1D", 128*1024, 64, 8, 1))
l2  = Cache(CacheConfig("L2",  16*1024*1024, 64, 16, 10))
```

## Development

```bash
uv venv --clear --quiet
uv pip install -e ".[dev]" --quiet
.venv/bin/python -m pytest tests/ -v --cov=cache --cov-report=term-missing
```
