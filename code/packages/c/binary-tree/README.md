# binary-tree (C)

A generic **binary tree** with traversals and shape predicates, in pure ISO C17.
A faithful port of the Rust `binary-tree` crate (DT03).

Unlike a search tree there is no ordering invariant — this is the shared
substrate the search-tree family reuses for traversal and shape checks.

## Shape predicates

| Predicate | Meaning |
|-----------|---------|
| `bt_is_full`     | every node has 0 or 2 children |
| `bt_is_complete` | every level filled except possibly the last, left-to-right |
| `bt_is_perfect`  | full **and** all leaves at the same depth (`n == 2^(h+1)-1`) |

## API

```c
#include "binary_tree.h"

/* Build from a level-order layout; present[i]==0 marks a gap. */
int vals[]    = {1, 2, 3, 0, 5};
int present[] = {1, 1, 1, 0, 1};
BinaryTree *t = bt_from_level_order(vals, present, 5);

int buf[16];
size_t n = bt_inorder(t, buf, 16);   /* also: preorder / postorder / level_order */

const BinaryTreeNode *node = bt_find(t, 5);
bt_is_complete(t);                   /* 1 or 0 */

char *diagram = bt_to_ascii(t);      /* malloc'd; caller frees */
free(diagram);
bt_free(t);
```

Build by hand with `bt_node_new` (assign `left`/`right` yourself) then
`bt_with_root`. Traversals copy values into a caller buffer and return the count.
`bt_to_array` writes the level-order layout with a parallel `present[]` array
(gaps set to 0) and returns the full length `2^(h+1)-1`. `bt_to_ascii` returns a
malloc'd indented diagram:

```
`-- 1
    |-- 2
    |   |-- 4
    |   `-- 5
    `-- 3
```

## Portability

Pure ISO C17 — compiles clean under GCC, Clang, and MSVC with `-pedantic-errors`
/ `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness). Element type is `int`.

## Development

```bash
# Compile and run the tests under every C compiler on PATH.
sh BUILD
```
