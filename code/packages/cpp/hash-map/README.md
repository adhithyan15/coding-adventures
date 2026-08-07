# hash-map (C++)

A pure ISO **C++17**, header-only generic hash map — a faithful port of the Rust
`hash-map` crate (DT18), in namespace `ca`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). No compiler
extensions, standard library only.

## What's inside

`ca::hash_map<K, V>` with two collision strategies and four hash functions,
exactly as in the Rust crate:

| | |
| --- | --- |
| **Chaining** | each bucket is a list of entries; resizes when load factor > 1.0 |
| **Open addressing** | one slot array, linear probing, tombstones on delete; resizes above 0.75 |
| **Hashes** | SipHash-2-4 (default), FNV-1a-32, MurmurHash3-32, djb2 |

```cpp
#include "hash_map.hpp"

ca::hash_map<std::string, int> m;          // defaults: cap 16, chaining, siphash
m.set("apples", 3);
std::optional<int> n = m.get("apples");    // 3
bool present = m.has("apples");
m.remove("apples");

// Choose strategy + hash explicitly, and use any trivially-copyable key type:
ca::hash_map<int, int> g(4, ca::collision_strategy::open_addressing,
                         ca::hash_algorithm::murmur3_32);
g.set(42, 1764);
```

`get` returns `std::optional<V>` (empty when absent). Keys may be `std::string`
(hashed by their characters) or any trivially-copyable type (hashed by its object
representation) — mirroring Rust's "serialise the key, then hash."

## Implementation notes

- **Four hash functions, self-contained** in `ca::detail` (SipHash-2-4,
  MurmurHash3-32, FNV-1a-32, djb2) — same constants/rounds as the C port and the
  Rust crate.
- **Tombstone reuse** on open-addressing insert, and a resize that moves entries
  into the doubled table via `std::move`.
- **No default-construction required** of `K`/`V`: open-addressing slots hold a
  `std::optional<entry>`, so empty and tombstone slots store nothing.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests run the behavioural suite against all eight (strategy × hash)
combinations, plus resize stress with both `std::string` and `int` keys and a
`keys()` completeness check.

## Where it fits

Part of the `code/packages/cpp` pure-ISO C++ set — the foundational associative
container that a `hash-set` builds on.
