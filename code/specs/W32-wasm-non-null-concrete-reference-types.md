# W32 — non-null concrete reference types (`(ref $t)`), scoping and design

## Purpose and how this slice was chosen

This is a **spec-only** PR, per this repo's "specs before implementation"
convention. It does not implement anything — it exists to turn a wall that
five independent investigations across this session's WASM-conformance
campaign have each hit and deferred (W20, W23, W24, a dedicated PR58
investigation, and most recently W31's `struct.wast`/`ref_null.wast`/
`type-subtyping.wast` work) into a single, concrete design document a future
session can implement against without re-deriving the scope from scratch.

As of this spec, the WASM conformance corpus (pinned at
`WebAssembly/testsuite@28864811cf03bdbf880733786148feaba339582d`) stands at
255/257 files vendored. The remaining gap is small in file count but
concentrated entirely behind this one wall:

- `ref_null.wast` — needs genuine **bottom reference types**
  (`nullfuncref`/`nullexternref`/`nullexnref`/`nullref`) as real subtypes of
  every compatible reference type, not the current lossy aliasing.
- `type-subtyping.wast` — needs real **structural subtype checking** for
  `call_indirect`/`ref.cast` against concrete (function and struct) types,
  which requires non-null refs to express precisely.
- `extern.wast` — partially blocked on this (also needs `anyref` round-trip
  conversion opcodes and declarative element segments, tracked separately).

Beyond the conformance corpus, this wall is also the single remaining blocker
for:

