# treap (C++)

A **treap** (tree + heap) — a randomized balanced binary search tree — in pure
ISO C++17, header-only, in namespace `ca::treap`. A faithful port of the Rust
`treap` crate (DT10).

Each key carries a random `priority`, and the tree keeps two invariants at once:

- **BST order** on the keys — left subtree < node < right subtree.
- **Max-heap order** on the priorities — every node's priority is ≥ its
  children's.

Because priorities are random, the heap constraint forces a shape that is
balanced *in expectation*: `O(log n)` search / insert / delete with high
probability, with no explicit rebalancing (unlike the sibling
[`avl-tree`](../avl-tree) / [`red-black-tree`](../red-black-tree) packages).
Insert restores the heap with rotations; erase does so by merging the two child
subtrees in priority order.

`split` and `merge` are the treap's signature operations (`O(log n)`):

```
split(key) -> (keys <= key, keys > key)     merge(l, r)   [all l-keys < all r-keys]
```

Each node caches its subtree size, so `kth_smallest` and order statistics are
`O(h)`.

## Priorities

`insert` takes `std::optional<double>`: a value uses that exact priority;
`std::nullopt` (the default) draws one from a built-in deterministic PRNG.

> The Rust crate seeds that PRNG through a global `AtomicU32` for cross-thread
> safety; this port uses a function-local `static` counter with the identical
> arithmetic (single-threaded). Supply priorities explicitly for reproducibility.

## Persistence

Like the Rust crate, updates are **persistent**: `insert`, `erase`, `split`, and
`merge` return brand-new treaps and leave their inputs untouched. This port keeps
that through value semantics — `Treap<K>` deep-copies on copy, and the update
methods are `const`, copying `*this` before working on the copy.

## API

```cpp
#include "treap.hpp"
using ca::treap::Treap;

Treap<int> t = Treap<int>::empty()
                   .insert(8, 0.8)      // explicit priority
                   .insert(3);          // nullopt -> PRNG priority

t.contains(8);                          // -> true
t.kth_smallest(1);                      // 1-based -> std::optional<int>

auto [lo, hi] = t.split(5);             // lo: keys <= 5, hi: keys > 5
Treap<int> back = Treap<int>::merge(lo, hi);
```

`Treap<K>` works with any less-than-comparable, copyable `K` (matching Rust's
`K: Ord + Clone`) — e.g. `Treap<std::string>`. Optional-returning queries
(`min_key`, `max_key`, `predecessor`, `successor`, `kth_smallest`) yield
`std::nullopt` when absent; `height()` is `-1` for an empty treap. `is_valid()`
checks both invariants and the cached sizes.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```
