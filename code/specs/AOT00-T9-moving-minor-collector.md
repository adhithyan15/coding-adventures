# AOT00-T9 — moving minor collector (young-generation compaction)

> Status: **landed, including PR-5** — PR-1 (this spec, sign-off) through PR-4
> (`collect_minor_compacting`, the full live cycle) were merged first; §5's optional PR-5
> (auto-triggering pacing, mirroring `should_compact`) has since landed too:
> `FlatHeap::should_compact_minor` (the moving-minor pacing predicate), the
> `__gc_collect_minor_compacting` C-ABI entry (+ `__twig_gc_collect_minor_compacting` alias), and
> a 4-way dispatch in both automatic collection sites (`gc-core-capi`'s `__gc_safepoint` and
> `vm-core`'s `run_safepoint`) that upgrades an already-decided minor cycle to a moving one when
> fragmentation independently warrants it. Per §7's own note, the `AOT00-T8` `auto_minor`
> attestation gate's documentation was revisited and updated for the strengthened obligation a
> moving minor cycle imposes before this wiring landed (see `FlatHeap::auto_minor`'s field doc).
>
> Builds on the now-complete precision ladder
> (`mark-and-sweep ✓ → interior-precise ✓ → generational ✓ → precise-roots ✓ → compacting ✓ → incremental ✓`)
> and [`AOT00-T8`](AOT00-T8-adaptive-safepoint-scheduling.md) (adaptive safepoint scheduling, now
> live end-to-end on `vm-core`). This is the one combination the ladder never built: **moving
> *and* generational at once** — a minor cycle that evacuates its young survivors into a compact
> arena, instead of sweeping them in place. Every other collector combination already exists;
> this is the last cell in that 2×2.

---

## 1. The gap

Today, `FlatHeap` has exactly two "does real relocation" entry points, and they are mutually
exclusive with generational scoping:

| | non-moving (in-place sweep) | moving (evacuate to arena) |
|---|---|---|
| **full scope** (old + young) | `collect`/`collect_region`/`collect_precise`/`collect_mixed` | `collect_compacting` |
| **young-only scope** | `collect_minor`/`collect_minor_region`/`collect_minor_mixed` | **— missing —** |

A minor cycle is the *cheap, frequent* one (AOT00-T8 just made it the automatic choice under a
low-survival-ratio workload) — exactly the collection that runs often enough for fragmentation
among young survivors to actually accumulate and matter. But it never moves anything: survivors
are aged/tenured in place, at whatever address they were allocated. A high-allocation-rate,
short-lived-object workload (the JS/Ruby/Python shape the whole precision ladder targets) is
precisely the case where the young generation churns fastest and a moving nursery earns back the
most locality — and it's the one case this collector can't do it for.

---

## 2. Why this isn't "just call `collect_compacting` with `young_only=true`"

`collect_compacting`'s soundness rests on one invariant, stated in its own doc comment:

> **A reachable object survives *in place* iff its `pinned` bit is set; every
> reachable-but-not-moved object is pinned.**

That invariant is *proven* by `classify_mobility`'s traversal, which only knows about two root
sources: `root_slots` (exact stack-map slots) and `regions` (conservative spans). It has no
concept of a remembered set, because a *full* collect doesn't need one — `collect_mixed`'s own
liveness mark doesn't consult the remembered set either; it just traces from `root_slots`/`regions`
directly, and that reaches everything (there is no "old generation" concept to route around).

A *minor* cycle's liveness mark is different in exactly the way that breaks this: `collect_minor_mixed`
reaches young survivors two ways — directly from `root_slots`/`regions` (`mark_word`/`mark_region`,
gated `young_only`), **and** transitively through the **remembered set** (`minor_finish`'s
`for parent in remembered { scan_payload(parent, work, true) }`). A young object reachable *only*
through a remembered old parent is correctly found *alive* by that second path — but naively
calling today's `classify_mobility(root_slots, regions)` on a minor cycle never seeds from the
remembered set at all, so that same object would be **absent from `classify_mobility`'s own
`precise`/`pinned` sets entirely**. It isn't wrongly *pinned* (safe-but-suboptimal); it's invisible
to the classification pass, which breaks the "reachable ⟺ pinned ∨ movable" invariant `collect_compacting`'s
step 4 (mark-survivors-in-place-via-`pinned`, sweep, integrate) depends on to decide what to free.
Silently omitting the remembered set doesn't just miss an optimization — reusing `collect_compacting`'s
sweep unmodified against that broken invariant would sweep a **live, reachable** young object as if
it were dead. **This is the actual reason the naive one-line "add `young_only`" approach is unsound
and this needs its own design, not a parameter.**

---

## 3. The fix — extend `classify_mobility`'s seeding to mirror the liveness mark exactly

The liveness mark and the mobility classification must see **the same reachability graph**, or the
invariant above breaks. `minor_finish`'s liveness mark already has the right shape — this spec
copies it into `classify_mobility`, split across its two waves the same way `scan_payload` already
splits per-object:

- `scan_payload(h, work, young_only)` (liveness) dispatches on `h`'s **own** kind: `for_each_ref_slot`
  (precise, kind-registered fields only) if `h.kind != 0`, else a full conservative payload scan.
- `classify_mobility` already has that exact split as two *named* helpers: `precise_children` (kind's
  tracked ref fields — used to grow the **precise** wave) and `conservative_children` (every word —
  used to grow the **pinning** wave).

So: for each remembered old parent `p`,

- if `p.kind != 0`: feed `precise_children(p)` into the **precise wave** (exactly as a normal
  precise-wave-discovered kind≠0 object's own children already are) — a young child reached only
  through a kind-tracked old parent's tracked field is precise-reachable, and thus a movability
  *candidate* (still subject to the usual "not pinned" test from every other angle).
- if `p.kind == 0`: feed `conservative_children(p)` into the **pinning wave** directly — a kind-0
  parent's fields are opaque raw words (exactly what the existing "any kind-0 object's children are
  conservative candidates" rule already says for objects discovered *within* the wave); relocating a
  child reachable only this way would leave a stale, unrewritable raw word in `p`'s payload, a real UAF.

This is the *only* seeding change. Everything downstream — the transitive precise/pinning
traversal, the final `movable = precise ∧ ¬pinned ∧ kind≠0` filter — is unchanged, **plus one more
conjunct**: `movable` additionally requires `generation == GEN_YOUNG`. An **old** object can appear
in the precise-reachable set (e.g. a root points at it directly, and it happens to be kind-registered
so its own young children are worth tracing through) — that's fine and necessary for traversal, but
it must never itself be relocated by a *minor* cycle: nothing rewrites old→old edges from *other* old
objects during a minor collect (that's a full-collect's job), so moving an old object here would
silently orphan any old parent's pointer to it.

```rust
pub unsafe fn classify_mobility_minor(
    &mut self,
    root_slots: &[usize],
    regions: &[(*const u8, usize)],
) -> HashSet<usize> {
    // ... identical precise-wave / pinning-wave setup and traversal to classify_mobility ...
    // EXTRA seeding step, before draining either wave:
    let remembered: Vec<usize> = self.remembered.iter().copied().collect();
    for parent in remembered {
        let h = (parent - HEADER_SIZE) as *mut FlatHeader;
        if (*h).kind == 0 {
            self.conservative_children(h, &mut cwork); // -> pinning wave
        } else {
            self.precise_children(h, &mut tmp); // -> precise wave (existing insert-and-enqueue loop)
        }
    }
    // ... existing transitive traversal, unchanged ...
    // Final filter gains one conjunct:
    // if !(*h).pinned && (*h).kind != 0 && (*h).generation == GEN_YOUNG { movable.insert(...) }
}
```

**Proof sketch that this restores the invariant** (the reviewable claim, not just an assertion):
`scan_payload`'s per-object dispatch (precise fields for kind≠0, full conservative scan for kind 0)
is *structurally identical*, object-by-object, to `precise_children`/`conservative_children`'s own
per-object dispatch — the same `for_each_ref_slot` call, the same fallback. The liveness mark's
transitive closure and `classify_mobility_minor`'s transitive closure therefore visit exactly the
same nodes via exactly the same edges, from the same seed set (`root_slots` ∪ `regions` ∪
remembered-parents' children) — so every object the liveness mark can reach is either in `precise`
(via the precise-wave path) or discovered by the pinning wave (conservative path), i.e. reachable ⟹
pinned ∨ (precise ∧ …). This is the same proof `collect_compacting`'s doc comment already sketches
for the full case; nothing about the argument is generation-specific except which seed set is used.

---

## 4. The reclaim/integrate step — a minor-scoped `collect_compacting`

`collect_compacting`'s step 4 (mark survivors-in-place via `pinned`, sweep, integrate the arena,
rebuild the remembered set) needs a young-scoped sibling, `collect_minor_compacting`:

1. **Mark**: liveness-mark exactly as `collect_minor_mixed` does today (`mark_word`/`mark_region`
   over `root_slots`/`regions`, then the remembered-parent scan, then drain `work`) — **unchanged**,
   this determines who survives.
2. **Classify**: run `classify_mobility_minor(root_slots, regions)` (§3) — determines, among young
   objects, who's movable.
3. **Evacuate**: `plan_compaction_minor`/`evacuate_and_fixup_minor` — young-scoped siblings of
   `plan_compaction`/`evacuate_and_fixup`, parameterized on `classify_mobility_minor`'s `movable`
   set. **Correction (PR-3, landed, revised across two review rounds):** this point originally
   claimed "no change to the copy/fixup mechanics themselves, only which set drives them" — that
   claim does not hold. `evacuate_and_fixup`'s existing fixup only rewrites (a) `root_slots` and (b)
   moved objects' own arena copies; neither touches an old parent's field pointing at a moved young
   child, since such a parent is never itself movable (old objects are never in `movable`).
   `evacuate_and_fixup_minor` therefore adds a genuine third step, (c) — but (c) must walk **two**
   populations, unioned, neither subsuming the other:
   - **`precise`** (every precise-reachable object `classify_mobility_minor`'s internal traversal
     actually *discovered as a node* — header addresses, movable or not, transitively including old
     objects reached through other old objects) — catches a parent reached *directly by a root* (or
     transitively through another precise-reachable old object), which may never appear in the
     remembered set at all if no barriered store ever named it.
   - **`self.remembered`** — catches a parent used *only* as a remembered-set **seed**: such a
     parent's children are consulted and fed into the traversal, but the parent itself is never
     independently *discovered* as a node by that traversal, so it is **not** a member of `precise`.
   A first-draft fix walked only `self.remembered` (necessary, not sufficient — missed the
   directly-rooted case, next bullet's citation). The fix for *that* walked only `precise` (also
   necessary, not sufficient — broke the original remembered-seed-only case, caught immediately by
   the existing test suite). The shipped version unions both; each half was independently confirmed
   load-bearing by reverting it and observing the exact predicted failure, and a third round of
   review confirmed no third population exists (traced through: a multi-hop
   old→old→young chain with no barriers, a parent that is a member of both sets simultaneously, and
   a tagged reference — all tested).
   - **Residual, and now correctly scoped, dependency on write-barrier fidelity**: a parent reached
     *only* through the very store that should have been barriered — not independently
     root/region/precise-reachable, and never recorded in `self.remembered` because the barrier was
     skipped — is covered by **neither** population. See §7's revised barrier-contract note: this is
     new, and stricter than what a non-moving minor cycle requires.
4. **Sweep + integrate**: **young-only** version of `collect_compacting`'s step 4 — sweep only young
   blocks (mirroring `sweep(true)`'s existing `young_only` branch, which already skips old blocks
   entirely and just counts them as survived-in-place), free unmarked young blocks, keep
   pinned-live young survivors in place (aged, possibly tenured — identical to today's minor sweep),
   re-thread the arena's moved objects into `self.all` (identical to `collect_compacting`'s own
   integration, generation-tagged young — freshly-moved survivors have *not* tenured this cycle,
   matching a same-place minor survivor's own aging), and — **do not clear the remembered set**
   (a minor cycle never invalidates it, same as `collect_minor`/`collect_minor_region` today). The
   remembered *set itself* holds parent addresses, not child addresses, so it does not need remapping
   (a remembered *parent* is always old, and old objects are never movable, so no remembered-set entry
   is ever itself stale after a minor-compacting cycle) — PR-3's step (c) above is what makes this true
   by construction, by rewriting every such parent's *field* in place (under an honest write barrier;
   see the residual dependency noted above) before this step runs, rather than this step needing to
   reconstruct anything from `forward`.
5. `minor_streak`/`adapt_threshold` bookkeeping — identical to `collect_minor_mixed`'s existing tail.

~~Point 4's parenthetical is the one genuinely new piece of reasoning versus a mechanical merge...~~
**Superseded by the PR-3 correction above**: the new reasoning turned out to belong in step 3 (an
explicit fixup step over two unioned populations, not a single set), not step 4 (a remap derived
after the fact) — the original framing had the right instinct (something extra was needed) but
placed and justified it incorrectly, twice, before landing on the correct shape. PR-3's two rounds of
adversarial review are the "own adversarial pass" this paragraph called for.

---

## 5. Staged PR plan (mirroring how `AOT00-T3` shipped: land the UAF-sensitive parts in isolation)

1. **PR-1 (this spec)** — sign-off gate, no code. ✅ merged.
2. **PR-2 — `classify_mobility_minor` alone**, dry-run only (returns the `HashSet`, relocates
   nothing) — reviewed and unit-tested against `classify_mobility`'s existing test shapes *plus* a
   remembered-set-specific case (a young object reachable *only* via a precisely-traced old parent is
   `movable`; reachable only via a non-precisely-traced old parent — kind-0, or a nonzero kind id never
   registered — is *not* `movable` but *is* found — i.e. `pinned` — proving it isn't silently
   invisible). Mirrors PR-3a's "land the classification in isolation" shape.
   ✅ **Landed** (`gc-core` 0.30.0; see 0.29.0 for the `is_precisely_traced` fix `classify_mobility_minor`
   was built on top of from the start) — 9 tests, including both load-bearing remembered-set cases and
   the two-prior-bugs-combined case an adversarial review specifically flagged as missing. Design held;
   the review found scoping/documentation gaps for future consumers (recorded in the function's own
   doc), not implementation bugs.
3. **PR-3 — evacuate + fixup**, parameterized on `classify_mobility_minor`'s output — proves the arena
   copy + pointer rewrite (roots, moved objects' own fields, **and** any remembered-parent field that
   pointed at a moved child) against a differential, still not wired into a live collection.
   ✅ **Landed** (`gc-core` 0.31.0) — `plan_compaction_minor`/`evacuate_and_fixup_minor`, 10 tests. Found
   and fixed a real gap in this very spec's §4 point 3/4 (see the correction there, revised across two
   review rounds): the field-rewrite this bullet promises is **not** free from reusing "the existing...
   mechanics unchanged" as originally written — it needed a genuine new step (c) that unions two
   populations (`precise` and `self.remembered`), neither of which alone is sufficient. Round 1 of
   review found the single-remembered-walk design's directly-rooted-old-parent gap; the fix for that
   (walking `precise` alone) was itself caught by the existing test suite as breaking the original
   remembered-seed-only case before it ever reached round 2 of review. Round 2 confirmed the union is
   complete under an honest write barrier and flagged the residual (and now documented) barrier-fidelity
   dependency in §7.
4. **PR-4 — `collect_minor_compacting`**, the full cycle (§4) wired end-to-end, plus the `should_collect_minor`
   family gains no new *scheduling* decision (out of scope here — whether an automatic minor cycle
   should sometimes also compact is a follow-up policy question, not this spec's; PR-4 only makes
   the moving-minor primitive *callable*, the same way `collect_compacting` shipped before
   `should_compact` existed).
   ✅ **Landed** (`gc-core` 0.33.0). **Design correction versus this bullet's own §4 point 1**: the
   spec describes "Mark" (a `collect_minor_mixed`-style liveness pass) as a step *separate* from
   "Classify" (`classify_mobility_minor`) — implying two independent traversals. The shipped
   implementation runs only **one**: `evacuate_and_fixup_minor`'s own internal classification call
   already computes a `pinned` bit on every object it traverses, proven (§3) to be the same closure
   a separate liveness mark would compute — exactly mirroring how `collect_compacting` itself needs
   no separate mark pass, reusing `classify_mobility`'s own `pinned` bits as its keep-in-place
   predicate. `collect_minor_compacting` does the same, restricted to `generation == GEN_YOUNG`.
   Proven via the canonical differential this section describes (below), plus 6 further tests, and
   confirmed load-bearing by reverting the young-survivor marking step and observing two tests fail
   exactly as predicted.
5. **PR-5 (optional, follow-up)** — wire `should_compact`-style pacing so an automatic minor cycle
   sometimes evacuates instead of sweeping in place, once PR-4 is proven stable. Not required for
   this arc to be "complete" — `collect_compacting` itself shipped useful and callable before
   `should_compact` existed to auto-trigger it.
   ✅ **Landed.** `FlatHeap::should_compact_minor` (deliberately does *not* reuse
   `AdaptivePolicy::evaluate`'s single mutually-exclusive top-1 pick — see that method's own doc for
   why a naive reuse would be structurally unable to ever fire once `should_collect_minor` has
   already observed `evaluate` recommend `Generational`; it instead re-checks `AdaptivePolicy`'s own
   fragmentation threshold directly, an independent second axis, not another priority rung) +
   `__gc_collect_minor_compacting` C-ABI entry (mirrors `__gc_collect_compacting`'s frame-pointer
   walk, gated by the same `auto_minor` attestation `__gc_collect_minor_precise` already enforces) +
   `__twig_gc_collect_minor_compacting` alias + a 4-way dispatch in both `__gc_safepoint`
   (`gc-core-capi`) and `run_safepoint` (`vm-core`). Per this spec's own §7 note, the `auto_minor`
   attestation gate's field/`set_auto_minor` doc comments were updated first, both to correct a
   now-stale claim (native-AOT/LLVM's `field_store`/`array_set` barrier emission had since shipped,
   contradicting an earlier "does not emit the barrier" claim) and to document the moving cycle's
   strictly stronger obligation. Execution-level regression coverage
   (`vm-core/tests/gc_heap.rs`'s `safepoint_stays_minor_scoped_when_should_compact_minor_also_fires`)
   proves the dispatch keeps minor SCOPE even when the fragmentation signal independently fires
   alongside the generational one — confirmed load-bearing via a revert check (routing to full
   `collect_compacting` instead) that reproduces the exact predicted failure. Does **not** prove
   actual relocation through `vm-core`, since every `vm-core` allocation is kind-0 today (no IIR op
   registers a movable kind there) — relocation correctness itself is `gc-core`'s own already-proven
   concern (PR-2 through PR-4's tests); what this rung proves is that the *dispatch* reaches the
   right primitive, not that the primitive relocates anything new.

Each PR: Miri-clean, adversarial security review (this is exactly the class of bug — a
reachable-but-unclassified object — the review process exists for), and a real differential proving
the specific new behavior (mirroring `end_to_end_gc_compacting_relocates_and_preserves`'s pattern
for PR-4: a program with an old parent, a live young child reachable only through it, and dead young
garbage; assert the child relocates *and* the old parent's field was rewritten to the new address
*and* the dead garbage was reclaimed *and* an unrelated genuinely-old, untouched object was never
scanned or moved).

---

## 6. Safety argument (summary; each PR's own review re-derives its slice)

- §3's seeding change is the only new *reachability* logic; §3's proof sketch shows it produces the
  same closure the already-proven liveness mark does, restoring the "reachable ⟺ pinned ∨ movable"
  invariant `collect_compacting`'s step 4 already relies on for the full-scope case.
- The `generation == GEN_YOUNG` conjunct on `movable` is a pure narrowing (never *adds* a candidate,
  only removes old objects that would otherwise be structurally eligible) — it cannot make anything
  unsoundly movable; it can only make the pass conservative in the safe direction (an old object
  simply isn't moved, exactly as before this spec).
- Evacuation/fixup **primitives** (`Arena`, `forwarded()`, `fixup_ref_fields`) are entirely reused,
  unmodified, from the already-shipped, already-reviewed `collect_compacting` machinery. **Correction
  (PR-3, landed, revised across two review rounds):** the *orchestration* around them is not a pure
  reuse, though — root-slot rewrite and moved-object field rewrite alone do **not** cover an old
  parent's field pointing at a moved young child, so `evacuate_and_fixup_minor` adds one new
  orchestration step — running the *same*, unmodified `fixup_ref_fields` primitive over the union of
  `precise` (every precise-reachable object the classifier's traversal discovered as a node — catches
  a directly-rooted or transitively-reached old parent) and `self.remembered` (catches an old parent
  used only as a remembered-set *seed*, never independently discovered as a node, so absent from
  `precise`). The primitives stayed unmodified; the claim that *no new step at all* was needed did
  not hold, and neither did the claim (from this step's own first correction) that walking one of the
  two populations alone would suffice. PR-3's two rounds of review each proved a gap empirically
  (revert the relevant half of the union, observe the exact predicted stale-field failure) before
  proving the fix closes it.
- **Residual barrier-fidelity dependency (round-2 finding, not eliminated):** the union above is
  complete only under an honest write barrier. A parent reached *only* through the very store that
  should have been barriered — not independently root/region/precise-reachable, and never recorded in
  `self.remembered` because the barrier was skipped — is covered by neither population. A **non-moving**
  minor cycle tolerates this exact missed barrier (the child, if independently reachable, is simply
  marked and kept live; if not, both die together, which is correct); a **moving** minor cycle does
  not, since nothing rewrites the parent's stale field once the child relocates. This is a strictly
  stronger barrier obligation than §7 originally claimed unchanged — see §7's revision.

---

## 7. What does *not* change

- `collect_minor`/`collect_minor_region`/`collect_minor_mixed` (non-moving minor) are untouched —
  this is a new, additional entry point, not a replacement.
- `collect_compacting` (full moving) is untouched.
- Tenuring and the AOT00-T8 `auto_minor` attestation gate itself are unchanged by this spec.
- **Correction (PR-3 round-2 review) — the write-barrier contract is NOT identical between a
  non-moving and a moving minor cycle; a moving cycle is strictly stricter.** The original claim here
  (that a moving minor "carries the identical barrier-coverage requirement" per AOT00-T8 §2b) does not
  hold and was disproven empirically (§6's residual-dependency bullet). A non-moving minor cycle
  tolerates a barrier the embedder skipped, *as long as the child is independently reachable* —
  nothing about that liveness mark depends on rewriting a stale field, since nothing relocates. A
  moving minor cycle does not have this slack: `evacuate_and_fixup_minor`'s fixup (§4 point 3, §6)
  covers a barrier-missed parent only if that parent is *itself* independently reachable via
  `root_slots`/`regions`/a precise chain from one of those — a barrier-missed parent that is reachable
  *only* through the very store the barrier should have recorded is invisible to both the `precise`
  and `self.remembered` populations the fixup walks, and its field goes stale (dangling, once PR-4
  wires reclamation in) the moment its child relocates. **Before PR-4 wires this into any automatically
  triggered cycle, the AOT00-T8 `auto_minor` attestation gate's own documentation and any embedder
  guidance must be revisited against this strengthened obligation** — an embedder whose barrier
  coverage was merely "good enough" for the existing non-moving minor (tolerates occasional missed
  barriers on independently-reachable objects) is not automatically good enough for a moving one.
