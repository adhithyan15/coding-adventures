# gc-core-capi

C ABI for [`gc-core`](../gc-core)'s flat-native heap — the garbage collector a
**native-AOT** executable links so its emitted heap ops (`alloc` / `field_load` /
`field_store` / `safepoint`) resolve to a real collector.

This crate compiles to `libgc_core_capi.a` (static) and `libgc_core_capi.dylib/.so`
(dynamic). It is the concrete realisation of LANG16's `gc_runtime_<target>.a`
companion archive, and it **supersedes `twig-aot/runtime/twig_gc.c`**: the same flat
mark-and-sweep model, but one generic Rust collector shared by every native consumer
(native-AOT, LLVM, WASM) instead of a Twig-specific C fork.

See the design in [`AOT00-T1-precise-gc.md`](../../../specs/AOT00-T1-precise-gc.md)
(§3.1 "two heap representations", §11 sequencing) and
[`LANG16-gc-core.md`](../../../specs/LANG16-gc-core.md).

## Where it fits

```
                 vm-core / jit-core ── gc-core (managed-object heap: Box<dyn HeapObject>)
                                         │
IIR alloc/field_*/safepoint ─ native-AOT ┴ gc-core FlatHeap (flat real-memory heap)
                                         │
                                    gc-core-capi  ── libgc_core_capi.a  (this crate)
                                         │
                              linked into the AOT'd native executable
```

The interpreters use `gc-core`'s managed-object collector; native output uses
`gc-core`'s `FlatHeap` through this C ABI. Both run the same algorithm and share
`gc-core`'s `HeapKind` / `RootSet` / `WriteBarrier` / adaptive-policy machinery.

## The ABI (`include/gc_core.h`)

| Symbol | Meaning |
|---|---|
| `int64_t __gc_alloc(int64_t n)` | allocate `n` zeroed bytes; returns a real pointer (as `int64`), `0` on failure/`n<=0` |
| `int64_t __gc_alloc_kind(int64_t n, uint16_t kind)` | as above, tagging the object with a `HeapKind` id (for later precise tracing) |
| `int64_t __gc_collect_roots(const int64_t *roots, int64_t count)` | mark from `count` root words, sweep; returns objects freed |
| `int64_t __gc_collect_region(const uint8_t *base, int64_t len)` | mark from every candidate pointer in a raw region, sweep; returns objects freed |
| `int64_t __gc_live_bytes(void)` | live payload bytes |
| `int64_t __gc_collection_count(void)` | collections run so far |
| `void __gc_reset(void)` | drop the whole heap; free everything |

`__gc_alloc` returns a **real, 16-byte-aligned pointer** to memory the caller reads
and writes directly. Tracing is conservative (raw + tag-stripped candidate words);
a live object is never freed. The heap is single-threaded (matching `twig_gc.c`), one
process-wide instance.

## Usage (C)

```c
#include "gc_core.h"

int64_t cell = __gc_alloc(16);        /* a cons cell: [car][cdr] */
*(int64_t *)cell = 42;                /* car = 42 */
int64_t roots[1] = { cell };
__gc_collect_roots(roots, 1);         /* cell is rooted → survives */
```

## Status

This is **T1 rung 0** — the linkable flat collector with explicit-root and
region-scan collection. `__gc_collect_region` is the platform-independent core of
the conservative stack scan: give it any span of raw memory and it roots from every
candidate pointer inside. Still to come (own PRs):

- Wire `twig-aot`'s `build.rs` to link this archive and retire `twig_gc.c`
  (golden/smoke parity).
- The argument-less **C-stack scan** so `collect` runs with no explicit roots (the
  drop-in for `twig_gc.c`'s `__twig_gc_collect`): discover the current stack pointer
  and the thread's stack base, spill callee-saved registers, and hand that span to
  `__gc_collect_region`.
- **Precise** roots (stack maps) and interior tracing (`HeapKind` field maps),
  then moving / generational — all as `gc-core` algorithms.

## Build & test

```
cargo build -p gc-core-capi     # produces libgc_core_capi.a
cargo test  -p gc-core-capi     # host tests exercising the exported ABI
```
