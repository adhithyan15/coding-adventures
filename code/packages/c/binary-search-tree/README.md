# binary-search-tree (C)

An unbalanced **binary search tree** with order statistics, in pure ISO C17.
A faithful port of the Rust `binary-search-tree` crate (DT07).

A BST keeps values ordered so that, for every node, everything in its left
subtree is smaller and everything in its right subtree is larger. Search,
insert, and delete are `O(h)` in the tree height `h` — `O(log n)` for a
balanced tree, `O(n)` worst case for a degenerate one. Every node caches its
subtree size, which makes `bst_rank` and `bst_kth_smallest` (order statistics)
`O(h)`.

Unlike the sibling [`avl-tree`](../avl-tree) package, this tree never rotates:
insertion order alone determines its shape. `bst_from_sorted_array` builds a
height-balanced tree from a sorted array by recursively taking the middle
element as each subtree root.

## Persistence

Like the Rust crate, updates are **persistent**: `bst_insert` and `bst_delete`
return a *new* tree and leave the input untouched (they deep-copy, then mutate
the copy). Every tree you obtain must be released with `bst_free`.

## API

```c
#include "binary_search_tree.h"

BST *t  = bst_empty();          /* or bst_from_sorted_array(sorted, n) */
BST *t1 = bst_insert(t, 8);     /* t is unchanged; t1 is a new tree    */
BST *t2 = bst_insert(t1, 3);

int out;
bst_contains(t2, 3);            /* -> 1                                */
bst_min_value(t2, &out);        /* out = 3, returns 1                  */
bst_kth_smallest(t2, 1, &out);  /* 1-based; out = 3                    */
bst_rank(t2, 8);                /* values strictly less than 8         */

BST *t3 = bst_delete(t2, 8);    /* new tree without 8                  */

bst_free(t3); bst_free(t2); bst_free(t1); bst_free(t);
```

Queries: `bst_search`, `bst_contains`, `bst_min_value`, `bst_max_value`,
`bst_predecessor`, `bst_successor`, `bst_kth_smallest`, `bst_rank`,
`bst_to_sorted_array`, `bst_size`, `bst_height` (`-1` when empty), `bst_is_valid`.
The out-parameter queries return `1` on success and `0` when the value is absent
(e.g. an empty tree or an out-of-range `k`).

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness). Element type is `int`.

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```
