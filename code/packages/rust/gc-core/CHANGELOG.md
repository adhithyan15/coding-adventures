# Changelog — gc-core

## 0.9.0 — 2026-07-18

### Added

- **`FlatHeap::collect_mixed(root_slots, regions)` — precise slots and conservative
  regions in one cycle.** The collection primitive a native **precise stack walk**
  needs when only some frames carry stack maps. A real walk sees a *mix*: frames a
  migrated backend stack-mapped contribute exact `root_slots` (via
  `frame_root_slots`), while frames it could not map — a C-runtime frame, the
  collector's own frames, a not-yet-migrated backend — must be scanned
  conservatively, each contributing its whole span as a `(base, len)` region. Both
  root kinds must be marked in the *same* mark phase and reclaimed by the *same*
  sweep (the heap has one live set; a precise-collect-then-region-collect would let
  the first sweep free what the second's roots keep). `collect_mixed` marks every
  slot word (exactly as `collect_precise`) **and** every candidate word in every
  region (exactly as `collect_region`), then sweeps once.
  - Strict generalisation of both siblings: `collect_precise(slots)` ≡
    `collect_mixed(slots, &[])`, and `collect_region(base, len)` ≡
    `collect_mixed(&[], &[(base, len)])`.
  - **Per-frame precision:** a mapped frame pins only its real references; an
    unmapped frame conservatively pins its span's look-alikes. Adding precise
    coverage to more backends strictly reduces floating garbage, and an unmapped
    frame is never *less* safe than today's fully-conservative scan.
  - Interior tracing, in-place sweep, remembered-set clearing and threshold
    adaptation are identical to the two siblings. 5 unit tests, including the
    headline mixed-frame cycle (mapped frame frees its unnamed-slot look-alike while
    an unmapped frame's span retains its objects) and the two equivalence proofs.
  - This is the platform-independent core the gc-core-capi `__gc_collect_precise`
    stack walk (a follow-up) is layered on, exactly as `collect_region` underpins
    the conservative `__gc_collect` C-stack scan.

## 0.8.0 — 2026-07-18

### Added

- **`FlatHeap` precise stack-map roots — the format/lookup + precise-mark core.**
  The next rung of the precision ladder (`AOT00-T1-precise-gc.md` §4, §6.1): the
  step from *conservative* root scanning (every stack word is a candidate pointer,
  so a look-alike integer keeps a dead object alive — "floating garbage") to
  *exact* roots (only the slots a stack map names as references are read).
  - **`collect_precise(root_slots)`** — collect from an enumerated set of exact
    root-slot **addresses**. Each is the address of a stack/register-spill slot a
    stack map named as live; the collector reads that one word and roots from it,
    looking at nothing else. No false roots: an integer one slot over from a real
    reference is never read, so the object it look-alikes can be reclaimed, and
    root-scan cost drops from O(stack depth) to O(live roots). Sweeps in place (no
    relocation), so it is strictly *additive* precision over `collect` /
    `collect_region` and cannot regress liveness. Interior tracing is unchanged
    (registered-`kind` field maps precise, otherwise conservative).
  - **`StackMapRecord`** — one safepoint's live-reference description (§4.1):
    `pc_offset`, `frame_size`, `slots` (FP-relative byte offsets, signed), and a
    `callee_saved_mask`. `StackMapRecord::new(pc_offset, slots)` for the common
    all-refs-on-stack shape. (`frame_size` / `callee_saved_mask` are consumed by
    the native stack *walker* in `gc-core-capi`; carried now so the emitted format
    is fixed once.)
  - **`StackMapTable`** — a function's records sorted by `pc_offset` with an exact
    binary-search `lookup(pc_offset)` (§4.2); `from_records` accepts any order.
    `len` / `is_empty` / `records`.
  - **`frame_root_slots(frame_base, rec, out)`** — the pure arithmetic bridge from
    a walked frame + its matched record to the exact slot addresses
    (`frame_base + signed offset`) `collect_precise` consumes.
  - 7 unit tests, including the headline precision proof — an object named by a
    stack-map slot survives while an object whose pointer sits in an *un-named*
    slot of the **same** frame is reclaimed — paired with its load-bearing
    contrast: the same frame scanned conservatively (`collect_region`) retains
    both (the false root precise roots remove). Plus transitive interior tracing
    from precise roots, precise-interior composition, multi-frame accumulation,
    empty-roots, and exact/sorted table lookup.
  - This is the **platform-independent half**. The native stack walk that
    *produces* `root_slots` — unwinding the frame-pointer chain, matching each
    return address to its `StackMapTable` record, and calling `frame_root_slots`
    — is the platform-specific half, layered on in `gc-core-capi` exactly as the
    conservative C-stack scan layers on `collect_region`. Backends emitting the
    records are further follow-ups. A frame the walker cannot map falls back to a
    conservative `collect_region` scan, so precision is additive, never required.

## 0.7.0 — 2026-07-18

### Added

- **`FlatHeap` generational minor GC: remembered-set write barrier +
  `collect_minor`** — the payoff of the young/old split (0.6.0). A minor cycle
  reclaims only **young** garbage and never scans or frees the old generation, so
  GC cost tracks the churny young gen instead of the whole heap (the win for
  high-allocation-rate languages — Ruby/Python/JS).
  - `write_barrier(parent, child)` — the generational write barrier, **O(1)**:
    the header sits exactly `HEADER_SIZE` bytes before the payload, so a store
    into an **old** parent records it in the remembered set with no heap search.
    `child` is never dereferenced (it may be null / a tagged immediate);
    recording an old parent that didn't store a young child is a harmless
    over-approximation.
  - `collect_minor(roots)` — marks from the roots **and** the remembered set
    (old objects holding old→young pointers), following only young objects, then
    sweeps only the young generation and tenures survivors.
  - Every **full** `collect` now **clears** the remembered set (it may free old
    objects, so entries could dangle); the barrier rebuilds it. `remembered_len()`
    introspection.
  - Marking is now generation-aware (`mark_word`/`scan_payload` take a
    `young_only` flag); the full-collect paths are unchanged in behaviour.
  - 6 unit tests, including the headline proof (a young object reachable *only*
    via a remembered old parent survives a minor GC) and its load-bearing
    contrast (the same store *without* the barrier reclaims the child — the
    barrier does real work). Adversarially security-reviewed (a missed
    remembered-set entry would be a use-after-free).
  - `collect_minor_region(base, len)` — the raw-memory-region (stack-scan)
    analogue of `collect_minor`, mirroring `collect_region`; the seam
    `gc-core-capi`'s `__gc_collect_minor` roots from the live stack.
  - **`GcAlgorithm::Generational::is_available()` now returns `true`** — the
    algorithm (minor GC + write barrier) is implemented. Only `Compacting` /
    `Incremental` remain planned. The `AdaptivePolicy` recommendation of
    Generational under low survival is now actionable.

## 0.6.0 — 2026-07-18

### Added

- **`FlatHeap` generational split (young/old) + promotion** — the foundation for
  a generational collector, the biggest throughput win for high-churn object
  allocation (Ruby/Python/JS). Each object's `FlatHeader` now carries a
  `generation` byte (`GEN_YOUNG` / `GEN_OLD`), stolen from the header's existing
  padding so `size_of::<FlatHeader>()` stays exactly 32 (compile-assert
  unchanged). New allocations are born **young**; any object that survives a
  collection is **promoted (tenured) to old** during sweep. New
  `object_count_by_generation() -> (young, old)` introspection + public
  `GEN_YOUNG` / `GEN_OLD` constants. 3 unit tests (born-young,
  survivor-promoted-then-new-alloc-young, old-stays-old). Every collect is still
  a full collect — this PR only establishes the split and tenuring; the
  remembered-set write barrier and the young-only **minor** GC that exploit it
  are the next rung. No behavioural change to existing collection semantics.

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
