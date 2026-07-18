# Changelog — gc-core-capi

All notable changes to this crate are documented here.

## [0.3.0] — 2026-07-17

### Added

- **`__gc_collect()`** — the argument-less conservative C-stack scan (new module
  `stack_scan`). The drop-in for `twig_gc.c`'s `__twig_gc_collect`: roots from this
  thread's live stack + callee-saved registers with **no caller-supplied roots**,
  the way the native backend's collect/safepoint points call it. Pure Rust, no C:
  a per-arch `asm!` block spills callee-saved integer registers to the stack
  (replacing `setjmp`), reads the stack pointer, finds the thread's stack base via
  bare `extern` bindings to the platform thread API (`pthread_get_stackaddr_np` on
  macOS, `pthread_getattr_np`/`pthread_attr_getstack` on Linux,
  `GetCurrentThreadStackLimits` on Windows), and hands `[sp, base)` to
  `__gc_collect_region`. Supported targets are exactly the native-AOT ones —
  aarch64 (macOS), x86_64 (Linux, Windows); any other `(arch, os)` is a hard
  `compile_error!`, never a silent unsound fallback. A `MAX_STACK_SCAN` (256 MiB)
  ceiling fences off a bogus stack base (algorithmic-DoS guard). Register-spill
  asm runtime-validated on both aarch64 and x86_64 (SysV); 3 unit tests (live
  stack local survives + dead object freed; SP sanity vs. stack base; real
  `FlatHeap`).
- With this, `gc-core-capi` covers the **whole** conservative collector
  `twig_gc.c` shipped — explicit roots, region scan, and stack scan — in generic
  Rust. Next PR wires `twig-aot` to link the archive and retires `twig_gc.c`.

## [0.2.0] — 2026-07-17

### Added

- **`__gc_collect_region(base, len)`** — C ABI over `FlatHeap::collect_region`: mark
  from every candidate pointer in a raw memory region, then sweep; returns objects
  freed. The primitive a native runtime uses to root from memory it must scan itself
  (a spilled register block, or the call stack between SP and the thread's stack
  base). The argument-less `__twig_gc_collect`/`safepoint` drop-ins (a follow-up)
  discover that stack range and hand it here. Host test extended to drive it.

## [0.1.0] — 2026-07-16

Initial release. C ABI for `gc-core`'s flat-native heap — LANG16's
`gc_runtime_<target>.a` companion, superseding `twig-aot/runtime/twig_gc.c`
(AOT00 T1, spec `AOT00-T1-precise-gc.md` §3.1 / §11).

### Added

- `libgc_core_capi.a` / `.dylib` / `.so` exposing a stable C ABI over
  `gc_core::FlatHeap`:
  - `__gc_alloc(n)` — real-pointer allocation of `n` zeroed, 16-byte-aligned bytes.
  - `__gc_alloc_kind(n, kind)` — as above with a `HeapKind` id for later precise
    tracing.
  - `__gc_collect_roots(roots, count)` — explicit-root mark-and-sweep; returns
    objects freed.
  - `__gc_live_bytes()` / `__gc_collection_count()` — introspection.
  - `__gc_reset()` — drop the whole heap.
- `include/gc_core.h` C header.
- Host test exercising the full alloc → write → collect → reset flow over the
  exported ABI (real pointers, conservative tracing, exact reclamation).

### Notes

- One process-wide heap behind a `Mutex` (single-threaded native runtime model,
  matching `twig_gc.c`).
- This is **T1 rung 0**: the linkable collector with explicit-root collection.
  Wiring `twig-aot` to link it (retiring `twig_gc.c`), the conservative C-stack
  scan (argument-less `collect`), and the precise/moving/generational rungs are
  follow-up PRs.
