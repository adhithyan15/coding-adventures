# binary-search-tree (C++)

An unbalanced **binary search tree** with order statistics, in pure ISO C++17,
header-only, in namespace `ca::bst`. A faithful port of the Rust
`binary-search-tree` crate (DT07).

A BST keeps values ordered so that, for every node, everything in its left
subtree is smaller and everything in its right subtree is larger. Search,
insert, and delete are `O(h)` in the tree height `h` — `O(log n)` for a
balanced tree, `O(n)` worst case for a degenerate one. Every node caches its
subtree size, which makes `rank` and `kth_smallest` (order statistics) `O(h)`.

Unlike the sibling [`avl-tree`](../avl-tree) package, this tree never rotates:
insertion order alone determines its shape. `from_sorted_array` builds a
height-balanced tree from a sorted vector by recursively taking the middle
element as each subtree root.

## Persistence

Like the Rust crate, updates are **persistent**: `insert` and `erase` are
`const` — they copy `*this` (a deep copy) and mutate the copy, so any tree you
already hold is unchanged. This mirrors Rust's `Box`-based deep-clone
persistence (not `Rc` structural sharing).

## API

```cpp
#include "binary_search_tree.hpp"
using ca::bst::BST;

BST<int> t  = BST<int>::empty();     // or BST<int>::from_sorted_array(sorted)
BST<int> t1 = t.insert(8);            // t is unchanged; t1 is a new tree
BST<int> t2 = t1.insert(3);

t2.contains(3);                       // -> true
t2.min_value();                       // -> std::optional<int>{3}
t2.kth_smallest(1);                   // 1-based -> std::optional<int>{3}
t2.rank(8);                           // values strictly less than 8
t2.to_sorted_array();                 // std::vector<int>, ascending

BST<int> t3 = t2.erase(8);            // new tree without 8
```

`BST<T>` works with any less-than-comparable, copyable `T` (matching Rust's
`T: Ord + Clone`) — e.g. `BST<std::string>`. Optional-returning queries
(`min_value`, `max_value`, `predecessor`, `successor`, `kth_smallest`) yield
`std::nullopt` when absent; `height()` is `-1` for an empty tree.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```
