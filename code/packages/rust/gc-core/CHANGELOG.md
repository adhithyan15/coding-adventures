# Changelog — gc-core

## 0.21.0 — 2026-07-28 — bounded incremental **sweep** — §4 (AOT00-T4)

Completes the incremental cycle's second half: the **sweep** pause is now bounded too, so a
full `start → step* → sweep_step* → finish` cycle never stops the mutator for longer than a
caller-chosen budget — the property a language runtime (JS/Ruby/Python) needs to keep frame
latency flat regardless of heap size.

- **New `FlatHeap::incremental_sweep_step(budget) -> bool`** — reclaims / ages at most `budget`
  blocks per call, returning `true` when the all-list is fully swept. Objects allocated between
  sweep steps are born **black** (`mark_in_progress` still set) so a running sweep never frees a
  mid-sweep newborn — it survives to the next cycle (verified by
  `incremental_newborn_during_sweep_survives`).
- **New `FlatHeap::incremental_sweeping() -> bool`** — introspection: is a stepped sweep
  outstanding.
- **`incremental_finish` now drains a partial stepped sweep.** Either drive style works and
  yields byte-identical results: `start → step* → finish` (finish sweeps monolithically) **or**
  `start → step* → sweep_step* → finish` (finish consumes the stepped tallies and drains any
  remainder). Backward compatible — existing callers that never call `sweep_step` are unchanged.
- **Shared `sweep_free_or_keep` helper** factors the per-block free/age/tenure decision so the
  monolithic `sweep` and the stepped `incremental_sweep_step` can never drift apart.
