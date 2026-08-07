# b-plus-tree (C++)

A pure ISO **C++17**, header-only, fully generic B+ tree (minimum degree `t`) — a
faithful port of the Rust `b-plus-tree` crate, in namespace `ca`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What sets a B+ tree apart

All values live in leaves; internal nodes hold only separator keys for routing;
and the leaves form a singly-linked list (`next`) so a range scan finds one leaf
and walks the chain. This port implements the full algorithm — leaf/internal
splitting on insert (bottom-up), borrow/merge rebalancing on delete — keeping the
leaf chain in sync.

## API

```cpp
#include "b_plus_tree.hpp"

ca::b_plus_tree<int, std::string> t(2);
t.insert(10, "ten");
t.insert(5, "five");
const std::string *v = t.search(10);           // -> "ten" (nullptr if absent)
auto rows = t.range_scan(5, 15);               // vector<pair<int,string>>
auto all  = t.full_scan();                     // every entry, sorted
t.remove(10);
```

| Group | Members |
| --- | --- |
| Map ops | `insert`, `remove`, `search` (→ `const V*`), `contains` |
| Scans | `full_scan`, `range_scan` (→ `std::vector<std::pair<K,V>>`) |
| Introspection | `min_key`, `max_key` (→ `std::optional<K>`), `len`, `empty`, `height`, `is_valid` |

Unlike the C sibling (specialised to `long → long`), this header is **fully
generic**: `ca::b_plus_tree<K, V>` for any less-than-comparable `K`. Nodes use
`std::vector` with `std::unique_ptr` children; the leaf `next` link is a
non-owning raw pointer into the owned tree — the C++ analogue of the crate's
`*mut` leaf chain.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests include torture runs (1000–2000 keys inserted out of order at degrees
`t = 2, 3, 6`) verifying a sorted full leaf-chain scan, `is_valid()`, search,
range scans, and deletion of half the keys, plus a `std::string`-value check for
genericity.
