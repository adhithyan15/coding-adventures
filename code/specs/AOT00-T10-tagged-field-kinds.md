# AOT00-T10 — tagged-field kind registration (vm-core joins the precise/movable GC for real)

> Status: **landed — all 3 PRs merged**. PR-1 (this spec) and PR-2 (`gc-core`'s
> `register_tagged_kind` mechanism, 0.35.0/0.36.0) are merged; PR-3 (`vm-core`
> wiring, 0.25.0 — `pair_kind` registration, `handle_gc_alloc` reroute, and the
> real relocation differential test proving a `gc_alloc`'d pair actually moves
> under a compacting collection) completes the arc. User directive (2026-08-11,
> verbatim intent): "The vm-core is currently not released. So feel free to change
> designs if needed to allow for vm-core and GC-core to work together. We shouldn't
> have 10 different GC implementations." This spec is the design change.

---

## 1. The problem, precisely

AOT00-T9 PR-5 shipped `vm-core`'s pacing wiring for the moving-minor collector,
but its own changelog is explicit about the gap it did *not* close: every
`vm-core` `gc_alloc` registers **kind 0** (opaque/conservative), so
`classify_mobility`'s `movable = precise ∧ ¬pinned ∧ kind≠0` rule pins every
object `vm-core` allocates. `collect_compacting`/`collect_minor_compacting`
degrade to non-moving whenever `vm-core` drives them — safe, but with zero
payoff. `vm-core` is also less *precise* than it could be: mark tracing
conservatively scans every payload word instead of tracing exactly the two
reference-shaped fields a cons/record cell actually has.

The obvious fix — mirror native-AOT's own `__dyn_cons`, which lazily
registers a `{0,8}` kind via `FlatHeap::register_kind` and allocates through
`__gc_alloc_kind` instead of the opaque path — was investigated during T9
PR-5 follow-up scouting and found **unsound as a direct port**. Documented in
lessons.md ("vm-core kind registration soundness note") and in a corrected
comment on `vm-core::handle_gc_alloc`; summarized here because closing it
properly is this spec's whole job:

`FlatHeap::register_kind`'s existing contract (call it **boxed mode**) says:
every `fixed`/`tail` offset is *always* a reference-candidate word — raw or
NaN-box-tagged with an **arbitrary** tag, both forms tried (`mark_word`,
`classify_precise_word`, `forwarded`). This is sound for native-AOT's own
records because native's own object model guarantees every field of a
kind-registered allocation is **boxed**: a genuine heap reference, never a
raw scalar (confirmed in the "records precise + movable" PR: "Record fields
are boxed (constructor params typed `any`) → `{0,8}` sound").

`vm-core`'s object model is different by design (`AOT00-T1v` §2.4): a
`gc_alloc`'d field is a **tagged word** — `Value::HeapRef` gets tag `0b111`
(low 3 bits, address masked so this is always exact — `FlatHeap::alloc`'s
16-byte payload alignment guarantees a real address's low 3 bits are
already clear), `Value::Int` gets tag `0b000` (the value is left-shifted 3
bits before storing, so its low 3 bits are *always* zero — never `0b111`).
A cons cell's car/cdr is *dynamically* typed (`type_hint = "ref<any>"`):
either field can legitimately hold a nested pair **or** a bare integer,
decided at runtime, not compile time. `(cons 1 2)` is completely ordinary
Lisp and stores two raw integers in a cell's ref-shaped slots.

Marking tolerates this fine — `mark_word` tag-strips before validating
against `find_header`, so a look-alike int that doesn't resolve to a live
block is simply never found (safe over-approximation, the same shape as
every conservative-scan false positive this codebase already accepts).
**Compaction does not.** `fixup_ref_fields`'s `forwarded()` helper rewrites
a precise field's bits whenever they match a key in `forward` (the map of
this cycle's relocated addresses), with **no way to distinguish** "this
word is really a reference" from "this word is really an int that
coincidentally has the same bit pattern as some unrelated object's old
address." That's not a safe bias-to-leak/pin-when-unsure case — it's a
real, wrong-direction correctness bug (an int's value silently corrupted to
a stale pointer) if it ever fired, categorically different from every other
probabilistic-collision argument already accepted elsewhere in this
codebase (all of which retain *too much*, never rewrite something wrongly).

**Astronomically unlikely in practice is not the same as sound**, and this
codebase's own culture (every GC PR in this arc gets an adversarial security
review specifically hunting this class of bug) would not accept shipping it
on a probability argument alone when an exact fix is available. It is:
vm-core's tag bits are not a *heuristic* — they are the ground truth the
program itself already computed about what a word is. `0b111` means
reference, unconditionally; anything else means not-a-reference,
unconditionally. A kind-registration mode that trusts the tag exactly,
instead of the boxed-mode "maybe a pointer, try both forms" approximation,
is sound for exactly the reason a boxed-mode registration is unsound: it
uses information boxed mode doesn't have.