- **Soundness fixes (Miri-verified).** Two bugs the bounded sweep surfaced and this release
  fixes in the shared path:
  - *Use-after-free in the sweep loop.* `sweep_free_or_keep`'s `Freed` arm deallocates the
    block, so the successor link is now read **before** the call, not after. The monolithic
    `sweep` had the same latent UAF (masked by its tight loop) and is fixed identically.
  - *Stale cross-slice provenance.* The resumable sweep cursor is persisted as the last-kept
    **block pointer** (into malloc'd memory), and the `&mut self.all` / `&mut (*resume).next`
    cursor is re-derived **freshly** under each slice's `&mut self`. Persisting a
    `self`-derived pointer across calls is Undefined Behaviour — each call's function-entry
    retag invalidates it (Stacked Borrows) — now caught clean by `cargo miri` on all
    `flat_heap::tests::incremental_*` tests.
- Tests: `incremental_stepped_sweep_equals_monolithic_sweep`, `incremental_sweep_step_is_bounded`,
  `incremental_finish_drains_partial_sweep`, `incremental_newborn_during_sweep_survives`.

## 0.20.0 — 2026-07-25 — `Incremental` algorithm marked available — PR-3 (AOT00-T4)

- **`GcAlgorithm::Incremental::is_available()` now returns `true`** — the incremental
  collector (`FlatHeap::incremental_start`/`incremental_step`/`incremental_finish` + the
  Dijkstra insertion write barrier, shipped in 0.18–0.19) is implemented, so the adaptive
  policy's existing high-pause `SuggestSwitch(Incremental)` recommendation becomes actionable
  rather than advisory. **All four `GcAlgorithm` variants are now available** — the precision
  ladder is complete: mark-and-sweep → interior-precise → generational → precise-roots →
  compacting → **incremental**. Tests updated (`gc_algorithm_incremental_is_available`,
  `every_gc_algorithm_is_now_available`).

## 0.19.0 — 2026-07-25 — incremental Dijkstra insertion write barrier — PR-2 (AOT00-T4)

Closes the incremental collector's soundness surface: the mutator may now safely store
references *between* mark steps.

- **`FlatHeap::write_barrier(parent, child)` extended** with a **Dijkstra insertion barrier**:
  while an incremental mark is in progress (`mark_in_progress`), the stored `child` is
  **shaded grey** (marked + pushed to the grey worklist) if it was white. This preserves the
  strong tri-colour invariant *"no black → white"* — without it, storing a white child into
  an already-scanned (black) parent and dropping the child's other in-edge would strand the
  child white and the sweep would free it while still live (a use-after-free). Handles a raw
  or NaN-box-tagged child pointer (both raw and tag-stripped forms shaded). **One barrier, two
  jobs:** the generational old→young remembered-set half is unchanged and still runs first;
  the incremental half is gated behind `mark_in_progress`, so outside a mark it is a single
  predictable-branch no-op (no new call site — the native/JIT emitters already emit exactly
  one `write_barrier` per ref store). New private `shade_grey` helper.
- **Tests (+3), all Miri-clean:** the load-bearing differential — a white child stored into a
  black parent (its other in-edge dropped) **survives** with the barrier (`freed == 0`); the
  **load-bearing twin** with the barrier call omitted **frees** it (`freed == 1`), proving the
  barrier is necessary, not decorative; plus a no-op-outside-a-mark check that the generational
  path is unchanged. 90 gc-core tests pass; clippy clean.
- This is PR-2 of 4 (spec §9). PR-3 = the C ABI (`__gc_collect_incremental_{start,step,finish}`
  + the `__gc_write_barrier` extension) and flipping `Incremental::is_available()`; PR-4 = the
  native builtin trio + end-to-end.

## 0.18.0 — 2026-07-24 — incremental (bounded-pause) marking — PR-1 (AOT00-T4)

The first rung of the incremental collector (spec `AOT00-T4-incremental-collector.md`): the
stop-the-world mark is decomposed into **bounded slices** so the mutator sees short pauses
instead of one long one.

- **`FlatHeap::incremental_start(root_slots, regions)` / `incremental_step(budget) -> bool` /
  `incremental_finish() -> GcCycleStats`** — the interruptible tri-colour mark cycle.
  `start` colours everything white, snapshots the roots **once**, and greys the roots; `step`
  scans up to `budget` grey objects, greying their still-white children (turning each scanned
  object black), returning `true` when the grey frontier empties; `finish` sweeps every white
  (unreachable) object, rebuilds the remembered set, and ends the phase. Tri-colour state:
  white = `!marked`, grey = `marked` ∧ on the new persistent `mark_worklist`, black = `marked`
  ∧ off it. **Header unchanged (still 32 bytes)** — grey is worklist membership, not a new bit.
- **Alloc-black during a mark:** `alloc` sets `marked = mark_in_progress`, so an object born
  mid-mark (outside the fixed reachable snapshot) is never swept by the running cycle.
- **Introspection:** `incremental_in_progress()`, `incremental_grey_count()`.
- **Mixing guard:** every stop-the-world `collect*` entry (`collect`, `collect_region`,
  `collect_minor`, `collect_minor_region`, `collect_precise`, `collect_mixed`,
  `collect_compacting`) now `debug_assert!(!mark_in_progress)` — a full/minor collect run
  *between* incremental steps would sweep blocks still on the grey worklist (dangling
  pointers a later step would pop), so the caller must drive one incremental cycle to
  `finish` before any other collector. Fenced in debug builds (a review follow-up).
- This is PR-1 of 4 (spec §9): a cooperative single-shot driver that does **not** mutate the
  heap during a mark, so no write barrier is needed yet (the Dijkstra insertion barrier is
  PR-2; the C ABI + `Incremental::is_available()` flip is PR-3; the native end-to-end is PR-4).
- **Tests (+3), all Miri-clean:** stepping one object per step frees **exactly** what a single
  atomic `collect_mixed` frees (decomposition, not different reachability); a step scans at
  most `budget` objects (grey-frontier assertions); an object allocated mid-mark is retained
  this cycle (born black) and reclaimed the next. 87 gc-core tests pass; clippy clean.

## 0.17.0 — 2026-07-24 — `Compacting` algorithm marked available (AOT00-T3 §5)

- `GcAlgorithm::Compacting::is_available()` now returns `true`: the moving/evacuating
  collector (`FlatHeap::collect_compacting`, shipped in 0.16.0) is implemented, so the
  adaptive policy's existing high-fragmentation `SuggestSwitch(Compacting, …)` recommendation
  becomes actionable rather than advisory. `Incremental` remains the one planned-but-absent
  rung. Precision ladder: mark-and-sweep ✓ → interior-precise ✓ → generational ✓ →
  precise-roots ✓ → **compacting ✓**.

## 0.16.0 — 2026-07-24 — the full moving cycle `collect_compacting` (AOT00-T3 PR-3c-2)

Completes the moving/compacting collector: `collect_compacting(root_slots, regions)` runs
one **complete relocating collection** and leaves the heap self-consistent and owning
everything. It builds on the pieces landed in 0.13–0.15 (mobility classification, the arena
+ forwarding map, pointer fixup, arena provenance) and adds step 4 of the cycle — reclaim
from-space and integrate to-space.

- **`collect_compacting`** (new `pub` entry): classify + evacuate + fix up (via
  `evacuate_and_fixup`), then:
  - **Mark survivors-in-place off the pin bit.** After classification the invariant
    `reachable ∧ ¬moved ≡ pinned` holds, so the `pinned` bit is a ready-made keep-in-place
    predicate: `marked = pinned` marks every survivor, leaving *both* the unreachable and
    the moved-from-space originals unmarked.
  - **Sweep** frees the unmarked blocks — reclaiming the dead *and* the now-orphaned
    from-space originals of moved objects (every live reference to them was rewritten in the
    fixup step, so none dangles) — and keeps + ages the pinned survivors. From-space
    originals are malloc'd, so they free normally; no arena slice is touched.
  - **Integrate the arena:** the moved objects' copies are re-threaded (their `next` fields
    are stale bytes from the `copy_nonoverlapping`) into one chain, aged/tenured like an
    in-place survivor, and prepended to the all-list; the arena is moved into `self.arenas`
    so its storage outlives the collection and is freed exactly once (never per-object).
  - **Rebuild the remembered set** over the post-integration all-list — remapping any moved
    old→young parent to its new address and re-deriving the promotion barrier, exactly as a
    full `collect_mixed` does.
  - Reports `freed` as the genuinely-dead count (`swept − moved`), so `freed + survived ==
    before` holds as for a non-moving collect. With nothing movable it degenerates to
    `collect_mixed` (the spec's strict generalization).
- **PR-3b reviewer follow-up:** `fixup_ref_fields` now carries a `debug_assert` that every
  precise reference field holds a **base** (or tagged-base) pointer, never an interior
  pointer — an interior pointer would silently escape `forwarded`'s base-only rewrite and
  dangle. Compiled out of release builds; exercised under tests + Miri.
- **Tests (+4):** the headline executing differential (a precise `a → b` chain moves, an
  unreachable `c` is reclaimed, the root is rewritten, a sentinel is byte-preserved across
  the move, and a *second* compaction re-moves the arena-backed copies); strict-
  generalization parity with `collect_mixed`; a UAF reuse-and-recollect stress; and the
  empty-roots degenerate case. **All moving-collector tests pass under Miri** (no
  double-free of a from-space block, no dealloc of an arena slice, no stale-`next` walk).

## 0.15.0 — 2026-07-24 — moving-collector arena provenance plumbing (AOT00-T3 PR-3c-1)

The UAF-safety plumbing for reclamation: an evacuated object lives inside an [`Arena`] (a
big single allocation), so it must never be handed to `dealloc` individually. This lands
that safety contract on its own, ahead of the full `collect_compacting` (PR-3c-2) that
populates it.

- `FlatHeader` gains a 1-byte `arena_backed` provenance flag, stolen from tail padding —
  the header stays exactly 32 bytes (`size_of == 32` assertion intact; `_pad` 9 → 8).
  `false` for a normal `alloc`'d block; `plan_compaction` sets it `true` on each arena copy.
- `FlatHeap` gains an `arenas: Vec<Arena>` it owns; a compacting collection will move its
  to-space arena here so the moved objects outlive the collection.
- **Every `dealloc` site is now provenance-aware:** `sweep` unlinks a dead arena-backed
  block from the all-list but does **not** free it (its arena will); `Drop` frees the
  malloc'd blocks and skips arena-backed ones, then `self.arenas` drops (after `Drop::drop`,
  per Rust field-drop order), releasing the arena storage exactly once — no double-free, no
  dealloc of an arena slice.

