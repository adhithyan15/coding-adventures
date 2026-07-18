# Changelog — gc-core

## 0.5.0 — 2026-07-17

### Added

- **`FlatHeap` precise interior tracing via reference-field maps** — the first
  rung of the precision ladder (conservative → precise) and the step that lets
  the collector serve typed-object languages. `register_kind(field_offsets) ->
  u16` records the ref-field byte offsets for one object layout and returns a
  1-based `kind` id; `scan_payload` now follows **only** those offsets for objects
  allocated with that kind (via `alloc(n, kind)`), instead of scanning every
  payload word conservatively. So a look-alike-pointer integer sitting in a
  non-reference field no longer pins a phantom child. Kind id `0` stays reserved
  for "opaque / trace conservatively", so existing `alloc(n, 0)` behaviour is
  unchanged; an unregistered kind id or an offset that runs past an object's
  payload safely falls back / is skipped (never an out-of-bounds read, never
  under-traces). New `registered_kinds()`. 6 unit tests, including the headline
  precise-reclaims-phantom vs. conservative-retains-phantom contrast.

## 0.4.0 — 2026-07-17

### Added

- **`FlatHeap` adaptive collection threshold** — the GC-pacing policy ported from
  `twig_gc.c`. New `collect_threshold` field (starts at `INITIAL_THRESHOLD` = 1
  MiB), plus `should_collect()` (live bytes ≥ threshold → a cycle is due) and
  `collect_threshold()`. After every cycle, `collect`/`collect_region` re-tune the
  threshold (`adapt_threshold`): **double** it (capped at `MAX_THRESHOLD` = 256
  MiB) when >½ the pre-cycle live set survived, else **halve** it (floored at 1
  MiB). The 256 MiB cap is a safety bound — without it a live-heavy program could
  grow the threshold toward `usize::MAX` and effectively disable the GC (a
  memory-exhaustion vector). `should_collect` is the *policy* half (when to
  collect); the *mechanism* half (finding roots) stays with the caller —
  `gc-core-capi`'s `__gc_safepoint` / `__gc_alloc` drive collection off it. New
  `pub const INITIAL_THRESHOLD` / `MAX_THRESHOLD`. 7 unit tests.

## 0.3.0 — 2026-07-17

### Added

- **`FlatHeap::collect_region`** — conservative collection rooted at a raw memory
  region `[base, base+len)` (plus its `mark_region` helper). Where `collect` takes a
  tidy slice of `usize` roots, `collect_region` scans arbitrary memory the collector
  must root from itself — a block of spilled callee-saved registers, or the machine
  call stack between the stack pointer and the thread's stack base. It is the
  platform-independent, unit-tested core the argument-less native
  `__twig_gc_collect`/`safepoint` entry points build on (stack-range discovery is
  layered separately). Same conservative semantics as `collect` (raw + low-3-bit
  tag-stripped candidates; false positives retain, never free-live). 5 unit tests
  (region-rooted survives / unrooted freed, tagged refs, empty region, transitive
  interior tracing).

### Added

- **`flat_heap` module + `FlatHeap`** — a real-memory mark-and-sweep collector
  (the second heap representation described in `AOT00-T1-precise-gc.md` §3.1).
  Where `GcCore` models the heap as `HashMap<usize, Box<dyn HeapObject>>` for the
  interpreters, `FlatHeap` allocates a real machine pointer per object (32-byte
  header + payload, 16-byte-aligned) so **native-AOT** output can read/write it
  directly at byte offsets. `alloc`/`collect` (explicit roots, conservative
  tracing — raw + tag-stripped candidate words), live-byte and collection
  accounting folded into `GcProfile`. This is the generic home of
  `twig-aot/runtime/twig_gc.c`'s flat model, shared by every native consumer
  (native-AOT / LLVM / WASM) via the new `gc-core-capi` C ABI. Re-exported at the
  crate root as `gc_core::FlatHeap`.

### Added

- `HeapRef` — opaque, newtype-wrapped heap address with null sentinel
  (`HeapRef::NULL = 0`). Display impl shows `null` / `ref(0x…)`.
- `HeapKind` / `KindRegistry` — layout descriptors (size, field offsets,
  type name, finalizer flag) stored in a sequentially-numbered registry;
  kind ids map directly to the second operand of the IIR `alloc` opcode.
  `HeapKind::opaque(size, name)` convenience for objects with no ref fields.
- `GcCycleStats` — per-cycle snapshot: freed, survived, pause_ns,
  heap_size_before, heap_size_after, survival_ratio().
- `GcProfile` — accumulated metrics over all cycles:
  - total_allocations, total_bytes_allocated
  - total_collections, total_freed, total_survived
  - max_pause_ns, total_pause_ns, avg_pause_ns()
  - peak_heap_size, last_survival_ratio, ema_survival_ratio (α=0.2),
    last_fragmentation
  - allocs_since_last_gc, peak_allocs_between_gc, avg_allocs_per_gc()
  - Algorithm-recommendation predicates: suggests_generational(),
    suggests_compacting(), suggests_incremental(), suggests_heap_growth()
  - summary() for diagnostic display
- `GcAlgorithm` enum: MarkAndSweep (available), Compacting/Generational/
  Incremental (planned stubs). is_available(), name().
- `PolicyDecision` enum: Continue | SuggestSwitch(GcAlgorithm, reason).
- `GcPolicy` trait — single evaluate(&GcProfile) → PolicyDecision.
- `DefaultPolicy` — always returns Continue; safe for tests and short runs.
- `AdaptivePolicy` — recommends based on configurable thresholds:
  - Pause > max_pause_ns_threshold → Incremental
  - EMA survival < generational_survival_threshold → Generational
  - Fragmentation > compacting_fragmentation_threshold → Compacting
  - min_cycles_before_advice prevents spurious early recommendations
- `GcAdapter` — wraps any `GarbageCollector` (garbage-collector crate):
  - GcAdapter::mark_and_sweep() convenience constructor
  - GcAdapter::from_gc(gc, wants_barrier) for custom implementations
  - alloc(obj, bytes) → HeapRef; deref(r); deref_mut(r); collect(roots)
  - write_barrier(parent, child) — no-op for M&S, real for generational
  - is_valid(r), heap_size(), gc_stats(), profile()
- `RootSet` — pre-collection root snapshot:
  - add_ref(HeapRef), add_address(usize), add_int_root(i64), add_values(&[Value])
  - as_slice() → &[Value]; len(); is_empty(); clear()
  - with_capacity() to avoid per-cycle allocation
- `WriteBarrier` trait — on_store(parent, child); is_active() default true.
- `NoOpBarrier` — zero-cost barrier for M&S; is_active() = false.
- `CardTableBarrier` — stub with AtomicUsize call counter; ready for
  generational GC implementation.
- `GcCore` — top-level facade:
  - with_mark_and_sweep() default constructor
  - Builder: with_policy(), with_adaptive_policy(), with_barrier(),
    with_safepoint_interval()
  - register_kind() → u16; kind(u16) → Option<&HeapKind>
  - alloc(obj, kind) → HeapRef
  - write_barrier(parent, child)
  - tick() — lightweight per-instruction safepoint counter
  - maybe_collect(roots) → Option<GcCycleStats>
  - force_collect(roots) → GcCycleStats
  - is_valid(r); deref(r); heap_size(); profile(); policy_advisories()
  - wants_write_barrier()
- 45 integration tests + 8 doc-tests, all passing.
