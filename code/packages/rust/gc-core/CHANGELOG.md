# Changelog — gc-core

## 0.32.0 — 2026-08-10 — fix: interior pointers in precise ref slots/root slots escaped relocation fixup — live use-after-free in `collect_compacting`

**Security fix, found by adversarial review of an unrelated in-progress PR (AOT00-T9 PR-3), confirmed
live and exploitable against already-shipped code, fixed and regression-tested here.**

- **The bug:** `find_header(addr)` matches any address inside `[payload, payload+size)` — it accepts
  **interior** pointers (an offset into an object's payload), not just base (payload-start) pointers.
  `push_candidates` (correctly permissive for conservative/pinning-wave sources, where anything found
  gets pinned regardless) was also being used, via `find_header`'s permissiveness, to seed the
  **precise** wave — from `root_slots` entries directly, and from declared reference fields via
  `precise_children`. But `forwarded()`, which rewrites pointers during relocation, only ever rewrites
  **base**-or-tagged-base keys in its forwarding map — it has no way to find or rewrite an interior
  pointer. So an interior pointer at a `root_slots` entry or in a declared ref field could reach an
  object, make it eligible for `movable`, and — once relocated — leave the interior pointer naming it
  **never rewritten**: a real dangling read into freed from-space memory once the original was swept.
  Confirmed live end-to-end against the already-shipped `collect_compacting` before the fix landed.
- **The fix:** `FlatHeap::classify_precise_word(&self, word, out, pin_out)` — tries the raw word, then
  its tag-stripped form, for an **exact** base-pointer match (in that order, both before concluding
  "interior"); an exact match joins `out` (the precise wave), and anything reached only via interior
  overlap is routed to `pin_out` (the pinning wave) instead — exactly like an edge from a
  non-precisely-traced object: never movable, so `forwarded()`'s inability to rewrite it is never
  exercised. `precise_children` and both `root_slots` seeding loops (`classify_mobility`,
  `classify_mobility_minor_sets`) now route through this predicate instead of the permissive
  `push_candidates`. Mathematically proven lossless versus checking each candidate reading
  independently (the try-both-before-concluding-interior structure is required precisely because every
  payload address is provably 16-byte aligned, by both the malloc-backed `alloc` path and the
  arena-backed `Arena::bump` path — see the function's own doc for the full argument).
- **A companion correction:** an existing `#[cfg(debug_assertions)]` sanity check in `fixup_ref_fields`
  used to unconditionally flag ANY interior pointer in a precise ref slot as an error — too strict once
  interior pointers are handled safely by pinning rather than forbidden. Narrowed to fire only if an
  interior pointer's resolved object is ALSO a `forward` key (i.e. actually got moved) — which the fix
  above guarantees is impossible; if it ever fires now, it means the fix itself has a bug, not that
  routine (if unusual) frontend data was encountered.
- 4 new regression tests, covering both the declared-ref-field and root-slot halves of the bug, each
  with both a direct classification check (verifying `pinned`, not just `movable`'s absence) and an
  end-to-end `collect_compacting` differential proving no dangling pointer survives a real collection.
  Each was **empirically verified** by reverting the fix and confirming it fails with exactly the
  predicted dangling-read symptom (a corrupted sentinel read through a stale interior pointer).
- Adversarial security review confirmed the fix mathematically (the 16-byte-alignment argument above)
  and empirically (11 additional probe scenarios: interior-and-tagged combined, dual base+interior edges
  to the same target, misaligned interior fields, ref-array-tail interior elements, and interior
  pointers through both halves of the minor-GC remembered-set/root-slot paths this session's other
  recent PRs added) — all passing, no findings. Two pre-existing, unrelated behaviors were noted as
  informational, not defects: a first-3-bytes interior pointer is indistinguishable from (and safely
  treated as) a tagged base pointer by design, and a one-past-the-end pointer is invisible to both the
  precise and liveness-marking waves alike (confirmed as pre-existing on `collect_mixed` too, not a
  regression from this fix).

## 0.31.0 — 2026-08-10 — `FlatHeap::plan_compaction_minor` / `evacuate_and_fixup_minor` — moving-minor evacuate + fixup, dry-run only (AOT00-T9 PR-3)

- **`FlatHeap::plan_compaction_minor`** (returns `(Arena, HashMap<usize, usize>, HashSet<usize>)` — the
  third element, `precise`, is new versus its full-scope sibling's shape, needed by the fixup step
  below) / **`FlatHeap::evacuate_and_fixup_minor`** — young-scoped siblings of
  `plan_compaction`/`evacuate_and_fixup`, driven by a new internal `classify_mobility_minor_sets`
  helper (`classify_mobility_minor` itself is now a thin wrapper over it, unchanged contract). Dry-run
  in the sense that nothing is freed or integrated into `self.all`/`self.arenas` — PR-4 wires
  reclamation (`collect_minor_compacting`, the full cycle). **Not heap-neutral like its full-scope
  sibling if the caller drops the arena** (see its doc's `# Safety` addendum): unlike
  `evacuate_and_fixup`, whose fixups only ever touch caller-owned roots and the arena's own copies,
  this function's fixup step (below) writes into live heap objects outside the arena — a future
  caller must integrate the arena (PR-4), not drop it. `#[allow(dead_code)]` — not called from any
  production path yet.
- **Found and fixed a real gap in `code/specs/AOT00-T9-moving-minor-collector.md`'s own §4 point 3/4,
  corrected across two rounds of adversarial review** (see the spec's inline `**Correction**` notes in
  §4, §5, §6, §7): the spec claimed evacuate/fixup for the minor case needed "no change to the
  copy/fixup mechanics themselves, only which set drives them," and separately that a remembered old
  parent's field pointing at a moved child was "already rewritten by the fixup pass." Neither holds —
  `evacuate_and_fixup`'s fixup only ever rewrites (a) `root_slots` and (b) moved objects' own arena
  copies, and an old parent (never itself movable, so never a `forward` key) is touched by neither.
  **Round 1** added a fixup step (c) walking `self.remembered` — necessary, but a security review then
  found it insufficient: an old, precisely-traced, *unpinned* parent reached only by a root (the
  `generation == GEN_YOUNG` filter excludes it from `movable`, and hence from the pinning wave's
  force-pin propagation, purely by generation, not by pinning) may never appear in `self.remembered` at
  all. **Round 2** fixed this by walking `precise` (every object the classifier's traversal discovered
  as a node) instead — which the test suite immediately caught as *also* insufficient: a remembered
  parent used only as a seed (its children are consulted, but the parent itself is never independently
  discovered as a node) is absent from `precise`. The shipped fix unions both populations; each half
  was independently confirmed load-bearing by reverting it and observing the exact predicted
  stale-field failure. A residual, now-documented dependency remains: a parent reached *only* through
  an unbarriered store is covered by neither population — a strictly stronger write-barrier obligation
  for a moving minor cycle than a non-moving one has, corrected in the spec's §7 (which previously
  claimed the barrier contract was unchanged).
- 10 new regression tests, covering: the pure-young root-reachable case (sanity); a young child
  reachable only through a remembered old parent (the original differential); the same with a
  misaligned parent ref field; a directly-rooted old parent with **no** remembered-set entry (round-1's
  finding); a remembered parent's field to a **pinned** (non-movable) child, expected untouched; an
  opaque/kind-0 remembered parent, expected a no-op; a parent that is a member of **both** `precise`
  and `self.remembered` simultaneously, proving the double fixup pass is idempotent; a multi-hop
  `old→old→young` chain with no barriers, proving `precise`'s transitive-discovery half of the union;
  a **tagged** reference through the new fixup step; and a permanent regression test directly composing
  (a)+(b) alone (skipping all of step (c)) to prove the field is left stale without it.

## 0.30.0 — 2026-08-10 — `FlatHeap::classify_mobility_minor` — moving-minor mobility classification, dry-run only (AOT00-T9 PR-2)

- **`FlatHeap::classify_mobility_minor(&mut self, root_slots, regions) -> HashSet<usize>`** —
  the young-generation-scoped sibling of `classify_mobility`, and the first landed piece
  of `AOT00-T9-moving-minor-collector.md`'s staged plan. Dry-run only: computes which
  young objects a future moving-minor collector may relocate, without relocating
  anything (mirrors how `classify_mobility` itself shipped as its own PR-2 scaffold
  before the compacting collector consumed it).
- **Why this needed its own function, not a `young_only` flag on `classify_mobility`:**
  that function's "reachable ⟺ pinned ∨ movable" soundness proof only holds for the seed
  set it was built for (`root_slots` ∪ `regions`). A minor cycle's liveness mark
  additionally reaches survivors through the **remembered set**; calling
  `classify_mobility` unmodified on a minor-scoped collection would leave a young object
  reachable *only* through a remembered old parent absent from the classification
  entirely — not conservatively pinned, just invisible, which would let a moving-minor
  sweep free a live, reachable object. `classify_mobility_minor` extends both waves'
  seeding with remembered-parent children, split by whether the parent is precisely
  traced exactly the way `scan_payload` already splits for liveness marking (precisely
  traced → precise wave via `precise_children`; otherwise → pinning wave via
  `conservative_children`, since an opaque/unregistered parent's raw word can never be
  found-and-rewritten by fixup). The final `movable` filter also gains a
  `generation == GEN_YOUNG` conjunct — an old object may still be precise-reachable (e.g.
  a root points at it directly, and tracing through it is necessary to reach its own
  young children) but must never itself be classified movable by a minor-scoped pass.
- Built directly on top of `is_precisely_traced` (0.29.0, below): both waves' seeding and
  the final movable filter gate on that predicate rather than a bare `kind` test, and the
  pinning wave unions `precise_children` alongside `conservative_children` — the same two
  fixes `classify_mobility` needed, applied here from the start rather than repeating the
  bug.
- Guards `debug_assert!(!self.mark_in_progress, ...)` at entry, matching every other
  minor-collect entry point (`collect_minor`, `collect_minor_region`,
  `collect_minor_mixed`) — calling this mid-incremental-mark would read the remembered
  set while it (and the objects it names) may be in an inconsistent, partially-freed
  state.
- `pub(crate)`, not `pub` — adversarial review noted this is `&mut self` (rewrites every
  header's `pinned` bit) with no in-tree caller yet to enforce ordering; narrowed
  visibility until a PR-3/PR-4 consumer actually needs it public.
- **Two load-bearing caveats for the PR-3/PR-4 consumers this scaffolds toward, found by
  the same review and recorded in the function's doc rather than fixed now (nothing
  calls this yet):** (1) the `GEN_YOUNG` filter means `pinned ∨ movable` is a complete
  partition of *young* objects only, not every live object like `classify_mobility`'s —
  a future sweep built on this must restrict itself to young blocks, or it will both
  misfree a live old object and leave stale mark bits on other old objects the pinning
  wave pinned; (2) a remembered *old* parent's precise ref slot can name a movable young
  object, which breaks `evacuate_and_fixup`'s existing "only moved objects' own copies
  need fixing up" premise — a future evacuation pass must additionally walk remembered
  parents' precise slots.
- New tests, including the two load-bearing cases the spec's derivation predicts: a young
  object reachable only via a remembered, precisely-traced old parent is movable; the
  same shape through a remembered, non-precisely-traced (opaque or unregistered-kind) old
  parent is not movable but is verified **pinned** (found, safely retained), not silently
  absent from both sets.
- See `code/specs/AOT00-T9-moving-minor-collector.md` §3 for the full derivation and
  proof sketch this implements, and §5 for the remaining staged plan (evacuate+fixup,
  then the full `collect_minor_compacting` cycle).

## 0.29.0 — 2026-08-07 — fix: `classify_mobility`'s pinning wave gated on the wrong predicate — a live use-after-free in `collect_compacting`

**Security fix, found by adversarial review of an unrelated in-progress PR (AOT00-T9), confirmed
live and exploitable against already-shipped code, fixed and regression-tested here.**

- **The bug:** `classify_mobility`'s pinning-wave seed and its final `movable` filter tested
  `(*h).kind == 0` / `kind != 0` directly to decide whether an object is traced *precisely* (so
  its fields can be found-and-rewritten) or *conservatively* (so it must pin). But the actual
  condition under which `for_each_ref_slot` — the single source of truth every consumer
  (`scan_payload`, `precise_children`, `fixup_ref_fields`, `points_to_live_young`) shares — takes
  the precise path is `kind != 0 AND field_maps.get(kind - 1).is_some()`. A **nonzero kind id that
  was never passed to `register_kind`** (reachable: `FlatHeap::alloc`/`alloc_kind` and the C ABI
  `__gc_alloc_kind` accept an arbitrary `u16` with no validation against the registry) also traces
  conservatively — but the old `kind == 0` test never routed such an object into the pinning wave,
  and `precise_children` correctly contributed nothing for it either. Its children ended up in
  **neither** the `precise` set nor pinned: invisible to the classification, not conservatively
  retained. Worse, the unregistered-kind object *itself* passed the old `kind != 0` movable
  conjunct, so `collect_compacting` would relocate it **without rewriting its own (unvisited)
  conservative field** — a moved object whose stale field still names its original, now-swept
  child. Confirmed end-to-end with a real `collect_compacting` differential before the fix landed.
- **The fix:** `FlatHeap::is_precisely_traced(&self, h) -> bool` — `kind != 0 &&
  field_maps.get(kind - 1).is_some()`, the exact complement of `for_each_ref_slot`'s conservative
  fallback. Every mobility-wave call site (`classify_mobility`'s pinning-wave seed and its
  `movable` filter) now gates on this predicate instead of a bare `kind` test. Doc comments on
  `for_each_ref_slot`, `precise_children`, `classify_mobility`, and `fixup_ref_fields` corrected —
  several stated the old, incorrect invariant as if it were guaranteed.
- 5 new regression tests, including an end-to-end `collect_compacting` differential
  (`collect_compacting_unregistered_kind_parent_does_not_dangle_its_child`) proving the exact
  use-after-free shape is closed: an unregistered-kind parent pointing at a registered-kind child
  no longer relocates either object, and reading through the parent's field after a compacting
  collect still returns the child's live, correct address.
- No API change — `is_precisely_traced` is a private helper; `classify_mobility`'s signature and
  documented contract (an object is movable iff precise-reachable, unpinned, and *actually*
  precisely traced) are unchanged, only now correctly enforced.
- **Second fix, found by the follow-up adversarial review of the fix above:** the pinning wave's
  own conservative scan (`conservative_children`) only visits **8-aligned** words, but a
  registered kind's declared ref field can sit at *any* offset — `for_each_ref_slot` reads it with
  `read_unaligned`, no alignment requirement. A **pinned** parent with a misaligned ref field
  pointing at a registered-kind child left that child reachable only through the *precise* wave —
  unpinned, and thus wrongly `movable` — even though the pinned (unrelocatable) parent's field
  could never be found-and-rewritten on relocation. Confirmed end-to-end (a real `collect_compacting`
  differential) before this half of the fix landed. Fixed by unioning `precise_children` into every
  pinning-wave step alongside `conservative_children`, so the pinning wave dominates every edge
  `for_each_ref_slot` can ever produce — a provable no-op for the 8-aligned layouts every current
  registrant uses (their precise-wave edges are already a subset of the aligned conservative scan).
  3 more regression tests, including two end-to-end `collect_compacting` differentials: a child
  named only through a *pinned* parent's misaligned field survives unmoved, and — the strictly
  more severe variant a third round of review flagged, since `collect_compacting` has no separate
  mark phase, so its live set is exactly `pinned ∪ movable` — a child named only through a
  *conservatively-reached* (region-root-only, no precise root at all) parent's misaligned field is
  not merely left un-relocated-but-dangling but was previously **swept while genuinely live**
  (premature free, not relocate-without-fixup); both are closed by the same fix.

## 0.28.0 — 2026-08-06 — `FlatHeap::should_collect_minor` — automatic generational enactment, gated (AOT00-T8)

- **`FlatHeap::should_collect_minor(&self) -> bool`** — the generational sibling of
  `should_compact`: whether the *next* paced collection should be a **minor**
  (young-generation-only) cycle instead of a full one, per `AdaptivePolicy`'s
  survival-ratio signal. Until now `AdaptivePolicy`'s `Generational`
  recommendation was purely advisory — nothing ever auto-selected a minor
  collection at an automatic collection site; only an explicit direct caller
  (`vm-core`) ever ran one.
- **`auto_minor: bool` (default `false`) + `set_auto_minor`/`auto_minor`** —
  `should_collect_minor` hardcodes `false` unless this is set. A minor
  collection's correctness depends entirely on the remembered set being
  *complete* (every old→young reference store must have called
  `write_barrier`), which `gc-core` cannot verify a given embedder's compiled
  output actually does. **Security-review finding, fixed before merge:**
  `vm-core`'s interpreter loop calls the barrier on every store, but the
  native-AOT/LLVM code generators' `field_store` lowering does not — enabling
  automatic minor collection unconditionally would have been a real
  use-after-free for every AOT-compiled program, not a corner case (immediate
  tenuring + a churny allocation loop is the *common* profile
  `AdaptivePolicy` reads as "recommend Generational"). Default `false` keeps
  every existing call site's behavior byte-for-byte unchanged; an embedder
  opts in only after confirming its own barrier coverage.
- **`minor_streak` + `max_minor_streak` (default `8`, `DEFAULT_MAX_MINOR_STREAK`)** —
  bounds how many consecutive paced minor collections may run before one is
  forced to be full, once `auto_minor` is on. A minor cycle never scans or
  frees the old generation, and the EMA survival-ratio signal driving
  `should_collect_minor` can stay low indefinitely, so without this cap a
  sustained low-survival workload would starve the old generation of
  collection forever (a leak, orthogonal to the UAF above).
  `set_max_minor_streak`/`max_minor_streak` (clamped to a minimum of `1`)
  mirror `set_tenure_age`/`tenure_age`. Every full-collect entry (`collect`,
  `collect_region`, `collect_precise`, `collect_mixed`, `collect_compacting`,
  `incremental_finish`) resets the streak; every minor entry increments it.
- **`FlatHeap::collect_minor_mixed(root_slots, regions)`** — the young-generation
  analogue of `collect_mixed`: traces exact root slots *and* conservative
  regions in one pass, young-only. Needed because a real precise stack walk
  produces exactly that mix (some frames stack-mapped, some not), and neither
  existing minor entry (`collect_minor` takes root *values*; `collect_minor_region`
  takes one raw span) matches that shape. Always directly callable (the
  `auto_minor` gate applies only to `should_collect_minor`'s automatic
  recommendation, not to this or any other explicit minor entry).
- **`minor_finish` now calls `adapt_threshold`**, mirroring every full-collect
  entry — a review-caught gap: without it, pacing state didn't re-tune after a
  minor cycle, so a heap sitting over threshold could stay `should_collect()
  == true` (re-walking the stack at every safepoint) until a full collect
  eventually ran.
- See `code/specs/AOT00-T8-adaptive-safepoint-scheduling.md` for the full design,
  the starvation-hazard analysis (§2), and the barrier-coverage hazard (§2b).
  `gc-core-capi` 0.24.0 wires this into `__gc_safepoint`.

## 0.27.1 — 2026-08-03 — regression test: array elements need a ref-array kind, not a no-ref one

No production code change — adds
`array_registered_under_no_ref_kind_loses_elements_only_reachable_through_it`,
a deterministic (explicit-roots, no conservative-stack noise) reproduction
of the confirmed cross-backend bug this round's `iir-to-llvm`/
`aarch64-backend`/`x86_64-backend` fix closes: a native/Twig `alloc_array`
block registered under `register_kind(&[])` (the old no-ref kind) loses
every element only reachable through it, exactly mirroring the existing
`ref_array_traces_elements_precisely` precedent but with the array's OWN
kind swapped to the pre-fix shape. See
`AOT00-T7-array-reference-tracing.md` for the full writeup.

## 0.27.0 — 2026-08-02 — `FlatHeap::should_compact` — the shared automatic-compaction policy

- **`FlatHeap::should_compact(&self) -> bool`** — whether the *next*
  collection should also relocate objects, per `AdaptivePolicy`'s
  fragmentation signal against the heap's own `GcProfile`. This is the
  **one** place the compaction-cadence decision lives, so every
  automatic-collection call site shares it identically instead of each
  reimplementing its own threshold: `gc-core-capi`'s `__gc_safepoint` and
  `vm-core`'s `safepoint` opcode both now call it (see their own
  changelogs). Defers to `AdaptivePolicy`'s existing priority order (pause
  time → survival ratio → fragmentation) — a cycle with an urgent pause-time
  or survival-ratio signal does not compact just because fragmentation is
  also high; a moving collection has its own pause cost, so it is not owed
  priority over a more urgent latency signal. Pure policy, like
  `should_collect`: names no roots, runs no collection itself.
- Closes the last half of a caveat identified by direct code inspection
  (not commit messages): automatic (safepoint-triggered) collection used to
  never compact at all — only reachable via the explicit
  `__gc_collect_compacting`/`gc_collect_compacting` builtins. It is now
  automatic whenever this policy says fragmentation warrants it.
- Test: `should_compact_follows_adaptive_policy_fragmentation_signal`.

## 0.26.0 — 2026-08-02 — `payload_size` + `HeapRef::as_mut_ptr` (for vm-core's direct FlatHeap integration)

- **`FlatHeap::payload_size(addr) -> usize`** — the live heap object's
  allocated payload size in bytes, or `0` for a null/non-heap/stale address.
  A sibling of the existing `kind_of(addr)`, same safety argument (resolved
  fresh from the header at `addr`, so it stays correct across a compacting
  collection without the caller needing an address-keyed side table of its
  own). Lets a consumer bounds-check a raw field access against an object's
  *actual* size. Tests: `payload_size_reports_allocated_bytes_and_zero_for_non_heap`,
  `payload_size_survives_compaction_at_the_new_address`.
- **`HeapRef::as_mut_ptr(&mut self) -> *mut usize`** — a raw pointer to a
  `HeapRef`'s interior address word, for a consumer that embeds a `HeapRef`
  in its own root storage (a VM register, a global slot) to hand that exact
  address to `collect_mixed`/`collect_compacting` as a root slot — reads the
  current address, and under compaction, has the post-move address written
  back through it. Test: `as_mut_ptr_reads_and_writes_through_to_the_ref`.
- Both exist to support `vm-core` 0.20.0 depending on `gc-core` directly and
  rooting its own `Value::HeapRef`s precisely, without going through the C
  ABI (`gc-core-capi`) that native-AOT backends use — see `vm-core`'s
  changelog for the consumer side.

## 0.25.0 — 2026-08-02 — remove the dead `GcCore`/`GcAdapter` facade

- **Removed `gc_core`, `adapter`, `root_set`, and `write_barrier` modules** —
  the `GcCore`/`GcAdapter` facade over a separate, standalone
  `garbage-collector` crate (a synthetic-address `HashMap<usize, Box<dyn
  HeapObject>>` heap model) was **100% dead code**: nothing outside its own
  crate (not `vm-core`, despite the removed doc comments' claims, not any
  other crate) referenced `GcCore` or `GcAdapter`, and its `write_barrier`
  method was a literal no-op. Dropped the now-unused `garbage-collector`
  path dependency entirely.
- Rewrote `lib.rs`'s and `flat_heap.rs`'s crate/module doc comments, and
  `README.md`, to describe what the crate actually is: `FlatHeap`, the one
  real collector, shared directly by both the native-AOT path (via
  `gc-core-capi`'s C ABI) and — as of the following `vm-core` integration —
  the bytecode interpreter, rather than each carrying its own engine.
- Corrected `README.md`'s precision-ladder section, which had gone stale:
  moving/compacting and incremental collection were listed as "planned" long
  after both actually shipped (AOT00-T3, AOT00-T4).
- No behavior change to `FlatHeap` itself in this release.

## 0.24.0 — 2026-07-29 — `FlatHeap::kind_of` — object-class accessor (AOT00-T6)

- **`FlatHeap::kind_of(addr) -> u16`** — the kind id of the live heap object containing payload
  address `addr`, or `0` if `addr` is not inside any live block (null, non-heap, or stale
  pointer). A frontend uses it to discriminate object *classes* that share the heap tag — e.g. a
  closure kind (`register_ref_array_kind([], 8)`) from a cons kind — for `procedure?` / `pair?`
  predicates. Safe: the address is validated against the live-block list before any header read,
  so a bogus value yields `0` (no out-of-bounds read). O(n) in the live-object count; intended for
  cold type predicates. Foundation for the native-closures arc (AOT00-T6). Test
  `kind_of_reports_object_kind_and_zero_for_non_heap`.

## 0.23.2 — 2026-07-29 — robustness-at-scale tests (AOT00-T5, tests only)

Test-only. Confirms the collector stays correct and O(n) as the heap grows to many thousands of
objects, and — the "solid enough to run a real language" property — that a **deep** object graph
does not overflow the stack during marking (gc-core marks from an explicit worklist `Vec`, never
by recursion).

- `scale_deep_chain_marks_without_stack_overflow`: a 20 000-node single-linked chain + 20 000
  garbage objects; rooting the head, exactly the 20 000 chain nodes survive and the garbage is
  reclaimed, and the chain walks end-to-end intact. A recursion-based mark would blow the stack
  at this depth; the worklist mark does not.
- `scale_wide_ref_array_relocates`: a single 4 000-element reference array of movable leaves is
  compacted — the array and all elements evacuate, every tail slot is fixed up, and spot-checked
  elements (first/middle/last) are reachable at their new addresses with sentinels byte-preserved.
  Proves the tail fixup is correct and O(len) over a large instance, not just a toy.

Counts are intentionally beyond Miri's practical range (the per-object mechanics are already
Miri-verified on the small graphs); these validate scale and the no-recursion mark guarantee.

## 0.23.1 — 2026-07-29 — object-model stress differential (AOT00-T5, tests only)

Test-only. Adds a combined stress differential exercising **every** object-model feature in one
heap graph — records (fixed ref fields), a reference array (tail region), a header+tail object
(fixed ref + non-reference length word + ref tail), a cycle, opaque leaves, and
look-alike-integer non-reference fields — driven through **both** the non-moving collector and
the compacting collector and checked against a hand-computed oracle.

- `stress_graph_mark_sweep_matches_oracle`: rooting one object, exactly the 8 reachable objects
  survive and the 3 garbage objects (including a phantom named only by a non-ref look-alike
  integer) are reclaimed — precise tracing across mixed layouts and a cycle in one pass.
- `stress_graph_compaction_relocates_whole_graph`: all 8 survivors are movable, so the whole
  graph evacuates; every edge (record field, array-tail slot, header+tail element, and the
  back-edge that closes the cycle) is fixed up to new addresses, and the non-ref sentinels are
  byte-preserved. Walking from the rewritten root reaches every object at its new location — a
  missed fixup anywhere would dereference a freed from-space block. Miri-clean.

No production-code change; validates the T5 object model holds together under real-language-style
graphs (mixed layouts + cycles + integers that merely look like pointers).

## 0.23.0 — 2026-07-28 — variable-length **reference arrays** — PR-2 (AOT00-T5)

Makes the collector trace **and relocate** the dominant heap object of a real language runtime
— a JS `Array`, a Ruby `Array`, a Python `list`, a vector, a hash's backing store — *precisely*
instead of conservatively. Builds directly on the `KindLayout::tail_from` slot and the
`for_each_ref_slot` tail walk landed (dormant) in 0.22.0.

- **New `FlatHeap::register_ref_array_kind(fixed, tail_from) -> u16`** — a kind traced as `fixed`
  reference fields (statically-known offsets, exactly like `register_kind`) **followed by a tail
  region**: every aligned 8-byte word in `[tail_from, size)` of the *instance's* payload is a
  reference. Because the tail's extent follows the instance's own `size`, **one kind describes
  arrays of every length** — the thing a fixed offset list cannot express. `tail_from` is rounded
  up to a multiple of 8 (so the tail scan stays 8-aligned); a near-`usize::MAX` argument saturates
  to an empty tail rather than wrapping to a small offset that would trace non-reference words.
- **Why it matters:** a conservatively-traced array (`kind 0`) pins itself *and every element it
  references*, so under the compacting collector nothing moves — arrays being the most common
  heap object, compaction was effectively inert on real workloads. A precise array and its
  elements are movable. This is the array-shaped analogue of the cons-cell relocation unlocked
  earlier.
- **Layout contract** (documented on the API): every word in `[tail_from, size)` must hold a
  reference (base/tagged-base pointer or null), never an inline non-pointer datum; a packed array
  of unboxed values must box them, exclude the non-ref region via `tail_from`, or stay `kind 0`
  (always safe). Mirrors the record-field contract; a violation is caught in debug builds by the
  compaction fixup's interior-pointer assertion.
- **Tests (+5), Miri-clean:** precise element trace (survivors = exactly the referenced elements;
  a dropped element is reclaimed), **array relocates under compaction vs. a pinned conservative
  twin** (the headline — the array and its elements move and the tail slots are fixed up; a
  missed fixup would dangle), fixed-header + tail compose (the `len` word between them is not a
  pointer), the tail feeds the generational old→young barrier, and bound edges (empty tail,
  `tail_from` past `size`, unaligned `tail_from`) are safe. 99 gc-core tests pass; clippy clean.
- This is PR-2 of 4 (spec §7); PR-3 adds the C ABI (`__gc_register_ref_array_kind`), PR-4 the
  native relocation differential.

## 0.22.0 — 2026-07-28 — `KindLayout` + `for_each_ref_slot` refactor — PR-1 (AOT00-T5)

Structural, **zero-behaviour-change** prep for variable-length reference arrays (spec
`AOT00-T5-variable-length-ref-arrays.md`). It de-risks the multi-site change before any tail
tracing lands — the collector behaves identically, proven by the unchanged test suite + Miri.

- **`field_maps: Vec<Box<[usize]>>` → `Vec<KindLayout>`**, where
  `KindLayout { fixed: Box<[usize]>, tail_from: Option<usize> }`. `register_kind` builds
  `KindLayout { fixed, tail_from: None }`, so every kind is a pure record and tracing is
  byte-for-byte identical to before. `tail_from == Some(start)` (the variable-length array
  tail — every aligned word in `[start, size)` is a reference) is reserved for PR-2's
  `register_ref_array_kind`; no registration sets it yet.
- **New shared `unsafe fn for_each_ref_slot(&self, h, f) -> bool`** — the *single* place a
  `KindLayout` is walked (fixed offsets, then the tail region when present), returning whether
  `h` had a registered kind. All **four** tracer sites now route through it so they cannot
  disagree about *which words are references* — the co-totality that keeps mark, relocate, and
  the remembered set in lockstep:
  - `scan_payload` (mark — precise slots, then conservative fallback for `kind 0`),
  - `precise_children` (compaction classify — precise out-edges),
  - `fixup_ref_fields` (compaction fixup — rewrite moved refs; the interior-pointer
    `debug_assert` is preserved inside the callback),
  - `points_to_live_young` (generational barrier — old→young edge, via a `found` flag).
- The wrap-safe bound `off <= size - 8` (never `off + 8 <= size`, which could overflow for a
  near-`usize::MAX` offset) is applied to every produced slot, fixed or tail.
- **No behaviour change:** all 94 gc-core lib tests pass unchanged, clippy clean,
  `cargo miri test` clean over the whole lib (the four sites are the UAF-critical surface),
  and `gc-core-capi` / `aarch64-backend` / `x86_64-backend` build unchanged. This is PR-1 of 4
  (spec §7); PR-2 adds tail tracing + `register_ref_array_kind`, PR-3 the C ABI, PR-4 the
  native relocation differential.

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