Purely additive; no existing behaviour change (the PR-3a byte-identity test updated to
expect the one intentional provenance-byte difference between an original and its copy).
2 new tests integrate an arena copy into the heap and exercise **both** `dealloc` sites
(sweep-skip and Drop-skip). **All 14 moving-collector tests pass under Miri** (no
double-free, no dealloc of an arena slice, correct Drop ordering). `arenas` carries
`#[allow(dead_code)]` until `collect_compacting` (PR-3c-2) populates and consults it.
gc-core-only. gc-core 0.14.0 → 0.15.0.

## 0.14.0 — 2026-07-24 — moving-collector pointer fixup (AOT00-T3 PR-3b)

Third step of the moving/compacting collector: the **pointer fixup** (moving-cycle steps
1–3). `evacuate_and_fixup(root_slots, regions)` runs `plan_compaction` (mark + copy into the
to-space arena, PR-3a) and then rewrites every pointer that named a moved object to its new
arena address:

- **roots** — each precise `root_slot` whose word names a moved object is updated in place;
- **interior** — each moved object's *arena copy* has its registered-kind reference fields
  rewritten (`fixup_ref_fields`).

Key simplification, proven from the mobility model and the reason this is UAF-safe: a moved
object is referenced **only by base pointers** — precise reference fields hold base pointers,
and the classification's conservative wave scans every pinned / `kind == 0` object *every
word*, which **pins all their targets**. So a moved object has no conservative in-edge and no
interior-pointer referrer. The `forwarded(word)` helper therefore rewrites only a word that is
*exactly* a moved object's old base (raw or low-3-tag-carrying, tag reattached) and **never** a
`kind == 0` / conservative word — so a non-pointer look-alike is never corrupted, and pinned
and `kind == 0` objects are skipped entirely during interior fixup.

