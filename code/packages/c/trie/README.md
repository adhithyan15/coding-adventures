# trie (C)

A **trie (prefix tree)** mapping byte-string keys to `int` values, in pure ISO
C17. A faithful port of the Rust `trie` crate: O(key-length) insert/search,
prefix queries, sorted enumeration, and longest-prefix match.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "trie.h"

trie t;
trie_init(&t);
trie_insert(&t, "cat", 1);
trie_insert(&t, "car", 2);

int v;
trie_search(&t, "car", &v);        /* v = 2 */
trie_starts_with(&t, "ca");        /* 1 */

char key[32];
trie_longest_prefix_match(&t, "cards", key, sizeof key, &v); /* key="car" */

/* Enumerate keys with a prefix, in sorted order, via a callback: */
trie_foreach_prefix(&t, "ca", my_visit, my_userdata);
trie_delete(&t, "car");
trie_free(&t);
```

The C port keys on **bytes** (256-way nodes), so UTF-8 strings are stored by
their byte sequence; enumeration is in ascending byte order (matching the
crate's sorted iteration). The trie owns its nodes — pair `trie_init` with
`trie_free`.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/trie`. Enumeration uses a visitor callback rather than
returning an allocated list (the idiomatic C form). See also the
[C++ port](../../cpp/trie/README.md).
