# b-tree (C++)

A pure ISO **C++17**, header-only, fully generic B-tree (minimum degree `t`) — a
faithful port of the Rust `b-tree` crate's full CLRS algorithm, in namespace
`ca`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What a B-tree is

With minimum degree `t`, every non-root node holds `t-1`..`2t-1` keys and
`t`..`2t` children, and all leaves are at the same depth. This port implements
the complete algorithm — proactive top-down splitting on insert, and pre-fill
(rotate from a sibling, or merge) on delete.

## API

```cpp
#include "b_tree.hpp"

ca::b_tree<int, std::string> t(2);   // minimum degree 2
t.insert(10, "ten");
t.insert(20, "twenty");
const std::string *v = t.search(10);         // -> "ten" (nullptr if absent)
t.min_key();                                 // std::optional<int>(10 or less)
std::vector<std::pair<int,std::string>> all = t.inorder();
t.remove(10);
```

| Group | Members |
| --- | --- |
| Map ops | `insert`, `remove`, `search` (→ `const V*`), `contains` |
| Extremes | `min_key`, `max_key` (→ `std::optional<K>`) |
| Traversal | `inorder`, `range_query` (→ `std::vector<std::pair<K,V>>`) |
| Introspection | `len`, `empty`, `height`, `is_valid` |

Unlike the C sibling (specialised to `long → long`), this header is **fully
generic**: `ca::b_tree<K, V>` works for any less-than-comparable key `K`. Nodes
use `std::vector` with `std::unique_ptr` children, mirroring the Rust crate's
node layout directly.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests include torture runs (1000–2000 keys inserted out of order at degrees
`t = 2, 3, 7`) verifying sorted traversal, `is_valid()` after every phase,
search, and deletion of half the keys, plus a `std::string`-value check for
genericity.
