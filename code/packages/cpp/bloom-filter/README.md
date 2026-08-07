# bloom-filter (C++)

A pure ISO **C++17**, header-only Bloom filter — a compact, probabilistic set
for membership tests, in namespace `ca`. A faithful port of the Rust
`bloom-filter` crate (DT22).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). No compiler
extensions, no third-party dependencies — not even libm.

## What a Bloom filter does

It answers **"definitely not in the set"** or **"possibly in the set."** No
false negatives (an added item always tests present); occasional false positives
whose rate rises as the filter fills. It stores only a bit array of `m` bits;
each element sets `k` bits chosen by `k` hash functions (`index_i = h1 + i·h2 mod
m`, via FNV-1a + djb2 finalised with `fmix32`).

## API

```cpp
#include "bloom_filter.hpp"

ca::bloom_filter bf(1000, 0.01);          // 1000 items, 1% false-positive rate
bf.add(std::string("hello"));
bool maybe   = bf.contains(std::string("hello"));   // possibly present
bool absent  = !bf.contains(std::string("world"));  // definitely absent

// Non-throwing construction:
std::optional<ca::bloom_filter> b = ca::bloom_filter::try_create(1000, 0.01);

// Explicit parameters:
auto c = ca::bloom_filter::from_params(/*bits=*/1024, /*hashes=*/3);

// Sizing math (static):
std::size_t m = ca::bloom_filter::optimal_m(1'000'000, 0.01);
std::size_t k = ca::bloom_filter::optimal_k(m, 1'000'000);
```

The throwing constructor / `from_params` throw `std::invalid_argument` on a bad
parameter (matching Rust's panicking `new`); the `try_create` / `try_from_params`
factories return `std::nullopt` instead (matching Rust's `try_*`).

## "Pure ISO" wrinkle: no libm

`optimal_m = ceil(-n·ln p / (ln 2)²)` needs a natural log, but the strict harness
does not link libm. This header carries a small, self-contained `ln`
(range-reduce to `[1,2)`, then the fast `2·atanh` series) and computes `ratio^k`
with a multiply loop instead of `std::pow`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests mirror the Rust suite: no false negatives over 200 inserts, consistent
stats, the sizing formulas (`optimal_k(optimal_m(1e6, 0.01), 1e6) == 7`), and
both the throwing and `std::optional` validation paths.

## Where it fits

Part of the `code/packages/cpp` pure-ISO C++ set — the probabilistic companion
to the exact structures (`trie`, `skip-list`, …).