## 2. The fix — a new field-encoding mode on `KindLayout`, not a new collector

Per the user's own framing, the goal is **one shared GC**, not a
vm-core-specific mechanism bolted alongside `gc-core`'s real one. This is
additive to `FlatHeap`'s existing kind-registration API, not a new type of
collector or a vm-core-private tracer:

```rust
struct KindLayout {
    fixed: Box<[usize]>,
    tail_from: Option<usize>,
    /// T10: whether `fixed`/`tail` slots hold NaN-boxed TAGGED words (a slot is a
    /// reference iff its low `NAN_BOX_TAG_BITS` bits equal `NAN_BOX_REF_TAG`;
    /// anything else is PROVABLY not a reference) rather than always-boxed
    /// references (the original "boxed mode" — try raw and tag-stripped forms,
    /// since either COULD be a pointer). `false` for every existing (boxed)
    /// registration; `true` only for kinds registered via `register_tagged_kind`.
    tagged: bool,
}
```

New crate-level constants (gc-core becomes the **canonical** definition;
`vm-core`'s own `FIELD_TAG_MASK`/`FIELD_TAG_HEAP_REF`/`FIELD_TAG_BITS`
literals are replaced with re-exports/imports of these — one definition,
not two copies that could drift):

```rust
/// Low-bit NaN-box tag width every tagged-mode field uses (3 bits — matches
/// `FlatHeap::alloc`'s 16-byte payload alignment, which guarantees a real
/// address's low 3 bits are always clear).
pub const NAN_BOX_TAG_BITS: u32 = 3;
pub const NAN_BOX_TAG_MASK: usize = 0x7;
/// The exact tag value that means "this word is a reference" under tagged
/// mode. Any other value in the low `NAN_BOX_TAG_BITS` bits is PROVABLY not
/// a reference — not "probably not", the caller-side invariant this mode
/// depends on (see `register_tagged_kind`'s own doc).
pub const NAN_BOX_REF_TAG: usize = 0x7;
```

**The entire fix is filtering `for_each_ref_slot`** — the one function every
consumer (`scan_payload`'s mark path, `precise_children`/`classify_mobility`,
`fixup_ref_fields`) already goes through to enumerate a kind's ref slots:

```rust
unsafe fn for_each_ref_slot(&self, h: *mut FlatHeader, mut f: impl FnMut(*mut usize)) -> bool {
    let kind = (*h).kind;
    if kind == 0 { return false; }
    let layout = match self.field_maps.get((kind - 1) as usize) {
        Some(l) => l, None => return false,
    };
    let base = h.add(1) as *mut u8;
    let size = (*h).size;
    let mut maybe_yield = |off: usize| {
        let slot = base.add(off) as *mut usize;
        if layout.tagged {
            let word = ptr::read_unaligned(slot as *const usize);
            if word & NAN_BOX_TAG_MASK != NAN_BOX_REF_TAG {
                return; // provably not a reference under the tagged convention — skip
            }
        }
        f(slot);
    };
    for &off in layout.fixed.iter() {
        if size >= 8 && off <= size - 8 { maybe_yield(off); }
    }
    if let Some(start) = layout.tail_from {
        let mut off = start;
        while size >= 8 && off <= size - 8 { maybe_yield(off); off += 8; }
    }
    true
}
```

**Why nothing downstream needs to change.** Every consumer only ever sees
slot pointers `for_each_ref_slot` actually yields:

- `scan_payload` (mark): reads the yielded slot's word and calls
  `mark_word`, which tag-strips before `find_header` — unaffected either
  way, and now never even attempts a `find_header` lookup on an `Int`
  field's bit pattern, since tagged mode already excluded it upstream.
- `precise_children`/`classify_mobility`: reads the yielded slot's word and
  calls `classify_precise_word` — same argument; an `Int` field is never
  passed in to be (mis)classified as base/interior in the first place.
