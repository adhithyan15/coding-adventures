# Changelog — gc-core

## 0.1.0 — initial release

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