Scope boundary (kept deliberately narrow to isolate the UAF surface): this **returns the
arena** and the caller must keep it alive while any rewritten pointer is dereferenced; the
from-space originals are left in place (not freed) and the heap's all-list is not re-threaded,
so the heap does not yet *use* the compacted copies. Reclaiming from-space + integrating the
arena into the heap (a provenance-aware sweep) is PR-3c. The remembered set is likewise left
pointing at the still-valid from-space addresses.

Purely additive; no existing symbol/behaviour change (75 prior tests unchanged). 3 new
differential tests: a precise chain moves and both its root slot and interior ref field are
rewritten so deref-through reaches the child at its new arena address; a conservative in-edge
pins an object (unmoved, root slot unchanged) even when also precisely rooted; a tagged
interior pointer is fixed up with its tag preserved. **All moving-collector tests pass under
Miri** (no UB in the arena copy / unaligned read-write / fixup). `evacuate_and_fixup` carries
`#[allow(dead_code)]` until the full `collect_compacting` (PR-3c) consumes it. gc-core-only.
gc-core 0.13.0 → 0.14.0.

## 0.13.0 — 2026-07-23 — moving-collector to-space arena + copy scaffold (AOT00-T3 PR-3a)

Second step of the moving/compacting collector: the **to-space arena** and the **copy +
forwarding-map** mechanics. Steps 1–2 of the moving cycle (spec §4) only — the mark and the
copy. Deliberately **does not fix up any pointer and does not free anything**, so the
arena/copy machinery lands and is reviewed in isolation before the (separately-reviewed)
pointer fixup (PR-3b) and from-space reclamation (PR-3c).

- New private `Arena`: a contiguous, 16-byte-aligned bump region (one `alloc`'d block, cursor,
  frees on drop). `bump(n)` rounds up to `ALIGN` so each copied `FlatHeader` — and its payload
  at `header + 32` — lands 16-aligned exactly as `alloc` guarantees. A zero-capacity arena
  owns nothing (a collection with no movable survivors).
- New `FlatHeap::plan_compaction(root_slots, regions) -> (Arena, HashMap<old_payload,
  new_payload>)`: classifies mobility (reusing `classify_mobility`), sizes the arena to the
  exact evacuation total `Σ align16(HEADER_SIZE + size)`, copies each **movable** object's
  header+payload verbatim into the arena, and records the old→new payload forwarding. Pinned
  objects are never copied. Because the arena is returned (and normally dropped), the heap is
  left unchanged — an observable dry run. The arena copies intentionally still hold stale
  (old-address) pointers; PR-3b rewrites them.

Purely additive; no existing symbol/behaviour change (72 prior tests unchanged). 3 new tests:
a movable object is copied byte-for-byte to a fresh 16-aligned arena address and forwarded;
pinned objects are never evacuated and an all-pinned heap yields an empty map + zero-capacity
arena; the forwarding map's keys are exactly the movable set with distinct new addresses.
`Arena` / `plan_compaction` carry `#[allow(dead_code)]` until `collect_compacting` (PR-3b)
consumes them — the same "ship the primitive ahead of its consumer" pattern used for
`StackMapTable` / `build_precise_roots`. gc-core-only. gc-core 0.12.0 → 0.13.0.

## 0.12.0 — 2026-07-23 — moving-collector mobility classification (AOT00-T3 PR-2)

