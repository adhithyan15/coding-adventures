# segment-tree (C)

A **segment tree** over ints with a caller-supplied associative combine
operation, in pure ISO C17. A faithful port of the Rust `segment-tree` crate.
O(log n) range queries and point updates for sum / min / max / gcd / any
associative op with an identity.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "segment_tree.h"

int values[6] = {1, 3, 5, 7, 9, 11};
segment_tree t;
segment_tree_init_sum(&t, values, 6);          /* or _min / _max / _init(op,id) */

int s = segment_tree_query(&t, 1, 3);          /* 3+5+7 = 15 (inclusive range) */
segment_tree_update(&t, 2, 10);                /* element 2 := 10 */
s = segment_tree_query(&t, 1, 3);              /* 20 */
segment_tree_free(&t);
```

Ranges are **inclusive and 0-based**. Out-of-range or inverted queries return
the identity element (never an out-of-bounds read). The tree owns a heap
allocation — pair `segment_tree_init*` with `segment_tree_free`.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/segment-tree`; the 1-indexed 4n layout and recursive
build/query/update match the crate. See also the
[C++ port](../../cpp/segment-tree/README.md).
