# AOT00-T4 — incremental (bounded-pause) collector (design)

> Status: **design, pre-implementation.** This is the spec-first sibling of
> [`AOT00-T3-moving-collector.md`](AOT00-T3-moving-collector.md); it is committed for
> sign-off before any code, exactly as T3 was. It closes the **last** unimplemented rung of
> the precision ladder:
>
> `mark-and-sweep ✓ → interior-precise ✓ → generational ✓ → precise-roots ✓ → compacting ✓ → incremental (this)`.

---

## 1. The problem — why bounded pauses, and why we can't measure them yet

Every `gc-core` collection today is **stop-the-world and atomic**: `FlatHeap::collect*`
marks the entire live set from the roots, then sweeps the entire heap, in one indivisible
call. The pause is proportional to the live set. For a batch program that is fine; for an
interactive or soft-real-time one (a game loop, a request handler, a UI thread) a single
multi-millisecond pause is a dropped frame or a latency spike.

The seams for the fix already exist but are inert:

- **`GcProfile::pause_ns` is hard-wired to `0`** (`adapter.rs:139`, `flat_heap.rs`'s
  `GcCycleStats { pause_ns: 0, .. }`). Nothing measures a pause because there is nothing to
  bound — a collection is all-or-nothing.
- **`AdaptivePolicy` already *recommends* `Incremental`** when `max_pause_ns >
  max_pause_ns_threshold` (default 10 ms, `policy.rs:226`), but
  `GcAlgorithm::Incremental::is_available()` returns `false`, so the recommendation is
  advisory only — exactly the state `Compacting` was in before T3.
- **`WriteBarrier`** is a first-class trait (`write_barrier.rs`) whose own doc table already
  names the incremental barrier's job: *"Maintain tricolour invariant during marking."* Only
  `NoOpBarrier` (mark-sweep) and the generational remembered-set barrier
  (`FlatHeap::write_barrier`) are wired today.

This rung makes the collector **interruptible**: mark in bounded slices, with a write barrier
holding the tri-colour invariant across the gaps, so the mutator sees short, predictable
pauses instead of one long one. It flips `Incremental::is_available()` to `true` and makes
`pause_ns` a real, bounded number.

