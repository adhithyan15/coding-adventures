# D01 — Cache Hierarchy

## Overview

The cache package simulates CPU caches — small, fast memory units that sit
between the processor and main memory (DRAM). Caches exist because of the
**memory wall**: modern CPUs can execute instructions in fractions of a
nanosecond, but fetching data from DRAM takes 50-100 nanoseconds. Without
caches, the CPU would spend most of its time waiting for memory.

A cache exploits two properties of real programs:

- **Temporal locality**: if you accessed address X, you will likely access X
  again soon (think: loop counter, frequently-used variable).
- **Spatial locality**: if you accessed address X, you will likely access X+1,
  X+2, etc. soon (think: iterating through an array).

By keeping recently-accessed data close to the CPU and fetching data in chunks
(cache lines), caches reduce average memory access time from ~100 cycles to
~1-3 cycles.

## Layer Position

```
Core (D05)
├── L1 Instruction Cache ← this package, configured for instructions
├── L1 Data Cache        ← this package, configured for data
├── L2 Unified Cache     ← this package, larger + slower
│
Shared L3 Cache          ← this package, shared across cores
│
Memory Controller → DRAM (100+ cycles)
```

**Depends on:** `clock` (cache accesses are cycle-driven)
**Used by:** `pipeline` (D04), `core` (D05)

## Key Concepts

### The Memory Hierarchy

Why not just make all memory fast? Because fast memory is expensive and
physically large (more transistors per bit). The solution: a hierarchy where
each level is larger but slower:

```
                    ┌─────────┐
                    │ Register│  ~0 cycles, ~1 KB
                    │  File   │  (inside the CPU)
                    └────┬────┘
                         │
                    ┌────┴────┐
                    │ L1 Cache│  1-3 cycles, 32-64 KB
                    │ (split) │  (per core, I$ + D$)
                    └────┬────┘
                         │
                    ┌────┴────┐
                    │ L2 Cache│  10-15 cycles, 256 KB - 1 MB
                    │(unified)│  (per core)
                    └────┬────┘
                         │
                    ┌────┴────┐
                    │ L3 Cache│  30-50 cycles, 4-64 MB
                    │(shared) │  (shared across all cores)
                    └────┬────┘
                         │
                    ┌────┴────┐
                    │  DRAM   │  100+ cycles, 8-128 GB
                    │         │  (main memory)
                    └─────────┘

Speed:  ████████████████░░░░░░░░  (fast at top, slow at bottom)
Size:   ░░░░░░████████████████░░  (small at top, large at bottom)
Cost:   ████████████░░░░░░░░░░░░  (expensive at top, cheap at bottom)
```

### Cache Line Structure

A cache does not store individual bytes. It stores **cache lines** — fixed-size
blocks of contiguous memory (typically 64 bytes). When the CPU requests one
byte, the cache fetches the entire 64-byte block, betting that nearby bytes
will be needed soon (spatial locality).

Each cache line has metadata:

```
┌───────┬───────┬──────────┬──────────────────────────────────┐
│ Valid  │ Dirty │   Tag    │          Data (64 bytes)          │
│ (1b)  │ (1b)  │ (N bits) │                                   │
└───────┴───────┴──────────┴──────────────────────────────────┘

Valid bit:  Is this line currently holding real data? (1 = yes, 0 = empty)
Dirty bit:  Has this line been modified but not yet written to memory?
            (only used in write-back caches)
Tag:        The high-order bits of the memory address, used to identify
            which memory block is stored in this line.
Data:       The actual cached bytes — one full cache line.
```

### Address Decomposition

When the CPU accesses memory address A, the cache splits A into three fields
to locate the data:

```
Memory Address (e.g., 32 bits)
┌──────────────┬──────────────┬──────────────┐
│     Tag      │  Set Index   │ Block Offset │
│  (remaining) │ (log2 sets)  │ (log2 line)  │
└──────────────┴──────────────┴──────────────┘

Block Offset:  Which byte within the cache line? (6 bits for 64-byte lines)
Set Index:     Which set in the cache? (depends on number of sets)
Tag:           Which memory block maps to this set? (remaining high bits)
```

**Concrete example:** 32-bit address, 4KB cache, 64-byte lines, 2-way
set-associative:

