# AOT00-T5 — variable-length reference arrays (precise + movable object model) (design)

> Status: **design, pre-implementation.** Committed for sign-off before any code, exactly as
> [`AOT00-T3-moving-collector.md`](AOT00-T3-moving-collector.md) and
> [`AOT00-T4-incremental-collector.md`](AOT00-T4-incremental-collector.md) were.
>
> The precision-ladder **algorithms** are complete
> (`mark-and-sweep ✓ → interior-precise ✓ → generational ✓ → precise-roots ✓ → compacting ✓ → incremental ✓`).
> This rung closes the remaining **object-model** gap — the one thing still standing between
> `gc-core` and *"solid enough to run JavaScript, Ruby, Python and compile to an AOT binary."*

---

## 1. The problem — a kind can describe a record, but not an array

`gc-core`'s precise tracing is driven by **kinds**. `register_kind(field_offsets)` records, per
kind, the byte offsets of the reference-typed fields of an object of that kind; the tracer then
follows exactly those offsets instead of scanning every payload word conservatively. This is
the mechanism that makes an object **precise** (no look-alike integer pins a phantom child) and,
because a precisely-traced object has no conservative in-edges, **movable** (the compacting
collector may relocate it — proven end-to-end for cons cells, PR #8936).

The offset list is **fixed per kind**. That is exactly right for a **record / struct** — a
`Point{x,y}`, a cons cell `{car, cdr}`, a closure header — whose reference fields sit at
statically-known offsets shared by every instance of the kind.

It **cannot describe a variable-length reference array**, where the number of reference slots
is a property of the **instance**, not the kind:

| language | value | payload shape |
|----------|-------|---------------|
| JavaScript | `Array` (packed elements) | `len` words, each a tagged reference |
| Ruby | `Array` | `RARRAY` — `len` `VALUE`s |
| Python | `list` | `ob_item` — `len` `PyObject*` |
| Lisp/Scheme | `vector` | `len` boxed slots |
| any | hash/dict backing store | `2n` key/value reference slots |

An array of length *n* would need a kind with offsets `[0, 8, 16, …, 8(n−1)]` — a **different
kind per length**, i.e. unbounded kinds, one registered per distinct array size. That is
absurd, so today every such object is allocated as **`kind == 0`** (opaque) and traced
**conservatively**. Conservative tracing has two costs that are individually tolerable but
jointly fatal to a real language runtime:

1. **False retention.** Any array element that *looks like* a pointer but is an inline integer
   pins a phantom object. Acceptable for a batch program; a steady leak for a long-running one.
2. **Pinning — the real blocker.** A conservatively-traced object has conservative *out*-edges,
   so the compacting collector's pin wave **pins it and every object it points at**. Arrays are
   the most common heap object in JS/Ruby/Python, so "arrays pin everything they reference"
   means **compaction relocates almost nothing** — the moving collector, though complete, is
   inert against real workloads. (This is the array-shaped analogue of the cons-cell blocker
   that #8936 fixed for pairs.)

So: the moving/incremental machinery is all built, but the dominant object of every target
language can't use it. Closing that is this rung.

---

## 2. The generalization — a "tail-ref region"

A kind's reference layout becomes **fixed reference offsets, plus an optional tail region every
word of which is a reference**:

```
kind layout ::= { fixed: [offset…],  tail_from: Option<offset> }
```

Tracing an object of this kind visits:

1. each `off` in `fixed` (a reference field at a statically-known offset — the record case), then
2. if `tail_from == Some(start)`, **every aligned 8-byte word** in `[start, size)` — the
   array-elements case, whose count follows from the instance's own `size`.

This single addition expresses every object model above with **one kind per class of object**,
not one per length:

| object | `fixed` | `tail_from` |
|--------|---------|-------------|
| record / cons cell (today) | `[car, cdr]` | `None` |
| opaque leaf (string, boxed number) | `[]` | `None` |
| **pure reference array** | `[]` | `Some(0)` |
| **length-prefixed array** (`{len:i64, elems…}`) | `[]` | `Some(8)` — the `len` at offset 0 is a non-ref word, skipped |
| **header + elements** (`{class_ptr, len, elems…}`) | `[0]` (class_ptr is a ref) | `Some(16)` |
| **closure** (`{code_ptr:non-ref, captures…}`) | `[]` | `Some(8)` |

`fixed` before `tail_from` composes cleanly: a boxed array object can carry a reference header
field *and* a variable ref tail. Everything the current fixed-offset model does is the
`tail_from == None` special case, so the change is a **strict generalization** — existing kinds
and every existing test are unaffected.

### Why a contiguous tail, not a per-word ref bitmap?

A bitmap (one bit per payload word: ref / non-ref) is strictly more expressive and is the
natural *next* rung if a language needs interior mixed layouts (e.g. a struct-of-arrays element
type). We deliberately do **not** start there: the tail-region model covers every array/vector/
list/hash backing store — the objects that actually block compaction — with a **two-word**
descriptor and **no per-object metadata**, keeping the header at 32 bytes and the hot tracer
loop branch-light. The bitmap is called out as future work in §8, and the layout type is shaped
so a `Bitmap(Box<[u64]>)` variant can be added later without touching the four tracer call sites.

---

## 3. Data-structure & API changes

### 3.1 `field_maps` becomes a richer per-kind layout

Today (`flat_heap.rs`):

```rust
field_maps: Vec<Box<[usize]>>,   // entry k = ref-field offsets of kind k+1
```

becomes

```rust
/// Per-kind reference layout, indexed by `kind_id - 1`. Kind 0 is reserved for
/// "opaque / trace conservatively" and never appears here.
field_maps: Vec<KindLayout>,

struct KindLayout {
    /// Reference fields at statically-known offsets (the record case). Sorted, deduped.
    fixed: Box<[usize]>,
    /// If `Some(start)`, every aligned 8-byte word in `[start, size)` is a reference
    /// (the variable-length array tail). `None` ⇒ pure record (today's behaviour).
    tail_from: Option<usize>,
}
```

`registered_kinds()` and the `kind_id - 1` indexing are unchanged. `KindLayout` derives nothing
that touches the header; it lives only in the side table.

### 3.2 API — additive

```rust
/// Register a pure record kind (unchanged; `tail_from == None`).
pub fn register_kind(&mut self, field_offsets: &[usize]) -> u16   // existing

/// Register a kind whose payload is `fixed` reference fields plus a tail region
/// `[tail_from, size)` of contiguous references. `tail_from` should be a multiple of 8
/// and ≥ the end of any fixed offset; a non-multiple is rounded up (documented) so the
/// tail scan stays 8-aligned.
pub fn register_ref_array_kind(&mut self, fixed: &[usize], tail_from: usize) -> u16   // new
```

`register_kind(offs)` is defined as `register_ref_array_kind`-with-`tail_from = None`
internally, so there is exactly one construction path.

### 3.3 C ABI (`gc-core-capi`)

```c
// existing
uint16_t __gc_register_kind(const uintptr_t* offsets, size_t count);
// new — tail_present==0 ⇒ pure record (equivalent to __gc_register_kind)
uint16_t __gc_register_ref_array_kind(const uintptr_t* fixed, size_t fixed_count,
                                      int tail_present, uintptr_t tail_from);
```

plus the `__twig_*` compat alias, following the `__gc_register_kind` precedent exactly. No new
allocation entry point is needed: an array object is still allocated with
`__gc_alloc_kind(size, kind)` — the kind now carries the tail descriptor, and `size` (already
per-instance) determines the element count.

---

## 4. The four tracer sites — must extend *co-totally*

The layout is consumed at **four** places, all of which currently iterate `offsets.iter()` over
the fixed list. Every one must learn the tail region, or an array is traced inconsistently
(the classic *"a validation walk must be co-total with the emitter"* hazard — a site that scans
a subset of what another site relocates causes a dangling pointer). Grounded line references
(`code/packages/rust/gc-core/src/flat_heap.rs`, pre-T5):

| # | site | role | tail behaviour to add |
|---|------|------|-----------------------|
| 1 | `scan_payload` (~1212) | **mark**: follow children | after fixed offsets, `mark_word` every word in `[tail_from, size)` |
| 2 | `precise_children` (~1278) | **compaction classify**: precise out-edges | push every tail word's target as a precise (movable, not pinning) child |
| 3 | `fixup_ref_fields` (~1511) | **compaction fixup**: rewrite moved refs | `forwarded()`-rewrite every tail word too |
| 4 | `points_to_live_young` (~1852) | **generational barrier**: old→young edge? | a tail word pointing at a young block counts |

The shared discipline for all four is one helper:

```rust
/// Visit every reference word of `h` (fixed offsets, then the tail region) with `f`,
/// applying the existing wrap-safe bound `size >= 8 && off <= size - 8` to each.
unsafe fn for_each_ref_word(&self, h: *mut FlatHeader, mut f: impl FnMut(usize /*word*/, usize /*off*/));
```

Refactoring the four sites onto `for_each_ref_word` (each passing its own `f`) means the
tail-region logic exists **once** and the sites cannot drift — the same tactic the sweep rung
used with `sweep_free_or_keep`. The bound stays the wrap-safe `off <= size - 8` (never
`off + 8 <= size`) so a near-`usize::MAX` `tail_from` from a bad FFI caller can't wrap.

**Bound & alignment safety.** The tail loop walks `off = tail_from; while off + 8 <= size { …; off += 8 }`
with `tail_from` clamped to a multiple of 8 at registration, so every read is 8-aligned and
in-bounds; `read_unaligned` is retained defensively. A `tail_from > size` array (shorter than
its header) yields an empty tail — safe, no reads.

---

## 5. Soundness argument

- **No under-marking.** A tail word is marked exactly as a fixed ref field is (`mark_word`,
  raw + tag-stripped). Any element the mutator can reach through the array is visited, so the
  array retains its live elements precisely — no live element is swept.
- **No new UAF under moving.** A precisely-traced array has **no** conservative out-edge, so the
  pin wave does not pin its elements — they become movable. The fixup site rewrites every tail
  word through `forwarded()` (base/tagged-base only, the existing invariant), so after
  relocation every in-edge from the array to a moved element points at the new copy. The
  existing debug-assert (a precise ref field holds a base/tagged-base pointer, not an interior
  pointer) is applied to tail words too.
- **Pin-when-unsure preserved.** An array *element slot that the frontend fills with a non-
  reference* (e.g. an unboxed small integer in a packed JS array) is the frontend's contract to
  honour: a tail region declares *"every word here is a reference."* A frontend with mixed
  packed elements must either box them or use a header/`tail_from` layout that excludes the
  non-ref region — **or** keep the object `kind 0` (conservative), which remains always-safe.
  The tail model never makes a conservatively-safe program unsafe; it only *upgrades* a frontend
  that opts in with a truthful layout. (This mirrors the cons-cell contract from #8936.)
- **Strict generalization.** `tail_from == None` reproduces today's tracer byte-for-byte, so
  every existing collector property and test is preserved.

---

## 6. Test plan (gc-core, Miri-clean)

1. **Precise array trace.** A length-3 ref array referencing objects A,B,C plus a look-alike
   integer element → A,B,C survive, the phantom is reclaimed; the conservative `kind 0` twin
   retains the phantom (load-bearing contrast).
2. **Array is movable.** Register an array kind, build an array of cons cells, `collect_compacting`
   → the array **and** its elements relocate to the arena and every element slot is rewritten in
   place; reads through the array hit the new locations. The `kind 0` twin pins them (contrast).
3. **Header + tail.** `{class_ptr, len, elems…}`: the `class_ptr` fixed ref and the element tail
   are both traced; the `len` non-ref word between them is not treated as a pointer.
4. **Mixed / bound edges.** `tail_from == size` (empty array) traces nothing and is safe;
   `tail_from` past `size` is empty; unaligned `tail_from` is rounded up at registration.
5. **Generational.** An old array storing a young element records the old→young edge
   (`points_to_live_young` sees the tail) → a minor GC keeps the young element.
6. **Cycle.** Array ↔ record cycle marks and sweeps correctly (no infinite loop — `marked`
   guards the worklist as today).
7. **C ABI.** `__gc_register_ref_array_kind` round-trips; a compiled/differential path (native)
   allocates an array kind and relocates it under `__gc_collect_compacting`.

---

## 7. PR breakdown (each gc-core-only until the last, each Miri'd + security-reviewed)

- **PR-1 — `KindLayout` + `for_each_ref_word` refactor.** Introduce `KindLayout { fixed, tail_from }`,
  route the four tracer sites through `for_each_ref_word`, keep `tail_from = None` everywhere.
  Pure refactor: **zero behaviour change**, all existing tests green — lands the risky
  four-site unification on its own, provable by "no diff in outcomes."
- **PR-2 — tail tracing + `register_ref_array_kind`.** Make `for_each_ref_word` walk the tail;
  add the registration API. Tests 1–6 above.
- **PR-3 — C ABI `__gc_register_ref_array_kind` + `__twig_*` alias.** Test 7 (round-trip).
- **PR-4 — native differential.** A frontend array/vector registers the kind and relocates
  under compaction end-to-end (mirrors #8936's cons-cell differential), on the locally-runnable
  aarch64 target first, then x86_64/Linux+Windows CI.

The first *observable* payoff (a real array relocating instead of pinning) lands at PR-4; PR-1
is a safe, self-contained de-risking of the multi-site change.

---

## 8. Non-goals / future rungs

- **Per-word ref bitmap** for arbitrary interior mixed layouts (struct-of-arrays element types,
  NaN-boxed inline-value arrays with a side tag map). The `KindLayout` type is shaped to gain a
  `Bitmap` variant without touching the four tracer sites.
- **Weak references / ephemerons** (JS `WeakMap`, Ruby `WeakRef`): a separate tracing discipline
  (don't mark through the weak slot; clear it if the target dies). Its own rung.
- **Finalizers at scale** (`__del__`, `ObjectSpace.define_finalizer`): the `HeapKind.finalizer`
  seam exists; ordering/resurrection semantics are a separate rung.
- **Concurrent (multi-mutator) collection**: still explicitly out of scope (single-mutator
  contract, as every `gc-core` collector).

These are the object-model tail; the tail-ref-region rung is the one that unblocks the
**common** case — every language's array — and thereby makes the moving/incremental collectors
load-bearing for JS/Ruby/Python.