**Scope guard (soundness first).** Incremental **marking + sweeping**, non-moving, single
mutator thread (the native runtime's contract, same as every other `gc-core` collector). It
is *not* concurrent (no mutator runs *during* a step — steps are short stop-the-world slices)
and it does *not* combine with compaction in one cycle (incremental + moving needs a read
barrier / forwarding-load barrier — deferred to a later rung, §7). This keeps the new
invariant small and reviewable, mirroring how T3 kept minor collections non-moving.

---

## 2. The tri-colour invariant (the soundness core)

Incremental marking colours every object:

| colour   | meaning                                          | representation in `gc-core`                       |
|----------|--------------------------------------------------|---------------------------------------------------|
| **white**| not yet proven reachable (candidate garbage)     | `FlatHeader::marked == false`                     |
| **grey** | proven reachable, children **not yet scanned**   | `marked == true` **and** on the persistent worklist |
| **black**| proven reachable, children **already scanned**   | `marked == true` **and** off the worklist         |

The **strong tri-colour invariant** the collector must preserve across every step boundary:

> **No black object holds a pointer to a white object.**

If that ever holds when the worklist empties, every white object is unreachable and the sweep
may free it. The danger is the *mutator*: between two mark steps it can store a white child
into a black parent (the black parent is already scanned, so the child would never be greyed)
**and** drop the last other path to that child — stranding a live white object that the sweep
then frees (a use-after-free). The **write barrier** (§5) is exactly what forbids this.

The existing `marked` bit already encodes white/black; the only new state is *which marked
objects are still grey* — i.e. a **persistent worklist that survives between steps** (§3).
The current collectors keep the mark worklist as a function-local `Vec` because they drain it
before returning; an incremental collector must hoist it onto the heap.

---

## 3. Header & heap changes

### 3.1 No header change

Tri-colour needs no new `FlatHeader` field: white/black is the existing `marked` bit; grey is
"marked and on the worklist." The 32-byte header (with its `const assert!(size_of == 32)`) is
untouched — unlike T3, this rung is header-neutral.

### 3.2 Incremental mark state on `FlatHeap`

Add a small, self-contained mark-phase state (all `#[allow(dead_code)]` until the C ABI /
step function consumes it, matching how T3 landed `Arena`/`StackMapTable` ahead of use):

```rust
/// Persistent grey set: objects marked but not yet scanned, carried BETWEEN
/// incremental steps. Empty ⇔ not in an incremental mark phase (or marking done).
mark_worklist: Vec<*mut FlatHeader>,
/// True while an incremental mark is in progress (between the first step and the
/// final sweep). Gates the write barrier's incremental shading (§5) and stops a
/// concurrent full `collect*` from starting mid-phase.
mark_in_progress: bool,
/// The roots snapshot for this incremental cycle: an incremental mark is rooted
/// ONCE at phase start (root slots + conservative regions, exactly as
/// `collect_mixed`), so later steps don't re-read a mutated stack. Held for the
/// phase; dropped at sweep.
mark_roots: Vec<usize>,
mark_regions: Vec<(*const u8, usize)>,
```

`mark_worklist` is the grey set as a stack; pushing greys, popping scans-to-black. The
invariant `mark_in_progress == !mark_worklist.is_empty() || <root-scan pending>` is asserted
in debug builds.

---

## 4. The incremental cycle

Three entry points; a full incremental collection is *start → step\* → finish*, each slice
bounded by a caller-supplied **work budget** (objects scanned, the unit the pause scales
with).

```
incremental_start(root_slots, regions):
  assert !mark_in_progress            // one incremental cycle at a time
  clear all `marked` bits             // everything white
  mark_in_progress = true
  mark_roots, mark_regions = snapshot(root_slots, regions)   // root ONCE, up front
  for each root word: shade grey (mark + push to mark_worklist)   // roots → grey

incremental_step(budget) -> done: bool
  assert mark_in_progress
  scanned = 0
  while scanned < budget and let Some(h) = mark_worklist.pop():   // grey → black
      for each child c of h (precise field-map if kind!=0, else conservative):
          if c is white: shade grey (mark + push)                 // white → grey
      // h is now black (popped, children greyed)
      scanned += 1
  return mark_worklist.is_empty()      // true ⇒ marking complete

incremental_finish() -> GcCycleStats
  assert mark_in_progress and mark_worklist.is_empty()
  (freed, survived, live, promoted) = sweep(young_only=false)   // free every white
  rebuild_remembered(); adapt_threshold(...)                   // as a full collect
  mark_in_progress = false; mark_roots/regions/worklist cleared
  record pause_ns per step; return stats
```

- **Rooting once, up front** (`incremental_start`) is what makes stepping sound without a
  read barrier: the reachable-at-start snapshot is fixed; anything the mutator makes reachable
  *after* start is caught by the write barrier (§5), not by re-scanning a moved stack.
- `incremental_step` is the *only* bounded-pause primitive; the budget maps directly to
  worst-case step latency (objects × per-object scan cost). A caller drives it to `done`,
  then calls `finish`.
- The **sweep is still monolithic** in this rung (it walks the all-list once). If sweep pause
  itself becomes the bottleneck, an incremental *sweep* (a sweep cursor carried between steps)
  is a strictly-additive follow-up; marking is the harder invariant and lands first.

---

## 5. The write barrier — holding the invariant across gaps

During a mark phase the mutator may store references. To preserve "no black → white," the
barrier uses a **Dijkstra insertion barrier**: when the mutator stores `child` into `parent`,
if a mark is in progress and `child` is white, **shade it grey** (mark it + push to the
worklist). This guarantees a freshly-installed edge never points black → white — the child is
grey, hence rescanned, hence not swept.

```
gc_write_barrier(parent, child):        // called after every ref field_store
  // (a) generational: unchanged — record an old→young edge (remembered set).
  existing_generational_barrier(parent, child)
  // (b) incremental: if marking, shade the stored child grey so it can't be
  //     stranded behind an already-scanned (black) parent.
  if mark_in_progress and child is a white heap object:
      mark(child); mark_worklist.push(child)
```

Key points:

- **One barrier, two jobs.** The native/JIT emitters already emit a single
  `__gc_write_barrier(parent, child)` at every ref store (generational). This rung *extends
  that same call*, adding the incremental shading behind the `mark_in_progress` flag — **no
  new emitter work, no new call site.** When no incremental mark is running, the incremental
  half is a single predictable-branch no-op.
- **Insertion (Dijkstra), not deletion (SATB).** Insertion shades the *new* child; it needs
  no record of the overwritten value and is simpler to prove for a non-concurrent stepped
  collector. (SATB — shade the *old* value — is the alternative if a future concurrent rung
  wants a weaker mutator-visible barrier; called out in §7, not adopted here.)
- **Conservative safety.** Shading a child that later turns out dead just retains it one extra
  cycle (floating garbage) — never a UAF. Erring toward grey is the safe direction, exactly
  like T3's "when unsure, pin."
- **New allocations during a mark** are born **black** (or grey) — a white new object with no
  in-edge yet could otherwise be swept. Simplest sound choice: allocate **marked** (black)
  while `mark_in_progress`, so this cycle never reclaims an object allocated mid-mark
  (reclaimed next cycle if it dies). `FlatHeap::alloc` sets `marked = mark_in_progress`.

---

## 6. C ABI

Three `gc-core-capi` entries in `stack_scan.rs`, mirroring `__gc_collect_precise`'s
frame-pointer walk for the root snapshot:

```c
/* Begin an incremental collection: snapshot precise roots (frame-pointer walk, as
 * __gc_collect_precise) + conservative regions, colour everything white, grey the roots.
 * Safe to call only when no incremental cycle is in progress. */
void    __gc_collect_incremental_start(void);
/* Advance marking by up to `budget` objects. Returns 1 when marking is complete
 * (caller should then call _finish), 0 if more steps remain. */
int64_t __gc_collect_incremental_step(int64_t budget);
/* Sweep every white object and end the cycle. Returns objects freed. */
int64_t __gc_collect_incremental_finish(void);
```

A frontend / runtime driver loop: `start(); while (!step(BUDGET)) { /* mutator runs a slice */
} finish();` — the mutator interleaves real work between steps, each bounded by `BUDGET`.
`Incremental::is_available()` flips to `true`; the existing `SuggestSwitch(Incremental, …)`
high-pause recommendation (`policy.rs`) becomes actionable.

**Root-snapshot caveat (documented, not a blocker).** Because roots are snapshotted at
`start`, a stack slot that becomes live *after* start and holds the *only* reference to a
*pre-existing white* object would be missed — **except** the write barrier catches it the
moment that reference is *stored* into any heap object, and a value that lives *only* in a
register/stack slot and is never stored is reachable from the next cycle's root scan anyway.
The one true obligation, identical to precise roots today: the driver must not *pop* a frame
that still holds the sole reference to a white object without that reference having passed
through a store. In the current frontend every heap reference lives in an `any` slot and every
inter-object link goes through a barriered `field_store`, so this holds. (A fully-robust
answer re-scans roots at `finish` before sweeping — a cheap, strictly-additive hardening
called out in §9.)

---

## 7. Interaction with generations and moving

- **Generational.** The incremental barrier *extends* the generational barrier (§5) — the
  same `__gc_write_barrier` does both. An incremental cycle is a **full** collection
  (both generations), so it rebuilds the remembered set at `finish`, exactly as the existing
  full collects do. An incremental **minor** (young-only stepped mark) is a later rung.
- **Moving/compacting.** Incremental marking is **non-moving** here. Combining incremental
  marking with relocation in one cycle needs a **read/forwarding barrier** (the mutator, running
  between steps, must not dereference a stale from-space pointer for an object already
  evacuated) — a materially larger invariant. This spec deliberately keeps the two orthogonal:
  you get *either* a bounded-pause non-moving cycle *or* a stop-the-world compacting cycle,
  chosen by the policy, not both at once. A unified "incremental compaction" is future work.
- **Precise vs conservative.** Unchanged: `incremental_step` scans a `kind != 0` object
  through its field map and a `kind == 0` object conservatively, identical to `scan_payload`.

---

## 8. Differential test plan (the proof this is load-bearing)

The property to prove is the **barrier's necessity**, mirrored on T3's "load-bearing
remembered set" tests:

1. **Barrier keeps a mutator-installed child alive.** Build a graph, `start`, `step` until a
   chosen parent P is **black** but some white object C is still only reachable via a path not
   yet scanned. Mutator stores C into P (barrier fires → C greyed) **and** drops C's other
   in-edge. Continue `step` to `done`, `finish`. **C must survive.** The load-bearing twin:
   the *same* sequence with the incremental barrier disabled frees C — proving the barrier is
   necessary, not decorative.
2. **Stepping ≡ stop-the-world result.** For an arbitrary graph, a full
   `start → step(1)\* → finish` (one object per step — maximal interruption) frees **exactly**
   the same objects as a single atomic `collect_mixed` with the same roots. Incremental is a
   strict decomposition, not a different reachability.
3. **New-during-mark object is retained.** An object allocated *between* two steps (born
   black) is never freed by the cycle that was already running.
4. **Bounded step.** `step(budget)` scans at most `budget` objects (assert via a scan
   counter) — the pause-bound contract.
5. **End-to-end (native).** A compiled program drives `start / step / finish` around a live
   `any`-rooted cons structure and reads it back intact afterward — the frontend-triggerable
   proof, mirroring the compaction relocation test.

---

## 9. PR breakdown (small, each CI-validated)

1. **PR-1 (gc-core core):** `mark_worklist` / `mark_in_progress` / root snapshot fields +
   `incremental_start` / `incremental_step` / `incremental_finish` on `FlatHeap`; alloc-black
   during mark. Tests 2–4 above (pure in-heap, no barrier yet — start with roots-only graphs
   that need no barrier). gc-core-only, Miri the step/sweep boundary.
2. **PR-2 (gc-core barrier):** extend `FlatHeap::write_barrier` with incremental shading;
   test 1 (barrier necessity) + its load-bearing twin. gc-core-only.
3. **PR-3 (C ABI):** `__gc_collect_incremental_{start,step,finish}` in `gc-core-capi` (root
   snapshot via the precise walk) + flip `Incremental::is_available()`; host-driven ABI test.
4. **PR-4 (frontend + differential):** a native `gc_collect_incremental_*` builtin trio +
   test 5 (end-to-end). Optional `finish`-time root re-scan hardening (§6 caveat).

Lower-risk alternative if the barrier proof proves fiddly: land PR-1 (interruptible mark,
roots-only, no mutation during mark — already useful for a cooperative single-shot driver) and
defer the barrier PRs, exactly as T3 offered a pin-only fallback.

---

## 10. Risks

- **Barrier omission = UAF.** The one hard invariant. Mitigated by test 1's load-bearing twin
  (the disabled-barrier version *must* free the object) and by the single-call-site design
  (the barrier already exists for generational; we extend it, not add a new obligation).
- **Root snapshot staleness (§6).** Bounded by the store-barrier argument; fully closed by the
  optional `finish`-time root re-scan. Documented, not silently assumed.
- **Floating garbage.** Insertion barrier + alloc-black retain some dead objects one extra
  cycle — a throughput/footprint cost, never a safety one; acceptable and self-correcting.
- **Sweep pause.** This rung bounds *mark*, not *sweep*. If sweep dominates, incremental sweep
  is a clean additive follow-up (a persistent sweep cursor).
