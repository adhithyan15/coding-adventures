# tree-set (C)

A pure ISO **C17** ordered **set** built on a balanced-tree backend. A faithful
port of the Rust `tree-set` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../iso-harness/). Depends on the sibling
[`avl-tree`](../avl-tree/) package for its backend.

## What it is

A set keeps each value at most once and its elements in sorted order. The Rust
crate is generic over any balanced ordered tree; this C port uses the crate's
**default backend**, the sibling `avl-tree`, so every membership/order query is
O(log n) and elements come out sorted. On top of that sit the set algebra
(union, intersection, difference, symmetric difference), the subset / superset /
disjoint tests, and range queries — all computed from the operands' sorted
sequences by a linear merge, exactly as the crate does.

### Persistence

Like the crate (and avl-tree), updates are **persistent**: `tset_insert`,
`tset_remove`, and the algebra operations return a *new* set and leave their
inputs untouched. Free every set you obtain with `tset_free`.

```c
#include "tree_set.h"

int vs[] = {7, 3, 9, 1, 5, 3};
TreeSet *s = tset_from_array(vs, 6);   /* {1,3,5,7,9} (dup 3 collapses) */

int buf[16];
size_t n = tset_to_sorted_array(s, buf, 16);   /* 1 3 5 7 9 */

int right_vs[] = {3, 4, 5, 6};
TreeSet *r = tset_from_array(right_vs, 4);
TreeSet *u = tset_union(s, r);          /* {1,3,4,5,6,7,9}; s and r unchanged */

tset_free(s); tset_free(r); tset_free(u);
```

## API

| Group | Functions |
|---|---|
| lifecycle | `tset_empty`, `tset_free`, `tset_from_array` |
| updates | `tset_insert`, `tset_remove` (persistent) |
| queries | `tset_size`, `tset_is_empty`, `tset_contains`, `tset_min_value`, `tset_max_value`, `tset_predecessor`, `tset_successor`, `tset_kth_smallest`, `tset_rank`, `tset_to_sorted_array`, `tset_range` |
| algebra | `tset_union`, `tset_intersection`, `tset_difference`, `tset_symmetric_difference` |
| relations | `tset_is_subset`, `tset_is_superset`, `tset_is_disjoint`, `tset_equals` |

Element type is `int`. Allocating operations return `NULL` only on allocation
failure (the size arithmetic behind the algebra is overflow-guarded, and the
per-set arrays use `calloc`'s checked multiply); inputs are never modified.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust crate's unit tests (ordered-set operations and set
algebra) and add persistence, range boundary cases, and the relation predicates,
under GCC and Clang via `iso-harness`.
