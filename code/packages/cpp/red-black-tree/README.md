# red-black-tree (C++)

A pure ISO **C++17**, header-only **left-leaning red-black (LLRB) tree** with
order statistics, in namespace `ca::rb`. A faithful port of the Rust
`red-black-tree` crate (DT09).

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What it is

A balanced binary search tree that keeps its height at O(log n) via two
invariants: no red node has a red child, and every root-to-leaf path crosses the
same number of black nodes. This is the **left-leaning** variant (Sedgewick),
equivalent to a 2-3 tree, so a single `fix_up` handles insert and delete. Each
node caches its subtree node **count**, making `kth_smallest` O(log n).

### Persistence via value semantics

Like the Rust crate, `insert` and `erase` are `const` and return a *new* tree,
leaving the receiver untouched. `RBTree<T>` deep-copies on copy.

```cpp
#include "red_black_tree.hpp"
using Tree = ca::rb::RBTree<int>;

Tree a = Tree::empty().insert(8).insert(3).insert(10);
Tree b = a.erase(3);          // a still contains 3

a.contains(3);                // true
a.kth_smallest(2).value();    // 8
a.black_height();             // >= 1
a.is_valid_rb();              // true
```

`erase` uses the idiomatic std name (Rust's `delete` is a C++ keyword). The
element type `T` must be less-than comparable and copyable (Rust's
`T: Ord + Clone`).

## API

`empty`, `insert`, `erase`, `find`, `contains`, `min_value`, `max_value`,
`predecessor`, `successor`, `kth_smallest`, `to_sorted_array`, `size`,
`black_height`, `root`, `is_valid_rb`. Lookups that may miss return
`std::optional<T>`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust crate's unit tests and add per-step delete verification,
neighbour queries, value-semantics independence, and a 0..199 ascending
insert/delete stress that re-checks the LLRB invariant throughout.
