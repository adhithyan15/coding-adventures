# Changelog — gc-core-capi

All notable changes to this crate are documented here.

## 0.19.0 - 2026-07-28 — C ABI `__gc_register_ref_array_kind` — PR-3 (AOT00-T5)

- **`__gc_register_ref_array_kind(fixed, fixed_count, tail_from) -> kind_id`** (new `#[no_mangle]`
  export + `gc_core.h` declaration) — the C-ABI seam for `gc_core::FlatHeap::register_ref_array_kind`
  (gc-core 0.23.0). Registers a **variable-length reference-array** kind: `fixed_count` fixed
  ref-field offsets followed by a tail region where every aligned 8-byte word in `[tail_from, size)`
  of an instance is a reference. One kind describes arrays of every length (the tail follows the
  instance's alloc size), so a native runtime's JS/Ruby/Python array, vector, or hash backing store
  is traced — and under the compacting collector **relocated** — precisely instead of conservatively
  (a conservative array pins itself and every element it references, defeating compaction). Mirrors
  `__gc_register_kind`: `fixed` null / `fixed_count <= 0` ⇒ tail-only; negative fixed offsets are
  ignored; a negative `tail_from` is treated as `0`. Layout contract (documented): every word in
  the tail must be a reference (base/tagged-base or null).
- **Test:** `c_abi_register_ref_array_kind_traces_tail` — one kind serves a length-2 and a length-3
  array; an element is retained via a tail slot and reclaimed once that slot is cleared (proving the
  tail, not another path, kept it alive).
- PR-3 of 4 (AOT00-T5 §7); PR-4 wires the native backend + an end-to-end relocation differential.

## 0.18.0 - 2026-07-27 — twig-compat aliases `__twig_gc_collect_incremental_*` (frontend incremental GC)

- **`__twig_gc_collect_incremental_{start,step,finish}`** (new `twig_compat` aliases →
  `__gc_collect_incremental_*`) — the `__twig_gc_*` linker names a native code generator emits
  for the `gc_collect_incremental_*` builtin trio, so a compiled program can drive the
  bounded-pause incremental cycle. Mirror `__twig_gc_collect_compacting`. No new `unsafe`
  beyond the delegated calls.

## 0.17.0 - 2026-07-25 — incremental collector C ABI `__gc_collect_incremental_{start,step,finish}` (AOT00-T4 §6)

- **Three new exports in `stack_scan.rs`** — the bounded-pause incremental collection cycle,
  driven as `start(); while (!step(BUDGET)) { …mutator… } finish();`:
  - `__gc_collect_incremental_start()` captures the precise roots **once** via the *same*
    frame-pointer walk as `__gc_collect_precise` (precise slots + conservative regions + the
    spilled callee-saved registers) and shades them grey. Reference stores the mutator makes
    *between* steps are caught by `__gc_write_barrier`'s incremental shading (the Dijkstra
    insertion barrier landed in gc-core 0.19.0) — no capi change needed there, it already
    forwards to `FlatHeap::write_barrier`.
  - `__gc_collect_incremental_step(budget) -> i64` advances marking by up to `budget` objects,
    returning `1` when complete / `0` otherwise (negative budget → 0).
  - `__gc_collect_incremental_finish() -> i64` sweeps the unreachable objects and returns the
    count reclaimed.
  - **Untrustworthy stack ⇒ safe no-op cycle:** if `start` can't trust the stack it enters no
    phase; `step` then returns `1` immediately and `finish` returns `0` (guarded by
    `FlatHeap::incremental_in_progress()`), so nothing is swept — the same bias-to-leak as
    `__gc_collect_precise`. Declared in `include/gc_core.h`.
- **Tests (+2):** an end-to-end smoke test drives the real start→step(budget 1)→finish protocol
  on this thread's stack (a live local is kept, a dead object reclaimed, several slices) + a
  no-phase-safety test (step/finish without a start free nothing). Real-stack asm path isn't
  Miri-able (same limitation as the `__gc_collect_precise` tests); the underlying gc-core
  `incremental_*` logic is Miri-clean.

## 0.16.0 - 2026-07-24 — twig-compat alias `__twig_gc_collect_compacting` (frontend GC.compact)

