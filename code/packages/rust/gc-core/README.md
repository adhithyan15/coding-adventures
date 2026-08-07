# gc-core

The shared GC engine for the LANG pipeline: `FlatHeap`, a real,
malloc-backed collector that every consumer needing a real collector links
against — native or interpreted alike — instead of each carrying its own.

## What this crate provides

| Type | Purpose |
|---|---|
| `FlatHeap` | The collector itself: mark-sweep, generational, moving/compacting, incremental |
| `HeapRef` | Opaque heap address; the runtime form of `ref<T>` in IIR |
| `HeapKind` / `KindRegistry` | Layout descriptors for GC tracing (field offsets, sizes) |
| `GcProfile` / `GcCycleStats` | Per-cycle metrics: allocation rate, survival ratio, pause time, fragmentation |
| `GcPolicy` | Trait for algorithm-switch / tuning strategies |
| `DefaultPolicy` | Never recommends a switch; for tests and short programs |
| `AdaptivePolicy` | Recommends a switch based on profiling heuristics; backs `FlatHeap::should_compact`/`should_collect_minor` |
| `GcAlgorithm` | Enum of GC algorithms `AdaptivePolicy` can recommend |
| `StackMapBuilder` | Producer side of the precise-root stack-map format for native code generators |

## Who links against it

- **Native-AOT / LLVM / WASM** backends link through the C ABI
  (`gc-core-capi`), which owns a process-global `FlatHeap` and exposes it as
  `extern "C"` entry points (`__twig_gc_alloc`, `__gc_write_barrier`,
  `__gc_safepoint`, ...).
- **`vm-core`** (the bytecode interpreter) depends on this crate directly as
  a Rust library, allocating GC-managed objects (`Value::HeapRef`) through
  its own `FlatHeap` instance and rooting them precisely from its own
  register/global/local storage — no C ABI, no stack scanning, because an
  interpreter already knows exactly where every reference lives.

An earlier design (`GcCore`/`GcAdapter` over a separate, synthetic-address
`garbage-collector` crate) explored a `HashMap`-based heap aimed at
interpreters specifically, but was never wired into any real consumer and
has been removed — `vm-core` shares `FlatHeap` directly instead, the same
way the native-AOT backends do.

## `FlatHeap`'s precision ladder

`FlatHeap` (`AOT00-T1-precise-gc.md`) climbs a precision ladder, each rung
strictly additive over conservative fallback. **All rungs are implemented:**

1. **Conservative mark/sweep** (`collect_region`) — every candidate word
   (raw + tag-stripped) is a root; a look-alike integer retains a dead
   object for one cycle.
2. **Precise interior tracing** — a registered `kind`'s ref-field map
   (`register_kind`) is followed exactly; non-ref fields pin nothing.
3. **Generational** (`collect_minor` / `collect_minor_region` /
   `collect_minor_mixed`) — young/old split, a remembered-set write barrier,
   young-only minor collections, tenuring after N survivals with an
   immediate promotion-barrier rebuild (closing a reviewer-caught
   use-after-free window on the promoted edge). Reachable automatically via
   `FlatHeap::should_collect_minor` (backed by `AdaptivePolicy`'s
   survival-ratio signal, capped by `max_minor_streak` so a sustained
   low-survival workload can't starve the old generation of collection
   forever — `AOT00-T8-adaptive-safepoint-scheduling.md`) — **gated behind
   `FlatHeap::set_auto_minor(true)`, off by default.** A minor collection's
   soundness rests entirely on the remembered set being complete, which
   requires *every* old→young reference store to go through `write_barrier`;
   `gc-core` can't verify that a given embedder's compiled output actually
   does (a security-review finding: the native-AOT/LLVM code generators
   don't emit it on `field_store` today, only `vm-core`'s interpreter loop
   does). Enabling `auto_minor` without real barrier coverage is a real
   use-after-free, not an imprecision — see the field's own doc comment.
4. **Precise roots** (`collect_precise` / `collect_mixed`) —
   `StackMapRecord`/`StackMapTable` describe exactly which slots hold
   references at each safepoint, so stack-integer false roots disappear for
   mapped frames; `collect_mixed` handles the realistic case of precise
   slots **and** conservative regions in one cycle (mapped frames mixed with
   unmapped ones). `gc-core-capi`'s automatic safepoint (`__gc_safepoint`)
   uses this path whenever any stack maps are registered, falling back to
   fully conservative only when none are.
5. **Moving / compacting** (`collect_compacting`) — evacuates movable
   survivors into a fresh arena and rewrites every pointer that named a
   moved object, including the caller's own root-slot storage (a compacting
   collection transparently updates a caller's variable in place). Reachable
   automatically, not just via an explicit builtin: `FlatHeap::should_compact`
   (backed by `AdaptivePolicy`'s fragmentation signal) decides when an
   automatic safepoint upgrades from a plain mark-sweep cycle to a
   compacting one, shared identically by `gc-core-capi`'s and `vm-core`'s
   safepoint handlers so the two can't drift apart.
6. **Incremental** (bounded-pause) — `incremental_start`/`incremental_step`/
   `incremental_finish`, tri-colour marking with a Dijkstra insertion write
   barrier for mutation mid-mark. Available as an explicit three-call
   cooperative cycle (`AOT00-T4-incremental-collector.md` §6).

## Dependencies

None beyond the Rust standard library.

## Relationship to other LANG packages

| Package | Relationship |
|---|---|
| `gc-core-capi` | Wraps a global `FlatHeap` behind the native-AOT C ABI |
| `vm-core` | Depends on `gc-core` directly; allocates/collects through its own `FlatHeap` instance |
| `twig-aot`, `aarch64-backend`, `x86_64-backend` | Native codegen backends that call into `gc-core-capi`'s C ABI |
