# rope (C++)

A pure ISO **C++17**, header-only rope — a binary tree of string chunks with
O(1) concatenation and cheap edits. A faithful port of the Rust `rope` crate
(DT16), in namespace `ca`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## Value semantics with structural sharing

Nodes are immutable and held by `std::shared_ptr`, so `ca::rope` is cheap to copy
and every operation returns a new rope while reusing untouched subtrees — the
C++-idiomatic take on the crate's `Clone` rope. (The C port instead uses a
consuming API for the same effect.)

```cpp
#include "rope.hpp"

auto r = ca::rope::concat(ca::rope::from_string("hello"),
                          ca::rope::from_string(" world"));
r.len();               // 11
r.index(1);            // std::optional<char>('e')
auto [a, b] = r.split(5);   // {"hello", " world"}

auto edited = ca::rope::from_string("ace").insert(1, "b").insert(3, "d");
edited.to_string();    // "abcde"
edited.erase(1, 2).to_string();  // "ade"   (delete is a keyword → erase)
```

| Group | Members |
| --- | --- |
| Construct | `rope()`, `from_string` |
| Join / edit | `concat`, `split`, `insert`, `erase`, `rebalance` |
| Read | `to_string`, `index` (→ `std::optional<char>`), `substring`, `len`, `empty`, `depth`, `is_balanced` |

## Notes

- **Byte-oriented** (like `std::string`): the crate counts Unicode scalar values,
  so results match for ASCII / single-byte text. Offsets are byte offsets.
- `index` uses a weighted tree descent; `concat` is O(1) and shares both subtrees.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the crate's own assertions plus empty-rope, clamping,
weighted-index, and copy-independence (structural-sharing safety) checks.
