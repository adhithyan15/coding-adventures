# avl-tree (C)

A pure ISO **C17** self-balancing **AVL tree** with order statistics. A faithful
port of the Rust `avl-tree` crate (DT08).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Standard library only.

## What it is

A binary search tree that keeps its height at O(log n) by rebalancing after
every insert and delete: for every node, the heights of its two subtrees differ
by at most one. Each node caches its subtree height and node **count**, so
order-statistic queries (`avl_rank`, `avl_kth_smallest`) are also O(log n).

### Persistence

Like the Rust crate, updates are **persistent**: `avl_insert` and `avl_delete`
return a *new* tree and leave the input untouched (they deep-copy, then mutate
the copy). Every tree you obtain — including the ones returned by updates — must
be freed with `avl_free`.

```c
#include "avl_tree.h"

AVLTree *a = avl_empty();
AVLTree *b = avl_insert(a, 20);   // a is still empty
AVLTree *c = avl_insert(b, 10);
AVLTree *d = avl_delete(c, 20);   // c still contains 20

int out;
avl_min_value(c, &out);           // 10
avl_kth_smallest(c, 2, &out);     // 20
size_t r = avl_rank(c, 20);       // 1

avl_free(a); avl_free(b); avl_free(c); avl_free(d);
```

## API

| Function | Purpose |
|---|---|
| `avl_empty` / `avl_free` | create / destroy a tree |
| `avl_insert` / `avl_delete` | persistent update (new tree returned) |
| `avl_search` / `avl_contains` | membership |
| `avl_min_value` / `avl_max_value` | extremes (via out-param, `1`/`0`) |
| `avl_predecessor` / `avl_successor` | neighbouring stored values |
| `avl_kth_smallest` / `avl_rank` | order statistics (1-based k) |
| `avl_to_sorted_array` | in-order dump into a caller buffer |
| `avl_size` / `avl_height` / `avl_balance_factor` | metadata |
| `avl_is_valid_bst` / `avl_is_valid_avl` | invariant checks |

Element type is `int`. `avl_insert`/`avl_delete` return `NULL` only on allocation
failure, leaving the input tree valid and unchanged.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust crate's unit tests (rotations rebalance; search and order
statistics), plus delete cases, predecessor/successor, persistence (the original
survives an update), and a 0..99 insert/delete stress that re-verifies the AVL
invariant throughout.
