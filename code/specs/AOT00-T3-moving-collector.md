# AOT00-T3 — moving / compacting collector (design)

Status: **design, needs sign-off.** The next rung of the gc-core precision ladder after
precise roots. Unlike the earlier rungs (interior-precise → generational → precise-roots),
a *moving* collector is not a drop-in on the current substrate — it forces a real decision
about heap representation. This spec lays out that decision, a mobility model that is
**sound given today's mix of precise and conservative roots**, a phased plan, and a
differential test. It deliberately surfaces the substrate mismatch so we choose the shape
before writing relocation code (relocation bugs are use-after-free).

Prerequisite already merged: **precise roots on all 3 native targets** (aarch64 macOS,
x86-64 Linux, x86-64 Windows). A moving collector *needs* those — moving an object means
rewriting every pointer to it, and the only mechanism that can find-and-update a
mutator-held pointer is a precise stack-map slot (the slot's address is known, so the new
value can be written back). See [[project_gc_core_native_convergence]].

---

## 1. The substrate reality (why this isn't a drop-in)

Grounding facts from the current `gc-core` (`flat_heap.rs`):

- **The heap is discontiguous.** Every object is an independent `alloc_zeroed` block from
  the system allocator, threaded onto an intrusive singly-linked list (`FlatHeap.all`).
  There is **no arena, no capacity, no bump/`top` cursor, and no free-list** — `sweep`
  reclaims by `dealloc`-ing each unmarked block in place.
- **`FlatHeader` is 32 bytes** (`next:*mut`, `size:usize`, `marked:bool`, `kind:u16`,
  `generation:u8`, `_pad:[u8;11]`), guarded by `const _: () = assert!(size_of == 32)`.
  Payload = `header + 32`. **The 11-byte `_pad` has room for an 8-byte forwarding word**
  with no size change.
- **The mutator holds raw payload pointers directly** — `__gc_alloc` returns
  `header+32` as an `i64`; there is **no handle table / indirection layer** to swap an
  object out from under the mutator.
- **The only address-keyed heap-object state is `remembered: HashSet<usize>`** (old-parent
  payload addresses) plus the `all` list. The kind registry is keyed by a small integer
  `kind_id`, and the stack-map registry is keyed by *code* addresses — both move-safe.
- **Precise field maps** (`register_kind(field_offsets)`) name exactly which payload words
  are pointers for a registered `kind`; `kind == 0` objects are traced **conservatively**
  (every word is a maybe-pointer).

Two consequences drive the whole design:

1. **Sliding (mark-compact in place) is impossible** on a per-object `malloc` heap — there
   is no contiguous region to slide within. A moving collector here is necessarily a
   **copying / evacuating** collector: copy each movable live object into a fresh
   contiguous **to-space**, leave a forwarding pointer behind, fix up all pointers, then
   free the from-space.
2. **The classic payoff of compaction — defragmentation — is muted**, because fragmentation
   of a `malloc` heap is the system allocator's concern, not gc-core's. The real wins a
   moving collector unlocks here are (a) **bump-pointer allocation** (O(1) `alloc` in the
   to-space arena) and (b) **locality** (survivors packed together, faster tracing). Framing
   the goal as *arena + bump-alloc + evacuation* — not "defrag a malloc heap" — keeps the
   effort honest.

> **Sign-off question A.** Is the goal worth the arena rewrite now, or should we bank the
> lower-risk **generational aging** rung first (survive-N-minors before tenuring — small,
> uses existing machinery; see §8) and revisit moving later? This spec is written so either
> answer is informed.

---

## 2. Mobility model — what may move, what must be pinned (soundness core)

Moving an object is sound **only if every pointer to it can be found and rewritten.** With a
mix of precise and conservative roots, that is not universally true, so objects partition:

