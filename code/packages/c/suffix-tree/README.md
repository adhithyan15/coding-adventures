# suffix-tree (C)

A pure ISO **C17** suffix index over a string — a faithful port of the Rust
`suffix-tree` crate (DT15). Substring search, occurrence counting, longest
repeated substring, and longest common substring.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## What it does

Like the reference crate, the "tree" is kept deliberately simple — a root with
one leaf per suffix — so the package really is a small bundle of string
algorithms over a stored copy of the text:

| Function | Result |
| --- | --- |
| `suffix_tree_search` | every start offset where a pattern occurs |
| `suffix_tree_count_occurrences` | how many times a pattern occurs |
| `suffix_tree_longest_repeated_substring` | longest substring that repeats |
| `suffix_tree_suffix` | borrow suffix `i` (`text[i..]`) |
| `suffix_tree_node_count` | `1 + text length` |
| `suffix_longest_common_substring` | LCS of two strings (free function) |

```c
#include "suffix_tree.h"

suffix_tree *t = suffix_tree_build("banana", 6);
size_t pos[8];
size_t n = suffix_tree_search(t, "ana", 3, pos, 8);   /* n == 2, pos = {1, 3} */

char buf[16];
size_t len = suffix_tree_longest_repeated_substring(t, buf, sizeof buf); /* "ana" */
suffix_tree_free(t);
```

`suffix_tree_search` returns the full occurrence count and writes up to `out_cap`
offsets, so a caller can size its buffer or pass `NULL`/`0` to just count.

## Notes

- **Byte-oriented.** The crate counts Unicode scalar values; this port works on
  bytes, so results match for ASCII / single-byte text. Offsets are byte offsets.
- **Overflow-safe scans.** The search bound is written `start <= len - plen`
  (never `start + plen`) so it cannot overflow `size_t`; the LCS routine rejects
  `blen == SIZE_MAX` before sizing its rolling DP rows.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the crate's own assertions (`search("banana", "ana")` →
`{1,3}`, `node_count` = 7, longest repeated = `"ana"`, LCS of `"xabxac"` /
`"abcabxabcd"` = `"abxa"`) plus empty-pattern, over-long-pattern, and empty-text
edge cases.
