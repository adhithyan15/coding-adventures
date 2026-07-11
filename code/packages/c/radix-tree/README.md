# radix-tree (C)

A pure ISO **C17** radix tree (compressed trie / Patricia trie) for string-keyed
prefix search. A faithful port of the Rust `radix-tree` crate.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## What a radix tree does

A trie stores keys character by character; a radix tree **compresses** every
chain of single-child nodes into one edge labelled with the whole shared
substring. That keeps the node count small while still answering prefix queries
fast:

```text
insert "search", "searcher", "searching":

   (root) --"search"--> (end) --"er"----> (end)
                             \--"ing"---> (end)      →  5 nodes, not 17
```

Each node's edges are kept sorted by their first byte, so key enumeration comes
out in order.

## API (`long` values)

```c
#include "radix_tree.h"

radix_tree *t = radix_new();
radix_insert(t, "application", 1);
radix_insert(t, "apple", 2);
long v;
radix_search(t, "apple", &v);                 /* v == 2 */
radix_starts_with(t, "appl");                 /* 1 */
char buf[64];
radix_longest_prefix_match(t, "apple pie", buf, sizeof buf); /* -> "apple" */
radix_free(t);
```

| Group | Functions |
| --- | --- |
| Map ops | `radix_insert`, `radix_search`, `radix_contains`, `radix_delete` |
| Prefix queries | `radix_starts_with`, `radix_longest_prefix_match` |
| Enumeration | `radix_keys`, `radix_words_with_prefix` (sorted visitor callbacks) |
| Introspection | `radix_len`, `radix_is_empty`, `radix_node_count` |

The Rust crate is generic over the value; C has no generics, so this port
specialises to a `long` value. The **C++ sibling package is generic**
(`ca::radix_tree<V>`).

## Implementation notes

- **Sorted edge arrays.** Each node holds its edges in an array sorted by the
  first byte of the label (binary-searched), matching the crate's `BTreeMap`
  ordering so `keys()` / `words_with_prefix()` are sorted.
- **OOM-safe insert.** An edge split allocates every new node/label *before*
  mutating the existing edge, so an allocation failure leaves the tree unchanged
  and valid.
- **Delete compresses.** Removing a key prunes a dead leaf, and folds a node
  left with a single child back into its parent edge (concatenating labels).
- **Byte-oriented.** The crate splits on Unicode chars; this port on bytes.
  Insert/search/delete are byte-consistent, so it is a correct prefix map for any
  byte string, and identical to the crate for ASCII keys.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the crate's suite: the app/apple/apt split cases, prune-and-merge
`node_count` after delete, mid-edge `starts_with`, sorted `words_with_prefix`,
`longest_prefix_match`, and empty-string keys.