First step of the moving/compacting collector (per `code/specs/AOT00-T3-moving-collector.md`):
classify which live objects a future copying collector may **relocate** and which must stay
**pinned**. No relocation happens yet — getting the pin/move decision right is the
use-after-free surface, so it is landed and unit-tested on its own first.

- `FlatHeader` gains a 1-byte `pinned` flag, stolen from the tail padding — the header stays
  exactly 32 bytes (`size_of == 32` assertion intact). It is a per-classification transient
  (born 0, cleared at the start of each classify). The 8-byte forwarding word the relocation
  phase needs is deferred to that phase, which reuses `next` during stop-the-world (spec
  §3.1), so no further header growth.
- `FlatHeap::classify_mobility(root_slots, regions) -> HashSet<usize>` runs a **two-color
  reachability** analysis and returns the movable objects' payload addresses. An object is
  **movable** iff it is *precise-reachable* (reached from the precise `root_slots` following
  **only** registered-kind reference edges), *not pinned* (no conservative `regions` root and
  not a child of any `kind == 0` object reaches it), **and** itself a registered kind (so its
  own pointers can be rewritten). Everything else is pinned. This is the simple, always-sound
  model (spec §2): **any conservative in-edge pins** — when unsure, pin. Erring toward pinning
  is safe; mis-classifying a pinned object as movable would be a use-after-free once
  relocation lands (a stale conservative pointer to its old address).
- Helpers `precise_children` (registered-kind ref offsets only), `conservative_children`
  (every aligned word), and `push_candidates` (raw + low-3-tag-stripped, matching `mark_word`)
  factor the two waves; the precise wave stops at `kind == 0` objects (their out-edges are
  conservative), whose children the pinning wave then pins.

Purely additive; no existing symbol or behaviour changes (66 prior tests unchanged). 5 new
tests: precise-only registered-kind object is movable; a conservative in-edge pins even a
precisely-reachable object; a `kind == 0` object is never movable; movability is transitive
along a precise chain but a kind==0 parent pins its child; pin bits are transient across
classifications. gc-core-only. gc-core 0.11.0 → 0.12.0.

## 0.11.0 — 2026-07-23 — generational aging (tenure after N survivals)

Adds a tunable **tenuring age** so a young object is promoted to the old generation only
after surviving a configurable number of collections, instead of always tenuring on its
first survival. This is the "aging = future tuning" item flagged when the generational
collector landed: keeping objects that die in their 2nd/3rd cycle in the young generation
lets a cheap *minor* GC reclaim them, rather than a full GC being needed to clear the old
generation of prematurely-tenured garbage.

- `FlatHeader` gains a 1-byte `age` field (collections survived while young), stolen from
  the existing tail padding — the header stays exactly 32 bytes (the `size_of == 32`
  compile-time assertion is unchanged).
- `FlatHeap::sweep` increments a young survivor's `age` (saturating) and promotes it to
  `GEN_OLD` only once `age` reaches the heap's `tenure_age` threshold. Old objects never
  age or demote.
- `FlatHeap::set_tenure_age(u8)` / `tenure_age()` configure the threshold; `set_tenure_age`
  clamps `0 → 1` so tenuring always terminates. New `DEFAULT_TENURE_AGE = 1`.

**Promotion barrier (generational-invariant correctness).** Aging breaks an assumption the
remembered set relied on: under immediate tenuring a parent and its child always tenured in
the *same* sweep, so a promoted parent could never point at a still-young child. With aging a
parent can tenure a cycle *before* its child — and because the parent→child store happened
while the parent was young, the write barrier (which records only already-old parents) never
fired. That old→young edge would be invisible to the next minor GC, which would free the live
young child (use-after-free). This was caught by an adversarial security review. Fix: the
sweep reports promoted objects; a **minor** collect records any that now point into the young
generation (`record_promoted_old_to_young`), and a **full** collect *rebuilds* the remembered
set from the surviving old→young edges (`rebuild_remembered`) instead of clearing it (a full
collect can now leave young objects alive, so a blanket clear would drop real edges). Both
trace with the same precise/conservative discipline the mark uses, so an edge is remembered
exactly when a minor scan would follow it. Regression test
`aged_promotion_records_old_to_young_edge_for_minor_gc` reproduces the exact scenario.

