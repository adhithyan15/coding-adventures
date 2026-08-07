# AOT00-T9 — moving minor collector (young-generation compaction) (design)

> Status: **design, needs sign-off** — same bar as [`AOT00-T3`](AOT00-T3-moving-collector.md) and
> [`AOT00-T4`](AOT00-T4-incremental-collector.md): committed for review before any relocation code
> lands, because a mobility-classification bug here is a use-after-free, not a slow path.
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
3. **Evacuate**: `plan_compaction`/`evacuate_and_fixup`'s existing arena-copy + pointer-fixup logic,
   parameterized on the `movable` set from step 2 instead of full `classify_mobility`'s — **no
   change to the copy/fixup mechanics themselves**, only which set drives them.
4. **Sweep + integrate**: **young-only** version of `collect_compacting`'s step 4 — sweep only young
   blocks (mirroring `sweep(true)`'s existing `young_only` branch, which already skips old blocks
   entirely and just counts them as survived-in-place), free unmarked young blocks, keep
   pinned-live young survivors in place (aged, possibly tenured — identical to today's minor sweep),
   re-thread the arena's moved objects into `self.all` (identical to `collect_compacting`'s own
   integration, generation-tagged young — freshly-moved survivors have *not* tenured this cycle,
   matching a same-place minor survivor's own aging), and — **do not clear the remembered set**
   (a minor cycle never invalidates it, same as `collect_minor`/`collect_minor_region` today), but
   **do remap any moved *old→young* remembered entry to the child's new address** (an old parent's
   recorded child address is now stale if that child just relocated — this is the one piece of
   bookkeeping `collect_compacting`'s full-scope `rebuild_remembered` gets for free by clearing and
   redoing everything from scratch, that a minor-scoped version must do surgically instead: for each
   `forward` entry, if the *old* address matches a value implicitly held by a remembered parent's
   field, that field was already rewritten by the fixup pass in step 3 — no separate bookkeeping
   needed there. The remembered *set itself* holds parent addresses, not child addresses, so it does
   not need remapping unless a remembered *parent* was itself young and moved — impossible, since the
   remembered set holds only old objects.)
5. `minor_streak`/`adapt_threshold` bookkeeping — identical to `collect_minor_mixed`'s existing tail.

Point 4's parenthetical is the one genuinely new piece of reasoning versus a mechanical merge of
"§3's classify" + "existing minor sweep" + "existing compacting integrate" — it needs its own
adversarial pass in review, not just a restatement here.

---

## 5. Staged PR plan (mirroring how `AOT00-T3` shipped: land the UAF-sensitive parts in isolation)

1. **PR-1 (this spec)** — sign-off gate, no code.
2. **PR-2 — `classify_mobility_minor` alone**, dry-run only (returns the `HashSet`, relocates
   nothing) — reviewed and unit-tested against `classify_mobility`'s existing test shapes *plus* a
   remembered-set-specific case (a young object reachable *only* via a kind-tracked old parent is
   `movable`; reachable only via a kind-0 old parent is *not* `movable` but *is* found — i.e. `pinned`
   — proving it isn't silently invisible). Mirrors PR-3a's "land the classification in isolation" shape.
3. **PR-3 — evacuate + fixup**, parameterized on `classify_mobility_minor`'s output — proves the arena
   copy + pointer rewrite (roots, moved objects' own fields, **and** any remembered-parent field that
   pointed at a moved child) against a differential, still not wired into a live collection.
4. **PR-4 — `collect_minor_compacting`**, the full cycle (§4) wired end-to-end, plus the `should_collect_minor`
   family gains no new *scheduling* decision (out of scope here — whether an automatic minor cycle
   should sometimes also compact is a follow-up policy question, not this spec's; PR-4 only makes
   the moving-minor primitive *callable*, the same way `collect_compacting` shipped before
   `should_compact` existed).
5. **PR-5 (optional, follow-up)** — wire `should_compact`-style pacing so an automatic minor cycle
   sometimes evacuates instead of sweeping in place, once PR-4 is proven stable. Not required for
   this arc to be "complete" — `collect_compacting` itself shipped useful and callable before
   `should_compact` existed to auto-trigger it.

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
- Evacuation/fixup mechanics (arena copy, `forwarded()`, root-slot rewrite, moved-object field
  rewrite) are **entirely reused, unmodified**, from the already-shipped, already-reviewed
  `collect_compacting` machinery — the only new code is *which set* feeds them.
- The one piece of bookkeeping without a direct full-scope analogue — remembered-parent fields
  pointing at a moved child — is covered by the *existing* fixup pass already visiting every moved
  object's own fields plus root slots; §4's point 4 argues this needs no separate step, but that
  argument is exactly what PR-3's differential (a live young child reachable *only* through a
  remembered old parent, relocated, then read back *through that same old parent's field*) must
  prove empirically, not just by this spec's prose.

---

## 7. What does *not* change

- `collect_minor`/`collect_minor_region`/`collect_minor_mixed` (non-moving minor) are untouched —
  this is a new, additional entry point, not a replacement.
- `collect_compacting` (full moving) is untouched.
- No change to the write-barrier contract, tenuring, or the AOT00-T8 `auto_minor` attestation gate —
  a moving minor cycle carries the *identical* barrier-coverage requirement a non-moving minor cycle
  already does (§2b of AOT00-T8), since it's still fundamentally a minor-scoped collection that
  depends on the remembered set for old→young reachability.
