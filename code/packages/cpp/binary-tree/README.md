# binary-tree (C++)

A generic **binary tree** with traversals and shape predicates, in pure ISO
C++17, header-only, in namespace `ca`. A faithful port of the Rust `binary-tree`
crate (DT03).

Unlike a search tree there is no ordering invariant — this is the shared
substrate the search-tree family reuses for traversal and shape checks.

## Shape predicates

| Predicate | Meaning |
|-----------|---------|
| `is_full`     | every node has 0 or 2 children |
| `is_complete` | every level filled except possibly the last, left-to-right |
| `is_perfect`  | full **and** all leaves at the same depth (`n == 2^(h+1)-1`) |

## API

```cpp
#include "binary_tree.hpp"
using ca::BinaryTree;

// Build from a level-order layout; std::nullopt marks a gap.
BinaryTree<int> t = BinaryTree<int>::from_level_order({1, 2, 3, std::nullopt, 5});

std::vector<int> in = t.inorder();   // also preorder / postorder / level_order
const auto* node    = t.find(5);     // -> const BinaryTree<int>::Node*
bool ok             = t.is_complete();
auto arr            = t.to_array();  // std::vector<std::optional<int>>
std::string diagram = t.to_ascii();
```

`BinaryTree<T>` works with any equality-comparable, copyable `T` (streamable via
`operator<<` for `to_ascii`), and has value semantics — copies are deep. Build by
hand with `make_node` (attach children) then `with_root`.

## Portability

Pure ISO C++17 — compiles clean under GCC, Clang, and MSVC with
`-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness). Standard library only.

## Development

```bash
# Compile and run the tests under every C++ compiler on PATH.
sh BUILD
```
