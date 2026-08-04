# cpu-cache (C)

A **configurable CPU cache hierarchy simulator** in pure ISO C17. A faithful
port of the Rust [`cpu-cache`](../../rust/cpu-cache) crate. Simulates a
multi-level cache (L1I / L1D / L2 / L3 / main memory) like those in modern CPUs;
the same `CaCache` serves as any level — only its configuration differs.

## What it models

- **Cache line** — the smallest cached unit: valid/dirty bits, tag, an owned
  data buffer, and an LRU timestamp.
- **Cache set** — N-way set-associative storage with **true LRU** replacement
  (invalid ways preferred; a dirty victim is reported for writeback).
- **Cache** — one configurable level. Decomposes an address into
  `(tag, set_index, offset)` by pure bit-slicing, handles read/write with
  write-allocate + write-back (or write-through), and tracks statistics.
- **Statistics** — reads, writes, hits, misses, evictions, writebacks, plus
  hit/miss rate.
- **Hierarchy** — walks L1→L2→L3→memory, accumulating latency, and refills
  higher levels on a hit (inclusive policy).

Address decomposition uses an exact **integer log2** of the (power-of-two) line
size and set count, so the port needs no `<math.h>`.

## API

```c
#include "cpu_cache.h"

CaCacheConfig l1cfg, l2cfg;
ca_cache_config_new(&l1cfg, "L1D", 1024, 64, 4, 1);
ca_cache_config_new(&l2cfg, "L2", 4096, 64, 8, 10);

CaCache l1, l2;
ca_cache_init(&l1, &l1cfg);
ca_cache_init(&l2, &l2cfg);

CaCacheHierarchy h;
ca_cache_hierarchy_init(&h, NULL, &l1, &l2, NULL, 100); /* takes ownership */
CaHierarchyAccess r = ca_cache_hierarchy_read(&h, 0x1000, 0, 0);
/* r.served_by == "memory" on the first (compulsory) miss */
ca_cache_hierarchy_free(&h);
```

- `ca_cache_config_new` validates the configuration (rejects non-power-of-2
  line size, indivisible totals, etc. — where the Rust panics).
- `ca_cache_read` / `ca_cache_write` return a `CaCacheAccess`; the hierarchy
  functions return a `CaHierarchyAccess`. Both are plain value types.
- `ca_cache_free` / `ca_cache_hierarchy_free` release the owned heap.

Every allocation guards `size_t` overflow (`calloc`'s checked multiply; the
config's `line_size * associativity` product is overflow-checked before use).
Verified clean under ASan + UBSan, the macOS `leaks` tool (0 leaks), and a
random-config/random-access fuzz sweep.

### Divergence from the Rust

The Rust `CacheAccess.evicted` is a full `Option<CacheLine>` clone. Because no
code path ever reads an evicted line's *data* (the hierarchy discards it; only
its dirty/tag matter), this port records the victim's metadata inline
(`has_evicted` / `evicted_dirty` / `evicted_tag` / `evicted_last_access`) rather
than copying its bytes. Every observable behavior is identical.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
