# Changelog

All notable changes to this package will be documented in this file.

## [0.1.27] - 2026-09-02 (feat: `Element::declared_type` -- W38 slices 4/5, GC array bulk ops)

Slices 4/5 of `code/specs/W38-wasm-gc-array-bulk-ops.md`'s six-slice plan
(the elem-segment three-layer fix, Correction 2, plus `array.init_elem`/
`array.new_elem` themselves).

- **`Element::declared_type: ValueType`** -- the segment's own declared
  reference type (its reftype tag -- `funcref`/`externref`/`i31ref`/
  `arrayref`/`(ref $t)`/etc.), or `ValueType::Funcref` for a plain
  funcidx-list segment (binary modes 0-3, matching the real spec's own
  implicit "elemkind" default). Lets `array.init_elem`/`array.new_elem`'s
  own `wasm-validator` arms statically enforce the real spec's
  `match-reftype` rule via the existing `is_assignable` relation -- zero
  new subtyping logic. Real, corpus-driven: without this field,
  `array_init_elem.wast`'s own `array.init_elem-invalid-2` case (a `(mut
  funcref)` array fed from an `externref`-tagged segment,
  `assert_invalid "type mismatch"`) has no way to be rejected, since this
  repo's `assert_invalid` grading only credits a real structural/static
  rejection, never a runtime trap.
- **`Element::item_exprs`'s own doc comment updated**: no longer "always
  empty" (W38 slice 0's own placeholder note) -- now populated for real,
  unconditionally, for every element segment this crate's `wasm-wast-
  parser`/`wasm-module-parser` build (see those crates' own CHANGELOGs).
- Every existing `Element { .. }` construction site in this crate's own
  test suite updated with the new field (`declared_type: ValueType::
  Funcref`, matching every one of them being a plain funcidx-based
  segment).

New unit test: none added directly here (the new field's real behavior is
exercised through `wasm-wast-parser`/`wasm-validator`/`wasm-runtime`'s own
test suites) -- the existing `element_with_function_indices` test updated
in place for the new field.

`cargo test -p wasm-types`: 119 passed, 0 failed.

## [0.1.26] - 2026-09-02 (feat: `ArrayRefAny`/`Element::item_exprs` -- W38 slice 0, GC array bulk ops)

Slice 0 of `code/specs/W38-wasm-gc-array-bulk-ops.md`'s six-slice plan for
the GC array bulk-operations instruction family (`array.copy`/
`array.fill`/`array.init_data`/`array.init_elem`/`array.new_data`/
`array.new_elem`).

- **`ValueType::ArrayRefAny`** -- `arrayref`/`(ref null array)`, the nullable
  abstract TOP of the array hierarchy (Correction 3), the exact array-
  hierarchy mirror of the existing `StructRefAny` (W37) one hierarchy over,
  and distinct from both `ArrayRef(u32)` (nullable but always CONCRETE) and
  the pre-existing `NonNullArrayAny` (W33 fourth slice; this variant's
  non-null counterpart). Encoded as a single byte `0x6A` -- the real GC
  proposal's own `array` abstract heap-type byte, one less than
  `StructRefAny`'s `0x6B`, matching the real spec's own adjacent
  assignment. `is_bottom_subtype_of` gains `NullRef <: ArrayRefAny`, and
  `canonicalize_value_type` gains a matching arm (reusing the
  already-existing `AbstractHeapKind::Array`).
- **`Element::item_exprs: Vec<Vec<u8>>`** -- one raw constant-expression
  byte sequence per element-segment item (Correction 2's data-model
  gap), added ALONGSIDE (not replacing) the existing
  `function_indices: Vec<Option<u32>>` so `table.init`/`table.copy`'s own
  already-shipped funcref-only fast path stays completely undisturbed.
  Purely additive in this slice -- always empty at every construction site
  in this crate's own dependents (`wasm-wast-parser`, `wasm-module-parser`,
  `wasm-module-encoder`, `wasm-validator` test fixtures); real population
  (capturing each item's raw bytes instead of eagerly resolving a function
  index) is a LATER W38 slice's own work (see that spec's "Recommended
  slice decomposition" §4), not this one's.

New unit tests: `arrayref_any_single_byte`,
`arrayref_any_byte_tag_matches_encode`,
`nullref_is_a_bottom_subtype_of_arrayref_any`,
`arrayref_any_is_never_a_bottom_subtype_of_nullref`,
`arrayref_any_is_distinct_from_every_other_reftype`,
`arrayref_any_canonicalizes_deterministically`.

## [0.1.25] - 2026-09-02 (feat: `Eqref`/`StructRefAny` -- W37 GC reftype tables)

Two new `ValueType` variants, per `code/specs/W37-wasm-gc-reftype-tables.md`:

- `Eqref` -- `eqref`/`(ref null eq)`, the nullable abstract top of the `eq`
  hierarchy. Encoded as a single byte `0x6D`, mirroring `Anyref`'s `0x6E`/
  `I31ref`'s `0x6C` exactly.
- `StructRefAny` -- `structref`/`(ref null struct)`, the nullable abstract
  top of the `struct` hierarchy, distinct from the existing `StructRef(u32)`
  (a nullable CONCRETE reference that always carries a type index). Encoded
  as a single byte `0x6B`.

`is_bottom_subtype_of` gains `NullRef <: Eqref` and `NullRef <: StructRefAny`,
mirroring the existing `NullRef <: Anyref`/`NullRef <: I31ref`/`NullRef <:
StructRef(_)` arms. `canonicalize_value_type` gains matching arms (both
using the ALREADY-EXISTING `AbstractHeapKind::Eq`/`::Struct` -- this crate's
canonical-type machinery was one `ValueType` arm away from covering these,
not missing the underlying representation).

`WasmModule::table_concrete_element_types`'s own doc comment is corrected:
it previously asserted `Some(vt)` is "always `ConcreteFuncRef`/
`NonNullConcreteFuncRef`... because no struct/array-typed table can arise
here" -- true only because nothing had ever populated it with anything else
yet, not because the field's own `Vec<Option<ValueType>>` type was
restricted. `wasm-wast-parser` 0.1.100 is the first version to write a
non-func-family `ValueType` into this field (for a table declared with any
of the reftypes this slice adds text-format recognition for).

**Why this is a smaller change than the motivating W36 spec's "GC proposal
remainder" framing suggested**: W37's own re-verification (throwaway-probe
method against the pinned corpus) found the ~550 NYS directives it cited
split into three separate causes, only ~302 of which trace to a table
declaration at all -- see that spec's own "Correction 1" for the full
breakdown. This slice addresses exactly the table-declaration-attributable
share.

New unit tests: `eqref_structref_any_single_byte`,
`eqref_structref_any_byte_tag_matches_encode`,
`nullref_is_a_bottom_subtype_of_eqref_and_structref_any`,
`eqref_and_structref_any_are_never_bottom_subtypes_of_nullref`,
`eqref_and_structref_any_are_distinct_from_every_other_reftype`, and both
new variants added to `every_abstract_heap_type_canonicalizes_
deterministically`'s existing table-driven test.

## [0.1.24] - 2026-09-01 (fix: `Element::is_declarative`, closing the active/declarative-elem-segment-drop gap)

New field on `Element`: `pub is_declarative: bool`. Root-caused while
chasing the 4 genuinely new real conformance failures `wasm-conformance`
0.1.117's baseline regen honestly reported (not fixed) after
`wasm-wast-parser` 0.1.96 let `table_init.wast`/`table_init64.wast`/
`elem.wast`'s own "Implicitly dropped elements" corpus cases parse for
the first time.

**Real spec semantics, verified against the spec text directly**: "after
an active or declarative element segment is initialized, it is dropped"
-- so a `table.init` naming an active OR declarative segment must always
trap "out of bounds table access", never succeed, because by the time any
code can run, that segment has already been implicitly dropped at
instantiation. This is DIFFERENT from a genuinely PASSIVE segment, which
stays resident and `table.init`-able until an EXPLICIT `elem.drop` (never
`table.init` itself, which does not auto-drop a passive segment).

**The gap this field closes**: `Element` had no way to represent
"declarative" at all before this change -- `wasm-wast-parser::module::
build_elem`'s own doc comment already documented the deliberate
simplification of folding a declarative segment into `is_passive: true`
(same "no table, no offset" shape a genuinely passive segment has), on
the theory that "nothing in this repo's vendored corpus ever `table.init`
s/`elem.drop`s a segment this parser marks declarative." That census was
wrong: `elem.wast`'s own "Implicitly dropped elements" section does
exactly that (`(elem $e declare func $f) ... (table.init $e ...)`), newly
reachable now that `wasm-wast-parser` 0.1.96 fixed an unrelated arity bug
that had kept this module from parsing at all. With `is_passive: true`
the ONLY signal available, `wasm-runtime::instantiate()` had no way to
tell a declarative segment apart from a genuinely passive one, so it
could never mark a declarative segment "already dropped" without also
wrongly marking every genuinely passive segment dropped too.

`is_declarative` is `false` for every existing active or genuinely-
passive segment (every pre-existing construction site across this
crate and its sibling `wasm-*` crates updated to pass `false`
explicitly, no `Default` derive on `Element`) and `true` only for a
segment `wasm-wast-parser::module::build_elem` recognizes as `(elem
declare ...)`. See `wasm-runtime`'s own CHANGELOG for how `instantiate()`
now consults this flag; see `wasm-conformance`'s own CHANGELOG for the
full corpus-diff verification (exactly the 4 known cases move from
`Fail` to `Pass`, nothing else in the 257-file corpus changes).

## [0.1.23] - 2026-09-01 (add: `WasmModule::missing_data_count_section`)

New field for the same fresh corpus-prioritization pass as `wasm-
module-parser`/`wasm-validator`'s own CHANGELOGs (`code/specs/
W07-wasm-post-mvp-epics.md`'s "Addendum (2026-09-01)" item 2,
`binary.wast`'s "memory.init/data.drop requires a data count section"
`assert_malformed` cases).

- **New `WasmModule::missing_data_count_section: bool`** (default
  `false` via the existing `#[derive(Default)]`). `true` only when this
  module was parsed from a BINARY module that had no data count section
  (§12, binary id `0x0c`) at all. Deliberately phrased as a "missing"
  flag rather than `has_data_count_section` so every EXISTING
  `WasmModule` construction site — every hand-built test fixture across
  this crate and its sibling crates, and every TEXT-form module `wasm-
  wast-parser::module::parse_module_expr` builds directly (no binary
  round-trip, so no literal "data count section" concept even exists
  there) — keeps its current, correct behavior for free via `bool`'s own
  `Default`, instead of needing to opt in one by one. Only `wasm-
  module-parser`'s binary path, the one place that actually knows
  whether §12 was present in the bytes it just parsed, ever sets this to
  `true`. `wasm-validator`'s type-checker is the one place that reads it
  (its `0x08`/`0x09` — `memory.init`/`data.drop` — opcode arms), since
  it's the only crate that walks function-body instructions precisely
  enough to know when those opcodes are actually used.
