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

## Addendum (2026-08-31): second slice shipped

This spec's second slice — section 1's `NonNullStructRef`/
`NonNullConcreteFuncRef` and section 2's non-null subtyping rules — has
SHIPPED. Structural subtyping (`type-subtyping.wast`) remains explicitly
out of scope, per this spec's own scoping; see below for what's still
open, including several NEW gaps this slice's own investigation found and
confirmed (not guessed).

### What landed

- `wasm-types` 0.1.14: `ValueType::NonNullStructRef(u32)`/
  `NonNullConcreteFuncRef(u32)`, tag byte `0x64`, plus
  `ValueType::is_non_null_subtype_of` — the section 2 lattice. `0x64` was
  independently re-verified (not re-asserted from this document) against
  the real reference interpreter's `interpreter/binary/decode.ml`
  (`ref_type`'s `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod 128 =
  0x64`) — it matches this spec's own claimed value exactly, so unlike
  the first slice's `is_bottom_subtype_of` derivation from `decode.ml`,
  there was no discrepancy to fix here either.
- `wasm-validator` 0.2.75: `is_assignable` now checks the non-null
  lattice; `call_ref`/`return_call_ref` (`0x14`/`0x15`) are real,
  type-checked opcodes; `ref.func`'s pushed type is now the real spec
  rule (`[] -> [(ref $t)]`, `$t` = the named function's own type, not a
  blanket `Funcref`); `decode_blocktype` gained `0x63`/`0x64` arms;
  untyped `select` now rejects reference-typed operands.
- `wasm-wast-parser` 0.1.87: `(ref $t)` text parsing (into
  `NonNullConcreteFuncRef`, same "no struct-type text declarations exist"
  caveat `(ref null $t)` already had); `call_ref`/`return_call_ref`
  folded+flat parsing; `(elem declare func ...)` (declarative element
  segments) parsing; and a fix for a PRE-EXISTING bug (silently
  misparsing `(type ... (struct/array ...))` into a bogus empty `(func)`
  type) that this slice's own new parsing surface is what first made
  observable.
- `wasm-module-parser` 0.2.12 / `wasm-module-encoder` 0.2.8: binary
  decode/encode for the new tag byte (encoder needed no code change —
  universal `ValueType::encode()`; decoder's `read_value_type`, the
  struct-FIELD-only path, gained the `0x64` arm alongside `0x63`).
- `wasm-execution` 0.9.78 / `wasm-runtime` 0.6.16: `call_ref`/
  `return_call_ref` runtime handlers (pop a ref, trap on null, call/
  tail-call through the function index the ref's `Some(handle)` already
  IS); `evaluate_const_expr` gained a `ref.func` arm (a real gap: the
  real spec calls `ref.func` a *constant instruction*, needed for globals
  like `(global $fac (ref $ll) (ref.func $fac))`); matching `0x63`/`0x64`
  blocktype arms; exhaustive-match completions for the two new variants.
- **`call_ref.wast`/`return_call_ref.wast` vendored with real, honest
  numbers** — investigated per this spec's own "purpose" section (they
  were listed as blocked on this exact wall) and confirmed NOT to need
  structural subtyping after all: `call_ref $t`'s real typing rule is
  `[t1* (ref null $t)] -> [t2*]` (nullable operand, traps on null) —
  independently verified against WebAssembly/function-references's own
  `Overview.md`, correcting this spec's OWN original assumption that the
  operand was non-null-only. `call_ref.wast`: `module` 0/4 → 4/4 (100%),
  `assert_return` 0/23 → 23/23 (100%), `assert_trap` 0/4 → 4/4 (100%).
  `return_call_ref.wast`: `module` 0/5 → 4/4 (100%) + 1 `not_yet_supported`,
  `assert_return` 0/31 → 31/31 (100%), `assert_trap` 0/4 → 4/4 (100%).
  Getting there also required two things this spec did not anticipate:
  declarative element segment parsing (`(elem declare func $f)`, needed to
  make `ref.func $f` legal on a function never otherwise referenced) and
  `evaluate_const_expr`'s `ref.func` fix above.
- **`struct.wast`/`array.wast` re-checked, per this spec's own ask** —
  did NOT improve (both still need real struct/array-type TEXT-format
  declarations this crate has never had, an unrelated, larger, later
  slice — see "What's still open" below). Their `module` pass count
  actually DECREASED (1→0 each), but this is a deliberate correctness fix,
  not a regression: the one module each previously "passed" only because
  of the pre-existing `wasm-wast-parser` bug above (a `(type ... (struct/
  array ...))` silently misparsed as an empty `(func)`) — this slice's
  `(ref $t)` parsing is what first made that bogus type REFERENCEABLE in
  a way that validated, so a full-corpus baseline diff caught it. Fixed
  at the parser (a clean rejection instead), which is what actually
  causes the pass-count decrease.
- Full-corpus regression check: diffed the regenerated baseline
  programmatically against the pre-change one across all 256 files. 9
  files showed a DECREASED pass count somewhere
  (`array.wast`/`call_ref.wast`/`func.wast`/`ref.wast`/
  `return_call_ref.wast`/`struct.wast`/`try_table.wast`/`type-rec.wast`/
  `unreached-invalid.wast`) — every one individually investigated (not
  assumed benign) and confirmed to be either the struct/array parse-bug
  fix above, or an honest `not_yet_supported` reclassification of a case
  that previously passed ONLY via a lucky module-parse failure (`(ref
  $t)`/`(ref func)`/`(ref exn)` being entirely unparseable before this
  slice) and is now correctly gradeable for the first time — never a
  newly introduced silent misbehavior. Two bounded, in-scope fixes were
  made specifically to minimize this list: extending
  `out_of_range_concrete_func_ref`'s bounds check to the new non-null
  variant (restored 3 of `ref.wast`'s 5 initially-regressed cases), and
  making untyped `select` reject reference-typed operands per the real
  spec (restored/improved `select.wast`). 26 directive-kind counts
  IMPROVED as a side effect (`elem.wast`, `instance.wast`, `ref_func.wast`,
  `select.wast`, `table.wast`, `table_grow.wast`, `type-equivalence.wast`,
  `unreached-valid.wast`, plus the four headline files above).

### What's still open for a later slice

- Structural subtyping for `call_indirect`/`ref.cast` against concrete
  function/struct types — `type-subtyping.wast`'s actual remaining gap,
  confirmed still open, unchanged from this spec's own original scoping.
- Real struct/array-type TEXT-format declarations (`(type $t (struct
  (field ...)))`/`(type $t (array ...))`) — `wasm-wast-parser` has NONE
  at all; this is the actual remaining blocker for `struct.wast`/
  `array.wast`, confirmed by this slice's own investigation, not merely
  restated from before. A real, substantial parser feature (field lists,
  mutability, `(rec ...)` type-group binding order) — out of scope here.
- Real recursive type groups' own forward-reference/nominal-identity
  rules (`(rec (type $a ...) (type $b ...))`) — confirmed, via this
  slice's own investigation of `type-rec.wast`'s regressed cases, to be
  more than a bounds check: a type declared inside a LATER position in
  the flat `module.types` array can spuriously appear "in bounds" to a
  simple `idx < types.len()` check even when the real spec's rec-group
  ordering rules would reject it as a forward reference. Unchanged from
  this spec's original "explicitly out of scope" section, now confirmed
  by a real failing case rather than assumed.
- Per-local definite-initialization tracking for non-defaultable
  non-null locals — the real spec's own rule (WebAssembly/
  function-references's `Overview.md`: "Track initialisation status of
  locals during validation and only allow `local.get` after a
  `local.set`/`tee` in the same or a surrounding block"), confirmed via
  `func.wast`'s own `type-local-uninitialized` regressed case
  (`(local $x (ref $t)) (drop (local.get $x))` must be rejected as
  "uninitialized local", which this crate's validator does not track at
  all). A genuine control-flow/liveness analysis feature, comparable in
  scope to structural subtyping — a new, separate slice, not attempted
  here.
- `try_table`'s own catch-clause payload-type checking against its
  destination label's declared types — confirmed via `try_table.wast`'s
  own regressed case (a `catch` clause pushing a NULLABLE tag-param type
  into a label expecting the NON-null counterpart currently validates
  when it should not) to be a pre-existing gap in `try_table`'s own
  instruction handling, not a defect in this slice's new subtyping rules
  — `try_table` does not check ANY payload-type compatibility today, per
  two OTHER already-`not_yet_supported` cases in the same file that
  predate this slice.
- `wasm-wast-parser`'s single-value-blocktype dedup (a `(block (result
  (ref $t)) ...)` gets rewritten into a brand-new anonymous type-section
  entry, per `parse_func_signature`-shaped logic) can make an
  intentionally out-of-range index in the ORIGINAL source spuriously
  land in bounds once that new entry is appended — confirmed via two of
  `ref.wast`'s regressed cases (`block-result-invalid`/
  `loop-result-invalid`). Needs the type index validated against the
  types that existed AT THE POINT OF REFERENCE, before any anonymous
  dedup — a parser-level fix, not touched here.
- Everything downstream of full structural subtyping
  (`ref_cast.wast`/`ref_test.wast`/`br_on_cast*.wast`/
  `br_on_non_null.wast`) — still blocked, exactly as this spec's
  "Purpose" section described.
