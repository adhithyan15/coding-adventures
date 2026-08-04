# hash-set (C++)

A pure ISO **C++17**, header-only hash set — a faithful port of the Rust
`hash-set` crate (DT19), in namespace `ca`. Exactly like the Rust crate, it is a
thin wrapper over the sibling [`hash-map`](../hash-map/) package
(`ca::hash_set<T>` = `ca::hash_map<T, unit>`).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/).

## API

```cpp
#include "hash_set.hpp"

ca::hash_set<std::string> s;   // defaults: cap 16, chaining, SipHash-2-4
s.add("apple");
bool member = s.contains("apple");
s.remove("apple");

ca::hash_set<int> a, b;
// ... populate ...
auto u  = a.union_with(b);
auto i  = a.intersection(b);
auto d  = a.difference(b);
auto sd = a.symmetric_difference(b);
bool sub = a.is_subset(b);
```

`union` is a C++ keyword, so the union operation is spelled `union_with`; the
other operations keep their names (`intersection`, `difference`,
`symmetric_difference`, `is_subset`, `is_superset`, `is_disjoint`, `equals`).
Elements may be `std::string` or any trivially-copyable type.

## Depends on `hash-map`

Header-only dependency: `BUILD` declares `# build-tool: deps=cpp/hash-map` and
`tools/run.sh` adds `../hash-map/include` to the include path. The set forwards
membership to `ca::hash_map<T, unit>` and builds set algebra on its `keys()`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust suite: membership, duplicate handling, the four set-algebra
operations, and the subset/superset/disjoint/equals relations, with both string
and integer elements.
