# b-plus-tree (C)

A pure ISO **C17** B+ tree (minimum degree `t`) — a B-tree variant tuned for
range scans. A faithful port of the Rust `b-plus-tree` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## What sets a B+ tree apart

- **All values live in leaves.** Internal nodes hold only separator keys for
  routing; a separator is a *copy* of the smallest key in its right subtree.
- **Leaves form a linked list.** Every leaf has a `next` pointer, so a range
  scan finds one leaf and then walks the chain — no repeated root-to-leaf
  descents. `bpt_full_scan` is an O(n) walk from the first leaf.

```text
            [30 | 60]              ← internal (separators only)
       [10|20] [40|50] [70|80]    ← internal
   L1───L2───L3───L4───L5───L6───L7──▶ ∅   ← leaves, linked in key order
```

## API (`long` keys → `long` values)

```c
#include "b_plus_tree.h"

bpt *t = bpt_new(2);
bpt_insert(t, 10, 100);
bpt_insert(t, 5, 50);
bpt_insert(t, 20, 200);
long v;
bpt_search(t, 10, &v);                 /* v == 100 */
/* range scan 5..15 visits (5,50) then (10,100) via the leaf chain */
bpt_range_scan(t, 5, 15, my_visitor, my_ctx);
bpt_free(t);
```

| Group | Functions |
| --- | --- |
| Map ops | `bpt_insert`, `bpt_delete`, `bpt_search`, `bpt_contains` |
| Scans | `bpt_full_scan`, `bpt_range_scan` (visitor callbacks over the leaf chain) |
| Introspection | `bpt_min_key`, `bpt_max_key`, `bpt_len`, `bpt_is_empty`, `bpt_height`, `bpt_is_valid` |

The Rust crate is generic; C has no generics, so this port specialises to
`long → long`. The **C++ sibling package is fully generic** (`ca::b_plus_tree<K,
V>`).

## Implementation notes

- **The leaf `next` chain is an ordinary C pointer.** The Rust crate builds it
  with `*mut` raw pointers (needing `unsafe`); in C, an intrusive linked list is
  the native, pure-ISO idiom. Splits splice a new leaf into the chain; merges
  relink (`left->next = right->next`); borrows leave the chain untouched.
- **Fixed-capacity nodes** (`2t` keys, `2t+1` children) so no in-node
  reallocation is needed; `t` is clamped so `2t+1` cannot overflow `size_t`.
- **OOM-safe insert.** Because splits propagate bottom-up, each full node
  pre-allocates its split node *before* mutating, so an allocation failure leaves
  the whole tree unchanged and valid. Delete allocates nothing.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests include torture runs (1000–2000 keys inserted out of order at degrees
`t = 2, 3, 6`) that verify a sorted full leaf-chain scan, `is_valid()` (which
also checks the leaf chain lists every key once), search, range scans, and
deletion of half the keys — exercising leaf and internal splits, borrows, and
merges.