- **MOVABLE** — an object is movable iff **both**:
  - **(a) every incoming reference is precisely known**: all roots that reach it come through
    precise stack-map slots (`collect_mixed`'s `root_slots`, whose *slot address* we hold),
    and every heap object on a path to it is a **registered `kind`** (so its ref fields are
    exactly enumerable and rewritable via the field map); and
  - **(b) it is itself a registered `kind`** (so *its own* outgoing pointers can be updated
    after it moves).
- **PINNED** — everything else. An object is pinned if it is reachable through **any**
  conservative root (a `collect_region` span, `__gc_collect`'s stack scan, a callee-saved
  register region) **or** through **any** `kind == 0` (conservatively traced) object — because
  a maybe-pointer word cannot be safely overwritten (it might be an integer that merely looks
  like the old address), and a conservatively-scanned root's slot address is unknown.

**Transitive pinning.** Pinning propagates *backwards along the only-conservative frontier*:
if the sole references to object X are maybe-pointers (in a conservative frame or a `kind==0`
object), X pins. But an object reachable by *at least one* precise path is still movable only
if it has *no* conservative in-edge — otherwise a stale conservative maybe-pointer to its old
address would dangle. **Conservative reachability is a pin, not a maybe.** Concretely: mark in
two colors — `precise-reachable` and `conservative-reachable`; an object is movable iff
`precise-reachable && !conservative-reachable && kind != 0` and every object on some precise
path to it is likewise. This is conservative-safe: when unsure, pin (never move).

Today essentially the whole stack top is conservative (runtime/collector frames are
unmapped), so **early on almost everything pins and little moves** — that is *correct*, and
the differential (§7) is written to still show a real, non-trivial move. As more frames carry
maps, more objects become movable. Pinning is the safety valve that lets this rung land
*before* every frame is precise.

> **Sign-off question B.** Accept "conservative in-edge ⇒ pin" (simple, always sound, moves
> less early) vs. a more aggressive model (e.g. per-object pin bits set only when a
> conservative root is *observed* that cycle)? Recommendation: start with the simple model.

---

## 3. Header & heap changes

### 3.1 Forwarding word + pin bit (no size change)

Reuse `_pad` (offset 21, 11 bytes) for:

- `forward: usize` (8 bytes) — during a moving cycle, holds the object's **new payload
  address** once copied; `0` = not yet moved. Zero elsewhen (header is `alloc_zeroed`).
- `pinned: bool` (1 byte) — sticky within a cycle; set during marking when a conservative
  in-edge is found. Cleared at cycle end.
- 2 bytes spare remain; the `assert!(size_of == 32)` is unchanged.

(Alternative considered: reuse `next` as the forwarding word since the `all` list is
re-threaded after a move. Rejected — keeping `next` valid during the move simplifies the
walk and debugging; `_pad` is free.)

### 3.2 To-space arena

Introduce an **arena** type: a contiguous `alloc`'d region with a bump cursor
(`base`, `top`, `end`) and 16-byte alignment. A moving cycle:

1. sizes the arena to the marked-live movable bytes (sum during mark), rounded up;
2. bump-allocates each movable survivor into it (header + payload copied verbatim);
3. after fixup, the arena **becomes the new backing** for those objects. Pinned survivors
   stay as their original `malloc` blocks. So the post-move heap is *arena survivors + pinned
   malloc blocks* — the `all` list is re-threaded to cover both.

Future `alloc` may bump within the current arena when there's room, falling back to a fresh
`malloc` block (which is then pinnable/movable next cycle) when the arena is full — i.e. the
arena is an *optimization layer over* the existing per-object model, not a replacement. This
keeps the change incremental and preserves the conservative fallback everywhere.

> **Sign-off question C.** Arena-as-optimization-layer (above) vs. a full two-space flip
> (all allocation bumps in a semi-space; every full GC copies to the other). The layer model
> is lower-risk and composes with pinning; the two-space model is cleaner long-term but a
> bigger bang. Recommendation: layer model first.

---

## 4. The moving cycle (`collect_compacting`)

Built on the `collect_mixed(root_slots, regions)` decomposition (the only entry that already
separates precise, updatable roots from conservative, must-pin regions):

```
collect_compacting(root_slots, regions):
  1. MARK (two-color): from root_slots mark precise-reachable; from regions mark
     conservative-reachable (and set `pinned` on every object a conservative edge touches,
     transitively through kind==0 objects). Sum movable-live bytes.
  2. PLAN: arena = new bump region sized to movable-live bytes.
     For each MOVABLE survivor (precise-reachable, !pinned, kind!=0):
        new = arena.bump(header_size + size); copy header+payload to `new`;
        old.forward = new_payload_addr.
  3. FIX UP POINTERS (every surviving object + every precise root):
        - roots: for each precise root_slot, if *slot resolves into a moved object,
          write forwarded(*slot) back into the slot.
        - interior: for each surviving object (moved or pinned), for each ref field
          (from its kind field map — pinned kind!=0 objects still have exact maps),
          if the field points into a moved object, rewrite it to the forward address.
          (Tag bits in the low 3 preserved.)
        - remembered set: rebuild — a remembered old-parent that moved is re-inserted at
          its new address.
  4. SWEEP: dealloc unmarked from-space blocks; free the emptied movable from-space blocks
     (their contents now live in the arena); re-thread `all` over arena survivors + pinned
     blocks; clear marks, forwards, pin bits.
```

**Invariants to prove (these are the UAF surface):**

- *No dangling pointer after fixup:* every pointer that pointed at a moved object's old
  address is rewritten (roots via slots, interior via field maps, remembered via rebuild).
  Nothing else can hold such a pointer — conservative holders forced their target to pin
  (step 1), so a moved object has *no* conservative in-edge.
- *No double-move / no lost object:* `forward` is written exactly once (first visit);
  subsequent visits read it. Pinned objects are never copied.
- *Interior/tagged pointers:* `forwarded(w)` strips the low-3 tag, resolves via
  `find_header`, adds the same tag back to the new address; a word that isn't a heap pointer
  (precise: not a ref field; conservative: maybe) is never rewritten in a pinned object.
- *`collect_compacting(slots, &[])` with all-movable ⇒ full evacuation; with `&[]` roots and
  a conservative region ⇒ everything pins ⇒ behaves exactly as `collect_mixed` (no move).*
  This "strict generalization" mirrors how `collect_mixed` generalized precise+region.

---

## 5. C ABI

- `__gc_collect_compacting()` in `stack_scan.rs` — same frame-pointer walk as
  `__gc_collect_precise` (producing `slots` + `regions`), then `collect_compacting` instead of
  `collect_mixed`. Degrades to conservative pinning (no move) when no maps are registered, so
  it is always safe to call.
- `GcAlgorithm::Compacting::is_available()` flips to `true` (`policy.rs`), and the existing
  `SuggestSwitch(Compacting, …)` recommendation path (fires at
  `fragmentation > compacting_fragmentation_threshold = 0.40`) becomes actionable.
- No change to `__gc_alloc`'s contract for the mutator *between* GC points — pointers stay
  valid until a collection, exactly as documented today; a compacting collection is a GC
  point, and precise roots are what make it safe.

---

## 6. Interaction with generations

A moving cycle is a **full** collection (both generations). It **rebuilds** the remembered set
(old parents that moved re-inserted at new addresses) and clears it of dead parents, exactly
as the non-moving full collects already clear it. Minor (young-only) collections stay
non-moving for now — a moving *minor* (evacuate young survivors into old) is a later rung
(classic copying-nursery); this spec keeps minor collections as-is to bound scope.

---

## 7. Differential test plan (the proof this is load-bearing)

All in gc-core unit tests first (synthetic roots/heap, no asm), then a gc-core-capi host test,
then — if we take it that far — an executing native differential.

1. **Evacuation moves a precisely-rooted object; a conservatively-rooted twin pins.**
   Two registered-`kind` objects A, B. A is reachable only via a precise `root_slot`; B via a
   conservative `region`. Run `collect_compacting`. Assert: `A.forward != 0` and the root slot
   now holds A's new address (A moved); `B.forward == 0` and B's address is unchanged (B
   pinned). Both survive. This is the headline: *precise ⇒ movable, conservative ⇒ pinned.*
2. **Interior pointer fixup.** Registered-kind parent P (a ref field → child C), both movable,
   both reachable via a precise root to P. After `collect_compacting`, P moved, C moved, and
   P's ref field holds C's *new* address (not the stale one). Deref-through-P still reaches C.
3. **Tagged/interior pointer preserved.** A ref field holding `child_addr | tag` (low-3) is
   rewritten to `new_child_addr | tag` — tag bits intact.
4. **Strict generalization.** `collect_compacting(slots, &[])` with all-movable frees the same
   dead set as `collect_precise`, and additionally leaves survivors relocated + forwarded;
   `collect_compacting(&[], &[region])` moves nothing and matches `collect_region`.
5. **Remembered-set rebuild.** An old parent that moves is present in the remembered set at its
   *new* address afterward; a dead old parent is gone.
6. **(capi) executing smoke.** `__gc_collect_compacting()` on a real stack with a registered
   map: an object reachable only via a mapped frame slot is relocated and the slot updated;
   the program keeps using the (new) pointer and exits cleanly (a UAF would crash/ASAN).

---

## 8. PR breakdown (small, each CI-validated) & the lower-risk alternative

- **PR-1** — this spec (sign-off).
- **PR-2** — header forwarding word + pin bit in `_pad` (size assertion intact) + the
  two-color mark + `movable`/`pinned` predicate. No relocation yet; unit-test the predicate
  (precise-only ⇒ movable; any conservative in-edge ⇒ pinned).
- **PR-3** — the arena (bump region) + `collect_compacting` evacuation & pointer fixup, gated
  on all-registered-kind inputs; unit tests #7.1–#7.5. gc-core-only.
- **PR-4** — `__gc_collect_compacting` capi + `is_available()=true` + host smoke #7.6.
- **PR-5** — (optional) executing native differential once a real frame carries a map.

**Lower-risk alternative rung (generational aging), if sign-off defers moving:** the current
generational collector tenures a survivor after **one** minor (immediate tenuring); a small
`age: u8` in the header `_pad`, incremented each minor and promoted only at a threshold,
reduces premature tenuring. One gc-core PR + a differential (an object stays young for N−1
minors, tenures on the Nth). This is the "aging = future tuning" item flagged when the
generational rung landed — tractable, low-risk, and complementary to moving.

---

## 9. Risks

- **Soundness of the mobility model (UAF class).** The entire correctness of relocation rests
  on "a moved object has no conservative in-edge." §2's two-color mark + transitive pin
  enforces it; PR-2's predicate tests and PR-3's differential #7.1 are the guardrails. Any
  doubt ⇒ pin.
- **Substrate scope creep.** The arena touches `alloc`/`sweep`/the `all` list. Keep it an
  *optimization layer* (§3.2) with the malloc path intact as the fallback, so a bug degrades
  to today's behavior rather than corrupting the heap.
- **Payoff clarity.** For a malloc heap the win is bump-alloc + locality, not defrag; if that
  isn't worth the risk now, generational aging (§8) is the honest smaller step. This is
  **Sign-off question A**.
