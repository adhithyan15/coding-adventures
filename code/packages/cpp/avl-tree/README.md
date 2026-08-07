# avl-tree (C++)

A pure ISO **C++17**, header-only self-balancing **AVL tree** with order
statistics, in namespace `ca::avl`. A faithful port of the Rust `avl-tree` crate
(DT08).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

A binary search tree that keeps its height at O(log n) by rebalancing after
every insert and delete. Each node caches its subtree height and node **count**,
so `rank` and `kth_smallest` (order statistics) are also O(log n).

### Persistence via value semantics

Like the Rust crate, updates are **persistent**: `insert` and `erase` are
`const` and return a *new* tree, leaving the receiver untouched. `AVLTree<T>`
deep-copies on copy, so two handles never share nodes.

```cpp
#include "avl_tree.hpp"
using Tree = ca::avl::AVLTree<int>;

Tree a = Tree::empty().insert(8).insert(3).insert(10);
Tree b = a.erase(3);        // a still contains 3

a.contains(3);              // true
a.min_value().value();      // 3
a.kth_smallest(2).value();  // 8
a.rank(10);                 // 2
a.to_sorted_array();        // {3, 8, 10}
```

`erase` is spelled with the idiomatic std name (the Rust `delete` is a C++
keyword). The element type `T` must be less-than comparable and copyable
(Rust's `T: Ord + Clone`).

## API

`empty`, `insert`, `erase`, `find`, `contains`, `min_value`, `max_value`,
`predecessor`, `successor`, `kth_smallest`, `rank`, `to_sorted_array`, `size`,
`height`, `root`, `balance_factor`, `is_valid_bst`, `is_valid_avl`. Lookups that
may miss return `std::optional<T>`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust crate's unit tests and add delete cases,
predecessor/successor, value-semantics independence, and a 0..99 insert/delete
stress that re-verifies the AVL invariant throughout.