- **`call_ref.wast` / `return_call_ref.wast`** (function-references
  proposal) — a `call_ref`/`return_call_ref` instruction's operand type is
  `(ref $t)`, non-null by construction (there is no such thing as a null
  `call_ref` target — that's what `call_indirect` + a trap is for).
- **`struct.wast`'s own function signatures and field lists** — e.g.
  `(param (ref $vec))` and mixed field lists like
  `(field i8 ... (ref 0) (ref null 1))` — non-null and nullable concrete refs
  side by side in ONE struct declaration.
- **`array.wast`/`array_new_data.wast`/`array_new_elem.wast`** completing
  past their current W31 "parses, mostly `not_yet_supported`" state.
- **`ref_cast.wast`/`ref_test.wast`/`br_on_cast.wast`/`br_on_cast_fail.wast`/
  `br_on_non_null.wast`** — every one of these instructions either produces
  or consumes a non-null concrete ref as its "success" type.

In short: this is now the **single highest-leverage remaining piece** of the
WASM interpreter's post-MVP-proposal surface. Implementing it doesn't just
unlock 2-3 more corpus files directly — it's the prerequisite for the entire
rest of the GC epic and the function-references half of the tail-calls epic.

## What already exists (verified against current code, not assumed)

`wasm_types::ValueType` (`code/packages/rust/wasm-types/src/lib.rs`) has:

```rust
pub enum ValueType {
    I32, I64, F32, F64,
    Anyref,              // (ref null any) — 0x6E
    I31ref,              // (ref i31) — 0x6C — NOTE: despite the name/doc,
                          //   this is NOT a nullable type at the WASM level;
                          //   this crate's I31ref is used as if non-null.
    StructRef(u32),       // (ref null $T) into the struct-type space — 0x63 <idx>
    ConcreteFuncRef(u32), // (ref null $t) into the func-type space — 0x63 <idx>
                          //   (shares the 0x63 tag with StructRef; disambiguated
                          //   by which index space the type index falls in)
    Funcref,              // funcref — 0x70
    Externref,            // externref — 0x6F
    V128,                 // v128 — 0x7B
    Exnref,               // (ref null exn) — 0x69
}
```

Every reference-carrying variant here is **nullable** (or, for `I31ref`,
treated as an unboxed non-heap value that doesn't participate in null
checks at all). There is no representation anywhere in this crate stack for:

1. A **non-null** concrete reference — `(ref $t)`, no `null` keyword, for
   either a struct type or a function type.
2. The four **bottom reference types** — `nullfuncref`, `nullexternref`,
   `nullexnref`, `nullref` — which the GC/function-references/exceptions
   proposals define as subtypes of *every* compatible reference type in
   their respective hierarchies (func/extern/exn/any), used as the type of
   a bare `(ref.null func)` etc. before it's been assigned to a more
   specific slot.

The binary encoding side (`code/packages/rust/wasm-module-encoder`,
`wasm-module-parser`) only knows how to round-trip the types above; the
text-format side (`wasm-wast-parser`) parses `(ref null $t)` (added across
W11-B and W26) but has no path for `(ref $t)` without `null`. The validator's
subtyping relation (`wasm-validator::type_check`, `is_assignable`-shaped
logic) only knows the flat nullable lattice — it has no concept of a
non-null variant being a strict subtype of its nullable counterpart, nor of
bottom types being subtypes of everything in their hierarchy.

## Design

### 1. `wasm-types::ValueType` — new variants

Add exactly four new variants, deliberately mirroring the existing nullable
ones rather than redesigning the enum's shape (this repo's `ValueType` is
consumed by many crates; a wholesale redesign — e.g. a generalized
`Ref { nullable: bool, heap_type: HeapType }` — would touch far more call
sites than this epic's actual scope requires):

```rust
/// `(ref $T)` — NON-NULL reference to a concrete struct type.
/// Binary: 0x64 <LEB128(idx)> (the function-references proposal's
/// "non-null" type-constructor byte, distinct from 0x63's "nullable").
NonNullStructRef(u32),

/// `(ref $t)` — NON-NULL reference to a concrete function type.
/// Binary: 0x64 <LEB128(idx)>, same tag-byte/index-space disambiguation
/// as ConcreteFuncRef vs StructRef today.
NonNullConcreteFuncRef(u32),

/// `nullfuncref` — bottom type of the func hierarchy. Binary: 0x73.
NullFuncref,

/// `nullexternref` — bottom type of the extern hierarchy. Binary: 0x72.
NullExternref,

/// `nullexnref` — bottom type of the exn hierarchy. Binary: 0x74.
NullExnref,

/// `nullref` (a.k.a. `none`) — bottom type of the any hierarchy,
/// subtype of Anyref/I31ref/StructRef(_)/NonNullStructRef(_). Binary: 0x71.
NullRef,
```

(Binary tag bytes above are the real GC/function-references proposal values
— verify against the reference interpreter's `interpreter/binary/decode.ml`
before implementing, the same discipline W24 used to catch the `exnref`
tag-byte bug.)

### 2. `wasm-validator` — subtyping lattice

The core new rule, applied wherever a value of one `ValueType` is used where
another is expected (`pop_expect`-shaped call sites):

- `NonNullStructRef(i) <: StructRef(i)` (same index) and, transitively,
  `NonNullStructRef(i) <: Anyref`.
- `NonNullConcreteFuncRef(i) <: ConcreteFuncRef(i) <: Funcref`.
- `NullFuncref <: Funcref` and `NullFuncref <: ConcreteFuncRef(_)` for
  every func-type index (it's the bottom of the WHOLE func hierarchy, not
  just the general one) — but `NullFuncref` is **not** `<: NonNullConcreteFuncRef(_)`
  (null can never satisfy a non-null slot).
- Same shape for `NullExternref <: Externref`, `NullExnref <: Exnref`.
- `NullRef <: I31ref`, `NullRef <: StructRef(_)` (any index), `NullRef <: Anyref`
  — but not `<: NonNullStructRef(_)`.
- The reverse direction never holds (a nullable type is never a subtype of
  its non-null counterpart) — this is the exact asymmetry W11-B's
  `return_call.wast`/`return_call_indirect.wast` work already encoded for
  `ConcreteFuncRef <: Funcref`; this epic generalizes the same pattern one
  level deeper, it does not change that existing one-directional rule.

`call_indirect`/`call_ref`'s own type-checking need real **structural**
subtyping for function types (checking one func type is a "subtype" of
another per the GC proposal's structural rules, not just index equality) —
this is `type-subtyping.wast`'s actual remaining gap per W31's investigation,
and is the one piece of this epic that's genuinely open-ended rather than a
bounded enum-and-lattice extension. Recommend scoping it as its own
follow-up slice AFTER the representation work above lands and is proven
against the corpus files that don't need structural subtyping.

### 3. `wasm-wast-parser` — text-format grammar

Add `(ref $t)` (no `null` keyword) parsing alongside the existing
`(ref null $t)` path (both live in the same `parse_value_type`-shaped
function per W11-B/W26's precedent). Add the four bottom-type keywords
(`nullfuncref`, `nullexternref`, `nullexnref`, `nullref`/`none`) as
recognized atoms wherever `funcref`/`externref`/etc. are today.

### 4. `wasm-module-parser` / `wasm-module-encoder` — binary round-trip

Add decode/encode support for the six new tag bytes (0x64, 0x71, 0x72, 0x73,
0x74). Also cross-check `wasm-execution`'s blocktype/type-index decoders for
the same "does a 2-byte tag prefix collide with a real multi-byte LEB128
type-index encoding" hazard that caused the pre-W24 `exnref` bug — every new
tag byte here must have its LEB128 continuation bit clear (0x64/0x71/0x72/
0x73/0x74 all do, verify this holds for whatever real values the spec
assigns before implementing).

### 5. `wasm-execution` — runtime semantics

Non-null concrete refs carry the same runtime representation as their
nullable counterparts (a `WasmValue::Ref(Some(handle))` — the "non-null" part
is a purely static, validator-enforced property, it never appears at the
value level, exactly like how `i32`/`u32` share one representation today).
This means most of the execution-side work is **already done** by whatever
already handles `StructRef`/`ConcreteFuncRef` — the new instructions this
unlocks (`call_ref`, `return_call_ref`, `ref.cast`, `ref.test`,
`br_on_cast`, `br_on_cast_fail`, `br_on_non_null`, `ref.as_non_null`) are
mostly about the validator accepting the module in the first place; their
runtime behavior is "do the equivalent of the existing nullable op, plus
trap-on-null where the spec says non-null is required."

## Explicitly out of scope for this spec

- Real recursive type groups (`(rec (type $a ...) (type $b ...))`) and their
  own nominal-vs-structural identity rules — `type-rec.wast` already grades
  mostly-honestly without this per W31; deepening it is a separate slice.
- Array types (`(type $t (array ...))`) and their instructions — separate
  from this epic's non-null-ref focus; `array.wast`'s current
  `not_yet_supported` state is a distinct, additional gap (no
  `ArrayType`/array-instruction support at all yet, per W31).
- `anyref`/`externref` round-trip conversion opcodes
  (`any.convert_extern`/`extern.convert_any`) and declarative element
  segments — `extern.wast`'s remaining blockers beyond the non-null-ref
  piece.
- The component model, real threading, and a JIT tier — unrelated,
  already-tracked-elsewhere epics (see W20's own "re-checking the other
  candidates" section, unchanged).

## Verification plan (for whatever session implements this)

- `cargo test -p wasm-types -p wasm-validator -p wasm-wast-parser
  -p wasm-module-parser -p wasm-module-encoder -p wasm-execution` green,
  with new unit tests specifically covering the subtyping lattice's
  asymmetric directions (each `<:` rule above needs both a positive test —
  the subtype is accepted — and a negative test — the reverse is rejected).
- Re-run the full conformance baseline
  (`cargo run --bin wasm_conformance_report -p wasm-conformance --
  --write-baseline`) and diff programmatically against the pre-change
  baseline: confirm zero regressions on all 255 already-vendored files.
- Attempt vendoring `ref_null.wast` first (narrowest real estate — bottom
  types only, no structural subtyping needed) as the smallest possible
  proof this design is sound before attempting `type-subtyping.wast` or the
  `call_ref`/GC-array follow-on work.

## Addendum (2026-08-31): first slice shipped

This spec's first slice — the four bottom reference types and their
subtyping rules (section 1's four `Null*` variants, section 2's
bottom-type rules) — has SHIPPED. `NonNullStructRef`/`NonNullConcreteFuncRef`
and structural subtyping were deliberately left for a later slice, per this
spec's own scoping; see below for what's still open.

What landed:

- `wasm-types` 0.1.13: `ValueType::NullFuncref`/`NullExternref`/
  `NullExnref`/`NullRef`, plus `ValueType::is_bottom_subtype_of` — the
  section 2 lattice. Tag bytes `0x73`/`0x72`/`0x74`/`0x71` were
  independently re-verified (not just re-asserted from this document)
  against the real reference interpreter's `interpreter/binary/decode.ml`
  — they match this spec's own claimed values exactly, so unlike W24's
  `exnref` bug, there was no discrepancy to fix here.
- `wasm-validator` 0.2.74: `is_assignable` now checks the bottom-type
  lattice; `decode_blocktype`'s and `wasm-execution`'s matching blocktype
  decoders gained explicit arms for the four new tag bytes (same
  defensive treatment as `exnref`'s `0x69`, closing the same class of
  type-index collision hazard pre-emptively, since none of the vendored
  corpus actually exercises a bottom type as a blocktype yet).
- `wasm-wast-parser` 0.1.86: the four value-type keywords
  (`nullref`/`nullfuncref`/`nullexternref`/`nullexnref`) and the four
  `ref.null` heap-type keywords (`none`/`nofunc`/`noextern`/`noexn`) now
  parse to the REAL `ValueType` variants and REAL tag bytes, replacing an
  earlier pass's lossy aliasing straight onto the nullable supertypes
  (which had already added text-format recognition for these keywords,
  just aliased, not subtyped).
- `wasm-module-parser` 0.2.11 / `wasm-module-encoder` 0.2.7: binary
  decode/encode for the four new tag bytes (encoder needed no code change
  — it already calls `ValueType::encode()` universally).
- `ref_null.wast` vendored: **2/2 module (100%), 32/32 assert_return
  (100%)**, zero `not_yet_supported`. Corpus: **256/257**. Baseline diffed
  programmatically against the pre-change one — the only difference is
  the new file's entry, confirming zero regressions elsewhere.

One discrepancy from this spec's own assumption, worth recording: the
real `ref_null.wast` (re-fetched fresh, not assumed) uses `none`/
`nofunc`/`noextern`/`noexn` as the `ref.null` **heap-type** keywords, but
`nullref`/`nullfuncref`/`nullexternref`/`nullexnref` as the **value-type**
keywords (e.g. `(global $g nullfuncref (ref.null nofunc))`) — two
different keyword spellings for the same underlying bottom type,
depending on grammatical position. This spec's section 3 anticipated
`nullref`/`none` as alternate spellings of the SAME keyword in the SAME
position ("`nullref`/`none`"); the real grammar instead uses them in two
different positions. Both were already implemented before this
discrepancy was even noticed (the value-type and heap-type parsing paths
are two separate functions in `wasm-wast-parser::module`), so this cost
nothing to accommodate — noted here only so a future spec-writer doesn't
re-assume they're interchangeable spellings.

What's left for a later slice (unchanged from this spec's own scoping):

- `NonNullStructRef`/`NonNullConcreteFuncRef` (non-null concrete refs,
  `(ref $t)` with no `null` keyword) — section 1's other two variants.
- Structural subtyping for `call_indirect`/`ref.cast` against concrete
  function/struct types — `type-subtyping.wast`'s actual remaining gap,
  confirmed still open (not touched by this slice).
- Everything downstream of those two (`call_ref.wast`/
  `return_call_ref.wast`, `struct.wast`'s non-null field types,
  `array.wast`'s remaining gaps, `ref_cast.wast`/`ref_test.wast`/
  `br_on_cast*.wast`/`br_on_non_null.wast`) — still blocked, exactly as
  this spec's "Purpose" section described.
