# W33 — WASM GC recursive type-group structural/nominal subtyping

## Purpose and how this slice was chosen

`code/specs/W32-wasm-non-null-concrete-reference-types.md` named
structural subtyping for `call_indirect`/`ref.cast` against concrete
function/struct types as its own single remaining open item, twice
deferred (its first and second addenda), with the file
`type-subtyping.wast` as the corpus evidence it was needed. Per that
spec's own instruction to re-investigate deferred items rather than
trust the deferral ("this session's own track record... is that
deferred items have repeatedly turned out more tractable than first
assumed" — true of `call_ref.wast`/`return_call_ref.wast` in W32's
second slice), this spec is the product of that re-investigation for
`type-subtyping.wast` specifically.

**Conclusion: the deferral was accurate this time.** Unlike
`call_ref`/`return_call_ref` (which turned out to need only the
*existing* nullable-operand rule already in the function-references
proposal, not new subtyping machinery), `type-subtyping.wast` genuinely
needs the WebAssembly GC proposal's full type system — not a narrower
subset of it. This document exists so a future session that picks this
up doesn't have to re-derive that scope from scratch, per this repo's
own "specs before implementation" convention and W32's own precedent for
what a properly-scoped deferral doc looks like.

## What the corpus file actually needs (verified against the real file, not assumed)

`type-subtyping.wast` was re-fetched fresh from the pinned SHA
(`WebAssembly/testsuite@28864811cf03bdbf880733786148feaba339582d`) and
read in full — all 989 lines, every module. It is now vendored at
`code/packages/rust/wasm-conformance/tests/fixtures/testsuite/type-subtyping.wast`,
so line numbers below are stable and checkable directly, not
transcribed from memory.

### 1. Explicit nominal subtype declarations: `(sub [final] $parent* (structtype))`

