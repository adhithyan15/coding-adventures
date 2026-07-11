# bloom-filter (C)

A pure ISO **C17** Bloom filter — a compact, probabilistic set for membership
tests. A faithful port of the Rust `bloom-filter` crate (DT22).

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies — not even libm.

## What a Bloom filter does

It answers **"definitely not in the set"** or **"possibly in the set."** It
never produces a false negative (an item you added always tests present) but may
produce a false positive, with a probability that rises as the filter fills. All
it stores is a bit array of `m` bits; each element sets `k` bits picked by `k`
hash functions.

| Function | Purpose |
| --- | --- |
| `bloom_init(bf, n, p)` | size a filter for `n` items at false-positive rate `p` |
| `bloom_init_params(bf, m, k)` | build from explicit bit count `m` and hash count `k` |
| `bloom_free(bf)` | release the bit array |
| `bloom_add(bf, data, len)` | insert an element |
| `bloom_contains(bf, data, len)` | 1 = possibly present, 0 = definitely absent |
| `bloom_optimal_m/k`, `bloom_capacity_for_memory` | sizing math |

The `k` bit indices use double hashing, `index_i = h1 + i·h2 (mod m)`, where
`h1`/`h2` come from two independent hashes (FNV-1a and djb2) run through the
`fmix32` finaliser — matching the Rust crate.

## Two "pure ISO" wrinkles worth noting

1. **No libm.** The optimal sizing `m = ceil(-n·ln p / (ln 2)²)` needs a natural
   log, but the strict harness does not link libm. This port carries a small,
   self-contained `ln` (range-reduce to `[1,2)`, then the fast `2·atanh` series),
   and computes `ratio^k` with a plain multiply loop instead of `pow`.
2. **Overflow-guarded allocation.** `(m + 7)` is checked against `SIZE_MAX`
   before computing the byte count, and the array is `calloc`'d (zeroed).

## Usage

```c
#include "bloom_filter.h"

bloom_filter bf;
if (bloom_init(&bf, 1000, 0.01) == BLOOM_OK) {   /* 1000 items, 1% FP rate */
    bloom_add(&bf, "hello", 5);
    if (bloom_contains(&bf, "hello", 5)) { /* possibly present */ }
    if (!bloom_contains(&bf, "world", 5)) { /* definitely absent */ }
    bloom_free(&bf);
}
```

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests mirror the Rust suite: no false negatives over 200 inserts, consistent
stats, the sizing formulas (`optimal_k(optimal_m(1e6, 0.01), 1e6) == 7`), and
parameter validation.

## Where it fits

Part of the `code/packages/c` pure-ISO C set — a probabilistic data structure to
sit alongside the exact ones (`trie`, `skip-list`, …).