```
Cache has: 4096 / 64 = 64 lines, organized as 64 / 2 = 32 sets

Address bits:  [31 .............. 11] [10 .. 6] [5 .. 0]
                      Tag (21 bits)    Set (5b)  Offset (6b)

To find data for address 0x0000_1A3C:
  Binary: 0000 0000 0000 0000 0001 1010 0011 1100
  Offset: 111100 = 60 → byte 60 within the cache line
  Set:    01000  = 8  → look in set 8
  Tag:    0000 0000 0000 0000 000 11 = 0x03

  → Go to set 8, check both ways: does either have tag 0x03 and valid=1?
    Yes → cache HIT (return the byte at offset 60)
    No  → cache MISS (fetch from next level, install in set 8)
```

### Cache Organization: Direct-Mapped vs Set-Associative

**Direct-mapped** (1-way): each memory address maps to exactly one cache line.
Simple, fast, but suffers from **conflict misses** — two addresses that map to
the same line keep evicting each other.

```
Direct-mapped (1-way set-associative):

Set 0: [ Line ]
Set 1: [ Line ]
Set 2: [ Line ]        Address A and B both map to Set 2 →
Set 3: [ Line ]        they fight for the same single line!
...
```

**N-way set-associative**: each set holds N lines. An address can go in any of
the N ways within its set. More ways = fewer conflict misses, but slower lookup
(must check N tags in parallel).

```
2-way set-associative:

Set 0: [ Line | Line ]
Set 1: [ Line | Line ]
Set 2: [ Line | Line ]     Address A and B both map to Set 2 →
Set 3: [ Line | Line ]     they can coexist (one in each way)!
...
```

**Fully associative** (N-way where N = total lines): any block can go anywhere.
No conflict misses, but expensive to search (must check every tag). Used only
for tiny caches like TLBs.

```
Real-world associativity:

L1 cache:  4-way or 8-way  (fast access, moderate conflict avoidance)
L2 cache:  8-way or 16-way (larger, more ways to reduce misses)
L3 cache:  12-way to 16-way (huge, needs high associativity)
```

### Replacement Policy: LRU

When a cache set is full and a new line needs to be installed, which existing
line do we evict? **Least Recently Used (LRU)** evicts the line that has not
been accessed for the longest time — betting that it will not be needed again
soon.

```
2-way set, LRU tracking:

Time 1: Access A → Set: [A(MRU), empty]
Time 2: Access B → Set: [B(MRU), A(LRU)]
Time 3: Access A → Set: [A(MRU), B(LRU)]   (A promoted to MRU)
Time 4: Access C → Set: [C(MRU), A(LRU)]   (B evicted — it was LRU)
```

For 2-way, LRU needs just 1 bit per set. For N-way, true LRU needs log2(N!)
bits, which gets expensive. Real CPUs use approximations (pseudo-LRU, RRIP).
We implement true LRU for correctness, with pseudo-LRU as a future extension.

### Write Policies

When the CPU writes to a cached address, there are two strategies:

**Write-back** (default, used by most real CPUs):
- Write only to the cache line, set dirty bit = 1
- Write to memory only when the dirty line is evicted
- Advantage: fewer memory writes, higher performance
- Disadvantage: memory is stale until eviction

**Write-through**:
- Write to both cache and memory simultaneously
- Advantage: memory is always up to date
- Disadvantage: every write goes to memory (slow)

**Write-allocate** (on a write miss):
- Fetch the line into cache, then write to it
- Almost always paired with write-back

**No-write-allocate** (on a write miss):
- Write directly to memory, do not bring line into cache
- Sometimes paired with write-through

```
Our default: write-back + write-allocate (matches most real CPUs)
Alternative: write-through + no-write-allocate (simpler, educational)
```

### Split L1: Instruction Cache vs Data Cache

Most CPUs split the L1 cache into two separate caches:

```
                    ┌─────────────┐
                    │  CPU Core   │
                    ├──────┬──────┤
                    │ L1I  │ L1D  │
                    │(inst)│(data)│
                    └──┬───┴──┬───┘
                       │      │
                    ┌──┴──────┴──┐
                    │  L2 (unified) │
                    └───────────────┘
```

Why split? Instructions and data have different access patterns:
- **L1I** (instruction cache): read-only during execution, sequential access
  (instructions are fetched in order, mostly)
