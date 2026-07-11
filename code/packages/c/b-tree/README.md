# b-tree (C)

A pure ISO **C17** B-tree (minimum degree `t`) — a balanced, high-fan-out search
tree. A faithful port of the Rust `b-tree` crate's full CLRS algorithm.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## What a B-tree is

With minimum degree `t`, every non-root node holds between `t-1` and `2t-1` keys
and between `t` and `2t` children, and **all leaves sit at the same depth**. High
fan-out keeps the tree short, so lookups touch few nodes — the structure behind
most database indexes and filesystems.

This port implements the complete algorithm:

- **Insert** — proactive *top-down splitting*: a full child is split before we
  descend, so the parent always has room for the promoted median.
- **Delete** — CLRS *pre-fill*: before descending into a child with only `t-1`
  keys, we either rotate a key in from a sibling or merge two children, so a key
  can always be removed without underflowing.

## API (`long` keys → `long` values)

```c
#include "b_tree.h"

btree *t = btree_new(2);            /* minimum degree 2 → a 2-3-4 tree */
btree_insert(t, 10, 100);
btree_insert(t, 20, 200);
long v;
btree_search(t, 10, &v);            /* v == 100 */
btree_delete(t, 10);
btree_free(t);
```

| Group | Functions |
| --- | --- |
| Map ops | `btree_insert`, `btree_delete`, `btree_search`, `btree_contains` |
| Extremes | `btree_min_key`, `btree_max_key` |
| Traversal | `btree_inorder`, `btree_range_query` (visitor callbacks) |
| Introspection | `btree_len`, `btree_is_empty`, `btree_height`, `btree_is_valid` |

The Rust crate is generic over an ordered key and any value; C has no generics,
so this port specialises to `long → long` (a sorted integer map) — exactly what
the crate's tests exercise. The **C++ sibling package is fully generic**
(`ca::b_tree<K, V>`).

## Implementation notes

- **Fixed-capacity nodes.** Each node's key/value arrays hold `2t-1` and its
  child array holds `2t`. Because the algorithm never lets a node exceed those
  bounds, there is no in-node reallocation and no size arithmetic that can
  overflow; `btree_new` clamps `t` so `2t` itself can't overflow `size_t`.
- **Allocation-frugal delete.** Deletion allocates nothing — merges free the
  absorbed node, and the root shrinks in place when it empties.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests include torture runs (1000–2000 keys inserted out of order at degrees
`t = 2, 3, 7`) that verify sorted in-order traversal, `is_valid()` after every
phase, correct search, and deletion of half the keys — exercising every split,
borrow, and merge path.
