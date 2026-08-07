# red-black-tree (C)

A pure ISO **C17** **left-leaning red-black (LLRB) tree** with order statistics.
A faithful port of the Rust `red-black-tree` crate (DT09).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

A balanced binary search tree that colours nodes red or black and keeps its
height at O(log n) via two invariants: no red node has a red child, and every
root-to-leaf path crosses the same number of black nodes. This is the
**left-leaning** variant (Sedgewick) — red links always lean left, so a single
`fix_up` on the way back up handles both insert and delete, and the structure is
exactly equivalent to a 2-3 tree. Each node caches its subtree node **count**,
making `rb_kth_smallest` O(log n).

### Persistence

Like the Rust crate, `rb_insert` and `rb_delete` are **persistent**: they return
a *new* tree and leave the input untouched (deep-copy, then mutate the copy).
Every tree you obtain must be freed with `rb_free`.

```c
#include "red_black_tree.h"

RBTree *a = rb_empty();
RBTree *b = rb_insert(a, 8);      // a is still empty
RBTree *c = rb_insert(b, 3);
RBTree *d = rb_delete(c, 8);      // c still contains 8

int out;
rb_kth_smallest(c, 1, &out);      // 3
rb_is_valid_rb(c);                // 1

rb_free(a); rb_free(b); rb_free(c); rb_free(d);
```

## API

| Function | Purpose |
|---|---|
| `rb_empty` / `rb_free` | create / destroy a tree |
| `rb_insert` / `rb_delete` | persistent update (new tree returned) |
| `rb_search` / `rb_contains` | membership |
| `rb_min_value` / `rb_max_value` | extremes (out-param, `1`/`0`) |
| `rb_predecessor` / `rb_successor` | neighbouring stored values |
| `rb_kth_smallest` | order statistic (1-based k) |
| `rb_to_sorted_array` | in-order dump into a caller buffer |
| `rb_size` / `rb_black_height` | metadata |
| `rb_is_valid_rb` | full LLRB invariant check |

Element type is `int`. `rb_insert`/`rb_delete` return `NULL` only on allocation
failure, leaving the input tree valid and unchanged.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust crate's unit tests and add per-step delete verification,
predecessor/successor, persistence, and a 0..199 ascending insert/delete stress
(a worst case for a plain BST) that re-checks the LLRB invariant throughout.