- **`__twig_gc_collect_compacting()`** (new `twig_compat` alias → [`__gc_collect_compacting`]) —
  the `__twig_gc_*` linker symbol a native code generator emits for the `gc_collect_compacting`
  builtin, so a compiled program can trigger a moving/compacting collection. Mirrors
  `__twig_gc_collect_precise`. No new `unsafe` beyond the delegated call.

## 0.15.0 - 2026-07-24 — moving/compacting collector C ABI `__gc_collect_compacting` (AOT00-T3 §5)

- **`__gc_collect_compacting()`** (new export in `stack_scan.rs`) — the argument-less
  **relocating** collection rooted precisely at this thread's stack: the relocating analogue
  of `__gc_collect_precise`. It runs the *same* frame-pointer walk (`build_precise_roots` →
  precise slots for stack-mapped frames + conservative regions for the rest, plus the spilled
  callee-saved registers), then drives `gc_core::FlatHeap::collect_compacting` instead of
  `collect_mixed`. Movable survivors (reachable purely precisely, no conservative in-edge) are
  evacuated into an arena and every pointer that named them is rewritten — including writing
  the forwarded address back into its precise root slot, so the mutator's stack points at the
  relocated objects on return. With no stack maps registered nothing is movable and it
  degrades to exactly `__gc_collect_precise`, so it is always safe to call. Declared in
  `include/gc_core.h`.
- **Tests (+2):** an end-to-end smoke test on a real thread stack (`__gc_collect_compacting`
  keeps a live local, frees a dead object, never crashes) and a synthetic-stack **relocation
  differential** (`walk_output_drives_compacting_collection_and_relocates`): a precisely-named
  registered-kind object with no conservative in-edge is *moved* — its address in the root
  slot is rewritten to the new arena location — while an unnamed sibling in the same mapped
  frame is reclaimed; `live_bytes` preserved.

## 0.14.0 - 2026-07-23 — generational tenuring-age C ABI

Exposes gc-core 0.11.0's tunable generational **tenuring age** across the C ABI so native
consumers can tune how many collections a young object survives before promotion:

- `void __gc_set_tenure_age(int64_t threshold)` → `FlatHeap::set_tenure_age`. The `i64`
  argument is clamped to `1..=255` (`0`/negatives → `1`, over-large → `255`) before the
  `u8` cast, so tenuring always terminates.
- `int64_t __gc_tenure_age(void)` → the current threshold (default `1` = immediate
  tenuring).

Both are thin `with_heap` wrappers over the process-wide heap, declared in `gc_core.h`.
One host test (`c_abi_set_and_get_tenure_age_clamps`) covers the round-trip + clamp; the
aging *behaviour* is covered by gc-core's own tests. Purely additive; no existing symbol
changes. gc-core-capi 0.13.0 → 0.14.0.

## 0.13.0 - 2026-07-21 — twig-compat precise/observability aliases (AOT00-T1 increment C)

Two `__twig_gc_*` aliases the native code generators emit for the GC-stress
`live_bytes` differential that makes precise roots observable end to end:

- **`__twig_gc_collect_precise()`** → `__gc_collect_precise` — a full collection
  rooted **precisely** at the caller's stack via the frame-pointer walk (mapped
  frames contribute exact reference slots, the rest conservative). Returns the freed
  count. `#[inline(never)]`, same stack-ownership contract as `__gc_collect_precise`.
- **`__twig_gc_stackmap_count()`** → `__gc_stackmap_count` — the number of registered
  functions, a diagnostic that confirms an AOT image's `__gc_init_stackmaps` ran.

Additive (`twig_compat` only); one new test. Enables the twig-aot `gc_collect_precise`
/ `gc_stackmap_count` builtins.

## [0.12.0] — 2026-07-22

### Added

