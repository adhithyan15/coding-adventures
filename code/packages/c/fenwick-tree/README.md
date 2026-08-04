# fenwick-tree (C)

A **Fenwick tree (Binary Indexed Tree)** over doubles, in pure ISO C17. A
faithful port of the Rust `fenwick-tree` crate. Supports O(log n) `update` and
`prefix_sum`, plus range sums, point queries, and a cumulative-frequency
`find_kth` search.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "fenwick_tree.h"

double values[4] = {1, 3, 2, 4};
fenwick_tree t;
fenwick_init_from_slice(&t, values, 4);

double sum;
fenwick_prefix_sum(&t, 3, &sum);      /* 1+3+2 = 6 */
fenwick_update(&t, 2, 5.0);           /* element 2 += 5 */
fenwick_range_sum(&t, 1, 2, &sum);    /* 1 + 8 = 9    */
fenwick_free(&t);
```

Indexing is **1-based** (valid indices `1..=n`); `prefix_sum` also accepts `0`
(the empty prefix). Fallible calls return a `fenwick_status`; the value is
written through an out-parameter. The tree owns a heap allocation — pair every
`fenwick_init*` with `fenwick_free`.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/fenwick-tree`; the index walks, 1-based layout, and
`find_kth` binary lifting match the crate exactly. See also the
[C++ port](../../cpp/fenwick-tree/README.md).
