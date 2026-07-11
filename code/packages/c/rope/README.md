# rope (C)

A pure ISO **C17** rope — a binary tree of string chunks that makes
concatenation O(1) and edits cheap. A faithful port of the Rust `rope` crate
(DT16).

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## Consuming (move) semantics

The Rust crate's operations take their ropes *by value* and return new ropes.
This C port mirrors that with a **consuming API**: every function that takes a
`rope *` argument **takes ownership of it** — after the call that pointer is dead
(don't use or free it), and the returned rope owns all the memory. This is how
the port stays allocation-frugal (subtrees are *moved*, not copied) while
matching the crate.

```c
#include "rope.h"

rope *r = rope_concat(rope_from_string("hello", 5),   /* both consumed */
                      rope_from_string(" world", 6));
char c;
rope_index(r, 1, &c);                                 /* 'e' — does not consume */

rope *left, *right;
rope_split(r, 5, &left, &right);                      /* consumes r */
/* left = "hello", right = " world" */
rope_free(left);
rope_free(right);
```

| Group | Functions |
| --- | --- |
| Construct | `rope_empty`, `rope_from_string`, `rope_free` |
| Join / edit (consuming) | `rope_concat`, `rope_split`, `rope_insert`, `rope_delete`, `rope_rebalance` |
| Read (non-consuming) | `rope_to_string`, `rope_index`, `rope_substring`, `rope_len`, `rope_depth`, `rope_is_balanced` |

`rope_concat` is O(1) and shares nothing — it moves both subtrees under a new
weighted internal node. The other edits flatten to bytes and rebuild, exactly as
the crate does.

## Notes

- **Byte-oriented.** The crate counts Unicode scalar values; this port works on
  bytes, so results match for ASCII / single-byte text. Offsets are byte offsets.
- **Overflow-safe.** `rope_concat` rejects a combined length that would overflow
  `size_t`; `rope_delete` computes its end offset without risking `start+length`
  overflow.
- On any allocation failure a consuming function still frees its inputs and
  returns NULL (or 0), so there is never a leak or a dangling half-consumed rope.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests are pinned to the crate's own assertions (concat/index/split of
`"hello"`/`" world"`, the insert→delete→rebalance chain yielding `"ade"` and a
balanced depth ≤ 3) plus empty-rope, clamping, and weighted-index edge cases.
