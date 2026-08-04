# skip-list (C)

An **ordered map** of `int` keys to `int` values, in pure ISO C17. A faithful
port of the Rust `skip-list` crate — which is itself an ordered map that reports
skip-list-style parameters (`max_level`, `probability`, a derived
`current_max`). Sorted-array backed: O(log n) lookup, order statistics, and
range queries.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "skip_list.h"

skiplist s;
skiplist_init(&s);
skiplist_insert(&s, 5, 50);
skiplist_insert(&s, 1, 10);

int v; size_t r;
skiplist_search(&s, 5, &v);        /* v = 50 */
skiplist_rank(&s, 5, &r);          /* r = 1 (0-based order) */
skiplist_range(&s, 1, 5, 1, cb, ud); /* inclusive range, sorted, via callback */
skiplist_free(&s);
```

Keys are kept sorted. `current_max` is derived as `ceil(log_{1/p}(n))` clamped
to `[1, max_level]`, computed without `<math.h>` (no libm). The map owns its
storage — pair `skiplist_init` with `skiplist_free`.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/skip-list`; enumeration uses a visitor callback (the
idiomatic C form). See also the [C++ port](../../cpp/skip-list/README.md).
