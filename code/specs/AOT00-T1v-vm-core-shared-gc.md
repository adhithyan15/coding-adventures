# AOT00-T1v — vm-core shares the real GC (capstone: complete)

> **Status: complete.** `vm-core` (the bytecode interpreter) now allocates
> and collects through `gc-core`'s `FlatHeap` — the exact same collector
> engine the native-AOT backends use via `gc-core-capi` — as a direct Rust
> dependency. This closes the interpreter-side half of a caveat the T1
> capstone (`AOT00-twig-native-gc-coverage.md`) never actually covered: that
> document is scoped to Twig's *native-AOT* heap surface only, and nothing
> in the repo previously gave the interpreter tier a real collector at all.

## 1. The problem, found by reading code, not commit messages

A verification pass (grounded in reading `gc-core`'s actual source and
running real `cargo test`, not trusting spec docs or commit titles) found
that `gc-core` shipped **two unconnected collector implementations**:

- `FlatHeap` (`gc-core/src/flat_heap.rs`) — the real, ~4700-line, heavily
  tested engine: mark-sweep, generational, moving/compacting, incremental.
  This is what `gc-core-capi` wraps behind a C ABI for the native-AOT
  backends.
- `GcCore`/`GcAdapter` (`gc-core/src/gc_core.rs`, `adapter.rs`, over a
  separate, standalone `garbage-collector` crate modeling the heap as a
  synthetic-address `HashMap<usize, Box<dyn HeapObject>>`) — a facade whose
  own doc comment claimed to be *"the single object that `vm-core` (LANG02)
  holds and calls."* A repo-wide grep found this was false: nothing outside
  `gc-core`'s own crate referenced `GcCore` or `GcAdapter` at all.
  `vm-core`'s actual `Cargo.toml` did not depend on `gc-core`; its
  `write_barrier` method was a literal no-op stub (`let _ = (parent,
  child);`).

So `vm-core` had **no real GC wiring whatsoever** — its `alloc`/
`alloc_array`/`field_store`/`field_load` opcodes (the E5/E6d heap-object
model) allocate into `ctx.arrays`, a plain `Vec<Vec<Value>>` bump arena that
is never freed until the whole `VMCore` drops, capped only by
`max_memory_entries`.

A second, related caveat, found the same way: `gc-core-capi`'s automatic
safepoint (`__gc_safepoint`) always ran a **fully conservative** stack scan
(`__gc_collect`), even when precise stack maps *were* registered for the
running program, and never compacted — both precision and compaction were
reachable only through explicit builtin calls a Twig program had to opt
into.

## 2. What this arc does

Per user direction, given the trade-off between deleting the dead facade
outright versus giving `vm-core` a real, *shared* collector: **wire
`vm-core` onto `FlatHeap` directly**, and make automatic collection upgrade
itself — precise roots, and, per a shared policy, compaction — with no
separate opt-in required, on both the native-AOT and interpreter paths.

### 2.1 Remove the dead facade (gc-core 0.25.0)

`gc_core.rs`, `adapter.rs`, `root_set.rs`, and `write_barrier.rs` (all four
exist only to support the dead facade) are deleted, along with the
now-unused `garbage-collector` path dependency. `gc-core`'s crate doc now
describes what the crate actually is: `FlatHeap`, shared directly by every
real consumer.

### 2.2 `vm-core` depends on `gc-core` directly (vm-core 0.20.0)

No C ABI — `vm-core` is already Rust, so it calls `FlatHeap` methods
natively. This is **additive**: the existing array-heap ops are unchanged.

- **`Value::HeapRef(gc_core::HeapRef)`** — a new value kind. `HeapRef` is
  reused verbatim from `gc-core` (already a bare `usize` newtype).