- **`__gc_register_stackmap_module(entries, n)` + `GcStackmapModuleEntry`** — the
  batch registration entry a native-AOT image's start-up path invokes to register
  **every** function's stack map in one call. It is a thin, allocation-free loop over
  `__gc_register_stackmap`, one call per `#[repr(C)]` `GcStackmapModuleEntry` (each a
  1:1 mirror of the `__gc_register_stackmap` arguments). This is what lets an image
  emit its whole stack-map table as one `.rodata` array of entries plus a single
  start-up `bl` — far cheaper to generate than an unrolled per-function call
  sequence — so `__gc_collect_precise` can finally resolve real frames instead of
  falling back to a conservative scan (`AOT00-T1-stackmap-emission.md`, the
  registration half of the emission rung). Returns the total records registered; a
  null table or `n <= 0` is a no-op. 2 unit tests (multi-function round-trip through
  `resolve`, degenerate-input inertness) + a C `gc_core.h` declaration.


## [0.11.0] — 2026-07-18

### Added

- **`__gc_collect_precise()` — the argument-less precise-root collect entry** (the
  `asm!` half that gives the precise machinery a real machine stack to walk). It is
  to `collect_mixed` what `__gc_collect` is to `collect_region`:
  - Spills callee-saved registers (via the same `spill_and_sp` as `__gc_collect`),
    captures the current **frame pointer** (`x29` / `rbp`, via a new
    `#[inline(always)]` `current_fp`) and the stack base, then hands `fp`/`sp`/`base`
    to `precise_walk::build_precise_roots`. The resulting precise **slots**
    (stack-mapped frames) and conservative **regions** (unmapped frames) go to
    `gc_core::FlatHeap::collect_mixed` in one cycle. The spilled registers are also
    handed over as an explicit conservative region — a reference live only in a
    callee-saved register is named by no stack map yet (that needs a
    `callee_saved_mask`, a later rung), so it must be scanned, exactly as
    `__gc_collect` scans it.
  - **Opportunistic + safe:** with no stack maps registered, every frame resolves
    conservatively and the regions tile all of `[sp, base)`, so it degrades to
    exactly `__gc_collect` — safe even if the captured frame pointer is garbage. As
    backends register maps, matching frames shed floating garbage; at that point a
    *valid* frame pointer becomes load-bearing (a garbage anchor whose `[fp+8]`
    aliased a stale return address into a mapped function could exclude a live span),
    so the map-emitting rung must build this crate with frame pointers (guaranteed by
    ABI on the aarch64 primary target; tracked as a prerequisite of that rung). If
    the stack base can't be established it collects **nothing** this cycle
    (bias-to-leak, matching `__gc_collect`).
  - Smoke-tested end-to-end (`precise_collect_keeps_live_local_frees_dead`): a live
    stack local survives, a dead object is reclaimed, and the asm-capture →
    frame-walk → `collect_mixed` path runs without crashing. (The precise reclaim of
    a named slot vs. an unnamed neighbour is proven by `precise_walk`'s
    synthetic-stack tests from 0.10.0.)

## [0.10.0] — 2026-07-18

### Added

