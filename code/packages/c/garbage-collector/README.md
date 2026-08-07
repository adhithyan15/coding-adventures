# garbage-collector (C)

A language-agnostic **mark-and-sweep garbage collector** in pure ISO C17. A
faithful port of the Rust [`garbage-collector`](../../rust/garbage-collector)
crate.

## What it does

A tracing GC finds and reclaims unreachable objects:

1. **Mark** — from the roots, follow every reference and mark reachable objects
   (the already-marked guard makes reference cycles terminate).
2. **Sweep** — free every object that was not marked.
3. **Reset** — clear the marks on survivors for the next cycle.

Heap objects are cons cells, interned symbols, or Lisp closures; each reports
the heap addresses it references. Roots are `GcValue`s — only address-like
values are followed. Addresses increase monotonically from `0x10000` (so they
never collide with small program integers) and are never reused.

## API

- Objects: `gc_cons_new`, `gc_symbol_new`, `gc_closure_new`, `gc_object_free`,
  `gc_object_type_name`, `gc_object_references`.
- Collector: `gc_new` / `gc_free`, `gc_allocate` (takes ownership → address),
  `gc_deref`, `gc_collect(roots)`, `gc_heap_size`, `gc_is_valid_address`,
  `gc_stats`.
- Roots: `gc_val_int` / `gc_val_address` / `gc_val_str` / `gc_val_bool` /
  `gc_val_nil` / `gc_val_list`, and `gc_value_free`.
- `GcSymbolTable`: `intern`, `lookup`, `count`, `contains` — interns symbols so
  equal names share one address.

## Design notes

- **Slot-array heap.** Because addresses are monotonic and never reused, the
  heap is a grow-only array where address `A` lives in slot `A - 0x10000`; a
  swept object leaves a NULL slot. This mirrors the Rust crate's incrementing
  `next_address` exactly (`0x10000`, `0x10001`, …) without a hash map.
- **Trait → tagged union.** The Rust `HeapObject` trait's three implementors
  become a tagged `GcObject`; `references()` and `type_name()` switch on the tag.
- Growable buffers guard against `size_t` overflow.

## Usage

```c
#include "garbage_collector.h"

GcHeap *gc = gc_new();
size_t a1 = gc_allocate(gc, gc_cons_new(42, -1));
gc_allocate(gc, gc_symbol_new("unreachable"));

GcValue roots[1] = {gc_val_address(a1)};
size_t freed = gc_collect(gc, roots, 1);   /* 1 — the symbol is unreachable */

gc_free(gc);
```

## Building

```sh
sh BUILD           # POSIX: gcc and/or clang via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