- **`gc_alloc [<size_bytes>] -> dest`** — allocates on `FlatHeap` (kind 0,
  opaque/conservative — always sound, the same "unregistered kind falls
  back to conservative" invariant `gc-core` already guarantees for itself).
- **`gc_field_load` / `gc_field_store`** — raw 64-bit-word field access, no
  NaN-boxing, mirroring exactly how the native cons-cell path represents a
  pair (`aarch64-backend`'s `alloc` op doc comment: *"Values are raw 64-bit
  words — no NaN-boxing"*). A word is either a nested `Value::HeapRef`'s raw
  address or a plain `Value::Int`; decoded on load by `instr.type_hint`
  (`"ref..."` vs. anything else) — the same convention `gc_core::HeapRef`'s
  own docs establish (*"registers typed `ref<T>`"*). Bounds-checked against
  the object's *actual* allocated size via a new, general-purpose
  `FlatHeap::payload_size(addr) -> usize` (a `kind_of` sibling — resolved
  fresh from the live block's header, so it never goes stale across a
  compacting collection, unlike an address-keyed side table a caller might
  otherwise be tempted to maintain). `gc_field_store` runs
  `FlatHeap::write_barrier` when the stored value is a `HeapRef`.
- **`safepoint`** (paced) / **`gc_collect`** (unconditional) — mirrors
  `gc-core-capi`'s own split between its paced `__gc_safepoint` and its
  unconditional `__gc_collect_precise`/`__gc_collect_compacting` builtins.

**Root-finding needs no stack scan at all.** `dispatch::build_roots` walks
every live `Value::HeapRef` vm-core itself can see — every register across
every active frame, every global, every `memory` slot, every array element
— and hands their exact storage addresses to `FlatHeap::collect_mixed`/
`collect_compacting` as root slots. An interpreter always knows exactly
where every reference lives; there is nothing to approximate. This is
*more* precise by construction than the native-AOT path's stack-map
machinery, which still degrades to conservative regions for unmapped
frames.

New `gc_core::HeapRef::as_mut_ptr(&mut self) -> *mut usize` makes this
root-slot mechanic sound: it addresses the `HeapRef`'s interior field
directly (no `#[repr(transparent)]` needed), so a compacting collection's
root-slot rewrite (`evacuate_and_fixup`, which already writes the forwarded
address back into each root-slot address in place) transparently updates
vm-core's own `Value::HeapRef` storage — no vm-core-side pointer-fixup code
needed at all; passing the same roots to `collect_compacting` instead of
`collect_mixed` is the entire diff between the non-moving and moving paths.

### 2.3 One shared compaction-cadence policy (gc-core 0.27.0)

`FlatHeap::should_compact(&self) -> bool` is the *one* place the
"should this collection also relocate objects" decision lives, implemented
via the pre-existing, already-decoupled `AdaptivePolicy::evaluate`/
`GcProfile` (confirmed to have zero dependency on the deleted
`GcCore`/`GcAdapter`). It defers to `AdaptivePolicy`'s own priority order
(pause time → survival ratio → fragmentation): a cycle with a more urgent
latency or survival signal does not compact just because fragmentation is
*also* high, matching a real collector's own trade-off (a moving collection
has its own pause cost, so it is not owed priority over a more urgent
signal).

Both automatic-collection call sites now consult it identically, so they
cannot drift apart:

- `gc-core-capi`'s `__gc_safepoint` (0.23.0): calls `__gc_collect_precise`
  (or `__gc_collect_compacting` when `should_compact()` fires) instead of
  the fully-conservative `__gc_collect`. This is always safe, not a new
  safety argument: both entry points already document, and already have
  tests proving, that they degrade *exactly* to the old conservative
  behavior when no stack maps are registered.
- `vm-core`'s `safepoint` opcode (0.21.0): calls `collect_compacting`
  instead of `collect_mixed` under the same policy, over its own
  always-precise root set.

## 3. What's explicitly out of scope (not a silent gap)

- **Migrating `ctx.arrays`/E5 array storage and E6d cons-cell lowering onto
  `FlatHeap`.** `Value` today is a heterogeneous, non-`Copy` Rust enum
  (`Str(String)` alone is 24 bytes) — `FlatHeap`'s tracing model assumes
  every traced word *is itself* a candidate pointer, which a `Vec<Value>`
  doesn't fit. Folding that storage onto `FlatHeap` too needs a `Value`
  word-collapse redesign (effectively NaN-boxing or an explicit tag byte)
  first — a separate, much larger project. `gc_alloc`'s raw-word model is
  deliberately scoped to *new* GC-managed objects only.
- **A per-kind precise interior trace for `gc_alloc`'d objects.**
  `gc_alloc` uses kind 0 (opaque/conservative) uniformly. Registering a kind
  per object shape (`FlatHeap::register_kind`) for exact ref-field tracing
  is a natural, low-risk follow-on — `gc-core`'s "unregistered kind falls
  back to conservative" invariant means kind 0 is never *unsound*, just less
  precise than it could be.
- **Concurrent GC.** Explicitly folded into the roadmap's T3 concurrency
  track, not this one.

## 4. Tests

- `gc-core`: `payload_size_reports_allocated_bytes_and_zero_for_non_heap`,
  `payload_size_survives_compaction_at_the_new_address`,
  `as_mut_ptr_reads_and_writes_through_to_the_ref`,
  `should_compact_follows_adaptive_policy_fragmentation_signal`.
- `vm-core`: `tests/gc_heap.rs` — round-trip, nested-`HeapRef`-chain,
  bounds/type-trap, and reclamation proofs, including
  `gc_collect_frees_an_object_whose_only_root_was_overwritten` (the headline
  proof: a collection genuinely reclaims an object once its only root is
  gone, not just that the API compiles) and
  `safepoint_over_threshold_collects_and_reclaims` (the paced dispatch
  collects for real once the adaptive threshold is crossed).
- `gc-core-capi`: existing `safepoint_throttles_then_collects_at_threshold`
  continues to pass unchanged — proof the precise/compacting upgrade didn't
  regress the paced trigger's own behavior.
- Full downstream sweep: every crate depending on `vm-core`
  (aarch64/x86_64/armv7-backend, jit-core, brainfuck-iir-compiler,
  vm-runtime, vm-concurrency, lang-aot, the `*-iir-compiler` crates) compiles
  clean; `vm-runtime`'s one real break (`VmResult::from_value`'s
  non-exhaustive match on the new `Value::HeapRef`) is fixed, mapping to the
  `VmResultTag::Ref`/`from_ref` case that type already anticipated.
