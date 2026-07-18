# Changelog — gc-core-capi

All notable changes to this crate are documented here.

## [0.7.0] — 2026-07-17

### Added

- **`__gc_register_kind(field_offsets, count) -> i64`** — C ABI over
  `FlatHeap::register_kind`. Registers a reference-field map (the byte offsets of
  an object layout's `ref`-typed fields) and returns a 1-based `kind` id to pass
  to `__gc_alloc_kind`. Objects of that kind are traced **precisely** — only the
  mapped offsets are followed during marking — so a look-alike-pointer integer in
  a non-reference field can't keep a dead object alive. This is the seam a native
  runtime / language frontend uses to teach the collector its object layouts
  (records, tuples, Ruby/Python/JS objects). Negative offsets are ignored; a null
  list / `count <= 0` registers a no-ref-field (opaque) kind. Host test proves a
  precise collect reclaims a pointee referenced only via a non-ref field.

## [0.6.0] — 2026-07-17

### Changed

- **Conservative stack scan now spills callee-saved FP/SIMD registers too**
  (aarch64 `d8`–`d15`; Win64 `xmm6`–`xmm15`, low 64 bits via `movsd`). System V
  x86-64 has no callee-saved xmm, so its path is unchanged. Closes the
  missed-root gap flagged in the #118b-1.5 review: a NaN-boxing runtime may keep
  a managed reference as an `f64` in a callee-saved FP register across a
  safepoint/alloc call; scanning only integer registers could miss it and free a
  live object (use-after-free). `twig_gc.c`'s `setjmp` saved these registers on
  exactly these ABIs — this restores parity. `SPILL_SLOTS` grows 10 → **18**
  words (the max across ABIs: aarch64 10 int + 8 FP; Win64 8 int + 10 FP), sized
  to the largest set so the spill never writes out of bounds. Runtime-validated
  on aarch64 (native, exercises `d8`–`d15`) + x86-64 SysV (Rosetta); Win64
  validates in CI. Same conservative semantics — a stale FP register is at worst
  a one-cycle false positive, never unsound.

## [0.5.0] — 2026-07-17

### Added

- **`__twig_gc_*` ABI aliases** (new module `twig_compat`) — `__twig_gc_alloc`,
  `__twig_gc_collect`, `__twig_gc_safepoint`, `__twig_gc_live_bytes`,
  `__twig_gc_collection_count`, thin `#[no_mangle]` wrappers forwarding to the
  generic `__gc_*` ABI. The native-AOT code generators (aarch64 + LLVM backends)
  and `dynval_runtime.c` reference the symbol names `twig_gc.c` exported; these
  aliases let `libgc_core_capi.a` satisfy those references so `twig_gc.c` can be
  retired without touching the emitters. Prototypes match `twig_gc.c` exactly,
  including the `void`-returning `__twig_gc_collect` / `__twig_gc_safepoint` (the
  generic entry points' freed-count is discarded). Verified exported as text
  symbols in the built staticlib; host test drives the full flow. Pure Rust, no C
  shim; deletable once the emitters emit `__gc_*` directly.

## [0.4.0] — 2026-07-17

### Added

- **`__gc_safepoint()`** — the throttled, paced collect (drop-in for `twig_gc.c`'s
  `__twig_gc_safepoint`). Runs `__gc_collect` only when the heap has reached its
  adaptive threshold (`FlatHeap::should_collect`), else returns `0`. The native
  backend emits a `safepoint` op at loop back-edges / function entries; collecting
  at every one would be ruinous, so each merely asks "over threshold yet?" —
  keeping GC cost proportional to allocation and stopping a tight allocation loop
  from ever starving the collector.

### Changed

- **`__gc_alloc` / `__gc_alloc_kind` now collect under pressure** — before
  allocating, if `should_collect()`, they run a conservative stack-scan collect
  (matching `__twig_gc_alloc`). Collecting *before* the new allocation means the
  new object does not exist yet and cannot be wrongly reclaimed; every root that
  must survive is the caller's, already live on the scanned stack. Below the 1 MiB
  threshold (host tests, light workloads) this path is never taken — allocation
  stays a plain bump. Host tests serialised via a shared `TEST_LOCK` (the
  process-wide `HEAP` is now touched by more than one test).

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