Every non-`assert_invalid` module in the file declares its types with
`(type $name (sub [final] $parent (structtype)))` — e.g. line 17: `(type
$e1 (sub $e0 (struct)))`. This is a GC-proposal construct this repo has
never parsed: `wasm-wast-parser` has **zero** matches for `"sub"` or
`"final"` as recognized keywords anywhere in `module.rs` (grepped, not
assumed, as of this spec). This is orthogonal to, and more than, the
"struct/array TEXT-format declarations" gap `struct.wast`/`array.wast`
already track (W32's second slice addendum) — even once struct/array
bodies parse, `(sub $parent ...)` is a distinct wrapper this repo has no
representation for at all: which supertype a type nominally declares,
and whether it forecloses further subtyping (`final`).

### 2. Real structural subtyping rules the declared `sub` relationship must satisfy

The file's "Invalid subtyping definitions" section (lines 816-950) is a
systematic negative-test sweep of the GC proposal's real structural
subtyping rules — every one of the following must be checked, and
rejected when violated:

- **Composite-type-kind invariance**: a `struct` cannot be declared a
  sub of an `array` or `func` and vice versa (lines 816-862 cover all 6
  cross-kind pairings).
- **Array element-type covariance, with mutability determining variance
  entirely**: `(array T)` (immutable) permits `(array T')` where `T' <:
  T` as a subtype (line 866's `i32`/`i64` mismatch correctly rejected —
  arrays are invariant in element type when the ELEMENT TYPES aren't
  related at all, but the deeper rule, per lines 880-910, is:
  - immutable field/element: **covariant** (`(ref none) <: (ref any)`
    lets a `(array (ref none))` be a valid sub of `(array (ref any))`,
    but not the reverse — lines 880-886 reject exactly the reverse
    declaration, a base type with `(ref none)` elements and a claimed
    sub with the WIDER `(ref any)`).
  - mutable field/element: **invariant** (line 888-893: `(mut (ref
    any))` sub `(mut (ref none))` rejected; line 896-901: `(mut (ref
    any))` sub non-mut `(ref any)` rejected — mutability itself must
    match exactly, not just be "compatible"; line 904-909: the reverse,
    non-mut sub mut, also rejected).
- **Struct field-list width AND depth subtyping**: a struct subtype may
  ADD fields (line 18-19's `$e2`→`$e3` goes from 1 field to 2) but may
  not change an EXISTING field's declared type incompatibly (lines
  872-878: `i32` field respecified as `i64` rejected) — and the same
  covariant/invariant-by-mutability split as arrays applies per-field
  (lines 912-942 repeat the exact same 4 array cases for struct fields).
- **Function type field-list invariance in arity, contravariance in
  params, covariance in results** — this IS the "simple pointwise rule"
  from the GC proposal (not function-references, which explicitly
  disclaims real subtyping — see "Why this needs GC, not
  function-references" below): line 944-949 rejects a supertype/subtype
  func pair that merely differ in param arity. Lines 28-31's `$f1`→`$f4`
  chain demonstrates the real rule in the positive direction: `$f1
  (func (param (ref $s')) (result anyref))`, its sub `$f2 (func (param
  (ref $s)) (result (ref any)))` — note `$s <: $s'` is FALSE ($s' is
  the sub of $s per line 25-26) but the param position is
  **contravariant**, so accepting the wider `(ref $s)` param in the
  subtype is correct; the result position is **covariant** (`(ref any)
  <: anyref`, accepted).
- **`final` forecloses further subtyping**: lines 780-811, four
  variants — a type with no explicit `sub` clause defaults to `final`
  per the MVP/pre-GC shape (line 780-786 rejects subtyping a plain
  `(type $t (func))`); an explicit `(sub final ...)` also forecloses
  (lines 796-802, 804-811, the latter three-level chain confirming
  finality isn't just "no default subtypers" but an active, checked
  property).

### 3. Recursive type groups: `(rec (type $a ...) (type $b ...))`

**Not currently parsed at all** — confirmed via this slice's own
investigation (a corpus-wide scan found `rec` is an unrecognized
top-level module keyword in exactly 7 files: `array.wast`,
`struct.wast`, `tag.wast`, `type-canon.wast`, `type-equivalence.wast`,
`type-rec.wast`, and `type-subtyping.wast` itself — see
`wasm-wast-parser` 0.1.88's changelog for the general parser-honesty fix
this investigation produced as a side effect, unrelated to actually
implementing `rec` semantics). Every module past line 37 in
`type-subtyping.wast` uses `rec`, and correctly implementing it needs
more than parsing the syntax:

- **Forward references within a group**: line 45's `$r1` references
  `$r1` itself (a self-referential struct field) before the type is
  "done" being declared — ordinary `idx < types.len()` bounds checking
  (this repo's current approach per W32's own "what's still open" list)
  is insufficient inside a `rec` group, per W32's own already-recorded
  finding from `type-rec.wast`'s investigation.
- **Recursive type-group canonical equivalence** — the genuinely hard
  part, and the reason this is a full type-theory feature rather than a
  bounded extension. Two SEPARATELY-declared `rec` groups, in different
  modules, with no shared identity beyond their *shape*, must be
  recognized as declaring "the same types" when appropriate:
  - Lines 620-630 (module `M3`): module A declares `(rec (type $f2 (sub
    (func))) (type (struct (field (ref $f2)))))` then exports a function
    of a type built from it; module B (a SEPARATE module, importing
    from A) declares its OWN structurally-identical `rec` group with
    different names (`$f1` instead of `$f2`) and successfully imports
    the function at its OWN type — this only type-checks because the
    real algorithm treats the two independently-declared, structurally
    identical `rec` groups as canonically equivalent, NOT because any
    shared name or index makes them "the same" (there is no shared type
    section between two modules at all).
  - Lines 652-666 (module `M5`, `assert_unlinkable`) shows the negative
    case, and it's a deliberately subtle one: the exporting module
    declares `$f1 (sub (func))` with struct field `(ref $f1)`, but its
    SECOND `rec` group member `$f2` also has a struct field `(ref
    $f1)` — **not** `(ref $f2)` as the isomorphic-looking pattern
    elsewhere in the file would suggest (compare line 653-654 here
    against line 668-670's superficially similar-looking group, which
    genuinely IS isomorphic in both modules and therefore links). The
    importing module's own `$f1`/`$g1` group is the "expected" shape
    (self-referencing `$f2`), so the two groups are NOT canonically
    equivalent despite looking almost identical at a glance — real
    canonicalization must catch this exact copy-paste-shaped mismatch
    as "incompatible import type", not accept it by accident. The
    `assert_invalid` case at lines 139-149 tests the same "one field
    reference wrong" mismatch shape within a SINGLE module's
    `ref.func`-to-global-type check instead of cross-module linking.
  - The "Subsumption" section (lines 68-113) demonstrates why this
    matters operationally, not just for linking: `$t1`'s own group is
    a 3-cycle (`$t1`'s param is `(ref $t3)`, `$t2`'s is `(ref $t2)`,
    `$t3`'s is `(ref $t1)`) type-checks calls between all three types
    specifically BECAUSE the subtyping chain `$t3 <: $t2 <: $t1`
    (declared via `sub`) composes through the cycle — this needs a
    real subtype-closure computation over the whole group, not a
    pairwise check.
  - The real algorithm (per the GC proposal's own spec text, not
    reproduced here — implement against the proposal directly) assigns
    each type a **canonical, structurally-normalized form** by
    numbering recursive references relative to their OWN group's start
    (a De Bruijn-style scheme) so that two groups are equivalent iff
    their canonical forms are byte-for-byte identical — this is what
    lets modules A and B above, with no shared numbering, agree.
    Implementing this is comparable in scope to writing a real
    type-equivalence checker for a language with recursive types (the
    task's own framing, confirmed accurate by this investigation, not
    an exaggeration).

### 4. Dynamic type checks reachable only after (1)-(3) exist

The "Runtime types" section (lines 283-343) and every `ref.test`
section (lines 402-534) exercise `call_indirect`, `ref.cast`, and
`ref.test` against real declared subtype relationships — e.g. line 301:
`(ref.cast (ref $t0) (table.get (i32.const 0)))` must succeed because
the table's static element type `funcref`'s runtime value (of dynamic
type `$t2`) is a real subtype of `$t0` per the module's own `sub` chain
(`$t0`←`$t1`←`$t2`). None of this is reachable without (1)-(3) above
existing first — every module that would exercise it fails to parse
today (confirmed: `assert_return`/`assert_trap` counts for this section
are 100% `not_yet_supported`, not merely low-passing).

## Why this needs the GC proposal, not function-references

The task that produced this investigation asked to verify, against
`WebAssembly/function-references`'s own `Overview.md`, whether that
proposal's "simple pointwise" function-type subtyping rule (declared
contravariant-params/covariant-results) is what this file needs — since
that would be a narrow, bounded, implementable rule, unlike full GC
subtyping. **It is not.** The function-references `Overview.md`
(`#### Type Indices` section) states its OWN real rule explicitly:

> Type indices are subtypes only if they define equivalent types...
> Note: Function types are invariant for now. This may be relaxed in
> future extensions.

And later, under `## Possible Extension: Function Subtyping`, it
describes the exact contravariant/covariant pointwise rule as a
**forward-looking possible extension**, explicitly deferring it to "the
GC proposal", and spends its own text discussing the unresolved tension
this creates with `call_indirect`'s performance requirements (the
"exact types" discussion) — i.e., function-references' own authors
treat this as unfinished, GC-proposal-owned territory, not baseline.

`type-subtyping.wast` is not exercising function-references' baseline
rule (which W32's second slice already correctly implemented and
verified sufficient for `call_ref.wast`/`return_call_ref.wast`) — it is
a GC-proposal conformance file through and through: `sub`, `final`,
struct/array types, and `rec` groups are ALL GC-proposal vocabulary,
absent from function-references entirely. This is the concrete evidence
that distinguishes this file from `call_ref.wast`'s case: the latter
turned out tractable specifically because its real spec requirement was
narrower than assumed; this file's real spec requirement is broader
than a narrow reading of "structural subtyping for call_indirect" might
suggest — it is the GC proposal's type system as a whole, entrance fee
included (recursive groups and their canonical equivalence), not an
isolated add-on to `call_indirect` alone.

## Recommended scope for a future implementation slice

In dependency order (each step's tests are largely gated on the
previous one, per the file's own escalating structure):

1. **`(sub [final] $parent* (...))` parsing** — a new `wasm_types`
   representation for "declared supertype index (if any) + finality
   flag" alongside a type's existing body. Bounded, mechanical parser
   work once (2) below exists for struct/array bodies (or done
   alongside it — they're adjacent gaps in the same file region).
2. **Struct/array TEXT-format type declarations** — already tracked
   (W32's "what's still open"), a prerequisite here too since nearly
   every `sub` case in this file targets a struct or array.
3. **Structural subtype-checking function** — given two fully-resolved
   composite types (func/struct/array) plus their declared `sub`
   parent chain, implement the width/depth/variance rules from section
   2 above. This part alone (assuming (4) below already resolved
   indices) is a bounded, well-specified algorithm — the GC proposal
   spells out every rule precisely, and section 2 above gives the exact
   corpus cases to test against.
4. **`(rec ...)` parsing with forward-reference-aware index resolution**
   — needs a two-pass or deferred-resolution scheme distinct from this
   crate's current single-pass `collect_symbols`/`build` split, since a
   `rec` group's later members must be resolvable from its earlier
   members' bodies (line 45's self-reference) before the WHOLE group is
   fully declared.
5. **Recursive type-group canonicalization** — the large, genuinely
   open-ended piece. Implement directly against the WASM GC proposal's
   own canonicalization algorithm (course-of-values / De-Bruijn-relative
   numbering of a group's internal cross-references, then structural
   comparison of canonical forms) — do not attempt to invent a
   simplified version; the cross-module linking cases (section 3 above)
   specifically probe for a CORRECT general algorithm, not a
   name-based or index-based shortcut (both of which the file's own
   `M3`/`M6` cases are specifically designed to defeat, since the two
   modules being compared share no names or indices at all).
6. **Wire (3) and (5) into `call_indirect`'s type-check and
   `ref.cast`/`ref.test`'s target-type check** — the actual consumer of
   all of the above; per this crate's `wasm-execution`, these currently
   do only an index-equality or bounds check.

This is a substantially larger slice than any single W32 addendum —
comparable in scope to a new type-theory subsystem, not a single
crate's incremental extension. Recommend it as its own multi-PR epic
(not a single slice) if picked up, given the real dependency chain
above.

## Explicitly out of scope for this spec

- `extern.wast`'s remaining gaps (struct/array declarations,
  `struct.new_default`/`array.new_default`, `ref.i31`,
  `any.convert_extern`/`extern.convert_any`, and a missing `ref.host`
  script literal that currently fails the WHOLE SCRIPT to parse) —
  tracked in `wasm-conformance`'s own 0.1.105 changelog entry, unrelated
  to subtyping.
- Per-local definite-initialization tracking, `try_table`'s catch-clause
  payload-type checking, and the wast-parser's single-value-blocktype
  dedup bug — all already tracked in W32's second addendum, unrelated to
  this spec's own scope.

## Verification plan (for whatever session implements this)

- Build steps 1-3 above first and verify against `struct.wast`/
  `array.wast`'s own struct/array-declaration cases (a smaller, more
  contained proof before attempting `rec`).
- Build step 4 next and re-run `type-rec.wast`'s own baseline — its
  currently-`not_yet_supported` forward-reference cases (W32's second
  addendum) are the right proof point.
- Only then attempt step 5 (canonicalization) — verify against
  `type-canon.wast` and `type-equivalence.wast` (both already vendored,
  both currently low-passing for exactly this reason) BEFORE attempting
  `type-subtyping.wast` itself, since they're narrower, more targeted
  proofs of the same algorithm.
- Re-run the full conformance baseline
  (`cargo run --bin wasm_conformance_report -p wasm-conformance --
  --write-baseline`) and diff programmatically against the pre-change
  baseline after EVERY step above, not just at the end — this epic's
  size makes an end-of-epic-only diff much harder to attribute
  regressions within.

## Addendum (first slice, 2026-08-31): items (1), (2), (3a) shipped; (3b)/(4) re-verified, both still needed

Implemented exactly the scope this spec's own "Recommended scope"
section labeled steps 1, 3, and 4 above (function-type field-list
subtyping rules; `final`/no-`sub` foreclosure; `(rec ...)` parsing with
forward-reference-aware index resolution) — WITHOUT step 2 (struct/array
TEXT-format bodies, a separate, still-open gap this slice didn't
attempt) and WITHOUT step 5 (canonicalization). Landed across four
crates: `wasm-types` 0.1.15 (`TypeSubtyping`, `WasmModule::
func_type_is_nominal_subtype`/`type_group_shape`), `wasm-wast-parser`
0.1.89 (`rec`/`sub`/`final` parsing), `wasm-validator` 0.2.76
(`check_type_subtyping`, `is_assignable`'s nominal-subtype arms),
`wasm-execution` 0.9.80 + `wasm-runtime` 0.6.17 (a conservative
cross-module import guard, added mid-slice once the baseline diff
surfaced a real regression risk — see below).

### Struct/array bodies stayed out of scope, and turned out not to gate this slice's own wins

The original "Recommended scope" listed struct/array TEXT-format parsing
(step 2) as a prerequisite, reasoning "nearly every `sub` case in this
file targets a struct or array." Re-checked directly against the corpus
rather than assumed: `type-subtyping.wast`'s "Subsumption" section
(lines 68-113, three-cycle and mutually-recursive-pair modules), its
"Finality violation" section (lines 780-811), the pure-arity-mismatch
case in "Invalid subtyping definitions" (lines 944-949), and "Testing
exported functions with non-flat types" (lines 954-976) are ALL
pure-`func` — no struct/array involved at all. These became this
slice's real, verified wins (see `wasm-conformance` 0.1.106's own
changelog for the exact tallies) entirely without struct/array support.
Struct/array remains its own, separate, still-fully-open gap.

### Re-verified item (4)'s dependency on (3b): PARTIALLY optimistic, as predicted the campaign sometimes finds

This spec's main text claimed items (1)-(3) must all exist before (4)'s
dynamic checks become reachable at all (`assert_return`/`assert_trap`
tallies for the "Runtime types" section were "100% `not_yet_supported`").
Re-verified directly this time (not re-assumed): once (1)+(2)+(3a) exist,
`type-subtyping.wast`'s "Runtime types" section's SINGLE-MODULE
`call_indirect`/`ref.cast` directives (lines 283-343, 344-371, 373-401)
turn out to depend on (3b) NOT AT ALL in principle — comparing a
function's actual declared type against a `call_indirect`/`ref.cast`
target type when BOTH live in the SAME module's type-section index space
never needs cross-module canonical identity, only the same
absolute-index nominal-chain walk (1) already implements. This is the
"optimistic" half of the campaign's own track record repeating (like
`call_ref.wast`/`return_call_ref.wast` in W32's second slice).

But the OTHER half held too, confirmed by running the real corpus rather
than reasoning abstractly: implementing (4)'s actual dynamic WIRING is
still real, non-trivial, genuinely-scoped work this slice did not
attempt (explicitly out of bounds per this slice's own task), and
running the corpus anyway (to see what NOT implementing it costs)
surfaced a DIFFERENT, more specific finding than "blocked on 3b" would
have predicted: `wasm-execution`'s existing runtime `call_indirect`
dynamic-dispatch check is a PRE-EXISTING simplification (plain `FuncType`
structural equality, no type-identity or nominal-subtype awareness at
all) that predates this slice entirely and was simply never reachable
before (no module using `sub`/`rec` could parse). `type-subtyping.wast`
lines 344-371's `fail1`/`fail2` (two structurally-identical-but-distinct
`$t1`/`$t2` types) now reach it and expose it directly: 2 new
`assert_trap` fails, confirmed via a per-directive diagnostic trace, not
guessed. So: (4)'s dependency on (3b) is real for the "Linking" section's
CROSS-MODULE `assert_unlinkable` cases (confirmed unavoidable, see
below), but NOT for the "Runtime types" section's single-module cases —
those are blocked on actually building (4)'s dynamic wiring itself
(which also needs a real type-identity-aware `call_indirect`/`ref.cast`
runtime check in `wasm-execution`, not just a `wasm-validator` change),
a smaller and more tractable-looking follow-on than the spec's own
blanket framing suggested, but still explicitly not attempted here.

### The conservative cross-module guard: what it catches, what it can't, and why the gap is real (3b), not a bug

`rec`-group parsing makes EVERY corpus module using it reachable at
once, including cross-module `assert_unlinkable`/linking directives this
slice's own scope (single-module-only nominal checking) doesn't cover.
Running the full baseline mid-slice surfaced a real regression risk: the
pre-existing `wasm-runtime` import-compatibility check (`host_func.
func_type() != &ft`, plain `FuncType` structural equality) is blind to
`rec`-group POSITION and to `final`-ness, both now reachable. Rather than
attempt full canonical equivalence (still explicitly out of scope), this
slice added two narrow, SOUND, strictly-additive guards — `wasm_types::
WasmModule::type_group_shape` (rec-group size/position) and a finality
check — ANDed onto the pre-existing structural check. Both are provably
safe: every pre-existing import trivially reports the same default on
both sides, so neither guard can ever newly ACCEPT something the old
check rejected, only newly REJECT something the old check wrongly
accepted. Verified via the diagnostic trace: without these two guards,
`type-subtyping.wast`'s `assert_unlinkable` tally would have been 4
fails, not 2 — `tag.wast`'s own position-mismatch case and `type-
subtyping.wast` lines 594-617's finality-mismatch pair are both real,
confirmed fixes.

The 2 REMAINING `assert_unlinkable` fails (`type-subtyping.wast`'s
M10/M11 pair, lines 746-774) are the genuine (3b) case these two guards
cannot reach by construction: the exporter's and importer's `rec` groups
have the SAME size, position, AND finality, but DIFFERENT internal
cross-reference topology (one group's second member's `sub` parent
points OUTSIDE its own group to an earlier, separately-declared group;
the other's points to its own sibling). Telling these apart needs the
real course-of-values/de-Bruijn canonical numbering this spec's own
section 3 describes — confirmed directly by tracing the specific
directive, not assumed from the general shape of the problem. No shallow
shape/finality/position check reaches it without becoming that
algorithm, which — per this slice's own explicit instruction not to
invent a simplified version of (3b) — was not attempted.

### A newly-discovered, THIRD gap: global init-expression type-checking, unrelated to `sub`/`final`/`rec`

Tracing two "honest reclassification" cases (`type-rec.wast`/`type-
subtyping.wast`'s `assert_invalid "type mismatch"` counts each dropping
by one, Pass → `not_yet_supported`, never `fail`) surfaced a real,
independent gap: **this repo's `wasm-validator` does not type-check a
global's init expression against its declared type anywhere** — checked
directly (not assumed): there is no const-expr type-checker at all, only
a bounds check on the declared type's own index. Every corpus case that
relies on `(global (ref $t) (ref.func $f))`-shaped type mismatches (both
files use this as their ONLY mechanism to probe several `sub`/`rec`
interactions) is honestly unreachable for real verification. This
predates W33 entirely and would affect a plain `(global i32 (i64.const
0))` mismatch too — it is NOT part of this spec's own scope (sub/final/
rec), and is recorded here only because this slice is what made it newly
OBSERVABLE (the surrounding `rec` syntax used to fail to parse first).
Worth its own future slice; not attempted here.

### What's confirmed still deferred to (3b)/(4), with the specific evidence (not just the spec's original framing)

- **(3b), cross-module canonical type-group equivalence**: confirmed
  necessary and sufficient to explain ALL 6 `type-equivalence.wast` fails
  (every one of its ~21 modules exists specifically to probe
  canonical-but-not-nominal equivalence between independently-declared,
  isomorphic-or-identical types) and the 2 remaining `type-subtyping.
  wast` `assert_unlinkable` fails (the M10/M11 topology-mismatch case
  above). `type-canon.wast` is fully passing already (its 2 modules
  don't need canonicalization at all, confirmed by this slice's own
  baseline diff) — it is NOT, contrary to a naive reading of this spec's
  original "verify against type-canon.wast... before attempting
  type-subtyping.wast" plan, a meaningful proof point for (3b)
  specifically; `type-equivalence.wast` is the real one.
- **(4), dynamic `call_indirect`/`ref.cast`/`ref.test` checks**: confirmed
  necessary for `type-subtyping.wast`'s entire "Runtime types" section
  (lines 283-534, still 100% `not_yet_supported` for `ref.cast`/`ref.
  test`-using modules specifically because this crate's wast-parser
  additionally can't parse `ref.cast`/`ref.test` nested inside a folded
  `(block (result T) ...)` form at all — a separate, pre-existing parser
  gap, confirmed via the diagnostic trace, distinct from (4) itself) and
  for the 2 `assert_trap` fails traced above (a real-but-reachable
  `wasm-execution` runtime-dispatch gap once (1)-(3a) exist). Per the
  finding above, the SINGLE-MODULE half of "Runtime types" does NOT
  additionally need (3b) — only its own dynamic-wiring implementation,
  not attempted this slice.
- **Struct/array TEXT-format bodies** (this spec's own step 2): fully
  unchanged, still fully open. Confirmed via the full baseline diff:
  `struct.wast`/`array.wast` show ZERO tally changes from this slice.
- **Global init-expression type-checking**: a new, third, independent
  gap (above) — not part of this spec's own scope, not previously
  tracked anywhere in this repo's specs, recorded here for whichever
  future slice picks it up.
