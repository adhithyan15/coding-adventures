# ringbuf

A fixed-capacity **ring (circular) buffer of ints**, in pure ISO C17. It stores
up to a fixed number of elements over a caller-supplied backing array and wraps
the indices around the ends, giving O(1) FIFO `push`/`pop` with no allocation
and no element shifting.

It is a sample package for the repo's C/C++ multi-compiler lane: it compiles and
runs under **GCC, Clang, and MSVC** with strict ISO-conformance flags
(`-pedantic-errors` / `/permissive-`, warnings-as-errors), via the shared
[`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "ringbuf.h"

int storage[4];
ringbuf r;
ringbuf_init(&r, storage, 4);   /* caller owns `storage` */

ringbuf_push(&r, 10);
ringbuf_push(&r, 20);

int value;
ringbuf_pop(&r, &value);        /* value == 10 (FIFO) */
```

### API

| Function | Purpose |
| --- | --- |
| `ringbuf_init(r, backing, cap)` | bind to a caller-owned array of `cap` ints |
| `ringbuf_push(r, v)` | append `v`; returns 0 if full |
| `ringbuf_pop(r, &out)` | remove oldest into `out`; returns 0 if empty |
| `ringbuf_peek(r, &out)` | read oldest without removing; returns 0 if empty |
| `ringbuf_count(r)` / `ringbuf_capacity(r)` | used / total slots |
| `ringbuf_is_empty(r)` / `ringbuf_is_full(r)` | state predicates |
| `ringbuf_clear(r)` | drop all elements |

The buffer does not own memory — the caller supplies (and outlives) the backing
array, so it works with stack, static, or heap storage.

## Development

```bash
# Compile + run the tests under every C compiler present (gcc, clang; MSVC on
# Windows), each with strict ISO-conformance flags:
sh BUILD
```

## Where it fits

Part of the C/C++ multi-compiler lane — see
[`code/specs/CCPP01-c-cpp-iso-multicompiler-lane.md`](../../../specs/CCPP01-c-cpp-iso-multicompiler-lane.md)
and the shared [`iso-harness`](../iso-harness/README.md).