- **L1D** (data cache): read + write, more random access patterns

Splitting eliminates structural hazards — the pipeline can fetch an instruction
and access data memory in the same cycle without conflict.

## Public API

```python
from enum import Enum
from dataclasses import dataclass
from typing import Optional

class WritePolicy(Enum):
    WRITE_BACK = "write_back"
    WRITE_THROUGH = "write_through"

class ReplacementPolicy(Enum):
    LRU = "lru"
    # Future: PSEUDO_LRU, RRIP, RANDOM

@dataclass(frozen=True)
class CacheConfig:
    """Configuration for a single cache level."""
    size_bytes: int            # Total cache size (e.g., 32768 for 32KB)
    line_size: int = 64        # Cache line size in bytes
    associativity: int = 4     # Number of ways (1 = direct-mapped)
    access_latency: int = 1    # Cycles to access this cache on a hit
    write_policy: WritePolicy = WritePolicy.WRITE_BACK
    replacement_policy: ReplacementPolicy = ReplacementPolicy.LRU

@dataclass
class CacheStats:
    """Statistics tracked by the cache."""
    hits: int = 0
    misses: int = 0
    evictions: int = 0
    writebacks: int = 0        # Dirty lines written back to next level

    @property
    def total_accesses(self) -> int:
        return self.hits + self.misses

    @property
    def hit_rate(self) -> float:
        if self.total_accesses == 0:
            return 0.0
        return self.hits / self.total_accesses

    @property
    def miss_rate(self) -> float:
        return 1.0 - self.hit_rate

@dataclass
class CacheAccessResult:
    """Result of a cache access."""
    hit: bool                  # True if data was in cache
    data: bytes                # The requested data
    cycles: int                # Total cycles for this access (including miss penalty)

class Cache:
    """A single cache level — configurable size, associativity, and policy."""

    def __init__(self, config: CacheConfig, next_level: Optional['Cache'] = None) -> None:
        """
        Create a cache.

        Args:
            config: Cache configuration (size, associativity, etc.)
            next_level: The next cache level (L2 for L1, L3 for L2, None for last level)
        """
        ...

    def read(self, address: int, num_bytes: int = 1) -> CacheAccessResult:
        """
        Read data from the cache.

        On hit: return data, cost = access_latency cycles.
        On miss: fetch from next_level (or memory), install in cache,
                 return data, cost = access_latency + next_level cost.
        """
        ...

    def write(self, address: int, data: bytes) -> CacheAccessResult:
        """
        Write data to the cache.

        Write-back: write to cache line, set dirty bit.
        Write-through: write to cache line AND next level.
        On write miss with write-allocate: fetch line first, then write.
        """
        ...

    def flush(self) -> int:
        """
        Write all dirty lines back to next level.
        Returns number of cycles consumed.
        """
        ...

    def invalidate(self) -> None:
        """Mark all lines as invalid (clear the cache)."""
        ...

    @property
    def stats(self) -> CacheStats:
        """Return current cache statistics."""
        ...

    def tick(self) -> None:
        """
        Advance one clock cycle.
        Used for multi-cycle cache accesses (the cache may be busy
        servicing a miss and cannot accept new requests).
        """
        ...


class CacheHierarchy:
    """
    A complete cache hierarchy (L1I + L1D + L2 + optional L3).

    This is the main entry point for the pipeline and core packages.
    """

    def __init__(
        self,
        l1i_config: CacheConfig,
        l1d_config: CacheConfig,
        l2_config: CacheConfig,
        l3_config: Optional[CacheConfig] = None,
        memory_latency: int = 100,
    ) -> None:
        ...

    def fetch_instruction(self, address: int, num_bytes: int = 4) -> CacheAccessResult:
        """Fetch an instruction through L1I → L2 → L3 → memory."""
        ...

    def read_data(self, address: int, num_bytes: int = 4) -> CacheAccessResult:
        """Read data through L1D → L2 → L3 → memory."""
        ...

    def write_data(self, address: int, data: bytes) -> CacheAccessResult:
        """Write data through L1D → L2 → L3 → memory."""
        ...

    @property
    def stats(self) -> dict[str, CacheStats]:
        """Return stats for each level: {'l1i': ..., 'l1d': ..., 'l2': ..., 'l3': ...}."""
        ...
```

