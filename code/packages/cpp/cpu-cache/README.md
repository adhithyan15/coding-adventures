# cpu-cache (C++)

A **configurable CPU cache hierarchy simulator** — header-only, ISO C++17. A
faithful port of the Rust [`cpu-cache`](../../rust/cpu-cache) crate, in namespace
`ca::cpu_cache`. Simulates a multi-level cache (L1I / L1D / L2 / L3 / main
memory); the same `Cache` serves as any level — only its configuration differs.

## What it models

- **`CacheLine`** — valid/dirty bits, tag, an owned `std::vector<uint8_t>` data
  buffer, and an LRU timestamp.
- **`CacheSet`** — N-way set-associative storage with **true LRU** (invalid ways
  preferred; a dirty victim is returned via `std::optional<CacheLine>`).
- **`Cache`** — one configurable level: power-of-two address decomposition,
  read/write with write-allocate + write-back (or write-through), and stats.
- **`CacheStats`** — reads, writes, hits, misses, evictions, writebacks, and
  hit/miss rate.
- **`CacheHierarchy`** — `std::optional<Cache>` per level; walks L1→L2→L3→memory
  accumulating latency and refills higher levels on a hit (inclusive policy).

Where the Rust `CacheConfig::new` panics on an invalid configuration, this port
throws `std::invalid_argument`. Address `log2` is an exact integer bit-count, so
the header needs no `<cmath>`.

## API

```cpp
#include "cpu_cache.hpp"
namespace cc = ca::cpu_cache;

cc::CacheHierarchy h(std::nullopt,
                     cc::Cache(cc::CacheConfig::create("L1D", 1024, 64, 4, 1)),
                     cc::Cache(cc::CacheConfig::create("L2", 4096, 64, 8, 10)),
                     std::nullopt, 100);
auto r = h.read(0x1000, /*is_instruction=*/false, /*cycle=*/0);
// r.served_by == "memory" on the first (compulsory) miss
```

- `Cache::read` / `write` return a `CacheAccess` (with an
  `std::optional<CacheLine> evicted`); the hierarchy methods return a
  `HierarchyAccess`.
- `CacheConfig::create` validates and throws on bad input; `with_write_policy`
  is the builder. All containers manage their own memory (RAII).

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`. Verified clean under ASan + UBSan.