- `fixup_ref_fields`: reads the yielded slot's word and calls `forwarded()`.
  For a genuinely ref-tagged word (`0b111`), `forwarded()`'s existing
  raw-lookup-then-tag-stripped-lookup-with-reattach logic is **already
  exactly right** — `forward`'s keys are untagged base addresses, so the raw
  lookup (word still carrying its `0b111` tag) always misses, falls through
  to the tag-stripped branch (`word & !0x7`), finds the real entry if the
  child moved, and reattaches the tag on rewrite (`nw | tag`) — this is the
  *same* mechanism the existing McCarthy-lisp tagged-pointer case already
  exercises for boxed-mode fields. Nothing in `forwarded`/`fixup_ref_fields`
  needs to change; it was already correct for a word it's *actually asked
  to look at* — the bug was only ever that boxed mode asked it to look at
  words it shouldn't have.

**New registration entry point, additive** (mirrors how `register_ref_array_kind`
was added alongside `register_kind`, not merged into it — and, critically,
does **not** touch `register_kind`'s existing signature, which
`gc-core-capi`'s `__gc_register_kind` C ABI wraps and native-AOT/LLVM
codegen already emits calls to in shipped compiled output; this spec's
scope is vm-core, not a breaking change to the native path):

```rust
impl FlatHeap {
    /// Register a **tagged-field** kind (T10) — `field_offsets` are NaN-boxed
    /// TAGGED words (reference iff low `NAN_BOX_TAG_BITS` bits == `NAN_BOX_REF_TAG`),
    /// not always-boxed references. Sound ONLY if the caller's own field-store path
    /// enforces the tag convention on every write to every slot named here — see
    /// this method's own Safety section. `vm-core`'s `handle_gc_field_store` already
    /// does (AOT00-T1v §2.4's tag scheme); a boxed-record producer (native-AOT/LLVM)
    /// must keep using `register_kind` instead.
    ///
    /// # Safety
    /// Every write to a registered offset must go through the same tag convention
    /// (`NAN_BOX_REF_TAG` for a reference, anything else for a non-reference) — this
    /// is what makes `for_each_ref_slot`'s tag check exact rather than heuristic. A
    /// caller that writes an untagged raw pointer, or a value whose tag
    /// coincidentally equals `NAN_BOX_REF_TAG`, into a tagged-mode slot breaks the
    /// soundness argument this whole mode rests on.
    pub unsafe fn register_tagged_kind(&mut self, field_offsets: &[usize]) -> u16 { ... }
}
```

## 3. Wiring `vm-core` onto it

`VMCore::new()` registers one tagged kind for the 2-field `{0,8}` shape
every current `alloc`/`gc_alloc` emission site uses (confirmed by grep: all
three sites in `iir-builtin-lowering::dyn_repr_structural` pass `vec![]`,
always the 16-byte default — matches `AOT00-T1v`'s own finding), alongside
the existing `set_auto_minor(true)` attestation:

```rust
let mut heap = gc_core::FlatHeap::new();
heap.set_auto_minor(true);
let pair_kind = unsafe { heap.register_tagged_kind(&[0, 8]) };
```

`handle_gc_alloc` (`vm-core/src/dispatch.rs`) uses `pair_kind` **only** when
`bytes == 16` — the one shape proven to match the registered layout;
anything else (a future caller passing a different `srcs[0]`) falls back to
kind 0, unconditionally sound per `FlatHeap`'s own "unregistered/kind-0
falls back to conservative" invariant, exactly the same conditional-kind
pattern `lower_alloc_array`'s `elem_is_gc_reference` branch already uses
for LLVM array allocation:

```rust
let kind = if bytes == 16 { ctx.pair_kind } else { 0 };
let ptr = ctx.heap.alloc(bytes as usize, kind);
```

`pair_kind: u16` threads through `DispatchCtx` as a plain `Copy` field
(alongside `u8_wrap`, `max_frames`, etc. — the existing by-value fields),
populated from `VMCore`'s own new `pair_kind` field wherever `DispatchCtx`
is constructed from `&mut VMCore`.

**No change needed to `handle_gc_field_store`/`handle_gc_field_load`** — the
tag scheme they already implement (AOT00-T1v §2.4) is *exactly* the
convention `register_tagged_kind`'s safety contract requires; this spec
does not touch them, it only tells `gc-core` to trust what they were
already doing.

**No change needed to `run_safepoint`'s dispatch** (AOT00-T9 PR-5) — once
`vm-core`'s cons/record objects are kind-registered and genuinely movable,
`should_compact()`/`should_compact_minor()` firing will make
`collect_compacting`/`collect_minor_compacting` **actually relocate them**
for the first time, closing PR-5's own "not yet load-bearing for
relocation" gap for free, with zero further wiring.

## 4. Safety argument (summary)

- **Marking**: unaffected. Tagged-mode filtering only *removes* candidates
  from what `for_each_ref_slot` yields; it never adds one `mark_word`/
  `classify_precise_word` wouldn't already have handled correctly. A
  removed (Int) candidate was never a real reference, so removing it from
  the precise wave cannot under-mark — and it was never conservatively
  reachable either (kind-registered objects don't feed
  `conservative_children`), so there's no double-counting concern.
- **Compaction rewrite**: this is the actual fix. Before: an `Int` field's
  bit pattern was a live candidate for `forwarded()`'s lookup — sound only
  probabilistically. After: `for_each_ref_slot` never yields an `Int`
  field's slot to `fixup_ref_fields` at all under tagged mode, so
  `forwarded()` is never even asked to consider it. The only words
  `fixup_ref_fields` sees under tagged mode are ones the field-store path
  itself tagged `0b111` at write time — genuine references, by
  construction, not by inference.
- **The `unsafe fn register_tagged_kind`**: unsafe (unlike `register_kind`,
  which is safe) because its soundness is conditional on a caller-side
  invariant (every write to a registered offset respects the tag
  convention) that `gc-core` cannot verify — the same class of contract
  `set_auto_minor`'s own `unsafe`-adjacent framing already establishes for
  barrier coverage. `vm-core`'s `handle_gc_field_store` satisfies it
  today, provably (traced in §1); a future caller must re-derive that
  proof for its own field-store path before calling this, not assume it.
- **Interaction with `classify_mobility`'s pin-when-unsure model**: a
  tagged-mode object's *own* out-edges are traced precisely (same as boxed
  mode) once this lands, so it participates in the movable set exactly like
  a boxed-mode record does — no change to the mobility algorithm itself,
  only to which words a given kind's `for_each_ref_slot` call considers
  candidates in the first place.
