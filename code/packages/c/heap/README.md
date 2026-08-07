# heap (C)

A **binary heap (priority queue)** of ints, in pure ISO C17. A faithful port of
the Rust `heap` crate (MinHeap / MaxHeap), plus an in-place `heap_sort`.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "heap.h"

heap h;
heap_init(&h, HEAP_MIN);      /* or HEAP_MAX */
heap_push(&h, 5);
heap_push(&h, 1);
heap_push(&h, 3);

int top;
heap_peek(&h, &top);          /* 1 (min at the root) */
heap_pop(&h, &top);           /* 1; draining yields ascending order */
heap_free(&h);

int arr[4] = {3, 1, 4, 1};
heap_sort(arr, 4);            /* {1, 1, 3, 4} */
```

The heap owns a growable array — pair `heap_init` with `heap_free`. Fallible
operations return `1` on success and `0` on failure (empty pop/peek, or an
allocation failure on push).

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/heap`; the sift-up/sift-down logic matches the crate.
`nlargest`/`nsmallest` (which return growable lists) are provided in the
[C++ port](../../cpp/heap/README.md). See also that port for the generic form.