**Backward-compatible default:** threshold `1` reproduces the exact immediate-tenuring
behaviour the generational rung shipped with (at that threshold no young object survives a
full collect, so `rebuild_remembered` yields the same empty set the old `clear()` did) — every
existing test is unchanged (66 gc-core tests pass, incl. the untouched promotion / remembered-
set / minor-GC tests). Aging is opt-in via `set_tenure_age` and can become the default in a
later workload-tuning pass. gc-core-only; no C-ABI change (a `__gc_set_tenure_age` capi shim
is a follow-up).

New tests: `raised_threshold_ages_before_tenuring` (stays young for `N−1` collections,
tenures on the `N`-th), `minor_gc_ages_young_survivor` (aging via a minor cycle),
`aged_promotion_records_old_to_young_edge_for_minor_gc` (the UAF regression),
`set_tenure_age_clamps_zero_to_one`, `default_tenure_age_is_one`.

## 0.10.0 — 2026-07-18

### Added

- **`StackMapBuilder` — the producer side of the precise-root stack-map format**
  (first implementation rung of `AOT00-T1-stackmap-emission.md`). `gc-core` already
  owned the *format* (`StackMapRecord` / `StackMapTable`) and the *consumer*
  (`frame_root_slots`); this is the helper a native code generator drives while
  lowering a function so it can actually hand the runtime a table. Until backends
  emit records, `resolve(return_address)` finds nothing and every frame falls back
  to a conservative scan — this is the first step of closing that gap.
  - Driven while lowering: `define_ref_slot(fp_relative_offset)` for each slot that
    holds a GC reference, `safepoint(pc_offset)` at each call site / safepoint, then
    `into_records()` / `into_table()`. The two calls are independent — see the
    order-independence note below.
  - **Rule R1 (flow-insensitive, safe by construction):** every safepoint names
    every stack slot the function ever uses for a GC reference. The named set is a
    superset of the live set at every PC, so a root can never be missed; it only
    over-approximates (retaining floating garbage a cycle, exactly as a conservative
    scan would). It still delivers the main prize — excluding **every non-reference
    slot**, so a stack integer that look-alikes a heap address stops pinning dead
    objects. An exact backward-liveness pass is a later refinement rung: pure
    precision, never a safety change.
  - **Deliberately order-independent.** An earlier flow-*sensitive* draft ("only the
    slots defined before this safepoint") was **unsound**: it equates the order code
    is *emitted* with the order it *executes*, which a backward edge breaks — in
    `loop { use(x); x = alloc(); }` the slot is declared after the loop-top safepoint
    yet holds a live reference there on iteration 2+. An *incomplete* record is worse
    than a missing one, because a `resolve` hit suppresses that frame's conservative
    scan, so the omission would free a live object. Slots and safepoint PCs are now
    collected independently and joined at `into_records()` time, so no drive order
    can produce an incomplete record. A regression test pins this.
  - **Documented backend safety contract:** references live across a safepoint must be
    spilled to a frame slot and declared (the builder describes stack slots only —
    `callee_saved_mask` is always `0`, so a reference kept solely in a callee-saved
    register is named by nobody); incoming reference parameters must be declared; only
    reference-typed slots may be declared. Naming a not-yet-written slot is safe —
    every slot word goes through the same validated candidate-pointer lookup as a
    conservative scan.
  - Duplicate safepoint PCs collapse to one record (two records at one PC would make
    `StackMapTable::lookup`\'s binary search return an arbitrary one), and PCs recorded
    out of order are sorted.
  - Slots are kept sorted and deduplicated, so records are canonical and
    byte-comparable; re-declaring a variable (the backends give each name one
    permanent slot) is idempotent.
  - A safepoint with **no** live references still emits a record: an absent record
    makes the walker fall back to scanning the frame conservatively, whereas an
    empty record is the precise claim "nothing here is a reference".
  - Offsets are frame-pointer-relative, matching `StackMapRecord::slots`. Both
    native backends make that free (aarch64 pins `x29 == sp`; x86-64 already
    addresses slots from `rbp`). Reference-ness stays the backend's decision, so
    `gc-core` keeps no compiler-frontend dependency.
  - 11 unit tests including the R1 full-set property, the loop-regression above,
    idempotent re-declaration, duplicate/out-of-order PCs, extreme `i32` offsets, the
    empty-safepoint record, a `lookup` round-trip, and an end-to-end hand-off into
    `frame_root_slots`.

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