- Two existing tests in this crate updated for the new field: `wasm_
  module_has_all_fields` (which deliberately uses struct-literal syntax
  with every field named, to catch exactly this kind of drift) and `wasm_
  module_default_is_empty` (asserts the new field's `Default` is
  `false`).

## [0.1.22] - 2026-09-01 (fix: tables can now carry a concrete element type)

Found while root-causing a real, previously-shipped regression: the whole
vendored `br_table.wast` corpus file (a foundational MVP-level control-
flow test, no GC-proposal syntax) was failing every single directive —
see `wasm-validator`'s and `wasm-conformance`'s own CHANGELOGs for the
full root-cause writeup and corpus-impact diff. This package's own piece
of the fix: `TableType::element_type` is only a `u8` tag (`0x70` funcref
/ `0x6F` externref) — there was no way to represent "this table holds
references to exactly function type `$t`" (the function-references
proposal's `(table $t (ref null $t) ...)`), only "this table holds some
funcref".

- **New `WasmModule::table_concrete_element_types: Vec<Option<ValueType>>`**
  — parallel to `tables`, same "allowed to be shorter than, or entirely
  absent from, the vec it augments" convention `type_kinds`/
  `type_subtyping` already established. `None` (or an index past the end)
  means the table's `element_type` byte is authoritative, exactly as
  before this field existed; `Some(vt)` (`vt` always `ConcreteFuncRef`/
  `NonNullConcreteFuncRef` in this crate's text format) is the table's
  REAL declared element type. Only module-DEFINED tables get an entry
  here — import tables have no concrete-typed-table text syntax in this
  crate, mirroring `functions: Vec<u32>`'s own "imports live in
  `imports`" convention.
  Purely additive: `element_type` keeps holding `FUNCREF` alongside a
  concrete entry (every concrete function reference is still funcref-
  family), so any pre-existing consumer that only reads the byte tag
  keeps working unchanged.
- `WasmModule` derives `Default`, so this field defaults to an empty
  `Vec` and every existing `WasmModule { ..., ..Default::default() }`
  test literal across the workspace kept compiling unchanged. The two
  spots that build a `WasmModule` with every field spelled out (this
  crate's own `wasm_module_has_all_fields` test, and `wasm-runtime`'s
  `tests/v128_persistent_storage.rs`) needed one line each added.

## [0.1.21] - 2026-09-01 (W34 fourth slice — cross-module canonical equivalence, epic closed)

Two new free functions supporting `wasm-runtime`'s cross-module
import-compatibility check (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`,
final slice): the same real canonical algorithm the third slice wired
into WITHIN-module checks, now usable when the two sides of a comparison
live in two entirely separate `canonical_types` tables (one per
independently-validated module, no shared numbering at all).

- **`canonical_type_entries_equivalent(a: Option<&(Rc<CanonicalGroup>, u32)>, b: Option<&(Rc<CanonicalGroup>, u32)>) -> bool`**
  — the actual comparison logic factored out of `canonical_types_equivalent`
  (which now just extracts two entries from ONE table and delegates here),
  so it can also be called directly with two entries from two DIFFERENT
  tables. `Rc::ptr_eq` fast path first (hits whenever both entries came
  from the SAME `canonicalize_types` call, thanks to that function's own
  interning), full derived `PartialEq` fallback otherwise — identical
  contract and cost profile to the pre-existing function, just no longer
  hardcoded to a single-table shape. `None` on either side is
  conservatively `false`, never a wrong `true`.
- **`canonical_chain_reaches(type_subtyping, canonical_types, start_idx, target) -> bool`**
  — the cross-module counterpart to `nominal_subtype_chain`'s own
  termination check: climbs ONE module's own local `sub` chain, starting
  at `start_idx` (reflexively including it), checking each ancestor
  against an EXTERNAL `target` from a different module's own canonical
  table. Needed because real WASM func-import compatibility is a
  SUBTYPING relation, not plain equality — an export declared `(sub
  $parent (func))` must be importable at its own `$parent` type, not only
  at its exact declared type (`type-subtyping.wast`'s own `M6`/`M7`
  "Linking" cases). A declared supertype relationship is only ever
  meaningful within the module that declared it, so this walks exactly
  ONE module's table, never crossing between two modules' own `sub`
  chains. Same `MAX_SUBTYPE_CHAIN_HOPS` bound as `nominal_subtype_chain`,
  for the identical termination/complexity reason.
- 6 new unit tests: `canonical_type_entries_equivalent` compared across
  two genuinely separate `canonicalize_types` calls (both the isomorphic-
  positive and differently-positioned-negative cases, plus every `None`
  combination); `canonical_chain_reaches` climbing a real declared chain to
  reach an external target (both the zero-hop reflexive case and the
  one-hop climb), and correctly failing to match a genuinely unrelated
  target.

**Security review finding, fixed before push**: a dedicated security-review
sub-agent, briefed on the NEW trust boundary this slice opens (the first
time this epic compares two INDEPENDENTLY-ATTACKER-CONTROLLED modules'
type data against each other, rather than one module's own data against
itself), found a real HIGH-severity DoS — this epic's FOURTH consecutive
slice with a genuine finding in its own review. `MAX_CANONICAL_TREE_
WEIGHT` bounds any ONE `CanonicalGroup` tree's shape at construction time
(so any ONE full structural comparison is itself bounded), but nothing
bounded how many times a full, near-max-weight comparison could be
ATTEMPTED across an entire `wasm-runtime::instantiate()` call: the
cross-module case can never hit the `Rc::ptr_eq` fast path that makes the
WITHIN-module case cheap (two different modules' `canonicalize_types`
calls never intern into the same allocation), and an attacker who
controls both the importing and exporting module can multiply one
expensive-but-capped comparison by an arbitrary, byte-cheap import count
(each import declaration costs only a few bytes, unlike the expensive
type itself) — `imports × hops (≤1,000) × per-comparison cost
(≤1,000,000 nodes)`, each factor individually capped but their PRODUCT
unbounded.

Fixed by adding:
- **`CrossModuleComparisonBudget`** — a shared, mutable work counter
  (`MAX_CROSS_MODULE_CANONICAL_COMPARISON_BUDGET = 4_000_000`, `new()`/
  `with_budget(u64)`/`remaining()`), meant to be created ONCE per
  `wasm-runtime::instantiate()` call and threaded `&mut` through that
  call's ENTIRE import-resolution loop (not reset per import). `take(cost)`
  uses `checked_sub`, floors at zero (never wraps, never panics), and
  reports "out of budget" once exhausted.
- **`canonical_type_entries_equivalent_budgeted`** and seven private
  per-field/per-variant helper functions (`group_equal_budgeted`,
  `subtype_equal_budgeted`, `comp_type_equal_budgeted`, `field_type_
  equal_budgeted`, `storage_type_equal_budgeted`, `val_type_equal_
  budgeted`, `heap_ref_equal_budgeted`) — a hand-written, budget-aware
  structural-equality walk mirroring derived `PartialEq`'s exact recursive
  shape (every field, every enum variant, every `Vec` length-checked
  before zipping), but charging the shared budget one unit per node
  visited — checked BEFORE recursing into any child, so exhaustion can
  never be preceded by uncharged work — and bailing out to "not equal"
  (fail CLOSED, never a false accept) the instant it's exhausted, rather
  than continuing an unconditional full walk. `canonical_chain_reaches`
  (this slice's own new cross-module chain walk) and `wasm-execution`'s
  `HostFunction::canonically_matches` (see that crate's own CHANGELOG)
  both now take this budget.
- Existing WITHIN-module comparisons (`canonical_types_equivalent`,
  `nominal_subtype_chain`, `is_assignable`'s termination check) are
  UNCHANGED and keep using the unbudgeted comparison — the amplification
  this fix closes is specific to the cross-module, attacker-controlled-
  import-count case; within one module, triggering many genuinely-
  different expensive comparisons requires declaring that many expensive
  types, itself bounded by module size.
- A re-review confirmed the fix's completeness directly, not just its
  presence: every field/variant of `CanonicalGroup`'s tree is covered by
  the budgeted walk (checked against the real struct/enum definitions,
  not assumed from the design), budget-before-work ordering holds in all
  seven helper functions, `Rc::ptr_eq`'s fast path can never fire
  incorrectly across two different modules' allocations, and the budget
  arithmetic has no overflow/panic/divide-by-zero path.
- New regression test `cross_module_comparison_budget_exhausts_and_fails_
  closed`: proves the budget genuinely exhausts and fails closed using a
  small custom budget (`with_budget(20)`), not by actually spending
  millions of real iterations — confirms both those two genuinely-
  different types stay correctly rejected throughout, and that a
  genuinely-equivalent (but not `Rc`-identical) pair is CONSERVATIVELY
  rejected once the budget runs out, proving the fail-closed direction
  applies even to real matches, not only mismatches.

Full 257-file conformance baseline re-confirmed byte-for-byte identical
before and after this fix, since it changes worst-case performance only,
never behavior.

## [0.1.20] - 2026-09-01 (W34 third slice — wire canonical equivalence into within-module subtyping)

`nominal_subtype_chain` (the shared, security-reviewed `sub`-chain walk
`wasm-validator`'s static `is_assignable` and `wasm-execution`'s runtime
`call_indirect`/`ref.cast`/`ref.test` dispatch both call) now upgrades its
reflexive base case AND every hop's own termination check from raw type-
index equality to real canonical equivalence, per the GC proposal's own
rule: "subtyping is nominal modulo type canonicalisation" — exactly the
one line of `MVP.md` this whole spec has been building toward.

- **Breaking change to `nominal_subtype_chain`'s signature**: gains a new
  `canonical_types: &[Option<(Rc<CanonicalGroup>, u32)>]` parameter
  (second position, matching `ValidatedModule::canonical_types`'/
  `WasmExecutionContext::canonical_types`'s own shape). Every existing
  caller either already has real canonical data to pass (`wasm-validator`,
  `wasm-execution` — see their own CHANGELOGs) or passes `&[]` (`WasmModule::
  func_type_is_nominal_subtype`, which never carried canonical data and
  stays nominal-only by design — see that method's own doc comment). An
  empty slice is a strict, zero-behavior-change superset of the pre-W34
  nominal-only rule: `canonical_types_equivalent` on an empty/too-short
  slice always reports `false`, proven by a new regression test
  (`nominal_subtype_chain_with_empty_canonical_table_matches_old_nominal_
  only_behavior`).
- **New `canonical_types_equivalent` free function** — the one shared
  comparison both `nominal_subtype_chain` and `wasm-validator::
  ValidatedModule::canonically_equivalent` now use, so the two copies of
  this comparison (chain-walk termination, public post-validation
  accessor) can never drift apart. `false`, conservatively, whenever
  either side is out of range or uncanonicalized (`None`) — never a wrong
  `true`.
- **3 new unit tests**: a positive case (two independently-declared,
  nominally-unrelated but canonically-equivalent types, accepted in both
  directions once real canonical data is supplied, correctly REJECTED by
  the nominal-only `func_type_is_nominal_subtype` on the same pair); a
  negative case (genuinely different shapes, still rejected even with
  real canonical data present); and the empty-table backward-compatibility
  proof above.

### Security fix (found by this slice's own review, fixed before push)

Wiring canonical equivalence into per-instruction call sites (`is_
assignable`, `call_indirect_type_matches`) turned a real, previously-
unreachable cost into a reachable one: `canonical_types_equivalent`
compared two `Rc<CanonicalGroup>`s via derived `PartialEq`, which walks
the FULL tree by CONTENT (never by pointer) whenever the two `Rc`s are
different allocations -- true for every pair of independently-declared,
canonically-equivalent-but-unrelated groups, exactly the case this slice
exists to accept. A security-review sub-agent built a real reproduction:
two ~19-level "doubling" `rec` chains (each near `MAX_CANONICAL_TREE_
WEIGHT`) referenced from two locals, with a function body repeatedly
flowing a value between them -- `validate()` took **over a minute** on a
~130KB crafted module (~123,000x slower than an equal-sized module that
never triggers the deep comparison), an entirely real, previously-
unreachable algorithmic-complexity DoS (this cost did not exist before
this slice wired canonical checks into per-instruction validation).

Fixed by interning: `canonicalize_types` now deduplicates content-
identical groups it produces WITHIN one call into a single shared `Rc`
allocation (a `HashSet<Rc<CanonicalGroup>>`, queried by borrowed
`&CanonicalGroup` content via `Rc<T>: Borrow<T>`, so no redundant clone
is needed just to look up a candidate), and `canonical_types_equivalent`
tries `Rc::ptr_eq` FIRST before falling back to full structural `==`.
Since interning guarantees identical content built by the SAME
`canonicalize_types` call always shares one allocation, this makes the
actually-reachable within-module case (everything this slice's own
call sites use) a genuine O(1) check after the first comparison, matching
`MVP.md`'s own Note 2 ("canonicalising them bottom-up in linear time
upfront... constant-time" comparison) precisely rather than only in
spirit. Interning itself costs at most one extra hash+lookup per group --
the same order of work `canonicalize_types` already pays to BUILD that
group, so this is a constant-factor addition, not a new algorithmic-
complexity class, and every existing `CanonicalCost` cap still bounds it
exactly as before. Cross-module comparison (two SEPARATE `canonicalize_
types` calls) is deliberately unaffected -- this cache is local to one
call, not a global/thread-shared interner, since no reachable call site
in this slice needs cross-module comparability yet (revisit if slice 4's
cross-module wiring measures a real need).

Verified empirically, not just reasoned about: a scaled-down reproduction
of the review's own attack shape (two 19-level doubling chains, 4,000
repeated cross-assignments) took **15.4s** with interning/the `Rc::ptr_eq`
fast path disabled and **~100ms** with them restored — confirmed by
temporarily reverting each fix in turn and re-running, not merely
asserted. New regression test
`identical_groups_within_one_module_intern_to_the_same_rc_allocation`
proves the mechanism directly (`Rc::ptr_eq`, not merely `==`, on two
independently-declared, differently-indexed, `sub`-unrelated identical
multi-member groups). Full conformance baseline re-confirmed byte-for-
byte identical before and after this fix (a pure performance change, zero
behavior change).

## [0.1.19] - 2026-09-01 (W34 second slice — canonical type-group equivalence, real multi-member `rec` groups)

Lifts the first slice's `rec_group_size == 1` restriction: `canonicalize_
types` now correctly canonicalizes real multi-member `rec` groups with
group-relative De Bruijn numbering, per `MVP.md`'s own "rolling"/"tying"
mechanism and the reference interpreter's `roll_rec_type` (re-verified
fresh against `WebAssembly/gc`'s current `interpreter/syntax/types.ml` and
`interpreter/valid/match.ml` — byte-for-byte identical to what the W34
spec cites).

- **Real group-relative `Rec(i)` numbering** — a reference to ANY member of
  the group currently being tied (not just to the referencing member
  itself) now ties to `CanonicalHeapRef::Rec(i)`, where `i` is that
  member's own position within the group (`target_idx - group_start`),
  not a module-absolute index. `resolve_heap_index` was generalized from a
  `self_idx`-based self-reference check to a `[group_start, group_end)`
  range check; every other helper (`canonicalize_value_type`,
  `canonicalize_field_type`, `canonicalize_comp_type`) is reused unchanged
  in shape, exactly as the first slice's own addendum predicted. A
  singleton group is now just the `group_end - group_start == 1` case of
  the same machinery, not a separate code path.
- **`canonicalize_types` now processes GROUPS, not individual flat
  indices** — a contiguous range of `rec_group_size` indices sharing one
  shape is built together as ONE `CanonicalGroup` (all members' bodies
  resolved against the SAME group bounds), then shared via `Rc::clone`
  across every one of that group's flat indices, differing only in the
  `u32` position half of `(Rc<CanonicalGroup>, u32)`. If ANY member fails
  to canonicalize, the WHOLE group's every member becomes `None` — never a
  partial group.
- **Two separately-declared multi-member groups with identical internal
  wiring canonicalize equal, regardless of flat-index numbering or which
  module they came from**; two groups with the same member count but
  different internal reference wiring do NOT — proven directly by new
  unit tests, not just asserted. Composition of the first slice's `Outer`
  (cross-group embedding) with the new multi-member `Rec` numbering is
  proven both directions: a later type referencing an earlier multi-member
  group, and a later multi-member group mixing an `Outer` reference and an
  in-group `Rec` reference within the SAME member.
- **New DoS finding, closed in the same slice that introduced its own
  precondition**: real multi-member groups make "one group referencing an
  earlier one from several sibling positions at once" far more natural
  than the first slice's singleton-only groups ever could. A chain of such
  branching references DOUBLES the total node count a full structural
  `PartialEq`/`Hash`/`Drop` traversal must visit at every level (while
  `Rc` sharing keeps actual memory linear), which the first slice's own
  `MAX_CANONICAL_OUTER_DEPTH` (bounding STACK depth, i.e. the longest
  single reference chain) does NOT catch, since branching leaves that
  longest chain short even as the total node count explodes
  exponentially. Closed by threading a second, independent cost dimension
  (`CanonicalCost::weight`, summed — not maxed — across sibling
  references) alongside the existing depth, capped at
  `MAX_CANONICAL_TREE_WEIGHT` (1,000,000). A new regression test
  (`outer_embedding_weight_is_capped_for_branching_reference_chains`)
  builds a 40-level doubling chain and confirms it is rejected quickly
  rather than hanging or exhausting memory.
- **9 new unit tests** in `wasm-types` (2-member mutual group group-
  relative numbering; cross-module comparability for a real multi-member
  group; same member count with different wiring correctly NOT equal;
  both directions of `Outer`+multi-`Rec` composition; the W33/W34 addenda's
  own worked "3-cycle" example, canonical forms confirmed directly per
  this slice's own scope boundary; `type-canon.wast`'s real 5-member
  fixture; inconsistent multi-member metadata still safely producing
  `None`; the branching-weight DoS regression above) plus 2 new
  `wasm-validator` tests exercising the same through the real `validate()`
  entry point.
- Corpus impact: none observable in this slice's own tally (nothing wires
  canonical equivalence into any validation/execution DECISION path yet —
  that remains slice 3/4's job); see the full 257-file baseline diff in
  `wasm-conformance`'s own CHANGELOG and `code/specs/
  W34-wasm-gc-canonical-type-equivalence.md`'s addendum for the
  measurement.

## [0.1.18] - 2026-09-01 (W34 first slice — canonical type-group equivalence, singleton groups)

Adds `CanonicalGroup`/`CanonicalSubtype`/`CanonicalCompType`/
`CanonicalFieldType`/`CanonicalStorageType`/`CanonicalValType`/
`CanonicalHeapRef`/`AbstractHeapKind` and the `canonicalize_types` free
function (`code/specs/W34-wasm-gc-canonical-type-equivalence.md`) — the
first slice of the real WasmGC canonical type-group equivalence algorithm
(MVP.md's own "tying"/"rolling" mechanism, and the reference interpreter's
`roll_rec_type`/`match_def_type`), grounded directly in both sources.

- **Scope: `rec_group_size == 1` groups only** — every plain,
  non-`rec`-wrapped `(type ...)` field, and every explicit
  `(rec (type ...))` with exactly one member. A self-reference inside such
  a group ties to `CanonicalHeapRef::Rec(0)` (the only in-group reference a
  singleton can express); a reference to an EARLIER singleton group embeds
  that group's already-computed canonical form wholesale via
  `CanonicalHeapRef::Outer` (an `Rc<CanonicalGroup>`, not the design
  sketch's `Box` — sharing, not deep-cloning, the referenced subtree at
  every embed site, matching the "cheap to clone" contract `wasm-validator::
  ValidatedModule`'s own new `canonical_types` cache field needs).
  Multi-member `rec` groups (real De Bruijn numbering across MORE than one
  member) are explicitly deferred to a later slice — every member of such a
  group canonicalizes to `None`, never a wrong or partial value.
- **`canonicalize_types` never recurses** — it processes flat type-section
  indices in strictly increasing order and only ever looks up
  ALREADY-COMPUTED entries for anything outside the group being built, so a
  cyclic or self-referential type structure (even one from a hand-built
  `WasmModule` that skipped validation entirely) can only ever produce a
  `None` entry, never a panic, infinite loop, or stack overflow.
- **Security fix (found in review, before push): `MAX_CANONICAL_OUTER_DEPTH`
  (1,000, mirroring this crate's own pre-existing `MAX_SUBTYPE_CHAIN_HOPS`
  convention)** — a security review empirically confirmed that while
  *building* a `CanonicalGroup` tree never recurses (see above), a long
  CHAIN of singleton groups each referencing only the immediately
  preceding one (no cycle needed) builds a genuinely nested `Outer`-
  embedding tree whose compiler-derived `Drop`/`PartialEq`/`Hash` DO
  recurse to tear down or compare — reliably crashing the process (real
  stack overflow) at tens of thousands of chained links, reachable from a
  small, realistic module. `resolve_heap_index` now refuses (`None`) to
  extend a chain past 1,000 links, the one place new depth is introduced,
  closing this for all three derived traversals at once with a wide
  safety margin below the depth that was shown to matter.
- **Cross-module comparability, proven directly**: two independently-built
  `WasmModule`s with isomorphic singleton-group shapes at completely
  different flat indices canonicalize to structurally-equal
  `CanonicalGroup` values (`derive(PartialEq, Eq, Hash)` all the way down,
  comparing contents through every `Rc`, never pointers) — see the new
  cross-module unit tests in `src/lib.rs`.
- **`AbstractHeapKind` correction vs. the spec's own design sketch**: the
  spec document's `AbstractHeapKind` sketch (written before re-verifying
  against current code) listed only the ten WasmGC-proposal-native kinds
  (`Any`/`Eq`/`I31`/`Struct`/`Array`/`Func`/`None`/`Extern`/`NoExtern`/
  `NoFunc`) and omitted `Exn`/`NoExn` — but `ValueType::Exnref`/
  `ValueType::NullExnref` (W24, the separate exceptions proposal) already
  exist in this crate and need somewhere to tie to. Added both; see the
  spec's own addendum for the full account.
- No corpus pass-count impact expected or observed from this slice alone
  (nothing wires `canonical_types` into any validator/execution decision
  point yet — that starts at a later slice); the full 257-file conformance
  baseline diff is byte-for-byte identical before/after this change.

## [0.1.17] - 2026-09-01 (W33 fourth slice — struct/array TEXT-format representation)

Adds the type-system vocabulary `wasm-wast-parser`'s struct/array TEXT-format
grammar needs, closing the gap the first three W33 slices' own addenda all
independently confirmed as the real remaining blocker (`code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s addenda):

- **`StorageType`** (`Val(ValueType) | I8 | I16`) — the GC proposal's real
  `storagetype` grammar, previously entirely unmodeled (`FieldType.val_type`
  was a bare `ValueType` with no way to express packed 8/16-bit field
  storage at all). `FieldType.val_type` is renamed to `FieldType.storage:
  StorageType` — a breaking rename, fixed at every call site in this
  workspace (`wasm-module-parser`, `wasm-module-encoder`, `iir-to-wasm`,
  this crate's own tests); `FieldType::plain(val_type, mutable)` is a new
  convenience constructor matching the old `{ val_type, mutable }` literal
  shape for the (overwhelming majority) non-packed case.
- **`ArrayType`** (`{ element: FieldType }`) — did not exist anywhere in this
  repo before this slice (confirmed via a repo-wide grep, not assumed); a
  new `WasmModule::array_types: Vec<ArrayType>` field holds them, alongside
  the pre-existing `struct_types`.
- **`TypeKind`** (`Func | Struct(u32) | Array(u32)`) and a new
  `WasmModule::type_kinds: Vec<TypeKind>` parallel ledger — needed because
  real WAT text freely interleaves `(type $t (struct ...))` among
  `(type $t (func ...))` declarations, and `wasm-wast-parser`'s own two-pass
  design can append MORE func types to `types` in its second pass (`dedup_
  type`, for an inline-only function signature) strictly AFTER a struct/array
  type earlier in the SAME module has already been assigned its flat
  type-section index — breaking the pre-existing `types.len() + k` offset
  formula the very first time either happens (both are real, common shapes
  in `struct.wast`/`array.wast`'s own vendored text, not hypothetical edge
  cases). `WasmModule::struct_type_at`/`array_type_at` resolve a flat index
  `type_kinds`-aware first, falling back to the legacy offset formula when
  `type_kinds` is empty — so every pre-existing binary-decoded or hand-built
  `WasmModule` (which never populates `type_kinds`) is completely unaffected.
- **`ValueType::ArrayRef(u32)`/`NonNullArrayRef(u32)`** — the array-hierarchy
  analogues of the existing `StructRef`/`NonNullStructRef`, same two-byte
  `0x63`/`0x64` encoding (disambiguated by index space, not by tag byte,
  exactly like `StructRef`/`ConcreteFuncRef` already are). Wired into
  `is_bottom_subtype_of` (`NullRef <: ArrayRef(_)`, any index) and
  `is_non_null_subtype_of` (`NonNullArrayRef(i) <: ArrayRef(i)` same index,
  `NonNullArrayRef(_) <: Anyref` any index) — the array-hierarchy mirror of
  every rule `StructRef`/`NonNullStructRef` already had.

11 new unit tests (`StorageType`/`ArrayType`/`TypeKind`/`struct_type_at`/
`array_type_at`/`ArrayRef` subtyping and encoding); all 78 tests in this
crate pass. Binary-format array/packed-field encoding (`wasm-module-parser`/
`wasm-module-encoder`) is deliberately NOT extended by this slice — this
repo's WASM conformance harness runs entirely through the TEXT-format
pipeline (`wasm-wast-parser` → `WasmModule` → `wasm-execution`, never through
a binary round-trip for a plain `(module ...)` script directive; see
`wasm-conformance`'s own pipeline doc comment), so it isn't required for
`struct.wast`/`array.wast` conformance and is out of scope here — recorded
as a real, still-open gap in this slice's own spec addendum.

## [0.1.16] - 2026-08-31 (W33 second slice — real dynamic dispatch, item 4)

Extracts the nominal reflexive/transitive subtype-chain walk out of
`WasmModule::func_type_is_nominal_subtype` into a new free function,
`nominal_subtype_chain(type_subtyping: &[TypeSubtyping], sub_idx, super_idx)
-> bool`, so `wasm-execution`'s new runtime `call_indirect`/`ref.cast`/
`ref.test` dynamic dispatch checks (W33's own item (4), see `code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s second addendum) can reuse the
EXACT same, already security-reviewed (cycle-safe, hop-capped) walk instead
of re-implementing it against a bare `&[TypeSubtyping]` slice — `wasm-
execution::WasmExecutionContext` deliberately doesn't hold a full
`WasmModule` (see that struct's own doc comments), so a method on
`WasmModule` alone couldn't be called from there. `WasmModule::
func_type_is_nominal_subtype` is now a one-line wrapper around this;
behavior and the `MAX_SUBTYPE_CHAIN_HOPS = 1_000` bound are unchanged.

### Added

- **`nominal_subtype_chain(type_subtyping: &[TypeSubtyping], sub_idx: u32,
  super_idx: u32) -> bool`**: the free-function form described above.
- **`any_declares_subtyping(type_subtyping: &[TypeSubtyping]) -> bool`**:
  whether ANY entry is non-default (a real declared `sub $parent`,
  non-final, or a real `>1`-member `rec` group). Needed because
  `wasm-wast-parser`'s `dedup_type` pushes a `TypeSubtyping::default()`
  placeholder for EVERY type it declares, `sub`-declared or not — so
  `type_subtyping.is_empty()` is NOT a reliable "this module never uses
  `sub`" signal (the vector is fully populated for nearly every real
  module). `wasm-execution`'s new dynamic-dispatch checks use this to
  decide between two rules: no real `sub` anywhere → the engine's original
  pre-W33 structural-equality check (zero regression risk for the 256
  vendored corpus files that never use `sub`); real `sub` present
  somewhere → the real nominal (reflexive-or-subtype) check GC-proposal
  type identity actually requires.

## [0.1.15] - 2026-08-31 (W33 first slice — GC nominal subtyping + `rec` groups)

Implements the `wasm-types` half of `code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s "first slice" scope: function-type
field-list subtyping rules and `(rec ...)` type-group SYNTAX (parsing +
within-module nominal `sub`/`final` checking), explicitly deferring the
spec's own item (3b) (cross-module canonical type-group equivalence) and
item (4) (dynamic `ref.cast`/`ref.test`/`call_indirect` checks against real
subtype relationships).

### Added

- **`TypeSubtyping`**: per-type-section-entry metadata for the GC
  proposal's `(sub [final] $parent (comptype))` declaration syntax —
  `supertype: Option<u32>` (the declared parent, if any), `is_final: bool`
  (whether further subtyping is foreclosed — `true` by default, matching
  the real GC proposal's own "no `sub` clause = final" rule), and
  `rec_group_size`/`rec_group_position: u32` (which `(rec ...)` group a
  type belongs to and its position within it — needed only for
  `wasm-runtime`'s cross-module import/tag type-compatibility check, see
  the field's own doc comment for why).
- **`WasmModule::type_subtyping: Vec<TypeSubtyping>`**: a parallel array to
  `types` (function types only this slice). Deliberately allowed to be
  shorter than `types` (or empty) — every accessor treats a missing entry
  as `TypeSubtyping::default()` (final, no supertype, singleton group),
  the exact semantics every type already had before this field existed.
  This means adding the field required touching only ONE of this
  workspace's many existing `WasmModule { .. }` literals (a `#[cfg(test)]`
  one in this crate itself using the fully-exhaustive form; every other
  literal in the workspace either uses `..Default::default()` already or
  is unaffected because `Vec<T>: Default` needs no `T: Default` bound).
- **`WasmModule::type_subtyping_at`**: safe, panic-free accessor for the
  above (falls back to `TypeSubtyping::default()` for any out-of-range or
  never-populated index).
- **`WasmModule::func_type_is_nominal_subtype(sub_idx, super_idx)`**:
  reflexive, transitive nominal subtype check by walking the declared
  `sub $parent` chain via absolute type-section index — correct WITHIN one
  module (an index is a unique, unambiguous identity there); bounded to
  `types.len()` hops so a malformed/cyclic chain can't loop forever.
  Deliberately does NOT attempt structural/canonical equivalence between
  two independently-declared types with no `sub` relationship, even if
  byte-identical in shape — that's W33's own explicitly-deferred item
  (3b).
- **`WasmModule::type_group_shape(idx)`**: `(rec_group_size,
  rec_group_position)` for a type — the input `wasm-runtime`'s
  cross-module comparison ANDs onto its pre-existing structural `FuncType`
  equality check, a conservative strengthening that can only prevent a
  false accept (two same-shaped-but-different-position rec-group members
  wrongly treated as the same type — see `tag.wast`'s own
  `assert_unlinkable` case), never introduce a new false reject beyond
  what the pre-existing simpler check already risked.

### Security review follow-up

- **`func_type_is_nominal_subtype` now bounds its chain walk to a fixed
  `MAX_SUBTYPE_CHAIN_HOPS` (1,000) constant instead of `self.types.len()`.**
  The original bound was correct for TERMINATION but not algorithmic
  complexity: this method is called from `wasm-validator::is_assignable`
  at roughly every instruction operand's `pop_expect` call site, so a
  module declaring one very long, entirely spec-legal `sub` chain (N
  types) plus M call sites checking assignability near the chain's root
  forced O(N·M) total validation work — confirmed to scale linearly per
  query via direct benchmarking (a security review finding, not a
  hypothetical). A chain longer than the cap now safely reports "not a
  nominal subtype" beyond the cutoff — a false negative (can only make
  the caller reject something a deeper walk might have accepted), never
  a false accept.

## [0.1.14] - 2026-08-31 (W32 second slice — non-null concrete reference types)

### Added

- **`ValueType::NonNullStructRef(u32)`/`NonNullConcreteFuncRef(u32)`**: the
  **non-null concrete reference types** from the GC/function-references
  proposals — `(ref $T)`/`(ref $t)`, no `null` keyword — per
  `code/specs/W32-wasm-non-null-concrete-reference-types.md`'s addendum
  section 1. Binary tag `0x64 <LEB128(idx)>`, independently verified
  against the real reference interpreter's `interpreter/binary/decode.ml`
  (`ref_type`'s `-0x1c -> (NoNull, heap_type s)` arm: `-28 mod 128 =
  0x64`) — one more than `StructRef`/`ConcreteFuncRef`'s existing `0x63`
  ("nullable"), matching the spec document's own claimed value exactly
  (no discrepancy to fix here, unlike W24's `exnref` bug or this same
  spec's own first-slice keyword-spelling correction).
- **`ValueType::is_non_null_subtype_of`**: the non-null subtyping lattice
  from the spec's section 2 — `NonNullStructRef(i) <: StructRef(i) <:
  Anyref` and `NonNullConcreteFuncRef(i) <: ConcreteFuncRef(i) <: Funcref`
  (both hops of each chain are direct rules, not composed, matching how
  `ConcreteFuncRef <: Funcref` (W11-B) and the four W32-first-slice bottom
  types were each direct rules too). The reverse direction never holds —
  a nullable type is never accepted where non-null is required, and
  NEITHER bottom type (`NullRef`/`NullFuncref`) satisfies a non-null slot
  either.

### Scope note

This is the **second slice** of W32: the two non-null concrete-ref
variants and their subtyping rules only. Structural subtyping for
`call_indirect`/`ref.cast` against concrete function/struct types (needed
by `type-subtyping.wast`), real recursive type groups' own nominal
identity rules, and array types remain a later slice — see this
package's own addendum to the spec document.

## [0.1.13] - 2026-08-31 (W32 first slice — bottom reference types)

### Added

- **`ValueType::NullFuncref`/`NullExternref`/`NullExnref`/`NullRef`**: the
  four **bottom reference types** from the GC/function-references/
  exceptions proposals (`nullfuncref`/`nullexternref`/`nullexnref`/
  `nullref`, a.k.a. `none`) — each a genuine strict subtype of every
  compatible nullable reference type in its own hierarchy (func/extern/
  exn/any), per `code/specs/W32-wasm-non-null-concrete-reference-types.md`
  section 1. Single-byte encodings `0x73`/`0x72`/`0x74`/`0x71`,
  independently verified against the real reference interpreter's
  `interpreter/binary/decode.ml` (`NoFuncHT = -0x0d`, `NoExternHT =
  -0x0e`, `NoExnHT = -0x0c`, `NoneHT = -0x0f`; SLEB128 single-byte
  encoding of a small negative value is `value mod 128`) rather than
  re-asserted from this crate's own prior doc comments — the same
  discipline W24's `exnref` tag-byte bug established. Replaces an earlier
  pass's "lossy aliasing" of these four keywords straight onto their
  nullable supertypes (`nullfuncref` == `funcref`, etc., in
  `wasm-wast-parser`), which made the bottom types indistinguishable from
  their supertypes and unable to express the asymmetric subtyping the
  spec (and the real corpus) requires.
- **`ValueType::is_bottom_subtype_of`**: the bottom-type subtyping lattice
  from the spec's section 2 — e.g. `NullFuncref <: Funcref` and
  `NullFuncref <: ConcreteFuncRef(_)` for every index (bottom of the
  WHOLE func hierarchy), `NullRef <: Anyref`/`I31ref`/`StructRef(_)`, etc.
  Deliberately NOT reflexive (`T <: T` is the caller's own `==` check) and
  deliberately does NOT encode non-null concrete refs or structural
  subtyping — both explicitly out of scope for this first slice; see the
  spec's own "Explicitly out of scope" section.

### Scope note

This is the **first slice** of W32: only the four bottom types and their
subtyping rules. `NonNullStructRef`/`NonNullConcreteFuncRef` (non-null
concrete refs) and structural subtyping for `call_indirect`/`ref.cast`
against concrete function/struct types are a separate, later slice — see
the spec's own addendum.

## [0.1.12] - 2026-08-26 (W11 addendum — concrete function-type refs)

### Added

- **`ValueType::ConcreteFuncRef(u32)`**: a nullable reference to a
  specific concrete FUNCTION type (`(ref null $t)` where `$t` names a
  `func` type) — the function-references-proposal analogue of the
  pre-existing `ValueType::StructRef(u32)`, but indexing
  `WasmModule::types` (the func-type array) directly instead of the
  struct-type array's `+ types.len()`-offset space. Needed for exactly
  one real-corpus construct: `return_call.wast`/
  `return_call_indirect.wast`'s "Result subtyping" test, which declares a
  helper function `(func $f (result (ref null $t)) (ref.null $t))` and
  then uses its result where `funcref` is expected. Encodes identically
  to `StructRef` (`0x63` followed by `LEB128(idx)`) — see the variant's
  own doc comment for why the two never collide despite sharing that tag
  byte. Deliberately does NOT add a non-null `(ref $t)` variant (the real
  "typed function references" wall, tracked separately, much larger in
  scope).

## [0.1.11] - 2026-08-26 (W26 — table64 proposal, first slice)

### Changed

- **`TableType` gains `pub is64: bool`** (table64 proposal): whether this
  table uses 64-bit addressing, mirroring `MemoryType::is64` (W25) exactly.
  Defaults to `false` at every existing call site — no behavior change for
  any 32-bit table. `Limits` (already widened to `u64` in W25) is reused
  as-is — table64's own real spec ceiling is `u64::MAX`, verified live
  against the reference interpreter's `check_tabletype`, not the smaller
  `2^48`-page bound memory64 uses.

See `code/specs/W26-wasm-table64-first-slice.md` for the full slice
(binary/text encoding, validator/executor address-width plumbing, vendored
`table64.wast` and `memory64-imports.wast`).

## [0.1.10] - 2026-08-26 (W25 — memory64 proposal, first slice)

### Changed

- **`Limits.min`/`max` widened from `u32`/`Option<u32>` to
  `u64`/`Option<u64>`**: a real, spec-valid 64-bit memory's limits can
  reach `2^48` pages (the memory64 proposal's own ceiling), which doesn't
  fit `u32`. `TableType` shares this same struct and stays well within
  `u32`'s range for every value this repo has ever built — a pure,
  numerically non-breaking widening for every existing table/32-bit-
  memory caller.
- **`MemoryType` gains `pub is64: bool`** (memory64 proposal): whether
  this memory uses 64-bit addressing. Defaults to `false` at every
  existing call site — no behavior change for any 32-bit memory.

See `code/specs/W25-wasm-memory64-first-slice.md` for the full slice
(binary/text encoding, validator/executor address-width plumbing,
vendored `memory64.wast`).

## [0.1.9] - 2026-08-26 (W24 — exceptions proposal, fourth slice: real exnref)

### Fixed

- **Security review finding**: `ValueType::Exnref::byte_tag`/`encode`
  changed from `0xE9` to `0x69`. `0xE9` was this variant's real spec type
  opcode `-0x17` mis-encoded as its two's-complement-mod-256 byte
  (`-23 + 256 = 233 = 0xE9`) rather than its correct single-byte SLEB128
  encoding (`-23 & 0x7F = 0x69`) — every OTHER abstract reference type
  here (`funcref` `-0x10`→`0x70`, `externref` `-0x11`→`0x6F`, `anyref`
  `-0x12`→`0x6E`, `i31ref`→`0x6C`) happens to have its raw byte value
  ALSO be its correct SLEB128 encoding, which is what let this go
  unnoticed since W22 first added `Exnref` — `exn`'s value (`-0x17`)
  is simply the first one where the two representations diverge. `0xE9`
  has its LEB128 continuation bit SET (`>= 0x80`), making it
  indistinguishable, in a blocktype decoder, from the leading byte of a
  genuine multi-byte type index — a real, attacker-reachable bug
  (surfaced and fixed while adding `exnref` blocktype-shorthand support
  for W24; see `wasm-execution`/`wasm-validator`'s own changelogs).
  `0x69` has its continuation bit clear, matching every other
  special-cased blocktype byte's own safe invariant. No test anywhere in
  this repo hard-coded the old `0xE9` value (confirmed via a repo-wide
  grep before changing it), so this is a clean, non-breaking fix.

### Changed

- `ValueType::Exnref`'s doc comment updated: no longer "deliberately
  inert" as of `wasm-execution` 0.9.68 — a `catch_ref`/`catch_all_ref`
  clause that matches now pushes a real, reified `exnref` value (a handle
  into `wasm-execution`'s new `exception_heap`), and `throw_ref` consumes
  one to re-raise the exception it names.

## [0.1.8] - 2026-08-25 (W22 — exceptions proposal: real catch/catch_all matching)

### Added

- `ValueType::Exnref` — the exceptions proposal's `exnref` type (real
  spec byte `-0x17`, i.e. unsigned `0xE9`). Deliberately inert: recognized
  purely so a module MENTIONING it (e.g. a `catch_ref`/`catch_all_ref`
  target block's declared result type) still parses/validates as a
  whole, since W14's per-module build isolation means one unrecognized
  value type anywhere in a module fails the ENTIRE module — and the real
  testsuite's own `try_table.wast` mixes `exnref`-typed functions
  alongside ordinary `catch`/`catch_all`-only ones in the SAME module.
  Never a real runtime value in this repo (no `catch_ref`/`catch_all_ref`
  clause is ever selected as a match — see `wasm-execution`'s own
  changelog). See `code/specs/W22-wasm-exceptions-catch-clause-matching.md`.

## [0.1.7] - 2026-08-25 (W21 — exceptions proposal: tag/throw first slice)

### Added

- `ExternalKind::Tag = 0x04` — matches the real exception-handling
  proposal's binary encoding exactly (live-fetched and confirmed against
  `WebAssembly/exception-handling`'s own `Exceptions.md`).
- `ImportTypeInfo::Tag(u32)` — a tag import's function-type index (the
  tag's underlying signature; its `results` must be empty, a rule
  `wasm-validator` enforces, not this crate).
- `WasmModule.tags: Vec<u32>` — module-defined tags' type indices, same
  "imports live in `imports`, this Vec is only the module-defined ones"
  convention `functions: Vec<u32>` already uses.

See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.

## [0.1.6] - 2026-08-17 (task #97 — passive/exprs-list element segments)

### Changed

- `Element.function_indices` widened from `Vec<u32>` to
  `Vec<Option<u32>>` -- `None` represents a `ref.null` entry in a
  passive exprs-list segment (`(elem funcref (ref.func $f) (ref.null
  func))`), `Some(idx)` a real function reference, reusing the same
  `Option<u32>` shape `Table::elements`/`WasmValue::Ref` already use
  rather than inventing a new one.
- `Element.is_passive: bool` added, mirroring `DataSegment.is_passive`
  (task #95) exactly: `true` for a segment declared with no table index
  or offset expression at all, so `wasm-runtime::instantiate()` never
  applies it automatically -- it stays resident until an explicit
  `table.init` copies from it or `elem.drop` frees it.
- Binary encoding scope (see `code/specs/W17-wasm-bulk-table-ops.md`
  for the real-corpus census that justified this): only 4 of the
  spec's 8 element-segment modes are represented (0/1/2/5 -- active-
  implicit funcidx-list, passive funcidx-list, active-explicit
  funcidx-list, passive exprs-list restricted to `ref.func`/`ref.null`).
  Modes 3/7 (declarative) and 4/6 (active+exprs) are non-goals; no
  vendored corpus file this repo uses needs them.

### Migration

- Every existing construction site (`Element { function_indices: vec![1,
  2], .. }`) now needs `vec![Some(1), Some(2)]`; every read site
  (`for func_idx in &elem.function_indices`) now receives
  `Option<u32>` instead of a bare `u32`.

## [0.1.5] - 2026-08-16 (task #95 — passive data segments)

### Added

- `DataSegment.is_passive: bool` -- `true` for a passive segment (bulk-
  memory proposal): declared with no offset expression at all
  (`(data $d "bytes")`, or binary segment-mode flag `0x01`), so
  `wasm-runtime::instantiate()` never applies it automatically -- it
  stays resident until an explicit `memory.init` copies from it or
  `data.drop` frees it. `false` for an ordinary WASM 1.0 active segment,
  unchanged. Additive field on an existing struct -- every existing
  construction site across the workspace needed `is_passive: false`
  added, but no other field's meaning changed.

## [0.1.4] - 2026-08-16 (task #96 — multi-table, `EXTERNREF` constant)

### Added

- `pub const EXTERNREF: u8 = 0x6F`, alongside the existing `FUNCREF`. Used
  by `wasm-wast-parser` to fix a real bug where a table's declared
  `externref` reftype was silently discarded during parsing in favor of
  a hardcoded `FUNCREF` default.

## [0.1.3] - 2026-08-15 (SIMD PR1a — `ValueType::V128`)

### Added

- `ValueType::V128` — the SIMD proposal's 128-bit lane vector type,
  encoded as a single byte `0x7B` (verified against the SIMD proposal's
  own binary-encoding table). `byte_tag()`/`encode()` both updated.
  Unlike the numeric types, its 16 raw bytes don't fit in this repo's
  shared `virtual-machine::Value` typed-stack slot (max 64 bits) — see
  `wasm-execution` 0.8.0 for how the value level carries it (a heap
  handle, mirroring `Anyref`/`I31ref`'s own `WasmValue::Ref` handle
  shape) and `code/specs/W13-wasm-simd-v128-first-slice.md` for the full
  design.

## [0.1.2] - 2026-08-15 (WASM18 — `shared` bit on `MemoryType`)

### Added

- `MemoryType` gained a new `pub shared: bool` field (threads proposal,
  binary-format flags bit 1). A **breaking** struct-field addition — every
  `MemoryType { limits, .. }` construction site across the workspace
  needed a `shared: false`/real value added; see `wasm-module-parser`
  0.2.2 and `wasm-wast-parser` 0.1.9 for the two places that now decode
  a real value instead of always defaulting it.

### Corrected (implementation-time, vs. the merged W09 spec)

- The merged `code/specs/W09-wasm-atomics-plain.md` spec claimed atomic
  instructions require the target memory be declared `shared`. The real,
  pinned-commit WebAssembly threads-proposal testsuite (`atomic.wast`)
  directly contradicts this with its own `;; unshared memory is OK`
  module, exercising every atomic op against a plain, non-shared memory
  and expecting success. `wasm-validator` 0.2.3 does NOT gate atomic ops
  on `shared`; this field exists purely so `shared` round-trips
  correctly through parse/encode, not to drive a validation rule.

## [0.1.1] - 2026-08-15 (WASM17 — funcref/externref as first-class value types)

### Added

- Two new `ValueType` variants: `Funcref` (`byte_tag()` = `Some(0x70)`) and
  `Externref` (`byte_tag()` = `Some(0x6F)`), reusing this repo's own
  `funcref` = `0x70` convention already established by the pre-existing
  `FUNCREF` constant (`TableType::element_type`'s default). Both encode as
  single bytes via `ValueType::encode`, matching `Anyref`/`I31ref`.
- Part of the WASM17 slice (see `code/specs/W08-wasm-funcref-externref.md`)
  unblocking real conformance-testsuite files (`global.wast`, `select.wast`,
  `br_table.wast`, `call_indirect.wast`) that reference `funcref`/`externref`
  as real value types, not just the implicit table element type.

## [0.1.0] - 2026-03-23

### Added

- Initial package scaffolding generated by scaffold-generator

## [0.2.0] - 2026-03-23

### Added

- Full WASM 1.0 type system implementation in `src/lib.rs`
- `ValueType` enum (`I32`, `I64`, `F32`, `F64`) with `#[repr(u8)]` discriminants
  matching WASM binary encoding (0x7C–0x7F)
- `BlockType` enum (`Empty`, `Value`, `TypeIndex`) for structured control flow
- `BLOCK_TYPE_EMPTY` constant (0x40)
- `ExternalKind` enum (`Function`, `Table`, `Memory`, `Global`) with `#[repr(u8)]`
  discriminants matching WASM binary encoding (0x00–0x03)
- `FuncType` struct for function signatures (params + results)
- `Limits` struct for min/max size constraints on memories and tables
- `MemoryType`, `TableType`, `GlobalType` structs
- `FUNCREF` constant (0x70) for the table element reference type
- `Import`, `ImportTypeInfo`, `Export` structs
- `Global`, `Element`, `DataSegment`, `FunctionBody`, `CustomSection` structs
- `WasmModule` struct — top-level container for all decoded module sections,
  with `#[derive(Default)]` for ergonomic construction
- 26 unit tests covering all types, constants, construction, equality, and edge cases
- Literate programming style throughout: ASCII diagrams of binary encoding,
  explanations of WASM execution semantics, and inline examples