- **Precise stack walk — the frame-pointer-chain root builder** (the walk-logic
  half of the native precise-root collection; the `asm!` entry that captures the
  running thread's registers and calls it is a follow-up):
  - Internal `precise_walk::build_precise_roots(start_fp, sp, base, &mut slots,
    &mut regions)` — walks the frame-pointer chain from `start_fp` toward the stack
    `base`, classifying each frame into the two inputs of `gc-core`'s
    `FlatHeap::collect_mixed`: a **mapped** return address (resolved via the
    stack-map registry) contributes exact precise slots (`frame_root_slots` relative
    to the caller's frame pointer); an **unmapped** one contributes its frame span
    `[fp, caller_fp)` as a conservative region. The collector's own frames below the
    first frame pointer (`[sp, start_fp)`) are always conservative.
  - The precise-root analogue of the fully-conservative `__gc_collect` C-stack scan:
    where that hands the whole `[sp, base)` span to `collect_region`, this classifies
    the stack frame-by-frame, so mapped frames shed their floating garbage while
    everything else stays exactly as safe as the conservative scan.
  - **Soundness (no missed root → no use-after-free):** the union of everything
    emitted covers every stack word that could be a heap reference. A mapped frame
    only excludes its non-reference locals and the saved-fp / return-address words
    (never heap pointers). A broken/again-unmappable chain link falls back to
    conservatively scanning the entire remaining `[fp, base)` before stopping.
  - **Guards (all fail *safe*, never dropping a root):** `fp + 16 <= base` keeps the
    two frame-pointer reads in-bounds; a `caller_fp` not strictly above `fp` (stack
    grows down) or outside the stack is rejected (chain end / corruption), which also
    guarantees termination; a `MAX_FRAMES` backstop bounds the loop unconditionally,
    and **both** a rejected link and budget exhaustion fall through to a conservative
    scan of the remaining `[fp, base)`. A `start_fp` outside `[sp, base]` (or
    degenerate `sp`/`base`) falls back to a whole-stack conservative scan rather than
    walking nothing — defense-in-depth for the `asm!` entry that feeds real registers.
  - Pure walk logic — **no `asm!`, no real thread stack** — so it is exhaustively
    unit-tested against *synthetic* stacks: all-unmapped (regions tile the stack),
    all-mapped (precise slots), mixed, backward / out-of-range `caller_fp` rejection,
    and an end-to-end drive through `collect_mixed` proving a precisely-named object
    survives while an unnamed local inside the same (excluded) mapped frame is
    reclaimed. `build_precise_roots` carries `#[allow(dead_code)]` until the `asm!`
    entry consumes it, exactly as `gc-core` shipped `collect_mixed` ahead of its
    consumer.

## [0.9.0] — 2026-07-18

### Added

- **Stack-map registry — the code-address → live-reference lookup for precise
  roots** (the format + lookup half of the native precise-root walk; the gc-core
  data structures landed in gc-core 0.8.0):
  - **`__gc_register_stackmap(func_start, func_len, num_records, pc_offsets,
    frame_sizes, callee_masks, slot_counts, slots_flat) -> i64`** — registers one
    compiled function's stack maps (its code range plus per-safepoint records, as
    parallel flattened arrays; `slots_flat` is demultiplexed record-by-record
    through `slot_counts`). Returns records stored, or `0` if rejected (`func_len
    == 0`, `func_len > u32::MAX` (a `pc_offset` is a `u32`), `num_records <= 0`, a
    required array null, `slots_flat` null while a record claims a positive count
    (fail-loud against under-marking a safepoint), the range wraps, or it overlaps
    an already-registered function). `frame_sizes`/`callee_masks` may be null (zero)
    and are carried for the walker; a negative `slot_counts[i]` is clamped to `0`.
  - **`__gc_stackmap_count()`** / **`__gc_stackmap_reset()`** — introspection and
    (test/teardown) clearing of the registry.
  - The registry keeps functions **sorted by code address and non-overlapping**, so
    an internal `resolve(return_address)` (consumed by the precise stack walker in a
    follow-up PR) finds the containing function in `O(log n)`, computes its
    `pc_offset`, and returns the `StackMapRecord` live there — or `None` for an
    unmapped address (a C-runtime frame or un-migrated backend), which the walker
    scans conservatively. This is the code-address analogue of `__gc_register_kind`
    (object-layout map); together they are the two maps a precise collector needs.
  - Pure-Rust: no `asm!`, no machine-stack dereference (that is the walker's job).
    The only `unsafe` is reading the caller's parallel arrays, under the same
    C-array contract as `__gc_register_kind`. 8 unit tests cover resolution,
    binary-search function selection, slot demultiplexing, overlap/degenerate
    rejection, and negative-count clamping.

## [0.8.0] — 2026-07-18

### Added

- **Generational C ABI** — completes the generational collector for the native
  path (the gc-core algorithm landed in gc-core 0.7.0):
  - **`__gc_write_barrier(parent, child)`** — the generational write barrier the
    native runtime calls on every heap-reference store; records an old parent so a
    minor cycle finds the young objects it points to. O(1); `child` not
    dereferenced. Wraps `FlatHeap::write_barrier`.
  - **`__gc_collect_minor()`** — a minor (young-only) collection rooted at this
    thread's live stack + callee-saved registers (same register-spill + stack-base
    discovery as `__gc_collect`), reclaiming young garbage without scanning the old
    generation. Wraps `FlatHeap::collect_minor_region`.
  - Host test drives the full ABI flow (barrier records an old→young store, minor
    collect retains the child + reclaims young garbage); runtime-validated on
    aarch64 + x86-64 SysV.

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
