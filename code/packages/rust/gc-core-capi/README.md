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
| `int64_t __gc_alloc_kind(int64_t n, uint16_t kind)` | as above, tagging the object with a registered `kind` id → precise interior tracing (`0` = conservative) |
| `int64_t __gc_register_kind(const int64_t *field_offsets, int64_t count)` | register a ref-field map, returns a 1-based `kind` id for `__gc_alloc_kind` (precise tracing) |
| `int64_t __gc_collect_roots(const int64_t *roots, int64_t count)` | mark from `count` root words, sweep; returns objects freed |
| `int64_t __gc_collect_region(const uint8_t *base, int64_t len)` | mark from every candidate pointer in a raw region, sweep; returns objects freed |
| `int64_t __gc_collect(void)` | conservative collection rooted at this thread's live stack + callee-saved registers (no caller roots); returns objects freed |
| `int64_t __gc_safepoint(void)` | paced collect — runs `__gc_collect` only when the live set has reached the adaptive threshold; returns objects freed (0 if throttled) |
| `void __gc_write_barrier(int64_t parent, int64_t child)` | generational write barrier — records an old `parent` so a minor cycle finds its young children (O(1); `child` not dereferenced) |
| `int64_t __gc_collect_minor(void)` | minor (young-only) collection rooted at this thread's stack + registers; reclaims young garbage without scanning the old generation; returns objects freed |
| `int64_t __gc_live_bytes(void)` | live payload bytes |
| `int64_t __gc_collection_count(void)` | collections run so far |
| `void __gc_reset(void)` | drop the whole heap; free everything |
| `int64_t __gc_register_stackmap(func_start, func_len, num_records, pc_offsets, frame_sizes, callee_masks, slot_counts, slots_flat)` | register a function's stack maps (code range + per-safepoint live-ref records) for precise-root resolution; returns records stored, `0` if rejected |
| `int64_t __gc_stackmap_count(void)` | number of functions registered |
| `void __gc_stackmap_reset(void)` | drop all registered stack maps (tests / teardown) |
| `int64_t __gc_collect_precise(void)` | full collect rooted precisely at this thread's stack — walks the frame-pointer chain (stack-mapped frames precise, rest conservative); returns objects freed |

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

This is **T1 rung 0** — the linkable flat collector. It now covers the full
conservative collector `twig_gc.c` shipped, in three layers: explicit roots
(`__gc_collect_roots`), a raw memory region (`__gc_collect_region`), and the
argument-less **conservative C-stack scan** (`__gc_collect`) — plus the **paced**
`__gc_safepoint` (collect only past the adaptive threshold) that `__gc_alloc` also
drives under memory pressure. All pure Rust, no C.

`__gc_collect` is the drop-in for `twig_gc.c`'s `__twig_gc_collect`: it spills the
callee-saved registers — both integer and FP/SIMD (`d8`–`d15` on aarch64,
`xmm6`–`xmm15` on Win64), so a NaN-boxed reference held only in an FP register is
never missed — to the stack (an `asm!` block replacing `setjmp`), reads the
stack pointer, finds the thread's stack base via the platform thread API (bare
`extern` bindings — `pthread_get_stackaddr_np` on macOS, `pthread_getattr_np` /
`pthread_attr_getstack` on Linux, `GetCurrentThreadStackLimits` on Windows), and
hands `[sp, base)` to `__gc_collect_region`. Supported targets are exactly the
native-AOT ones: aarch64 (macOS) and x86_64 (Linux, Windows); anything else is a
hard `compile_error!` rather than a silent unsound fallback.

The archive also exports **twig-compat aliases** — `__twig_gc_alloc` /
`__twig_gc_collect` / `__twig_gc_safepoint` / `__twig_gc_live_bytes` /
`__twig_gc_collection_count` (module `twig_compat`) — thin wrappers over the
`__gc_*` ABI. The AOT code generators and `dynval_runtime.c` reference the names
`twig_gc.c` exported; these aliases let this archive satisfy them so `twig_gc.c`
can be retired without changing the emitters.

**Precise interior tracing** is available: `__gc_register_kind` records an
object layout's ref-field offsets and returns a `kind` id for `__gc_alloc_kind`,
so typed objects are traced exactly (only their ref fields) rather than
conservatively. **Generational** collection is wired through the C ABI too
(`__gc_write_barrier` + `__gc_collect_minor`).

**Precise roots** are now under way. The `gc-core` side — `StackMapRecord` /
`StackMapTable` and `collect_precise` (mark from exactly the slots a stack map
names) — landed in `gc-core` 0.8.0. This crate now adds the code-address half:
`__gc_register_stackmap` registers each compiled function's stack maps into a
sorted, non-overlapping registry, so an internal `resolve(return_address)` can
map any unwound return address to the `StackMapRecord` live there in `O(log n)`.
That is the code-address analogue of `__gc_register_kind` (object-layout map) —
together, the two maps a precise collector needs.

The precise stack walk is now wired end-to-end: `precise_walk::build_precise_roots`
unwinds the frame-pointer chain into precise slots (`frame_root_slots`) for mapped
frames and conservative `[fp, caller_fp)` regions for unmapped ones, and the
argument-less **`__gc_collect_precise`** captures the running thread's frame pointer
/ stack pointer / base (mirroring `__gc_collect`'s register spill), walks, and
collects both in one `collect_mixed` cycle. With no stack maps registered it degrades
to exactly `__gc_collect`; as frames are mapped they shed floating garbage. Safe by
construction — an omitted frame pointer or unwalkable stack degrades to a
conservative scan, never a missed root.

Still to come (own PRs):

- **Backend record emission** — the code generators (aarch64 / x86_64 / LLVM) emit
  `StackMapRecord`s at safepoints and call sites and register them via
  `__gc_register_stackmap`, so `__gc_collect_precise` actually resolves real frames
  in production (until then every frame is unmapped → conservative). Precision also
  needs the image built with frame pointers (`-Cforce-frame-pointers`).
- **Moving / compacting**, then **incremental** — the rest of the ladder, all as
  `gc-core` algorithms.

## Build & test

```
cargo build -p gc-core-capi     # produces libgc_core_capi.a
cargo test  -p gc-core-capi     # host tests exercising the exported ABI
```