## Data Structures

### Internal Cache Line Representation

```python
@dataclass
class CacheLine:
    valid: bool = False
    dirty: bool = False
    tag: int = 0
    data: bytearray = field(default_factory=lambda: bytearray(64))
    last_access_time: int = 0  # For LRU tracking

@dataclass
class CacheSet:
    ways: list[CacheLine]      # One CacheLine per way in this set
```

### Predefined Configurations

```python
# Typical L1 instruction cache (ARM Cortex-A78)
L1I_DEFAULT = CacheConfig(
    size_bytes=64 * 1024,      # 64 KB
    line_size=64,
    associativity=4,
    access_latency=1,
)

# Typical L1 data cache
L1D_DEFAULT = CacheConfig(
    size_bytes=64 * 1024,      # 64 KB
    line_size=64,
    associativity=4,
    access_latency=1,
)

# Typical L2 unified cache
L2_DEFAULT = CacheConfig(
    size_bytes=256 * 1024,     # 256 KB
    line_size=64,
    associativity=8,
    access_latency=12,
)

# Typical L3 shared cache
L3_DEFAULT = CacheConfig(
    size_bytes=8 * 1024 * 1024,  # 8 MB
    line_size=64,
    associativity=16,
    access_latency=40,
)

# Simple teaching cache
L1_SIMPLE = CacheConfig(
    size_bytes=4 * 1024,       # 4 KB
    line_size=16,
    associativity=1,           # Direct-mapped
    access_latency=1,
)
```

## Test Strategy

### Unit Tests (per Cache instance)

- **Initialization**: verify correct number of sets and ways from config
- **Cold miss**: first access to any address is always a miss
- **Hit after miss**: second access to same address is a hit
- **Spatial locality**: access address X (miss), then X+1 within same line (hit)
- **Eviction**: fill a set beyond capacity, verify LRU line is evicted
- **LRU ordering**: access lines A, B, C, D in a 4-way set; access A again;
  then access E — B should be evicted (not A)
- **Dirty writeback**: write to a line, evict it, verify writeback to next level
- **Write-through**: write to a line, verify immediate write to next level
- **Flush**: write to multiple lines, flush, verify all dirty lines written back
- **Invalidate**: invalidate cache, verify all subsequent accesses are misses
- **Address decomposition**: verify tag/set/offset calculation for known addresses
- **Statistics**: verify hit count, miss count, hit rate calculations

### Integration Tests (CacheHierarchy)

- **L1 hit**: data in L1, verify 1-cycle access
- **L1 miss, L2 hit**: data not in L1 but in L2, verify L1 latency + L2 latency
- **L1 miss, L2 miss, L3 hit**: verify cumulative latency
- **Full miss**: data not in any cache, verify memory latency applied
- **Inclusion**: verify data fetched into L1 also exists in L2
- **Split L1**: verify instruction fetch goes through L1I, data access through L1D

### Configuration Tests

- **Direct-mapped**: associativity=1, verify conflict misses
- **Fully associative**: associativity=total_lines, verify no conflict misses
- **Various sizes**: 1KB, 4KB, 32KB, 256KB — verify correct set/way counts
- **Various line sizes**: 16, 32, 64, 128 bytes — verify offset bits

### Cycle-Accuracy Tests

- **Multi-cycle access**: verify cache returns data after correct number of ticks
- **Pipelining**: multiple outstanding requests (future)

## Future Extensions

- **Pseudo-LRU replacement**: approximate LRU for high-associativity caches
- **RRIP replacement**: Re-Reference Interval Prediction (used in Intel CPUs)
- **Prefetching**: predict future accesses and fetch speculatively
  - Sequential prefetcher: detect stride, fetch next N lines
  - Stride prefetcher: detect non-unit strides (e.g., matrix column access)
- **Non-blocking cache**: allow hits while a miss is being serviced (MSHRs)
- **Coherence protocols**: MESI/MOESI for multi-core cache coherence
- **Victim cache**: small fully-associative cache for recently-evicted lines
- **TLB simulation**: translation lookaside buffer for virtual-to-physical address mapping
- **Cache partitioning**: way partitioning for QoS in multi-core systems
