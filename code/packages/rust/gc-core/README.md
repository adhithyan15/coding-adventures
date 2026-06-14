# gc-core — LANG16

Adaptive GC adapter layer that wires the standalone `garbage-collector` crate
into the LANG VM pipeline.

## What this crate provides

| Type | Purpose |
|---|---|
| `HeapRef` | Opaque heap address; the runtime form of `ref<T>` in IIR |
| `HeapKind` / `KindRegistry` | Layout descriptors for GC tracing (field offsets, sizes) |
| `GcAdapter` | Wraps any `GarbageCollector` with profiling instrumentation |
| `GcProfile` | Accumulates metrics: allocation rate, survival ratio, pause time, fragmentation |
| `GcPolicy` | Trait for algorithm-switch strategies |
| `DefaultPolicy` | Never switches; for tests and short programs |
| `AdaptivePolicy` | Recommends switching based on profiling heuristics |
| `GcAlgorithm` | Enum of available and planned GC algorithms |
| `RootSet` | Collects live heap roots before each GC cycle |
| `WriteBarrier` | Trait for collectors that need inter-object write tracking |
| `NoOpBarrier` | Zero-cost barrier for mark-and-sweep |
| `CardTableBarrier` | Stub for future generational GC |
| `GcCore` | Top-level facade that `vm-core` holds and calls |

## Adaptive GC selection

Different GC algorithms suit different workloads. `GcProfile` tracks the
signals that distinguish them:

| Signal | What it means | Recommended switch |
|---|---|---|
| EMA survival ratio < 15% | Most objects die young | Generational GC |
| Max pause > 10 ms | Latency budget exceeded | Incremental / concurrent |
| Fragmentation > 40% | Heap has many small holes | Compacting GC |

`AdaptivePolicy` reads these signals after every N cycles and emits a
`PolicyDecision::SuggestSwitch`. `GcCore` logs the advisory and — once more
algorithms land — will carry out the switch mid-execution without program
restart.

Today only `MarkAndSweep` is implemented; the policy infrastructure is fully
in place so that adding `Generational`, `Compacting`, and `Incremental`
implementations is mechanical work.

## Quick start

```rust
use gc_core::GcCore;
use gc_core::root_set::RootSet;
use gc_core::kind::HeapKind;
use garbage_collector::Symbol;

// Create a GcCore with mark-and-sweep and adaptive policy.
let mut gc = GcCore::with_mark_and_sweep()
    .with_adaptive_policy()
    .with_safepoint_interval(4096);

// Register layout descriptors for every heap-object kind in your language.
let sym_kind = gc.register_kind(HeapKind {
    kind_id: 0,
    size: 32,
    field_offsets: vec![],
    type_name: "Symbol".to_string(),
    finalizer: false,
});

// Allocate during VM execution (from the `alloc` IIR opcode handler).
let r = gc.alloc(Box::new(Symbol::new("hello")), sym_kind);

// Advance the safepoint counter on each instruction tick.
gc.tick();

// Collect at `safepoint` IIR opcodes or when the interval elapses.
let mut roots = RootSet::new();
roots.add_ref(r);
let stats = gc.force_collect(&roots);

// Inspect adaptive policy advisories for diagnostic output.
for advisory in gc.policy_advisories() {
    println!("[cycle {}] advisory: {} — {}", advisory.at_cycle,
             advisory.algorithm.name(), advisory.reason);
}
```

## IIR opcodes (LANG16)

| Opcode | Operands | Effect |
|---|---|---|
| `alloc` | `(size, kind)` | `dest = GcCore::alloc(...)` |
| `box` | `(value,)` | Box a primitive on the heap |
| `unbox` | `(ref,)` | Dereference; trap if null |
| `field_load` | `(ref, offset)` | Read a field |
| `field_store` | `(ref, offset, v)` | Write a field; call write barrier |
| `is_null` | `(ref,)` | `dest = ref == null` |
| `safepoint` | `()` | `GcCore::force_collect(roots)` |

## Dependencies

- `garbage-collector` — provides `GarbageCollector` trait, `MarkAndSweepGC`,
  `HeapObject`, `Value`

## Relationship to other LANG packages

| Package | Relationship |
|---|---|
| `interpreter-ir` (LANG01) | LANG16 adds `ref<T>` type and 7 new opcodes |
| `vm-core` (LANG02) | Holds a `GcCore`; calls `tick()`, `alloc()`, `force_collect()` |
| `jit-core` (LANG03) | Emits stack maps at safepoints; calls `write_barrier` |
| `aot-core` (LANG04) | Emits stack-map and root-table sections |
| `vm-runtime` (LANG15) | Level-3 GC hooks filled in by this spec |