- **What does NOT change**: `register_kind`/`register_ref_array_kind`'s
  existing signatures, behavior, and every existing boxed-mode kind
  (native-AOT/LLVM's cons/record/ref-array kinds) — `tagged: false` for all
  of them, byte-for-byte the pre-T10 code path. `gc-core-capi`'s C ABI is
  untouched (no `__gc_register_tagged_kind` — vm-core links `gc-core`
  directly as a Rust dependency, no C boundary to cross for this).

## 5. Staged PR plan

1. **PR-1 (this spec)** — sign-off gate, no code.
2. **PR-2 — `gc-core` core mechanism.** `KindLayout.tagged` + the
   `NAN_BOX_*` constants + `for_each_ref_slot`'s filter + `register_tagged_kind`.
   gc-core-only. Tests: (a) a tagged kind's ref-tagged field IS traced and
   relocated correctly under `collect_compacting`; (b) **the headline
   regression proof** — a tagged kind's Int-tagged field whose bit pattern
   is deliberately engineered to coincidentally equal a real moved object's
   old address is **never rewritten** (the exact defect this spec exists to
   close — construct the collision on purpose, assert the int's value is
   unchanged after compaction); (c) every existing boxed-mode
   (`register_kind`) test still passes unmodified, proving `tagged: false`'s
   default path is untouched. Miri-clean. Adversarial security review.
3. **PR-3 — `vm-core` wiring.** `register_tagged_kind(&[0,8])` at
   `VMCore::new()`, `pair_kind` threaded through `DispatchCtx`,
   `handle_gc_alloc`'s conditional dispatch, gc-core's `NAN_BOX_*` constants
   replacing vm-core's own duplicate `FIELD_TAG_*` literals. Tests: a real
   relocation differential (`tests/gc_heap.rs`) proving a cons cell
   allocated via `gc_alloc`/`(cons ...)`-shaped IIR actually **moves** under
   a forced `collect_compacting`/`collect_minor_compacting` and stays
   correct (car/cdr readable, values intact) at its new address — the proof
   PR-5's own changelog said vm-core couldn't yet offer. Full
   `lang_matrix.rs` Vm/Jit column re-verified green (the E6d cons/list/
   record/union/closure regression backstop `AOT00-T1v` §4 already
   establishes). Security review.

Each PR: Miri-clean, adversarial security review (this is precisely the bug
class — a field whose bits get rewritten when they provably shouldn't be —
the review process exists for), matching this arc's own established bar.
