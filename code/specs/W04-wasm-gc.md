# W04 — real garbage collection for the in-repo WASM engine

> Status: complete. Closes a gap found by direct investigation (reading
> `wasm-execution`'s actual source, not its own doc comments): the WasmGC
> struct heap (`gc_heap: Vec<GcStruct>`, LANG77 L3b-3a-3b) was an
> **append-only arena with no reclamation**, justified by a claim — "bounded
> by the VM's instruction budget" — that does not hold: no such budget
> exists anywhere in `wasm-execution` (confirmed by grep: no
> `max_instructions`/`fuel`/instruction-counter of any kind). A long-running
> WASM-compiled Twig program could allocate without bound.

## 1. Why this isn't just "port `gc-core`'s `FlatHeap`"

`FlatHeap` is a raw-pointer collector: `alloc(n, kind) -> *mut u8`, and a
`HeapRef` is a real machine address. `wasm-execution`'s heap is
fundamentally different: `gc_heap: Vec<GcStruct>`, and a
`WasmValue::Ref(Some(handle))` **is a `Vec` index** — a WASM-spec-mandated
representation (`ref.null`/non-null `anyref` handles), not something this
collector is free to redesign. That one fact drives the whole design below:

- **Compaction is out of scope.** Removing a dead entry by shifting the
  `Vec` would silently invalidate every other live handle pointing past the
  removed index. Making that safe would need a full transitive trace *and*
  a rewrite pass over every handle-holding location (nested struct fields,
  locals, saved call frames, the operand stack) — the classic moving-GC
  cost, and there is no per-field type schema today to drive precise
  pointer rewriting even if it were in scope. `gc-core`'s own
  `AdaptivePolicy` already treats `GcAlgorithm::Compacting` as advisory-only
  until an algorithm actually implements it (`policy.rs`); this collector
  keeps that framing rather than enacting it.
- **Precise root/field scanning is free, for the opposite reason
  `gc-core`'s native collector needs a registered `kind`.** `FlatHeap`
  traces raw, untyped 8-byte words, so it needs a `HeapKind` field-offset map
  to know which words are pointers. `WasmValue` is a tagged Rust enum — a
  field is *exactly* `Ref(Some(_))`, `Ref(None)`, or a numeric variant, with
  no ambiguity and no schema needed. An `i31ref` payload is carried as a
  plain `I32`, never as the same representation as a real handle, so there
  is no possible confusion between a boxed small int and a heap reference.

## 2. Design: a tombstone + free-list slot arena

The standard non-moving pattern for a `Vec`-backed heap (a "generational
arena" without the generation tag): `gc_heap: Vec<GcStruct>` becomes
`gc_heap: Vec<Option<GcStruct>>` (`Some` = live, `None` = free), plus a new
`gc_free_list: Vec<u32>` of reclaimed indices that `struct.new` checks
before growing the `Vec`.

**Why no generation tag is needed (the "ABA problem" this pattern usually
guards against doesn't apply here):** mark-sweep's ordinary soundness
argument — nothing left unmarked is reachable, provided the root walk is
exhaustive — is sufficient. Every handle a WASM program can ever hold either
sat in a location the mark phase scans (a global, a local, a saved frame's
locals, the operand stack, or transitively through a marked struct's own
fields) or was freshly minted by `struct.new`. There is no way for code to
manufacture or retain a handle from *outside* that root set, so a
reclaimed-and-reused slot can never be aliased by a stale reference — the
same invariant `gc-core`'s `FlatHeap` and `vm-core`'s `FlatHeap`-backed
`gc_alloc` already depend on, just without needing `FlatHeap`'s pointer
machinery to enforce it.

`struct.get`/`struct.set` on a `None` slot (out-of-range *or* tombstoned)
trap cleanly — the same fail-closed convention `struct_get_on_null_traps_cleanly`
already established, just extended to cover a reclaimed handle the same way
it covers a dangling one.

**The arena's length shrinks when it can (a security-review finding, not
just tidiness).** Because the free-list reuses tombstoned indices rather
than shrinking `gc_heap`, its length is otherwise a monotonically
non-decreasing high-water mark for the life of a call — and `mark`/`sweep`
both cost O(`gc_heap.len()`), not O(live objects). A program that
transiently spikes the live count (and, via `adapt_threshold`, the
collection threshold) high, then settles into a low-retention
allocate/discard steady state, would otherwise pay that peak cost on every
subsequent collection for the rest of the call. `sweep` closes this by
dropping `gc_heap`'s trailing run of tombstoned slots after reclaiming them
(and removing those indices from the free list, since they no longer exist
in the — now shorter — `Vec` at all). This is **not** compaction: no live
object moves or is renumbered, so it needs none of §2's handle-rewriting
machinery — it only drops Vec capacity that's provably all garbage at the
tail. A live object sitting in the middle of the arena blocks truncation
from reaching past it, so this is a partial, honest mitigation for the
realistic churn pattern the finding describes, not a full compaction in
disguise.

## 3. Root set — transitive and cycle-safe (the heap is graph-shaped)

`struct.set` can store a `Ref` into a field after construction, so cycles
are constructible (`obj A`'s field → `obj B`, `obj B`'s field mutated to →
`obj A`). The mark phase is a worklist walk with a `marked: Vec<bool>`
visited set (cycle-safe by construction — a cycle just means the worklist
re-adds an index that's already `true` and the check no-ops), seeded from
every `WasmValue::Ref(Some(_))` found in:

- `ctx.globals`
- `ctx.typed_locals` — the active call frame
- **every** `ctx.saved_frames[*].locals` — every suspended caller frame; a
  paused caller can hold a live reference a callee doesn't know about, so
  missing this would free something still in use
- the interpreter's operand stack (`GenericVM::typed_stack`, decoded via the
  existing `REF_TAG`/`REF_NULL_SENTINEL` convention `wasm-execution` already
  uses to round-trip a `WasmValue::Ref` through the generic typed stack)

then transitively through each marked object's own `fields: Vec<WasmValue>`.

## 4. When to check: the existing "safepoints at back-edges and calls" convention

Rather than a per-instruction counter (which `wasm-execution` doesn't have,
and which would need threading through the generic, WASM-agnostic
`virtual-machine` crate's dispatch loop — a crate this collector must not
couple to `wasm-execution`'s GC), collection is checked at the same two
places every WASM-level "more work is about to happen" event already
funnels through, regardless of which of `wasm-execution`'s two independent
dispatch loops is running:

- **`execute_branch`** — the single shared helper every taken `br`/`br_if`/
  `br_table` routes through. A loop's back-edge is a branch to a loop label;
  checking here catches every loop iteration.
- **the internal `call_function` helper** — the single shared helper every
  `call`/`call_indirect` routes through, including nested/recursive calls
  (it runs its own inlined instruction loop against the same
  `context_handlers` table `execute_with_context` uses, so a check placed
  here does not need to also be placed in that inlined loop separately).

Both of `wasm-execution`'s dispatch loops (`GenericVM::execute_with_context`,
and the hand-inlined loop inside the internal `call_function`) dispatch
through the *same* `vm.context_handlers` map and, transitively, the same
`execute_branch`/`call_function` helpers — so instrumenting those two
functions covers both loops without touching either loop's own code, and
without adding anything WASM-specific to the generic `virtual-machine`
crate.

Threshold policy mirrors `FlatHeap::should_collect`/`adapt_threshold`
(`gc-core/src/flat_heap.rs`) conceptually, adapted to an **object-count**
threshold (there's no byte-size concept for a `Vec<Option<GcStruct>>` heap):
collect when the live count crosses the threshold; double it if more than
half the pre-cycle live set survived, halve it otherwise. The live count
itself is tracked incrementally (`gc_live_count`, incremented on
`struct.new`, decremented per object during sweep) rather than recomputed
by scanning — the same "avoid an O(n) count on a hot path" reasoning behind
`vm-core`'s `gc_object_count` field (`vm-core` 0.21.1's security fix).

## 5. Reusing `gc-core`'s policy/profile types

`gc_core::profile::{GcProfile, GcCycleStats}` and
`gc_core::policy::{AdaptivePolicy, GcPolicy, PolicyDecision, GcAlgorithm}`
are pure numeric accumulators with no `FlatHeap`/pointer coupling — reused
as-is here via a new `gc-core` path dependency, recording a `GcCycleStats`
after every cycle for diagnostic consistency with the native-AOT and
`vm-core` paths. Nothing here acts on an `AdaptivePolicy::SuggestSwitch`
recommendation (compaction stays advisory-only, per §2).

## 6. Lifetime: per-call, matching the existing architecture

`gc_heap` (and, now, the free list/threshold/profile alongside it) is
rebuilt fresh on every `WasmExecutionEngine::call_function` — there is no
cross-call heap continuity in this engine today, and this design does not
change that. What changes is that a **single long-running call** — one that
loops or recurses enough to allocate heavily — no longer grows its heap
without bound.

## 7. Explicitly out of scope

- Compaction (§2) and generational collection — `GcProfile`/`AdaptivePolicy`
  can *suggest* either; nothing implements them here.
- `vm-core`/`jit-core`'s separate, still-uncollected bump arena for what
  Twig's `alloc`/`field_load`/`field_store` ops actually emit — a different
  crate, a different fix, tracked separately.
- Real external WASM engines (wasmtime/V8) — this collector is for our own
  in-repo interpreter only; a real engine already has its own GC for the
  same emitted WasmGC bytecode regardless of what this collector does.
