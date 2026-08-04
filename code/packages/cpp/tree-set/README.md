# tree-set (C++)

A pure ISO **C++17**, header-only ordered **set** generic over a balanced-tree
backend, in namespace `ca::tree_set`. A faithful port of the Rust `tree-set`
crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Depends on the
sibling [`avl-tree`](../avl-tree/) (default backend) and
[`red-black-tree`](../red-black-tree/) packages.

## What it is

`TreeSet<T, Backend>` is generic over its backend, exactly like the Rust crate:
any ordered balanced tree providing `insert / erase / contains / min_value /
max_value / predecessor / successor / kth_smallest / to_sorted_array / size`
works. The default is `ca::avl::AVLTree<T>`; `ca::rb::RBTree<T>` satisfies the
same interface, so `TreeSet<int, ca::rb::RBTree<int>>` works too (both are
exercised in the tests).

Set algebra (union, intersection, difference, symmetric difference), subset /
superset / disjoint tests, and range queries are all computed from the operands'
sorted sequences by a linear merge — backend-independent, as in the crate.

### Persistence via value semantics

`insert`, `remove`, and the algebra operations are `const` and return a *new*
set, leaving the receiver untouched.

```cpp
#include "tree_set.hpp"
using Set = ca::tree_set::TreeSet<int>;

Set s = Set::from_list({7, 3, 9, 1, 5, 3});   // {1,3,5,7,9}
Set r = Set::from_list({3, 4, 5, 6});

s.union_with(r).to_sorted_array();   // {1,3,4,5,6,7,9}
s.intersection(r).to_sorted_array(); // {3,5}
s.range(3, 7, true);                 // {3,5,7}
```

The union operation is spelled **`union_with`** because `union` is a C++
keyword. `T` must be less-than comparable and copyable.

## API

`empty`, `from_list`, `insert`, `remove`, `erase`, `contains`, `size`,
`is_empty`, `min_value`, `max_value`, `first`, `last`, `predecessor`,
`successor`, `kth_smallest`, `rank`, `to_sorted_array`, `range`, `union_with`,
`intersection`, `difference`, `symmetric_difference`, `is_subset`,
`is_superset`, `is_disjoint`, `equals`, `backend`. Lookups that may miss return
`std::optional<T>`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust crate's unit tests on both the AVL and red-black backends,
plus persistence, range boundary cases, and the relation predicates.
