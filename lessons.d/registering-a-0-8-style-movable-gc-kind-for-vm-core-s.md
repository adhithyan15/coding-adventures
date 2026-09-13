# Registering a `{0,8}`-style movable GC kind for `vm-core`'s generic `gc_alloc` is NOT a safe drop-in, unlike native-AOT's `__dyn_cons`/records — vm-core's fields are tagged words, not always-boxed (AOT00-T9 PR-5 follow-up scouting)

While scoping the next GC work item after AOT00-T9 PR-5 (moving-minor pacing) landed, I
looked at `vm-core`'s own "not yet load-bearing for relocation" limitation that PR-5's
changelog documents: every `vm-core` `gc_alloc` registers kind `0` (opaque/conservative),
so `collect_compacting`/`collect_minor_compacting` never actually relocate anything when
driven by vm-core — they degrade to non-moving behavior every time, safely but with zero
payoff. The obvious-looking fix: mirror native-AOT's own `__dyn_cons`, which lazily
registers a `{0,8}` kind (both 8-byte fields are reference slots) via
`__gc_register_kind` and allocates through `__gc_alloc_kind` instead of the opaque
`__gc_alloc` — `FlatHeap::register_kind`/`alloc(n, kind)` are already `pub`, so wiring
this into `vm-core::handle_gc_alloc` looks like a small, mechanical change.

**It is not sound, and I did not implement it.** Native-AOT's `{0,8}` cons/record kind
is safe only because native's own object model guarantees every field of a
kind-registered allocation is **boxed** — a genuine heap reference, never a raw scalar
(see the "records precise + movable" PR: "Record fields are boxed (constructor params
typed `any`) → `{0,8}` sound"). `vm-core`'s object model is different: `handle_gc_field_store`
(`vm-core/src/dispatch.rs`) stores a **tagged word** per field — `Value::HeapRef` gets one
tag (`FIELD_TAG_HEAP_REF`, `0b111`), `Value::Int` gets a different one (shifted, no `0b111`
low bits) — so a field vm-core allocates via `gc_alloc` can legitimately hold a raw integer,
not just a reference. This is true even restricted to cons cells specifically: `(cons 1 2)`
routinely stores bare integers as car/cdr, so "just register cons kind, not the generic
`alloc`" doesn't dodge the problem — cons is precisely the case where a field is sometimes a
ref and sometimes not.

Traced why this matters for *compaction* specifically (not just marking, which already
tolerates this fine): `mark_word` (`gc-core/src/flat_heap.rs`) tag-strips before checking
`find_header`, so a raw tagged int is simply never found as a live block — safe, standard
over-approximation-is-fine marking. But `fixup_ref_fields`'s `forwarded()` helper — the
function that decides whether to *rewrite* a precise field's bits during a moving
collection — looks the word (raw or tag-stripped) up as a **key in the `forward`
HashMap** (the set of addresses that were *actually* relocated this cycle) and only
rewrites on a hit. A raw int field is therefore rewritten **only if its bit pattern
happens to exactly equal some unrelated object's old base address** — astronomically
unlikely in practice, but a real, wrong-direction correctness bug if it ever fired (an
int's value silently corrupted to a stale pointer bit pattern), not a "safe
over-approximation" the way conservative-scan false positives are. This is a
categorically different risk from the pin-when-unsure/bias-to-leak arguments that justify
every *other* accepted probabilistic-collision case in this codebase (all of which retain
*too much*, never rewrite something wrongly).

**Not pursued further this session** — fixing this for real needs either (a) type-directed
field maps so vm-core can tell gc-core which specific field offsets are *always* references
for a given allocation site (a real new feature: field-level type tracking doesn't exist in
vm-core's IIR-op interface today), or (b) an explicit, reviewed decision to accept the
collision risk with a written soundness argument bounding it (this codebase's security
reviews have not been asked to accept this exact class of risk before, unlike the
already-reviewed conservative-scan collision cases). Either is a real design decision, not
a quick follow-up PR — flagging it here so a future session doesn't rediscover the same
trap by implementing the "obvious" mechanical version.

**Resolved in AOT00-T10** (2026-08-11, following an explicit owner directive that vm-core,
being unreleased, is free to change design rather than work around it: "We shouldn't have
10 different GC implementations"). Option (a) above — type-directed field maps — turned out
not to be the only way to give gc-core "which words are references" ground truth: vm-core
already computes that ground truth *dynamically*, once per store, via the tag bits this exact
lesson describes (`FIELD_TAG_HEAP_REF` vs. everything else). `gc-core` gained a second,
**tagged** kind-registration mode (`FlatHeap::register_tagged_kind`) that trusts a slot's own
tag bits at scan time instead of assuming every slot is always a reference — closing the
`forwarded()` collision risk exactly, not just bounding it. See
`code/specs/AOT00-T10-tagged-field-kinds.md` for the full design and `vm-core`'s
`VMCore::pair_kind` for the wiring.
