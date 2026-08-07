# treap (C)

A **treap** (tree + heap) — a randomized balanced binary search tree — in pure
ISO C17. A faithful port of the Rust `treap` crate (DT10).

Each key carries a random `priority`, and the tree keeps two invariants at once:

- **BST order** on the keys — left subtree < node < right subtree.
- **Max-heap order** on the priorities — every node's priority is ≥ its
  children's.

Because priorities are random, the heap constraint forces a shape that is
balanced *in expectation*: `O(log n)` search / insert / delete with high
probability, with no explicit rebalancing (unlike the sibling
[`avl-tree`](../avl-tree) / [`red-black-tree`](../red-black-tree) packages).
Insert restores the heap with rotations; delete does so by merging the two
child subtrees in priority order.

`split` and `merge` are the treap's signature operations (`O(log n)`):

```
split(key) -> (keys <= key, keys > key)     merge(l, r)   [all l-keys < all r-keys]
```

Each node caches its subtree size, so `treap_kth_smallest` and `treap_rank`-style
order statistics are `O(h)`.

## Priorities

`treap_insert` takes a `const double *priority`: pass a pointer to use a
specific priority, or `NULL` to draw one from a built-in deterministic PRNG.

> The Rust crate seeds that PRNG through a global `AtomicU32` for cross-thread
> safety; this port uses a plain `static` counter with the identical arithmetic
> (single-threaded). Supply priorities explicitly for reproducibility or
> thread-safety.

## Persistence

Like the Rust crate, updates are **persistent**: `treap_insert`, `treap_delete`,
`treap_split`, and `treap_merge` return *new* treaps and leave their inputs
untouched (deep-copy, then work on the copy). Every treap must be released with
`treap_free`.

## API

```c
#include "treap.h"

Treap *t  = treap_empty();
double p  = 0.8;
Treap *t1 = treap_insert(t, 8, &p);      /* explicit priority */
Treap *t2 = treap_insert(t1, 3, NULL);   /* PRNG priority     */

int out;
treap_contains(t2, 8);            /* -> 1                        */
treap_kth_smallest(t2, 1, &out);  /* 1-based order statistic     */

Treap *lo = NULL, *hi = NULL;
treap_split(t2, 5, &lo, &hi);     /* lo: keys <= 5, hi: keys > 5 */
Treap *back = treap_merge(lo, hi);

treap_free(back); treap_free(lo); treap_free(hi);
treap_free(t2); treap_free(t1); treap_free(t);
```

Queries: `treap_search`, `treap_contains`, `treap_min_key`, `treap_max_key`,
`treap_predecessor`, `treap_successor`, `treap_kth_smallest`,
`treap_to_sorted_array`, `treap_size`, `treap_height` (`-1` when empty),
`treap_is_valid` (checks both invariants + cached sizes).

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness). Key type is `int`.

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```
