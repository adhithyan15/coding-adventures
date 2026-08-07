# radix-tree (C++)

A pure ISO **C++17**, header-only, generic radix tree (compressed trie / Patricia
trie) for string-keyed prefix search — a faithful port of the Rust `radix-tree`
crate, in namespace `ca`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## What a radix tree does

A radix tree compresses each chain of single-child trie nodes into one edge
labelled with the shared substring, keeping the node count small while answering
prefix queries fast. Each node's children live in a `std::map` keyed by the first
byte of the edge label, so traversals emit keys in sorted order.

## API

```cpp
#include "radix_tree.hpp"

ca::radix_tree<int> t;
t.insert("application", 1);
t.insert("apple", 2);
const int *v = t.search("apple");                    // -> 2 (nullptr if absent)
bool pre = t.starts_with("appl");                    // true
auto lp  = t.longest_prefix_match("apple pie");      // std::optional<string>("apple")
auto ks  = t.words_with_prefix("app");               // sorted vector<string>
t.remove("apple");                                   // delete is a keyword
```

| Group | Members |
| --- | --- |
| Map ops | `insert`, `search` (→ `const V*`), `contains`, `remove` |
| Prefix queries | `starts_with`, `longest_prefix_match` (→ `std::optional<std::string>`) |
| Enumeration | `keys`, `words_with_prefix` (→ sorted `std::vector<std::string>`) |
| Introspection | `len`, `empty`, `node_count` |

Unlike the C sibling (specialised to a `long` value), this header is generic:
`ca::radix_tree<V>` stores any value type. Nodes use `std::map<unsigned char,
edge>` with `std::unique_ptr` children.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the crate's suite (split cases, prune/merge node counts, prefix
queries, empty-string keys, sorted enumeration) plus a `std::string`-value check
for genericity.
