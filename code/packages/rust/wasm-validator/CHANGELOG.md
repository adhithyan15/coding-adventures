# Changelog

All notable changes to this package will be documented in this file.

## [0.2.84] - 2026-09-01 (fix: memarg/prefixed-sub-opcode LEB128 under-strictness + missing data-count-section gate)

A fresh corpus-wide prioritization pass after the W32/W33/W34 campaign
closed (`code/specs/W07-wasm-post-mvp-epics.md`'s "Addendum (2026-09-01)"
item 2) found `binary-leb128.wast` (7 failures of 58 `assert_malformed`)
and `binary.wast` (2 failures of 107) failing with the identical harness
message `"binary module parsed but should have been rejected as
malformed"`. All 9 looked like one bug from the outside; direct-probing
each specific failing directive (`wasm_conformance::run_wast_source`,
one throwaway test per file) showed they're actually **two unrelated
root causes**, both in this crate:

### Fixed — memarg align/offset and `0xFC`/`0xFD` sub-opcode LEB128 strictness (7 of the 9)

`wasm-module-parser` never walks function-body instructions (bodies are
stored as raw bytes — see that crate's own `parse_code_section` doc
comment), so every LEB128 immediate INSIDE a function body — a memory
instruction's `align`/`offset` memarg fields, and the `0xFC`
(saturating-conversion/bulk-memory)/`0xFD` (SIMD) prefix's own
sub-opcode — is decoded here, in `type_check.rs`, while walking
instructions for real type-checking. All of them went through the
native-64-bit-budget `decode_unsigned` instead of a width-bounded
`decode_unsigned_bounded`, so a 6+-byte (over the real `u32` budget) or
high-bit-set (out-of-range for 32 bits) encoding of a small, otherwise
legal value parsed successfully instead of being rejected — the exact
same "as u32 truncates silently"/"no byte-count cap" bug class
`wasm-module-parser`'s own `read_u32leb` closed for section-level fields
in the prior `wasm-leb128` 0.2.0 hardening pass, just never propagated to
this crate's instruction-level decode sites.

Fixed at every memarg site in this file: plain memory ops (`0x28..=
0x3E`), atomics (`0xFE`), and all five `v128`-prefixed memarg
instructions (`0xFD`'s `Load`/`Store`/`*Lane` families) — not just the
two opcodes the failing tests happen to exercise, since it's the exact
same decode call copy-pasted across all of them. Also fixed the `0xFC`
and `0xFD` prefix's own sub-opcode reads (`binary-leb128.wast`'s
"i64_trunc_sat_f64_u with 6 bytes" case) the same way.

`align` is unconditionally `u32` per spec, so it's now always
`decode_unsigned_bounded(.., 32)`. `offset` is genuinely
context-dependent and got this wrong in BOTH directions before landing
on the fix below:

- **First attempt bounded `offset` to 32 bits unconditionally**, on the
  theory that the previous W25 comment's claim ("offset is `u64`
  unconditionally in the real spec's binary grammar, verified live
  against the spec page") was checking the spec's CURRENT
  (post-memory64-merge) text rather than the pinned testsuite commit
  this repo's `wasm-conformance` corpus is actually graded against
  (`28864811cf03bdbf880733786148feaba339582d`) — and `binary-leb128.wast`
  itself supports that: it asserts a 6-byte offset encoding of the value
  `2`, on a plain 32-bit memory, IS malformed ("integer representation
  too long"), which only holds under a 5-byte/32-bit budget.
- **That first attempt regressed `binary_leb128_64.wast`** (caught by
  the mandatory full-corpus baseline diff, not inspection): its own
  non-`assert_malformed` `module` directive uses a 10-byte offset
  encoding of `2^64 - 1` on an `is64` (memory64) memory and expects it to
  parse fine, while `2^64` itself (one bit further) is its own
  `assert_malformed` case. So `offset` really is `u64`-wide, but ONLY
  when the addressed memory is actually `is64` — not unconditionally
  either way.
- **Final fix**: `align` decoded first (always 32-bit-bounded), which
  also yields the multi-memory flag bit (`has_memidx`) before `offset` is
  decoded. When `!has_memidx`, the target memory is implicitly memory 0,
  known immediately — decode `offset` at 64 bits if memory 0 is `is64`,
  else 32. When `has_memidx` IS set, the real target memory index isn't
  decoded until AFTER `offset` in this binary format, so which memory
  `offset` even addresses is still unknown at decode time — falls back
  to the full native 64-bit budget rather than guess, which is exactly
  this crate's previous (correct, un-regressed) behavior for that
  combination. No corpus file exercises a multi-memory memarg with an
  offset value a 32-bit budget would have rejected, so that fallback is
  a pure no-op today, not a knowingly-loose carve-out.

### Fixed — `memory.init`/`data.drop` without a data count section (2 of the 9)

`binary.wast`'s other two failures ("memory.init requires a data count
section", "data.drop requires a data count section") are a completely
unrelated bug: the spec requires a data count section (§12) whenever
`memory.init`/`data.drop` (`0xFC 0x08`/`0xFC 0x09`) appears anywhere in
the code section, independent of whether the referenced data segment
actually exists — and nothing enforced that at all. `wasm-module-parser`
already parses §12 but previously discarded whether it was PRESENT once
the (separate, pre-existing) count-vs-`module.data.len()` cross-check was
done; it now threads that fact forward as `WasmModule::
missing_data_count_section` (new field, see `wasm-types`'s changelog),
deliberately phrased as a default-`false` "missing" flag so every
existing `WasmModule` construction site (including every TEXT-form
module — no binary "data count section" concept even exists there) keeps
its current behavior for free. This crate's `0x08`/`0x09` opcode arms
now check that flag FIRST, before their existing out-of-bounds
data-segment-index checks, so it fires even for an otherwise-valid
`data_idx`.

### Added

- `wasm-module-parser`, `binary-leb128.wast`/`binary_leb128_64.wast`-
  inspired regression tests (`src/lib.rs`): `missing_data_count_section`
  set/unset correctly.
- This crate's own regression tests (`src/lib.rs`): memarg align overlong
  + out-of-range rejected (with a minimal-encoding positive control),
  memarg offset overlong on a 32-bit memory rejected, memarg offset
  widening to 64 bits on an `is64` memory (both the `2^64 - 1`-parses and
  `2^64`-rejected directions, directly from `binary_leb128_64.wast`),
  `0xFC` sub-opcode overlong rejected (with a minimal-encoding positive
  control), and `memory.init`/`data.drop` with/without
  `missing_data_count_section` set.

### Verification

Full `wasm-conformance` baseline diff (all 257 vendored files, via
`--write-baseline` + a programmatic before/after diff of `tests/fixtures/
testsuite-status.json`'s `files` map): exactly 2 files changed —
`binary-leb128.wast` (`assert_malformed` 51/58 with 7 failing -> 58/58)
and `binary.wast` (`assert_malformed` 105/107 with 2 failing -> 107/107).
Zero regressions anywhere else, including `binary_leb128_64.wast` (which
DID regress — `module` 1/1 -> 0/1 — during the first, unconditionally-
32-bit `offset` attempt described above, caught by this same diff before
it shipped, and is back to 1/1 pass / 0 fail with the final context-aware
fix). `cargo test`/`cargo clippy -- -D warnings` clean across
`wasm-module-parser`, `wasm-wast-parser`, `wasm-execution`,
`wasm-runtime`, `wasm-validator`, `wasm-conformance`, `wasm-types`,
`wasm-leb128`.

## [0.2.83] - 2026-09-01 (fix: `br_table.wast` total-failure regression — two independent bugs)

While prioritizing the vendored testsuite corpus for a fresh pass, found
that `br_table.wast` — a foundational MVP-level control-flow file, no
GC-proposal syntax involved — was TOTALLY failing: `module 0/1`,
`assert_return 0/161 fail`, all cascading from the one module failing to
validate. The baseline-diff mechanism only detects CHANGES from a prior
baseline, not absolute correctness, so this had been silently broken and
un-investigated for a while. A probe (`run_wast_source` against the raw
file) reported: `ValidationError: TypeMismatch: expected
ConcreteFuncRef(1), found Funcref` on `(table $t (ref null $t) (elem
$tf))` (line ~1019) — about as ordinary a construct as the corpus has,
no struct/array GC syntax anywhere near it. The initial hypothesis (an
`is_assignable` direction swap) turned out to be wrong; root-causing it
properly surfaced TWO independent, real bugs that both had to be fixed
before the file passed:

- **Bug 1 — `table.get`/`table.set` couldn't see a table's CONCRETE
  element type.** `table_element_types` used to be a `Vec<u8>` (just the
  raw `0x70`/`0x6F` tag), with every opcode arm doing `0x6F => Externref,
  _ => Funcref` — so a table declared `(ref null $t)` looked identical to
  a plain generic-`funcref` table to `table.get`/`table.set`/
  `table.grow`/`table.fill`. `table.get $t` pushed generic `Funcref`
  unconditionally, discarding the real declared type. Fixed by changing
  `table_element_types` to `Vec<ValueType>` (see `wasm-types`'s CHANGELOG
  for the new `WasmModule::table_concrete_element_types` field this reads
  from, falling back to the byte tag when the module doesn't populate
  it) — all four opcode arms (`table.get`/`table.set`/`table.grow`/
  `table.fill`) collapse to a single `ctx.table_element_types[idx]` read,
  no more inline byte match.
- **Bug 2 — `br_table`'s multi-target type check was ORDER-DEPENDENT,
  which the real WASM typing rule is not.** `br_table` requires that the
  SAME operand value(s) be simultaneously assignable to every listed
  target AND the default target — a "meet" over all of them. The old
  algorithm instead checked each target in LISTED order and re-pushed
  that target's OWN declared type before checking the next one (a
  left-to-right chain, not an independent check) — so checking a WIDER
  target (e.g. `(ref null func)`) before a NARROWER one (e.g. `(ref null
  $t)`) irreversibly widened the value away, and the narrower target's
  check then failed even though the actual value is genuinely assignable
  to both. `meet-funcref-1`'s label list `$l1 $l1 $l2` (wide targets
  before the narrow default) hits this exactly; `meet-funcref-2`'s `$l2
  $l2 $l1` (narrow first) happened to pass even on the old code, which is
  why this needed its own dedicated order-sensitive test rather than
  trusting one passing permutation. Fixed by popping the default target's
  own arity worth of values from the real stack exactly ONCE into a small
  `operands` vec, then checking every target (labels AND default) against
  that SAME fixed snapshot via a new `check_stacktype_assignable` helper
  (factored out of `pop_expect`) — no target's check can influence any
  other's, and the real `stack` is only ever touched by that one pop.
  **Security-hardened during review**: an earlier draft of this fix
  instead cloned the WHOLE operand stack once per target — correct, but
  `O(target_count * stack_depth)`, and `br_table`'s target count and the
  operand stack's depth are BOTH independently attacker-controlled within
  a single instruction, making that a real quadratic-blowup DoS vector.
  The `operands`-snapshot approach is `O(target_count * arity)` instead —
  the exact same asymptotic cost the original (order-dependent, but not
  DoS-prone) implementation already had, so this fix adds no new
  complexity-attack surface.
- Bug 2 alone would NOT have fixed `br_table.wast` (bug 1 had to land
  first so `table.get` had a concrete type to hand `br_table` in the
  first place) — confirmed by reverting each fix independently and
  re-running the new regression tests below; both fail with the original
  `TypeMismatch` when either fix alone is reverted.
- **2 new regression tests** in `tests/type_check.rs` (this crate's
  established `assert_valid`/`assert_invalid`-over-real-`.wat`-text
  convention): `valid_br_table_targets_type_check_regardless_of_listed_
  order` (bug 2, using `br_table.wast`'s own `meet-multi-ref` shape) and
  `valid_table_get_on_a_concrete_funcref_table_keeps_its_concrete_type`
  (bug 1). Also added
  `valid_generic_funcref_table_with_an_elem_segment_naming_a_concrete_
  function` — the literal "table declared generic `funcref`, elem segment
  names a concrete function" shape this session's regression report
  first hypothesized as the bug; it turns out this exact shape already
  validated correctly even before this fix (`lib.rs`'s own
  `valid_element_segment` unit test already covered it at the
  `WasmModule`-struct level), so this is a same-shape end-to-end
  confirmation from real text, not a new fix.
- **Full corpus impact**: baseline regenerated and diffed programmatically
  against the pre-fix `testsuite-status.json` across all 257 files — see
  `wasm-conformance`'s own CHANGELOG for the diff. Exactly one file's
  tally changed (`br_table.wast`, `module`/`assert_return` both going
  from total failure to 100%); every other file that also uses a
  `(table ... (ref ...) ...)`-shaped declaration (`table.wast`, `elem.wast`,
  `type-subtyping.wast`, `ref.wast`, etc.) already hit an EARLIER
  `NotYetSupported` gate for unrelated reasons and so was untouched by
  either fix — confirmed by inspecting each one's before/after entry, not
  assumed from the aggregate.

## [0.2.82] - 2026-09-01 (W34 third slice — wire canonical equivalence into within-module checks)

Canonicalization (built by the first two slices, consumed by nothing)
is now wired into real validation decisions.

- **`is_assignable` (`type_check.rs`)** gains six new arms —
  `StructRef`/`NonNullStructRef`/`ArrayRef`/`NonNullArrayRef`, in the
  same three-shape pattern (`(i,i)`, `(NonNull i, NonNull j)`, `(NonNull
  i, j)`) the three existing `ConcreteFuncRef` arms already used. This
  function had ZERO struct/array arms before this slice at all — a real,
  previously-open gap this spec's own research flagged. All nine arms
  (three func, six new) now terminate via a new `TypeContext::
  nominal_or_canonical_subtype` helper — nominal `sub`-chain OR canonical
  equivalence, whichever holds.
- **New `TypeContext<'a>` wrapper** — a `Copy`, `Deref<Target = WasmModule>`
  bundle of `&'a WasmModule` plus this module's own canonicalized
  type-group table, threaded through this file's ~150 existing internal
  call sites (`is_assignable`, `pop_expect`, `pop_expect_many`,
  `push_ctrl`, `pop_ctrl`, `results_assignable`, `func_is_structural_
  subtype`, `check_const_operand`, `check_const_expr_result`,
  `decode_blocktype`, `struct_field_count`, `array_element_field`,
  `struct_field`, `type_check_numeric`) with ZERO call-site syntax
  changes — `Deref` keeps every existing `module.<field/method>` access
  compiling unchanged, and `Copy` keeps every existing `ctx.module`
  argument-passing site compiling unchanged, since only the FIELD's type
  (`ModuleContext.module`) changed. `array_element_field`/`struct_field`
  reach through the wrapper's own `.module` field explicitly rather than
  via `Deref`, to avoid a real lifetime trap: a `Deref`-based auto-ref
  through a by-value wrapper parameter ties the returned reference's
  lifetime to the wrapper's own ephemeral borrow, not the wrapper's
  underlying `'a` — see that function's own doc comment.
- **`check_type_subtyping`'s structural checker is no longer func-only**
  — dispatches on each side's real `TypeKind` (`Func`/`Struct`/`Array`)
  instead of always reading `module.types[i]` (an unused dummy `FuncType`
  for a struct/array-kind index). New `field_is_structural_subtype`/
  `struct_is_structural_subtype`/`array_is_structural_subtype` implement
  the real GC-proposal rule: struct width subtyping (child may have MORE
  fields), per-field covariance-if-immutable/invariance-if-mutable, and a
  declared `sub` relationship between two DIFFERENT composite-type kinds
  is always rejected. Verified directly against `type-subtyping.wast`'s
  own "Definitions"/"Invalid subtyping definitions" sections (re-fetched
  fresh, not paraphrased) — every corpus case there now passes for the
  right reason. **Correction found mid-slice, not assumed**: this
  function's struct/array field-covariance checks turned out to need REAL
  canonical data too (not the empty table this slice first tried), since
  `type-subtyping.wast`'s own "Static matching of recursive types" module
  requires canonical equivalence between two separate, unrelated
  multi-member `rec` groups to satisfy a declared struct `sub`'s own field
  covariance — `check_type_subtyping_is_acyclic` was hoisted out of
  `check_type_subtyping` and into `type_check_module` so canonicalization
  can run BEFORE (not after) `check_type_subtyping`'s own checks.
- **`ValidatedModule::canonical_types()`** — a new public accessor
  exposing the whole per-flat-index canonical-type table as a slice (not
  just the existing single-index `canonical_type_at`), so `wasm-runtime`
  can clone it once for threading into `wasm-execution`'s runtime
  dispatch. `canonically_equivalent` now delegates to `wasm_types::
  canonical_types_equivalent` (the same helper `nominal_subtype_chain`
  uses) instead of its own inline match, so the two can never drift apart.
- **Two pre-existing tests' own premise deliberately overturned, rewritten
  not deleted**: `call_argument_rejects_an_unrelated_concrete_func_ref`
  and `invalid_global_ref_t_initialized_with_ref_func_of_an_unrelated_
  type_is_rejected` both used two byte-identical, `sub`-less types as
  their "unrelated" negative case — exactly the case canonical equivalence
  now correctly ACCEPTS. Both reshaped to use genuinely different types
  instead, with a new sibling test each proving the overturned case is now
  correctly accepted.
- **New tests**: struct/array positive+negative canonical-equivalence
  cases at a real `call` site (mirroring the func-type ones above);
  struct/array `sub`-declaration structural tests (width subtyping,
  covariant immutable fields, mutable-field invariance, cross-kind
  rejection) grounded directly in `type-subtyping.wast`'s own corpus text.

## [0.2.81] - 2026-09-01 (W34 second slice — real multi-member `rec`-group canonicalization)

No code changes to this crate's own logic — `ValidatedModule::canonical_
type_at`/`canonically_equivalent` and the `validate()` call site are
byte-for-byte unchanged, since `wasm_types::canonicalize_types`'s public
signature didn't change either. Bumped alongside `wasm-types` 0.1.19
because its own dependency now correctly canonicalizes real multi-member
`rec` groups (previously always `None`) — every existing caller through
this crate's `ValidatedModule` picks that up automatically. Two new tests
added exercising a real 2-member mutual `rec` group and a metadata-
inconsistent one through the real `validate()` entry point (see
`wasm-types`'s own CHANGELOG for the underlying algorithm change, and
`code/specs/W34-wasm-gc-canonical-type-equivalence.md`'s addendum for the
full accounting).

## [0.2.80] - 2026-09-01 (W34 first slice — canonical type-group equivalence caching)

`ValidatedModule` gains a private `canonical_types` field, computed by
`validate()` right after `check_type_subtyping_is_acyclic` (part of Check
11) confirms the module's `sub`/`rec` reference ordering is well-founded —
the ordering guarantee `wasm_types::canonicalize_types`'s own termination
argument depends on. Exposed via two new `ValidatedModule` methods:

- `canonical_type_at(idx) -> Option<(Rc<CanonicalGroup>, u32)>`
- `canonically_equivalent(i, j) -> bool` (conservatively `false` whenever
  either side isn't canonicalized yet — this slice only canonicalizes
  `rec_group_size == 1` groups, see `wasm-types`'s own CHANGELOG)

`ValidatedModule::module`'s existing privacy (the W33-era security fix: the
only way to construct a `ValidatedModule` at all is a successful
`validate()` call) extends for free to this new field too — there is no
code path that can produce a `canonical_types` value without going through
real validation first. Nothing in this slice wires canonical equivalence
into any validation DECISION yet (`is_assignable`, `check_type_subtyping`,
etc. are all unchanged) — that's a later slice's job, per `code/specs/
W34-wasm-gc-canonical-type-equivalence.md`'s own "Recommended slice
decomposition." Full workspace-adjacent conformance baseline (all 257
files) is byte-for-byte unchanged by this release.

## [0.2.79] - 2026-09-01 (W33 fourth slice — static struct/array instruction checks)

Adds real static type-checking for `struct.new_default`/`struct.get_s`/
`struct.get_u` and the whole `array.*` instruction family, previously
falling into the `0xFB` catch-all (immediates not consumed, byte-
desyncing every later instruction the moment `wasm-wast-parser` could
emit them).

- `struct_field_count`/`array_element_field`/`struct_field` resolve via
  `WasmModule::struct_type_at`/`array_type_at` (`type_kinds`-aware)
  rather than the legacy `types.len() + k` offset, needed now that a
  TEXT-format module can interleave struct/func/array declarations.
- `struct.set`/`array.set` now reject an immutable field/element
  (`struct.wast`'s/`array.wast`'s own "immutable field"/"immutable
  array" `assert_invalid` cases).
- `array.new_fixed`'s literal element-count immediate is capped at
  `MAX_ARRAY_NEW_FIXED_COUNT` (1,000,000) before the pop loop runs — an
  unbounded loop over an attacker-controlled count is a real
  algorithmic DoS even though no single iteration allocates memory.
- `const_expr_type` (the global-initializer/segment-offset checker) now
  accepts `struct.new`/`struct.new_default`/`array.new`/
  `array.new_default`/`array.new_fixed` as real constant instructions,
  matching the real GC proposal's own extension to constant
  expressions — `struct.wast`'s/`array.wast`'s own module-level globals
  use exactly this shape. `array.new_data`/`array.new_elem` are
  deliberately not accepted.

11 new integration tests in `tests/type_check.rs`. All 465 tests in
this crate pass.

## [0.2.78] - 2026-08-31 (W33 second slice — `ref.cast` byte-layout fix, item 4)

### Fixed

- **The static function-body type-checker never consumed `ref.cast`/
  `ref.cast null`'s (`0xFB 0x16`/`0x17`) LEB128 heap-type immediate** —
  they fell into the existing `_ => {}` "no immediates" catch-all for
  unrecognized `0xFB` sub-opcodes, which would silently desync this
  checker's `offset` from every REAL instruction after it in the same
  function body the moment `wasm-wast-parser` could emit the bytes (which
  it now can, see that crate's own changelog). Added an explicit `0x16 |
  0x17` arm mirroring `0x14`/`0x15`'s (`ref.test`/`ref.test null`)
  existing shape: consume the heap-type LEB, pop one ref, push
  `StackType::Unknown` back (real dynamic-type checking is a `wasm-
  execution` runtime concern — see that crate's own changelog — this pass
  only needs to keep the abstract stack's byte layout and height
  accurate, exactly like every other GC op this checker already handles
  this way).

## [0.2.77] - 2026-08-31 (const-expr type-checker — the W33 addendum's "third gap")

Fills a real, pre-existing, general gap this crate had for its entire
history and independently of W33's own `sub`/`final`/`rec` scope: **no
const-expression type-checker existed at all**. `crate::validate`'s
"Check 4c" bounds-checks a global's own DECLARED `ConcreteFuncRef`/
`NonNullConcreteFuncRef` type INDEX, but nothing ever compared what a
global's `init_expr` (or an element-/data-segment's `offset_expr`)
actually EVALUATES TO against that declared type — confirmed directly by
grepping this crate for any production read of `globals[..].init_expr`/
`elements[..].offset_expr`/`data[..].offset_expr` outside test fixtures:
there were none. Surfaced by `code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s "A newly-discovered, THIRD
gap" addendum while tracing two honest reclassifications in
`type-rec.wast`/`type-subtyping.wast`; this predates W33 entirely and
would affect even a plain MVP `(global i32 (i64.const 0))` mismatch.

### Added

- **`type_check::const_expr_type`**: determines the static result TYPE of
  a constant expression — the same opcode set `wasm_execution::
  evaluate_const_expr` interprets at runtime (`i32.const`/`i64.const`/
  `f32.const`/`f64.const`, `global.get`, the extended-const proposal's six
  arithmetic ops, `v128.const`, `ref.i31`, `ref.null <heap_type>`,
  `ref.func`) — without executing any arithmetic. Falls back to the exact
  same permissive `Unknown` the function-body checker's own `ref.null`
  `0x63`-tagged-concrete-index handler already uses for an out-of-range
  index, so this never introduces a false reject for a shape the rest of
  the crate already treats as "not fully typed."
- **`type_check::check_const_exprs`**: applies `const_expr_type` (checked
  via `is_assignable`, so it inherits every W32/W33 non-null/bottom/
  nominal-subtype rule already in this crate — not bare equality) to
  every global initializer, and every ACTIVE element-/data-segment offset
  expression (expected type `i64` for an `is64` table/memory per the
  table64/memory64 work, `i32` otherwise). Wired into `type_check_module`,
  so it runs for every module unconditionally.
- **`global.get` real spec rules, neither previously checked anywhere**:
  (1) the referenced global must be IMMUTABLE; (2) within a GLOBAL's own
  initializer, the reference must be to a STRICTLY EARLIER global in the
  combined (imports-first) index space — forward references, including
  self-reference, are invalid (element-/data-segment offsets have no such
  restriction: the entire global section is already fully declared by the
  time either runs).
- **`ref.func`'s funcidx**, previously entirely unchecked inside a
  const-expr (only bounds-checked inside a function body): now bounds-
  checked via `decode_unsigned_bounded(.., 32)`, the same truncation-safe
  decoder a prior security review already required for this exact opcode
  in `wasm_execution::evaluate_const_expr`'s own `0xD2` arm — reused here
  rather than the plain `decode_idx` this file's `ref.null` arm uses, so
  this new code doesn't reintroduce the identical silent-truncation bug
  class in a second place.
- **A constant expression must leave EXACTLY one value on the stack**:
  `global.wast`'s own `(global i32 (i32.const 0) (i32.const 0))` and
  `(global i32 (global.get 0) (global.get 0))` `assert_invalid "type
  mismatch"` cases (an extra value left over after `end` pops the first)
  are real corpus evidence this rule matters, not merely a theoretical
  nicety.

### Real corpus impact (`wasm-conformance` 0.1.107 regenerated the
### baseline; see that crate's own changelog for the full per-file
### accounting)

52 `assert_invalid` directives, across 7 files, flip `NotYetSupported`
→ `Pass` (zero new `fail`/`trap` anywhere, in any directive kind, in any
file): `data.wast` (+14), `elem.wast` (+14), `global.wast` (+17),
`func_ptrs.wast` (+3), `type-subtyping.wast` (+2), `type-rec.wast` (+1),
`ref_func.wast` (+1, an out-of-range `ref.func` funcidx inside a global
initializer). This is exactly the class of case `type-rec.wast`'s
`(global (ref $ft) (ref.func $f))` and `type-subtyping.wast`'s two
unrelated-`rec`-group variants of the same shape needed — both now
correctly rejected via real nominal-subtype checking, not bare equality.

### Added — unit tests

At minimum (see `tests/type_check.rs`'s new "Const-expr type-checking"
section): a valid `(global i32 (i32.const 0))`; an invalid `(global i32
(i64.const 0))`; a valid `(global (ref $t) (ref.func $f))` where `$f`'s
type is a real NOMINAL SUBTYPE of `$t` (not equal to it, so this
genuinely exercises `is_assignable`'s nominal-subtype arm, not bare
equality); an invalid version with an unrelated type; a `global.get`
against a valid prior immutable global (accept); a `global.get` against a
mutable global (reject) and against a later-declared global (reject,
real corpus shape); an element-/data-segment offset type mismatch
(reject); and `is64` table/memory offset variants (accept `i64`, reject
`i32`, and vice versa).

## [0.2.76] - 2026-08-31 (W33 first slice — GC nominal subtyping validation)

Implements the `wasm-validator` half of `code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s "first slice" scope: structural
CHECKING of the `sub`/`final` declarations `wasm-wast-parser` 0.1.89 now
parses, plus wiring the resulting nominal subtype relation into every
existing static assignability check (`call`, locals, globals, block
types, ...) via `is_assignable`. Does NOT implement cross-module canonical
type-group equivalence (item 3b) or dynamic `call_indirect`/`ref.cast`/
`ref.test` checks against real subtype relationships (item 4) — both
remain explicitly out of scope, see the spec's own addendum for this
slice's re-verification of exactly what's still blocked on each.

### Added

- **`check_type_subtyping`**: validates every `module.types` entry with a
  declared `sub $parent` (via `wasm-types` 0.1.15's `type_subtyping`) —
  called at the top of `type_check_module`, so it runs for every module
  regardless of whether any function body ever exercises the declared
  type. Two things reject a declaration:
  - **the parent is final** (`type-subtyping.wast`'s "Finality violation"
    section, lines 780-811) — checked for both the implicit default
    (no `sub` clause at all) and an explicit `(sub final ...)`.
  - **the declared child/parent pair fails `func_is_structural_subtype`**
    (below) — e.g. an arity mismatch (`type-subtyping.wast` lines
    944-949) or a param/result variance violation.
  - Struct/array composite-type-kind invariance and field-list
    width/depth/variance rules (the rest of `type-subtyping.wast`'s
    "Invalid subtyping definitions" section) are NOT checked — this
    slice's parser only ever produces `FuncType` entries (struct/array
    TEXT-format bodies remain unparseable), so there's nothing of that
    shape to validate yet; those cases stay correctly rejected for the
    pre-existing "struct/array not yet supported" parse-time reason
    instead, an honest `NotYetSupported`-shaped non-regression.
- **`func_is_structural_subtype`**: the GC proposal's REAL function-type
  subtyping rule — invariant arity, contravariant params, covariant
  results — independently verified (per the spec's own "Why this needs
  GC, not function-references" section) against the narrower
  function-references `Overview.md` rule ("invariant for now") to confirm
  this file needs the wider GC rule, not that one.
- **`is_assignable` gained three new arms** (and, with it, `pop_expect`/
  `pop_expect_many`/`results_assignable`/`push_ctrl`/`pop_ctrl`/
  `type_check_numeric` all now take a `module: &WasmModule` parameter to
  thread it through): `ConcreteFuncRef(i)`/`NonNullConcreteFuncRef(i)`
  flowing into a `ConcreteFuncRef(j)`/`NonNullConcreteFuncRef(j)` slot
  where `i` is a declared NOMINAL subtype of `j` (`WasmModule::
  func_type_is_nominal_subtype`). This is what makes `call`'s existing
  argument-type check (and every other static assignability check) honor
  a declared `sub` chain — e.g. `type-subtyping.wast`'s "Subsumption"
  section's `$f2 (param (ref $t2)) (call $f1 (local.get $r))` validates
  because `$t2 <: $t1` per the module's own chain, not mere structural
  equality (every arm predates this slice and never looks at `module` at
  all, so a `WasmModule` that never populated `type_subtyping` behaves
  exactly as before). Deliberately does NOT attempt canonical/structural
  equivalence between two independently-declared, `sub`-unrelated types
  even if byte-identical in shape (item 3b, out of scope).

Full-corpus baseline diff (`wasm-conformance`) confirms zero regressions
— see that crate's own changelog entry for the exact before/after
tallies across every affected file.

### Security review follow-up

- **`check_type_subtyping` now rejects a cyclic `sub` chain** (new
  `check_type_subtyping_is_acyclic`, an O(number of types) three-color
  traversal). A security review found that `(rec (type $t1 (sub $t2
  (func))) (type $t2 (sub $t1 (func))))` used to validate successfully:
  each type's own IMMEDIATE parent link checks out fine in isolation
  (invariant arity trivially satisfied both directions on empty func
  shapes), so nothing rejected the cycle as a whole — confirmed directly
  that this made `WasmModule::func_type_is_nominal_subtype(0, 1)` AND
  `(1, 0)` both return `true`, i.e. two independently-declared,
  differently-indexed types became mutually interchangeable via
  `is_assignable`. That is exactly the "canonical equivalence between
  unrelated types" this crate's own `is_assignable` doc comment says must
  stay unimplemented (W33's own item 3b) — a wrong ACCEPT here is a real
  soundness risk, not just a missed capability. Confirmed via the full
  corpus re-run: zero real `.wast` files declare a cyclic `sub` chain, so
  this fix causes no baseline changes.

wasm-validator 0.2.76 (this security-review follow-up landed within the
same unreleased version as the fix it patches, not a separate bump).

## [0.2.75] - 2026-08-31 (W32 second slice — non-null concrete reference types)

### Added

- `is_assignable` now also checks `wasm_types::ValueType::
  is_non_null_subtype_of` — `NonNullStructRef(i) <: StructRef(i) <:
  Anyref` and `NonNullConcreteFuncRef(i) <: ConcreteFuncRef(i) <:
  Funcref`, never the reverse. Per `code/specs/
  W32-wasm-non-null-concrete-reference-types.md`'s addendum section 2.
- `call_ref`/`return_call_ref` (`0x14`/`0x15`, function-references
  proposal) are now real, type-checked opcodes: `call_ref $t : [t1*
  (ref null $t)] -> [t2*]`, traps on null (independently verified
  against WebAssembly/function-references's own `Overview.md` — NOT
  restricted to a non-null-only operand the way this package's own W32
  spec document first assumed before this slice checked);
  `return_call_ref` adds the same tail-call result-assignability +
  dead-code-after rule `return_call`/`return_call_indirect` already have.
- `ref.func`'s type-check now reflects the real spec rule: `ref.func $f :
  [] -> [(ref $t)]` where `$t` is `$f`'s own function-type index (real
  spec text verified directly), a genuinely more precise static type than
  the pre-existing placeholder of pushing bare `Funcref` for every
  `ref.func` — every pre-existing use where a plain `funcref`-typed slot
  was expected keeps validating (`NonNullConcreteFuncRef(i) <:
  ConcreteFuncRef(i) <: Funcref`), this is strictly more information, not
  a behavior change for anything that only ever checked assignability.
  Needed a new `ModuleContext::func_type_indices` (parallel to
  `func_types`, same combined index space) since the resolved `FuncType`
  alone can't recover which type-SECTION index a function declared.
- `out_of_range_concrete_func_ref`'s bounds check now also covers
  `NonNullConcreteFuncRef` (real corpus regression found and fixed, not
  hypothetical: `ref.wast`'s own `type-func-param-invalid`/`func-param-
  invalid`/`func-local-invalid`/etc. `assert_invalid` cases, which
  `wasm-wast-parser`'s new `(ref $t)` parsing made newly reachable).
- `type_check::decode_blocktype` and `wasm-execution`'s matching
  blocktype decoders gained explicit arms for `0x63`/`0x64` (nullable/
  non-null concrete-ref single-value blocktype results) — the same
  defensive treatment the four W32-first-slice bottom types and `exnref`
  already have: both are plausible real type-section indices, and both
  carry a trailing LEB128 index the generic fallback doesn't know to skip.
- Untyped `select` (`0x1B`) now rejects a reference-typed operand pair
  (real corpus regression found and fixed: `select.wast`'s own `type-ref-
  implicit`/`type-funcref-implicit`/`type-externref-implicit` cases) —
  the real spec restricts the untyped form to `numtype`/`vectype`
  operands only; a reference-typed pair needs the explicit `(result t)`
  form, which this crate does not implement.

### Fixed

- (See "Added" above — every item is a real regression `(ref $t)`'s new
  parsing surface exposed, found via a full-corpus diff against the
  pre-change baseline, not a hypothetical gap.)

### Scope note

This is the **second slice** of W32: non-null concrete-ref representation,
its direct subtyping rules, and the minimum instruction/opcode support
needed to make it soundly checkable (`call_ref`/`return_call_ref`,
`ref.func`'s real result type, blocktype/bounds-check parity, `select`'s
reference-type restriction). Still explicitly open: structural subtyping
for `call_indirect`/`ref.cast` (`type-subtyping.wast`'s real remaining
gap), real recursive type groups' own forward-reference/nominal-identity
rules (`type-rec.wast`), per-local definite-initialization tracking for
non-defaultable non-null locals (`func.wast`'s `uninitialized local`
case), and `try_table`'s own catch-clause payload-type checking against
its destination label — each confirmed, not guessed, via the same
full-corpus diff. See this package's own addendum to the spec document.

## [0.2.74] - 2026-08-31 (W32 first slice — bottom reference-type subtyping)

### Added

- `is_assignable`'s reference-type subtyping now also checks
  `wasm_types::ValueType::is_bottom_subtype_of` — the four bottom
  reference types (`NullFuncref`/`NullExternref`/`NullExnref`/`NullRef`)
  are accepted wherever their nullable hierarchy supertype (or, for
  `NullFuncref`/`NullRef`, a specific concrete/struct index) is expected,
  never the reverse. Per `code/specs/
  W32-wasm-non-null-concrete-reference-types.md` section 2.
- `type_check::decode_blocktype` and `wasm-execution`'s matching
  blocktype decoders gained explicit arms for the four new tag bytes
  (`0x71`/`0x72`/`0x73`/`0x74`), the same defensive treatment `exnref`'s
  `0x69` already has — each byte is a plausible real type-section index
  for a large module, so it must be special-cased rather than falling
  into the generic signed-LEB128 type-index branch (the exact class of
  bug W24's `exnref` fix closed).
- `type_check`'s `ref.null` (`0xD0`) handler now recognizes the four new
  heap-type immediate bytes and pushes the genuine bottom-type
  `ValueType` instead of falling back to `Unknown`.

### Tests

- New `assert_valid`/`assert_invalid` pairs in `tests/type_check.rs`
  covering every bottom-type subtyping direction from the spec's section
  2 (positive: accepted; negative: the reverse direction, and
  cross-hierarchy assignment, rejected) — see the spec's own
  "Verification plan".
- `NullRef <: StructRef(_)` (unreachable via this crate's text-format
  parser — no struct-type text-format declarations exist) is covered by
  a directly-constructed `WasmModule` test in `src/lib.rs`, matching the
  existing `ConcreteFuncRef` bounds-test pattern.

## [0.2.73] - 2026-08-31 (W30 follow-up — real memory64 bulk operations)

### Added

- `memory.copy`/`memory.fill`/`memory.init`'s type-check rules now pick
  `I64` vs. `I32` for their dest/src/len operand(s) per the relevant
  memory's own `is64`, reading the already-existing per-memory
  `memory_is64: Vec<bool>` (W25) the same way `table_is64` (W26) is
  already read for the analogous table ops — previously all three
  unconditionally required `I32`, rejecting a real `is64` memory's own
  correctly-`i64`-typed operands.
- `memory.copy`'s `dest`/`src` operands type-check independently against
  the DESTINATION/SOURCE memory's own `is64`; `len` type-checks as `I64`
  ONLY when BOTH memories are `is64` — otherwise `I32`, mirroring
  `table.copy`'s identical mixed-index-width rule (W26).
- `memory.init`'s `dest` operand type-checks per the TARGET memory's own
  `is64`; `src`/`len` (positions within the passive data segment) always
  stay `I32`.

### Tests

- 9 new `assert_valid`/`assert_invalid` cases covering `memory.fill`/
  `memory.copy`/`memory.init` against a real `is64` memory, plus a
  binary-encoded (`module_with_body`) mixed is64/is32 `memory.copy` case
  — the text form can't reach it: `wasm-wast-parser`'s `memory.copy`
  encoder has no leading-memidx-token support at all (unlike
  `memory.fill`/`memory.init`), always emitting the implicit `memidx=0`
  pair, so a genuinely two-memory `memory.copy` can only be constructed
  at the binary level today. No real vendored corpus file needs the text
  form either (every `*64.wast` file declares a single memory), so this
  is left as a pre-existing, unrelated gap rather than fixed here.

## [0.2.72] - 2026-08-26 (W26 follow-up — real table64 operations)

### Added

- New `ModuleContext::table_is64: Vec<bool>` field (combined
  imports-first-then-declared index space, mirroring the existing
  `table_element_types`/`memory_is64`), populated from each table's own
  `TableType::is64`.
- `table.get`/`table.set`/`table.grow`/`table.size`/`table.fill`/
  `table.init`/`table.copy`/`call_indirect`/`return_call_indirect`'s
  type-check rules now pick `I64` vs. `I32` for their index/dest/src/len/
  delta operand(s) (and, for `table.grow`/`table.size`, the pushed
  result) per the TARGET table's own `is64` — previously every one
  unconditionally required `I32`, rejecting a real `is64` table's own
  correctly-`i64`-typed operands.
- `table.copy`'s `len` operand type-checks as `I64` ONLY when BOTH the
  source and destination tables are `is64` — otherwise `I32`, even when
  exactly one side is `is64` (verified against the real
  `table_copy_mixed.wast` corpus; see `code/specs/
  W26-wasm-table64-first-slice.md`'s addendum for the full rule and why
  an initial "always matches destination" draft of this rule was wrong).
- `table.init`'s `dest` operand type-checks per the TARGET table's own
  `is64`; `src`/`len` (positions within the passive element segment)
  always stay `I32`.

### Tests

- 16 new `assert_valid`/`assert_invalid` cases covering every affected
  instruction against a real `is64` table, including the exact mixed
  is64/is32 `table.copy` scenarios `table_copy_mixed.wast`'s own
  `assert_invalid` cases exercise (wrong `len` width, wrong `src` table's
  width, wrong `dest` table's width, checked independently).

## [0.2.71] - 2026-08-26 (W27 — census batch: real multi-memory data segments)

### Fixed

- **Check 8 (data segments) widened from "must target memory 0" to a
  real bounds check against the actual memory count.** The old check
  rejected ANY active data segment whose `memory_index` wasn't 0 —
  deliberate at the time (`wasm-runtime::instantiate` only ever applied
  a segment to memory 0 regardless of its declared index), but a real
  scope boundary this batch's `wasm-runtime` fix (see that crate's own
  CHANGELOG) removes. Now: `seg.memory_index >= total_memories` is the
  only rejection condition, matching every other multi-memory bounds
  check in this file (`memory.init`/`memory.fill`/etc. in
  `type_check.rs`).
- **A real, independently-found bug in the same check: a PASSIVE data
  segment was wrongly required to reference a real memory.** The old
  code's `total_memories == 0 && !module.data.is_empty()` branch never
  looked at `seg.is_passive` at all, so a passive-only module declaring
  zero memories (the real corpus's own `token.wast`, `(data $l "a")`
  with no `(memory ...)` anywhere) was incorrectly rejected at
  validation time. Passive segments are now skipped entirely by this
  check — they carry no real memory reference (`seg.memory_index` is
  kept `0`/unset by convention), and are only ever consumed by an
  explicit `memory.init`, possibly into a DIFFERENT module's memory
  entirely.
- Real corpus impact (`wasm-conformance`'s own CHANGELOG has the full
  per-file accounting): unblocks `address0.wast`/`address1.wast`/
  `binary0.wast`/`data_drop0.wast`/`float_exprs1.wast`/
  `float_memory0.wast`/`imports2.wast`/`linking2.wast`/`load0.wast`/
  `memory_trap1.wast`/`start0.wast`/`store2.wast`/`token.wast`, and
  incidentally fixed one of the ALREADY-vendored `linking.wast`'s own
  pre-existing `assert_unlinkable` fails (49/50 → 50/50) as a side
  effect — confirmed via a full before/after baseline diff that no
  other already-vendored file's tally changed at all.

### Added

- Two new tests: `valid_data_segment_targets_non_zero_memory_in_multi_
  memory_module` (the widened acceptance case) and `valid_passive_data_
  segment_in_module_with_no_memory` (the passive-segment bug fix).
  `rejects_data_segment_bad_memory_index`'s existing case (now genuinely
  OUT of bounds, not just "non-zero") still rejects, unchanged.

## [0.2.70] - 2026-08-26 (W11 addendum — concrete function-type ref subtyping)

### Added

- **`is_assignable(actual, expected)`**: the one reference-type subtyping
  rule this crate implements — a nullable ref to a specific concrete
  function type (`ValueType::ConcreteFuncRef`) is assignable wherever the
  general `funcref` is expected, never the reverse. `pop_expect` (the
  single choke point every type check in this module already goes
  through) now calls this instead of bare `==`, so `return`/block-end/
  branch-target checks all pick it up automatically.
- **`results_assignable(callee_results, function_results)`**:
  `return_call`/`return_call_indirect`'s own special-cased "callee results
  must match the current function's declared results" check now allows
  this same one-directional subtyping per result position (previously a
  strict `Vec` equality check), covering the real corpus's
  `return_call.wast`/`return_call_indirect.wast` "Result subtyping" tests
  (both the valid and the mirror-image `assert_invalid` direction).
- `0xD0` (`ref.null`)'s heap-type-immediate decoder gained a `0x63` arm:
  decodes the trailing `LEB128` type-section index and, when it's `<
  ctx.module.types.len()`, pushes a real `StackType::Known(ValueType::
  ConcreteFuncRef(idx))` instead of the polymorphic `Unknown` every other
  unrecognized heap-type byte still gets. An index `>= types.len()` is
  left as `Unknown` too (not hard-rejected) — a security-review round
  found that `0x63` is ALSO `StructRef`'s own tag byte, whose index lives
  in a different, offset space (`types.len() + k`); erroring on every
  `>= types.len()` index would incorrectly reject a legitimate struct-type
  `ref.null` the moment this crate's text-format parser grows struct-type
  declarations, not just an actually-malformed one.
- **`validate()` (Check 4c, new)**: a bare NUMERIC `(ref null N)`/
  `ref.null N` has no declaration-time guarantee its index is in range
  (unlike a `$name` reference, always assigned an in-range index at
  declaration time) — `wasm-wast-parser` will happily produce a
  `ConcreteFuncRef` with an out-of-range index, so structural validation
  now scans every function signature (`module.types`), global type
  (declared and imported), and function-body local for one and rejects
  it with `TypeIndexOutOfBounds`, mirroring the existing Check 4 (a
  function's own declared type index) for the exact same failure mode.
  Found in the same security-review round as the `0x63` fix above.

## [0.2.69] - 2026-08-26 (W26 — table64 proposal, first slice)

### Changed

- **Check 2b (table limits ≤ `MAX_TABLE_ELEMENTS`) is now `is64`-aware**:
  unchanged for `is64: false` tables (same implementation-defined
  `MAX_TABLE_ELEMENTS` validation-time cap as before — no 32-bit table's
  validation outcome changes); an `is64` table (`TableType::is64`,
  `wasm-types` 0.1.11) is checked against the REAL spec ceiling instead
  (`u64::MAX`, verified live against the reference interpreter's
  `check_tabletype` — table64's ceiling is NOT the same `2^48`-page bound
  memory64 uses), which no `u64` value can ever exceed. `is64` tables are
  also excluded from the 32-bit `total_table_elements` aggregate, mirroring
  Check 1b's own `is64` memory exclusion (W25).
- Table-import linking (`wasm-runtime`) gains an `is64` mismatch check,
  mirroring the existing memory-import one exactly.

See `code/specs/W26-wasm-table64-first-slice.md`.

## [0.2.68] - 2026-08-26 (W25 — memory64 proposal, first slice)

### Added

- `ModuleContext.memory_is64: Vec<bool>` — each memory's `is64`-ness
  (memory64 proposal), same combined imports-first-then-declared
  index-space ordering as `table_element_types`.
- Load/store (`0x28..=0x3E`), `memory.size` (`0x3F`), and `memory.grow`
  (`0x40`) now require an `I64` address/delta/result instead of `I32`
  when the TARGET memory (its real `memidx`) is `is64`.
- **New**: `min <= max` is now actually checked for both memories and
  tables — a real, pre-existing gap this repo never checked before (found
  chasing `memory64.wast`'s own `"size minimum must not be greater than
  maximum"` `assert_invalid` case, which nothing rejected for ANY memory,
  32- or 64-bit, or any table, before this slice).
- Check 1b (the 65536-page spec-ceiling + DoS-aggregate check) is now
  `is64`-aware: a 64-bit memory's own spec ceiling is `2^48` pages (not
  `2^16`) — verified against `memory64.wast`'s own real `assert_invalid`
  boundary. The DoS-motivated AGGREGATE aspect stays 32-bit-only
  (validation never allocates, so a large-but-spec-valid `is64`
  declaration is correctly accepted here; the practical allocation risk
  is handled separately, at instantiation — see `wasm-runtime`'s own
  changelog).

See `code/specs/W25-wasm-memory64-first-slice.md`.

## [0.2.67] - 2026-08-26 (Exceptions proposal, fourth slice W24: throw_ref + catch_ref/catch_all_ref arity)

### Added

- `throw_ref` (`0x0A`) type rule: pops one `Exnref`, then marks the rest
  of the block unreachable — the same "pop one operand, dead code after"
  shape `throw`/`unreachable`/`br`/`return` already use.
- `try_table` (`0x1F`)'s `catch_ref`/`catch_all_ref` clauses now get a
  real arity/type check: the target label's declared type must equal
  EXACTLY the tag's params (`catch_ref`) or nothing (`catch_all_ref`)
  followed by `Exnref` — catching `try_table.wast`'s own real
  `assert_invalid` cases (a target label that doesn't account for the
  `exnref` a matching clause now genuinely pushes). Plain `catch`/
  `catch_all` are deliberately left unchanged (still no arity check,
  W21/W22's own scope) — no regression risk to already-passing
  directives.

### Fixed

- `decode_blocktype`: `0x69` (`exnref`) recognized as a single-value
  shorthand blocktype byte — the same real gap `0x7B`/`0x70`/`0x6F`
  (v128/funcref/externref) closed once already, now hit by a real corpus
  shape (`throw_ref.wast`'s `(block $h (result exnref) ...)`). Previously
  fell through to the signed-LEB128 type-index branch and misread
  trailing bytes as a bogus type index.
- **Security review finding**: this arm was originally keyed on `0xE9`
  (matching `wasm-types`' then-incorrect `ValueType::Exnref` wire byte),
  which has its LEB128 continuation bit SET — indistinguishable from the
  leading byte of a genuine multi-byte type index, so a module declaring
  234+ types could trigger a silent blocktype misparse. Fixed at the
  source: `wasm-types` 0.1.9 corrects the wire byte to `0x69` (continuation
  bit clear, spec-correct SLEB128 encoding of `-0x17`), and this arm
  updated to match.

See `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`.

## [0.2.66] - 2026-08-25 (Exceptions proposal, first slice W21: tag/throw/try_table type rules)

### Added

- `throw` (`0x08`) type-check rule: pops the named tag's declared param
  types (an out-of-bounds tag index is a hard validation error, "unknown
  tag" -- `throw.wast`'s own case), then marks the rest of the current
  block unreachable, same shape `unreachable`/`br`/`return` already use.
- `try_table` (`0x1F`) type-check rule: decodes exactly like `block`
  (same blocktype immediate, same `push_ctrl(..., FrameKind::Block, ...)`
  -- no new `FrameKind` needed), plus real (if narrow) validation of its
  own catch-clause list: each tag index (`catch`/`catch_ref`) must be in
  bounds, each label index must resolve to a real enclosing block (same
  `resolve_label_target` every branch instruction already uses).
  Catch-target type-arity matching (tag params vs. label's own declared
  types) is explicitly NOT checked -- no directive in this slice's corpus
  needs it.
- `crate::validate`: tag import/module-defined-tag type-index bounds
  checks, the "tag's function-type `results` must be empty" rule
  (`tag.wast`'s own "non-empty tag result type" cases), and an
  `ExternalKind::Tag` arm for export-index bounds checking.
- `ModuleContext.tag_types: Vec<FuncType>` -- combined imported +
  module-defined tag types, same index-space convention as `func_types`.

See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.

## [0.2.65] - 2026-08-25 (GC epic, first slice W20: i31.get_u type rule)

### Added

- `i31.get_u` (WasmGC `0xFB` sub-opcode `0x1E`, new): joins the existing
  `i31.get_s` (`0x1D`) type-rule arm -- pop a value, push `I32`. The
  unsigned-vs-signed distinction is purely a runtime concern, invisible
  to the type checker.
- Renamed the `0x1C` case's doc comment from `i31.new` to `ref.i31` (the
  real spec instruction name; this crate's own comment had drifted from
  it) -- no behavior change.
- Unit tests (in `tests/type_check.rs`, built via `wasm-wast-parser`'s
  new text syntax): `i31.get_u`/`i31.get_s` validate on a fresh
  `ref.i31`, on `(ref.null i31)` (statically valid -- the null trap is a
  runtime concern), `(ref i31)`/`(ref null i31)`/`i31ref` value-type
  syntax in params/results/locals/globals (including a `(global (ref
  i31) (ref.i31 ...))` const-expr end to end), and an empty-stack
  rejection.

## [0.2.64] - 2026-08-25 (Relaxed SIMD epic PR6: i16x8/i32x4.relaxed_dot_i8x16_i7x16_s/_add_s)

### Added

- `SimdOpKind::RelaxedDotI8x16I7x16S` joins the existing BINARY `(v128,
  v128) -> v128` type-rule arm (`DotI16x8S` etc.) -- pop two `V128`s,
  push one `V128`. The narrower `i8x16` input width and the "signed *
  signed" semantic choice for its `i7x16`-named operand are both
  runtime concerns, invisible to the type checker.
- `SimdOpKind::RelaxedDotI8x16I7x16AddS` joins the existing TERNARY
  `(v128, v128, v128) -> v128` type-rule arm (`Bitselect`/
  `RelaxedMaddF32x4` etc.) -- pop three `V128`s, push one `V128`. The
  FIRST ternary op in this arm whose third operand is a genuine numeric
  accumulator rather than a bitwise mask or a second fused-arithmetic
  input is, like every other numeric distinction in this match, entirely
  invisible at the type level.

## [0.2.63] - 2026-08-25 (Relaxed SIMD epic PR5: f32x4/f64x2.relaxed_madd/relaxed_nmadd)

### Added

- `SimdOpKind::RelaxedMaddF32x4`/`RelaxedNmaddF32x4`/`RelaxedMaddF64x2`/
  `RelaxedNmaddF64x2` now share `Bitselect`'s/`RelaxedLaneselectI8x16`'s
  existing TERNARY type-check arm in `type_check.rs`: pop three
  `v128`s, push one `v128`. The fact that this family's runtime body is
  fused-multiply-add floating-point arithmetic rather than a bitwise
  blend is entirely a runtime concern, invisible to the type checker.
- New tests: `valid_relaxed_madd_nmadd_pops_three_v128_pushes_v128` and
  `invalid_relaxed_madd_nmadd_given_an_i32_operand_instead_of_v128`,
  mirroring the existing `relaxed_laneselect` coverage for all 4 new
  opcodes.

## [0.2.62] - 2026-08-25 (Relaxed SIMD epic PR4: i8x16/i16x8/i32x4/i64x2.relaxed_laneselect)

### Added

- `SimdOpKind::RelaxedLaneselectI8x16`/`RelaxedLaneselectI16x8`/
  `RelaxedLaneselectI32x4`/`RelaxedLaneselectI64x2` now share
  `Bitselect`'s existing TERNARY type-check arm in `type_check.rs`:
  pop three `v128`s, push one `v128`. The implementation-defined-vs-
  bitselect distinction the relaxed-simd spec draws is entirely a
  runtime concern, invisible to the type checker.
- New tests: `valid_relaxed_laneselect_pops_three_v128_pushes_v128` and
  `invalid_relaxed_laneselect_given_an_i32_operand_instead_of_v128`,
  mirroring the existing `v128.bitselect` coverage for all 4 new
  opcodes.

## [0.2.61] - 2026-08-25 (Relaxed SIMD epic PR3: f32x4/f64x2 relaxed_min/relaxed_max)

### Added

- Type-check coverage for `f32x4.relaxed_min`/`relaxed_max` and
  `f64x2.relaxed_min`/`relaxed_max` (`SimdOpKind::RelaxedMinF32x4`/
  `RelaxedMaxF32x4`/`RelaxedMinF64x2`/`RelaxedMaxF64x2`): join the
  existing shared `(v128, v128) -> v128` binary-shape arm right
  alongside `PminF32x4`/`PmaxF32x4`/`PminF64x2`/`PmaxF64x2` -- identical
  type rule (pop two `V128`, push one `V128`); their implementation-
  defined NaN/signed-zero handling is entirely a runtime concern,
  invisible at this layer. Third/fourth/fifth/sixth opcodes of the
  relaxed-simd epic -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`.
- New tests: `valid_f32x4_relaxed_min_max_pop_two_v128_push_v128`,
  `invalid_f32x4_relaxed_min_max_given_an_i32_operand_instead_of_v128`,
  and their `f64x2` mirrors.

## [0.2.60] - 2026-08-25 (Relaxed SIMD epic PR2: i16x8.relaxed_q15mulr_s)

### Added

- Type-check coverage for `i16x8.relaxed_q15mulr_s` (`SimdOpKind::
  RelaxedQ15mulrI16x8S`): joins the existing shared `(v128, v128) ->
  v128` binary-shape arm right alongside `Q15mulrSatI16x8S` -- identical
  type rule (pop two `V128`, push one `V128`); its implementation-
  defined single-overflow-lane saturate-vs-wrap behavior is entirely a
  runtime concern, invisible at this layer. Second opcode of the
  relaxed-simd epic -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`.
- New tests: `valid_i16x8_relaxed_q15mulr_s_pops_two_v128_pushes_v128`,
  `invalid_i16x8_relaxed_q15mulr_s_given_an_i32_operand_instead_of_v128`.

## [0.2.59] - 2026-08-25 (Relaxed SIMD epic PR1: i8x16.relaxed_swizzle)

### Added

- Type-check coverage for `i8x16.relaxed_swizzle` (`SimdOpKind::
  RelaxedSwizzle`): joins the existing shared `(v128, v128) -> v128`
  binary-shape arm right alongside `Swizzle` -- identical type rule (pop
  two `V128`, push one `V128`); its implementation-defined out-of-range
  behavior is entirely a runtime concern, invisible at this layer. First
  opcode of the relaxed-simd epic that follows the now-complete base
  SIMD epic (PR1-PR47) -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`.
- New test: `valid_i8x16_relaxed_swizzle_pops_two_v128_pushes_v128`,
  `invalid_i8x16_relaxed_swizzle_with_an_i32_operand`.

## [0.2.58] - 2026-08-25 (SIMD PR47: v128.load64_lane/store64_lane)

### Added

- Type-check coverage for `v128.load64_lane`/`v128.store64_lane`
  (`SimdOpKind::Load64Lane`/`Store64Lane`): a new sibling match arm to
  PR46's `Load32Lane`/`Store32Lane` arm -- decodes a memarg (align,
  offset[, memidx], same multi-memory-memidx-rejection discipline)
  FOLLOWED BY a lane-index byte, verified against BinarySIMD.md's own
  encoding order (`m:memarg, i:ImmLaneIdx2`).
- REAL lane-index bounds checking via `read_lane_index`: lane index must
  be `0..=1`, matching `i64x2`'s 2-lane width -- a dedicated `>= 2`
  check, NOT the 32-bit pair's `>= 4` (an `i64x2` v128 has only 2 lanes,
  not `i32x4`'s 4; reusing the wider bound would silently accept an
  invalid lane index 2-3).
- Type rule: `Load64Lane` pops the existing `v128` (top of stack) then
  the `i32` address, pushes an updated `v128`; `Store64Lane` pops the
  same pair, pushes nothing -- identical shape to `Load32Lane`/
  `Store32Lane`.
- 7 new dedicated integration tests in `tests/type_check.rs`: valid
  (bare and with explicit `offset=`/`align=` attributes, boundary lanes
  0 and 1), invalid with no declared memory, invalid wrong-operand-type
  (address/v128 swapped), invalid out-of-range lane index (`2`, not
  `4`) for both directions, and a security-review-style cross-decoder
  consistency check (multi-memory flag bit set, explicit `memidx=0`)
  mirroring PR44/PR45/PR46's own regression tests for the 8-bit/16-bit/
  32-bit pairs. Closes the entire lane-load/store family's type-check
  coverage (PR44-47).

## [0.2.57] - 2026-08-25 (SIMD PR46: v128.load32_lane/store32_lane)

### Added

- Type-check coverage for `v128.load32_lane`/`v128.store32_lane`
  (`SimdOpKind::Load32Lane`/`Store32Lane`): a new sibling match arm to
  PR45's `Load16Lane`/`Store16Lane` arm -- decodes a memarg (align,
  offset[, memidx], same multi-memory-memidx-rejection discipline)
  FOLLOWED BY a lane-index byte, verified against BinarySIMD.md's own
  encoding order (`m:memarg, i:ImmLaneIdx4`).
- REAL lane-index bounds checking via `read_lane_index`: lane index must
  be `0..=3`, matching `i32x4`'s 4-lane width -- a dedicated `>= 4`
  check, NOT the 16-bit pair's `>= 8` (an `i32x4` v128 has 4 lanes, not
  `i16x8`'s 8; reusing the wider bound would silently accept an invalid
  lane index 4-7).
- Type rule: `Load32Lane` pops the existing `v128` (top of stack) then
  the `i32` address, pushes an updated `v128`; `Store32Lane` pops the
  same pair, pushes nothing -- identical shape to `Load16Lane`/
  `Store16Lane`.
- 7 new dedicated integration tests in `tests/type_check.rs`: valid
  (bare and with explicit `offset=`/`align=` attributes, boundary lanes
  0 and 3), invalid with no declared memory, invalid wrong-operand-type
  (address/v128 swapped), invalid out-of-range lane index (`4`, not
  `8`) for both directions, and a security-review-style cross-decoder
  consistency check (multi-memory flag bit set, explicit `memidx=0`)
  mirroring PR44/PR45's own regression tests for the 8-bit/16-bit pairs.

## [0.2.56] - 2026-08-25 (SIMD PR45: v128.load16_lane/store16_lane)

### Added

- Type-check coverage for `v128.load16_lane`/`v128.store16_lane`
  (`SimdOpKind::Load16Lane`/`Store16Lane`): a new sibling match arm to
  PR44's `Load8Lane`/`Store8Lane` arm -- decodes a memarg (align,
  offset[, memidx], same multi-memory-memidx-rejection discipline)
  FOLLOWED BY a lane-index byte, verified against BinarySIMD.md's own
  encoding order (`m:memarg, i:ImmLaneIdx8`).
- REAL lane-index bounds checking via `read_lane_index`: lane index must
  be `0..=7`, matching `i16x8`'s 8-lane width -- a dedicated `>= 8`
  check, NOT the 8-bit pair's `>= 16` (an `i16x8` v128 has 8 lanes, not
  `i8x16`'s 16; reusing the wider bound would silently accept an
  invalid lane index 8-15).
- Type rule: `Load16Lane` pops the existing `v128` (top of stack) then
  the `i32` address, pushes an updated `v128`; `Store16Lane` pops the
  same pair, pushes nothing -- identical shape to `Load8Lane`/
  `Store8Lane`.
- 7 new dedicated integration tests in `tests/type_check.rs`: valid
  (bare and with explicit `offset=`/`align=` attributes, boundary lanes
  0 and 7), invalid with no declared memory, invalid wrong-operand-type
  (address/v128 swapped), invalid out-of-range lane index (`8`, not
  `16`) for both directions, and a security-review-style cross-decoder
  consistency check (multi-memory flag bit set, explicit `memidx=0`)
  mirroring PR44's own regression test for the 8-bit pair.

## [0.2.55] - 2026-08-25 (SIMD PR44: v128.load8_lane/store8_lane)

### Added

- Type-check coverage for `v128.load8_lane`/`v128.store8_lane`
  (`SimdOpKind::Load8Lane`/`Store8Lane`): a NEW match arm, distinct from
  the existing `Load`/`Store`/etc. memarg-only arm -- decodes a memarg
  (align, offset[, memidx], same multi-memory-memidx-rejection
  discipline as every other SIMD load/store opcode) FOLLOWED BY a
  lane-index byte, verified against BinarySIMD.md's own encoding order
  (`m:memarg, i:ImmLaneIdx16`).
- REAL lane-index bounds checking via the existing `read_lane_index`
  helper (PR37's own precedent: reject an out-of-range VALUE at
  validation time, not just check the immediate's presence) -- lane
  index must be `0..=15`, matching `i8x16`'s 16-lane width.
- Type rule: `Load8Lane` pops the existing `v128` (top of stack) then
  the `i32` address, pushes an updated `v128`; `Store8Lane` pops the
  same pair, pushes nothing -- mirrors `Store`'s own pop order and
  no-result shape exactly.
- 6 new dedicated integration tests in `tests/type_check.rs`: valid
  (bare and with explicit `offset=`/`align=` attributes), invalid with
  no declared memory, invalid wrong-operand-type (address/v128 swapped),
  and invalid out-of-range lane index (`16`) for both directions.

## [0.2.54] - 2026-08-25 (SIMD PR42: v128.load_extend family)

### Added

- Type-check coverage for the 6 new load-extend opcodes
  (`v128.load8x8_s`/`_u`, `v128.load16x4_s`/`_u`, `v128.load32x2_s`/
  `_u`): joined the existing `Load`/`Store`/`Load8Splat`/`Load32Zero`/
  etc. memarg-decoding match arm -- identical memarg parsing (align,
  offset[, memidx]) and identical multi-memory-memidx rejection (fail
  closed until real multi-memory support lands, same security-review
  discipline as `v128.load`/`v128.store` from PR15/task #162-164 and the
  `load_splat`/`load_zero` families from PR40/PR41), just with the same
  pop-I32/push-V128 type rule already shared by `Load`/`Load8Splat`/etc.
  (which loaded lane gets sign- vs. zero-extended is a pure
  execution-time concern, invisible to the type checker).
- 6 new unit tests in `tests/type_check.rs`: `valid_v128_load_extend_
  family` (all 6 opcodes build cleanly with a declared memory),
  `invalid_v128_load_extend_family_with_no_memory_at_all`,
  `invalid_v128_load_extend_family_wrong_operand_type` (mirrors the
  upstream `simd_load_extend.wast` corpus's own type-mismatch checks),
  and `invalid_v128_load8x8_s_explicit_nonzero_memidx_is_rejected_not_
  silently_redirected_to_memory_0` (mirrors the existing
  `v128.load`/`v128.load8_splat`/`v128.load32_zero` security-review
  tests for the shared memidx-rejection code path).

## [0.2.53] - 2026-08-25 (SIMD PR41: v128.loadN_zero family)

### Added

- Type-check coverage for the 2 new load-zero opcodes
  (`v128.load32_zero`/`load64_zero`): joined the existing `Load`/
  `Store`/`Load8Splat`/etc. memarg-decoding match arm -- identical
  memarg parsing (align, offset[, memidx]) and identical
  multi-memory-memidx rejection (fail closed until real multi-memory
  support lands, same security-review discipline as `v128.load`/
  `v128.store` from PR15/task #162-164 and the `load_splat` family from
  PR40), just with the same pop-I32/push-V128 type rule already shared
  by `Load`/`Load8Splat`/etc. (the "zero" half of the semantics is a
  pure execution-time concern, invisible to the type checker).
- 6 new unit tests in `tests/type_check.rs`: `valid_v128_load_zero_
  family` (both opcodes build cleanly with a declared memory),
  `invalid_v128_load_zero_family_with_no_memory_at_all`,
  `invalid_v128_load_zero_family_wrong_operand_type` (mirrors the
  upstream `simd_load_zero.wast` corpus's own type-mismatch checks), and
  `invalid_v128_load32_zero_explicit_nonzero_memidx_is_rejected_not_
  silently_redirected_to_memory_0`/`invalid_v128_load64_zero_explicit_
  nonzero_memidx_is_rejected_not_silently_redirected_to_memory_0`
  (mirror the existing `v128.load`/`v128.load8_splat` security-review
  tests for the shared memidx-rejection code path).

## [0.2.52] - 2026-08-25 (SIMD PR40: v128.loadN_splat family)

### Added

- Type-check coverage for the 4 new load-splat opcodes
  (`v128.load8_splat`/`load16_splat`/`load32_splat`/`load64_splat`):
  joined the existing `Load`/`Store` memarg-decoding match arm --
  identical memarg parsing (align, offset[, memidx]) and identical
  multi-memory-memidx rejection (fail closed until real multi-memory
  support lands, same security-review discipline as `v128.load`/
  `v128.store` from PR15/task #162-164), just with a pop-I32/push-V128
  type rule shared with `Load` (the "splat" half of the semantics is a
  pure execution-time concern, invisible to the type checker).
- 5 new unit tests in `tests/type_check.rs`: `valid_v128_load_splat_
  family` (all 4 opcodes build cleanly with a declared memory),
  `invalid_v128_load_splat_family_with_no_memory_at_all`,
  `invalid_v128_load_splat_family_wrong_operand_type` (mirrors the
  upstream `simd_load_splat.wast` corpus's own type-mismatch checks),
  and `invalid_v128_load8_splat_explicit_nonzero_memidx_is_rejected_
  not_silently_redirected_to_memory_0` (mirrors the existing `v128.load`
  security-review test for the shared memidx-rejection code path).

## [0.2.51] - 2026-08-25 (SIMD widen PR39: f32x4/f64x2 rounding family)

### Added

- Type-check coverage for the 8 new rounding opcodes (`f32x4.ceil`/
  `floor`/`trunc`/`nearest`, `f64x2.ceil`/`floor`/`trunc`/`nearest`):
  joined the existing UNARY "pop one V128, push one V128" match arm
  alongside `AbsF32x4`/`AbsF64x2` -- the per-lane IEEE-754 rounding-mode
  selection, including `nearest`'s ties-to-even semantics, is entirely
  a runtime concern invisible to the type checker.
- 12 new unit tests in `tests/type_check.rs`: `valid_f32x4_rounding_
  family`/`valid_f64x2_rounding_family` (all 4 opcodes per shape build
  cleanly) plus 8 `invalid_*` tests covering wrong-operand-type,
  no-operand, and wrong-result-type rejections across both shapes.

## [0.2.50] - 2026-08-24 (task #229-231 — SIMD widen PR38: i8x16.shuffle, elevated-risk validation-time bounds gate)

### Added

- Type-check arm for `i8x16.shuffle`: pops TWO V128 operands (the same
  BINARY shape as `i8x16.swizzle`/`i8x16.add`/etc.), pushes one V128.
- `read_shuffle_lane_indices`: reads AND validates the instruction's
  16-byte raw (non-LEB128) lane-index immediate, one byte per output
  lane. Direct extension of `read_lane_index`'s single-byte pattern
  (SIMD widen PR37) to all 16 bytes at once, with a WIDER valid range
  than any prior lane-index family: `0..=31`, not `0..=15`, because
  `shuffle` indexes into the COMBINED 32-lane space of its two operands,
  not one operand's own narrower lane count.
- **Security: this is the highest-scrutiny opcode in the SIMD widen
  campaign so far** -- 16 independently attacker-controlled immediate
  bytes, each used as an array index into a 32-element gather space.
  `read_shuffle_lane_indices` rejects the module at VALIDATION time if
  ANY of the 16 bytes is `> 31` (checked in a loop over every position,
  not just the first or last), before the module can ever execute. This
  is what lets `wasm-execution`'s own gather treat a bad index as
  provably unreachable for any module that passed validation (see that
  crate's own changelog for its matching defense-in-depth guard on the
  execution side).
- 7 new tests: a valid identity-shuffle module, a valid module spanning
  the full `0..=31` range (confirming `31` itself validates), 3
  out-of-range tests targeting DIFFERENT byte positions specifically
  (position 0, a middle position 8, and the last position 15, each with
  a different out-of-range value) to confirm every position is actually
  checked and not just the first/last, and a stack-shape test (only one
  v128 operand supplied) confirming the BINARY pop requirement is
  genuinely enforced.

## [0.2.49] - 2026-08-24 (task #226-228 — SIMD widen PR37: extract_lane/replace_lane family, remaining shapes + lane-index bounds retrofit)

### Added

- Type-check arms for the 10 new opcodes: `i16x8.extract_lane_s`/`_u`
  (pop V128, push I32), `i16x8.replace_lane`/`i32x4.replace_lane` (pop
  I32 + V128, push V128), `i64x2.extract_lane` (pop V128, push I64 --
  the first `extract_lane` family member whose result isn't I32),
  `i64x2.replace_lane` (pop I64 + V128, push V128), `f32x4.extract_lane`
  (pop V128, push F32), `f32x4.replace_lane` (pop F32 + V128, push
  V128), `f64x2.extract_lane` (pop V128, push F64), `f64x2.replace_lane`
  (pop F64 + V128, push V128).
- **Lane-index bounds validation, new AND retrofitted onto the existing
  4 opcodes.** Before this PR, the type checker only confirmed the
  lane-index immediate byte was PRESENT (not truncated) -- never that
  its VALUE was in range, so an out-of-range lane index (e.g.
  `i32x4.extract_lane 4`) would pass validation and only be caught by
  `wasm-execution`'s runtime bounds check, contrary to the WASM spec's
  own requirement that an out-of-range `laneidx` makes the module
  INVALID at validation time, not merely trapping at runtime. New shared
  `read_lane_index` helper reads the immediate byte (still the common
  truncation check); every lane-immediate `SimdOpKind` arm -- the 10 new
  ones AND the 4 pre-existing ones (`ExtractLane`/`ExtractLaneI8x16S`/
  `ExtractLaneI8x16U`/`ReplaceLaneI8x16`) -- now applies its own
  shape-specific range check immediately after (0-15 `i8x16`, 0-7
  `i16x8`, 0-3 `i32x4`/`f32x4`, 0-1 `i64x2`/`f64x2`), rejecting via
  `ValidationError::Other` before the module can ever be instantiated.
  Retrofitting the pre-existing opcodes (not just the 10 new ones) was
  necessary for real conformance-suite correctness: the vendored
  `simd_lane.wast` file's `assert_invalid` directives exercise
  out-of-range lane indices for `i8x16`/`i32x4` too, and the WASM spec
  test harness convention (`assert_invalid` = "module fails
  VALIDATION") only grades correctly once the validator itself performs
  the rejection.
- New tests: an `assert_invalid` case for every one of the 14
  lane-immediate opcodes' out-of-range lane index, plus valid/operand-
  type-mismatch coverage for the 10 new opcodes.

## [0.2.48] - 2026-08-24 (task #223-225 — SIMD widen PR36: i64x2.extend_low/high_i32x4_s/u type rules)

### Added

- `SimdOpKind::ExtendLowI32x4S`/`ExtendHighI32x4S`/`ExtendLowI32x4U`/
  `ExtendHighI32x4U` join the existing UNARY `v128->v128` arm alongside
  `ExtendLowI16x8S`/etc. (PR26) -- the third and FINAL rung of the
  "extend" family, one lane width up. Even though the runtime reads a
  narrower (`i32`) source lane width and writes a wider (`i64`) result
  lane width, the type checker still only ever sees the opaque `V128`
  type on both sides, same pop-one-push-one shape as every other rung.
- New tests: `valid_simd_i64x2_extend_low_high_family`,
  `invalid_i64x2_extend_low_i32x4_s_given_an_i32_operand_instead_of_v128`,
  `invalid_i64x2_extend_high_i32x4_u_given_an_i32_operand_instead_of_v128`,
  `invalid_i64x2_extend_low_i32x4_s_given_no_operand_at_all`.

## [0.2.47] - 2026-08-24 (task #220-222 — SIMD widen PR35: f64x2.abs/min/max/pmin/pmax type rules)

### Added

- `SimdOpKind::MinF64x2`/`MaxF64x2`/`PminF64x2`/`PmaxF64x2` join the
  existing BINARY `v128,v128->v128` arm alongside their `f32x4`
  equivalents -- same pop-two-push-one `V128` shape. `min`/`max`'s
  NaN-canonicalizing/signed-zero runtime subtlety and `pmin`/`pmax`'s
  deliberately simpler `<`-based conditional-select semantics are both
  entirely runtime concerns, invisible to the type checker.
- `SimdOpKind::AbsF64x2` joins the existing UNARY `v128->v128` arm
  alongside `AbsF32x4`/`NegF64x2`/`SqrtF64x2` -- a pure bit operation,
  no new type-checker machinery needed, same pop-one-push-one `V128`
  shape.
- New tests: `valid_f64x2_abs_min_max_pmin_pmax_family`,
  `invalid_f64x2_abs_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_abs_given_no_operand_at_all`,
  `invalid_f64x2_min_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_max_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_pmin_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_pmax_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_pmax_given_only_one_operand_instead_of_two`,
  `invalid_f64x2_pmin_given_no_operand_at_all`,
  `invalid_f64x2_min_given_an_i32_result_type_instead_of_v128`.

## [0.2.46] - 2026-08-24 (task #217-219 — SIMD widen PR34: f32x4.max/pmin/pmax type rules)

### Added

- `SimdOpKind::MaxF32x4`/`PminF32x4`/`PmaxF32x4` join the existing
  BINARY `v128,v128->v128` arm alongside `MinF32x4`/`MulF32x4` -- same
  pop-two-push-one `V128` shape. `max`'s NaN-canonicalizing/signed-zero
  runtime subtlety (mirroring `min`) and `pmin`/`pmax`'s deliberately
  simpler `<`-based conditional-select semantics are both entirely
  runtime concerns, invisible to the type checker.
- New tests: `valid_f32x4_max_pmin_pmax_family`,
  `invalid_f32x4_max_given_an_i32_operand_instead_of_v128`,
  `invalid_f32x4_pmin_given_an_i32_operand_instead_of_v128`,
  `invalid_f32x4_pmax_given_an_i32_operand_instead_of_v128`,
  `invalid_f32x4_pmax_given_only_one_operand_instead_of_two`,
  `invalid_f32x4_pmin_given_no_operand_at_all`,
  `invalid_f32x4_max_given_an_i32_result_type_instead_of_v128`.

### Added

- `SimdOpKind::AddSatI8x16S`/`AddSatI8x16U`/`SubSatI8x16S`/
  `SubSatI8x16U`/`AddSatI16x8S`/`AddSatI16x8U`/`SubSatI16x8S`/
  `SubSatI16x8U` join the existing BINARY `v128,v128->v128` arm alongside
  `NarrowI16x8S`/`NarrowI16x8U`/`NarrowI32x4S`/`NarrowI32x4U` -- same
  pop-two-push-one `V128` shape. The compute-in-a-wider-type-then-clamp
  saturation arithmetic is entirely a runtime concern, invisible to the
  type checker.
- New tests: `valid_simd_sat_add_sub_family`,
  `invalid_i8x16_add_sat_s_given_an_i32_operand_instead_of_v128`,
  `invalid_i8x16_add_sat_u_given_an_i32_operand_instead_of_v128`,
  `invalid_i8x16_sub_sat_s_given_an_i32_operand_instead_of_v128`,
  `invalid_i8x16_sub_sat_u_given_an_i32_operand_instead_of_v128`,
  `invalid_i16x8_add_sat_s_given_an_i32_operand_instead_of_v128`,
  `invalid_i16x8_add_sat_u_given_an_i32_operand_instead_of_v128`,
  `invalid_i16x8_sub_sat_s_given_an_i32_operand_instead_of_v128`,
  `invalid_i16x8_sub_sat_u_given_an_i32_operand_instead_of_v128`,
  `invalid_i8x16_add_sat_s_given_only_one_operand_instead_of_two`,
  `invalid_i16x8_sub_sat_u_given_no_operand_at_all`,
  `invalid_i8x16_add_sat_s_given_an_i32_result_type_instead_of_v128`.

## [0.2.44] - 2026-08-24 (task #211-213 — SIMD widen PR32: f64x2 eq/ne/lt/gt/le/ge type rules)

### Added

- `SimdOpKind::EqF64x2`/`NeF64x2`/`LtF64x2`/`GtF64x2`/`LeF64x2`/`GeF64x2`
  join the existing BINARY `v128,v128->v128` comparison type-check arm
  alongside `EqF32x4`/`NeF32x4`/`LtF32x4`/`GtF32x4`/`LeF32x4`/`GeF32x4`
  -- a direct 2-lane mirror, same pop-two-push-one `V128` shape (the
  SIMD comparison convention: the RESULT is still `v128`, a per-lane
  boolean mask, never a plain `i32`). The IEEE-754 comparison and
  NaN-handling semantics are entirely a runtime concern, invisible
  here.
- New tests: `valid_f64x2_cmp_family`,
  `invalid_f64x2_eq_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_lt_given_no_operands_at_all`,
  `invalid_f64x2_ge_given_an_i32_result_type_instead_of_v128`.

## [0.2.43] - 2026-08-24 (task #208-210 — SIMD widen PR31: f64x2 neg/sqrt/add/sub/mul/div type rules)

### Added

- `SimdOpKind::NegF64x2`/`SqrtF64x2` join the existing UNARY
  `v128->v128` type-check arm alongside `NegF32x4`/`SqrtF32x4` -- direct
  2-lane mirrors, same pop-one-push-one `V128` shape.
- `SimdOpKind::AddF64x2`/`SubF64x2`/`MulF64x2`/`DivF64x2` join the
  existing BINARY `v128,v128->v128` type-check arm alongside
  `AddF32x4`/`SubF32x4`/`DivF32x4` -- direct 2-lane mirrors plus `mul`
  on the same shape, still just two `V128` pops, one `V128` push. The
  IEEE-754 arithmetic semantics (including `div`'s TOTAL behavior on a
  zero divisor) are entirely a runtime concern, invisible here.
- New tests: `valid_f64x2_arith_family`,
  `invalid_f64x2_add_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_mul_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_sqrt_given_an_i32_operand_instead_of_v128`,
  `invalid_f64x2_neg_given_no_operand_at_all`,
  `invalid_f64x2_div_given_no_operands_at_all`.

## [0.2.42] - 2026-08-24 (task #205-207 — SIMD widen PR30: f32x4 eq/ne/lt/gt/le/ge type rules)

### Added

- `SimdOpKind::EqF32x4`/`NeF32x4`/`LtF32x4`/`GtF32x4`/`LeF32x4`/`GeF32x4`
  join the existing BINARY `v128,v128->v128` comparison type-check arm
  alongside `Eq`/`EqI16x8`/`EqI8x16`/`EqI64x2` etc. -- the SIMD
  boolean-mask convention (result is still a `V128`, not a plain `I32`)
  and the IEEE-754 float-comparison/NaN semantics are entirely a runtime
  concern, invisible here; still just pop-two-push-one `V128`.
- New tests: `valid_f32x4_cmp_family`,
  `invalid_f32x4_eq_given_an_i32_operand_instead_of_v128`,
  `invalid_f32x4_lt_given_no_operands_at_all`,
  `invalid_f32x4_ge_given_an_i32_result_type_instead_of_v128`.

## [0.2.41] - 2026-08-24 (task #202-204 — SIMD widen PR29: f32x4 add/sub/div/neg/sqrt type rules)

### Added

- `SimdOpKind::NegF32x4`/`SqrtF32x4` join the existing UNARY `v128` op
  type-check arm (pop one `V128`, push `V128`) alongside `AbsF32x4` --
  their sign-flip/IEEE-754-sqrt runtime behavior is entirely invisible
  to the type checker, still just pop-one-push-one `V128`.
- `SimdOpKind::AddF32x4`/`SubF32x4`/`DivF32x4` join the existing BINARY
  `v128,v128->v128` type-check arm alongside `MulF32x4`/`MinF32x4` --
  ordinary IEEE-754 arithmetic (including `div`'s TOTAL, non-trapping
  behavior on a zero divisor) is entirely a runtime concern, invisible
  here.
- New tests: `valid_f32x4_arith_family`,
  `invalid_f32x4_add_given_an_i32_operand_instead_of_v128`,
  `invalid_f32x4_sqrt_given_an_i32_operand_instead_of_v128`,
  `invalid_f32x4_neg_given_no_operand_at_all`.

## [0.2.40] - 2026-08-19 (task #199-201 — SIMD widen PR28: promote/demote/convert_low family type rules)

### Added

- `SimdOpKind::DemoteF64x2Zero`/`PromoteLowF32x4`/`ConvertLowI32x4S`/
  `ConvertLowI32x4U` join the existing UNARY `v128` op type-check arm
  (pop one `V128`, push `V128`) alongside `ExtendLow/HighI8x16S/_U`/
  etc. -- even though these four cross both lane COUNT (4<->2) and
  lane TYPE (int/float, f32/f64) boundaries at runtime, the type
  checker only ever sees the opaque `V128` type on both sides; the
  zero-fill (`DemoteF64x2Zero`) vs. lane-dropping (the other three)
  distinction is entirely a runtime concern, invisible here.
- 7 new tests: one valid-module case covering all 4 new ops, four
  invalid-module regressions confirming each op genuinely rejects an
  `i32` operand instead of `v128`, and two confirming an empty stack
  (no operand at all) is also rejected.

### Notes

- **Campaign complete, corpus now vendored.** These 4 opcodes are the
  third and FINAL PR of a 3-PR sequence (`extend_low`/`high` done in
  PR26, `narrow` done in PR27, `promote`/`demote`/`convert_low` here)
  needed to land all 16 opcodes the upstream `simd_conversions.wast`
  corpus file's modules bundle together -- see `wasm-conformance`'s
  own CHANGELOG for the vendoring result (100% pass, 280/280
  directives).

## [0.2.39] - 2026-08-19 (task #196-198 — SIMD widen PR27: narrow saturating family type rules)

### Added

- `SimdOpKind::NarrowI16x8S`/`NarrowI16x8U`/`NarrowI32x4S`/
  `NarrowI32x4U` join the existing BINARY `v128` op type-check arm (pop
  two `V128`s, push `V128`) alongside `AddI8x16`/`SubI8x16`/
  `ExtmulLowI16x8S`/etc. -- the per-lane saturating clamp and the
  operand-to-half (first operand -> low half, second operand -> high
  half) concatenation are entirely runtime concerns, invisible to the
  type checker, which only ever sees the opaque `V128` type in both
  operand slots and the result.
- 6 new tests: one valid-module case covering all 4 new ops, four
  invalid-module regressions confirming each op genuinely rejects an
  `i32` operand instead of `v128`, one confirming a single-operand
  stack (only one of the required two `v128`s) is rejected, and one
  confirming an empty stack (no operand at all) is also rejected.

### Notes

- **Staged campaign, no corpus vendoring yet.** These 4 opcodes are the
  second of a 3-PR sequence (`extend_low`/`high` done in PR26, `narrow`
  here, `promote`/`demote`/`convert_low` in a future PR) needed to
  unlock the upstream `simd_conversions.wast` corpus file. This PR is
  opcode-only.

## [0.2.38] - 2026-08-19 (task #193-195 — SIMD widen PR26: extend_low/high family type rules)

### Added

- `SimdOpKind::ExtendLowI8x16S`/`ExtendHighI8x16S`/`ExtendLowI8x16U`/
  `ExtendHighI8x16U`/`ExtendLowI16x8S`/`ExtendHighI16x8S`/
  `ExtendLowI16x8U`/`ExtendHighI16x8U` join the existing UNARY `v128`
  op type-check arm (pop one `V128`, push `V128`) alongside
  `ExtaddPairwiseI8x16S`/`_U`/`ExtaddPairwiseI16x8S`/`_U` -- the LOW/
  HIGH lane selection and sign/zero extension are entirely runtime
  concerns, invisible to the type checker, which only ever sees the
  opaque `V128` type in and out.
- 6 new tests: one valid-module case covering all 8 new ops, four
  invalid-module regressions confirming each family genuinely rejects
  an `i32` operand instead of `v128`, and one confirming an empty stack
  (no operand at all) is also rejected, not just a wrong-typed one.

### Notes

- **Staged campaign, no corpus vendoring yet.** Part of the 16-opcode
  set (`extend_low`/`high` here, `narrow` and `promote`/`demote`/
  `convert_low` in future PRs) needed to unlock the upstream
  `simd_conversions.wast` corpus file. This PR is opcode-only.

## [0.2.37] - 2026-08-19 (task #190-192 — SIMD widen PR25: i32x4.trunc_sat_f64x2_s/u_zero type rule)

### Added

- `SimdOpKind::TruncSatF64x2SZero`/`TruncSatF64x2UZero` join the
  existing UNARY `v128` op type-check arm (pop one `V128`, push
  `V128`) alongside `TruncSatF32x4S`/`_U`/`ConvertI32x4S`/`_U` -- even
  though the runtime reads 2 `f64` lanes and writes 4 `i32` lanes (2
  zero-filled), WASM's type system doesn't distinguish lane shapes, so
  this is the same pop-one-push-one shape as every other kind in that
  arm.
- 3 new tests: a valid-module case covering both new ops, plus two
  invalid-module regressions confirming each genuinely rejects an `i32`
  operand instead of `v128`, not just accepting whatever's on the
  stack.

## [0.2.36] - 2026-08-19 (task #183-185 — SIMD widen PR22: i16x8.q15mulr_sat_s type rule)

### Added

- `SimdOpKind::Q15mulrSatI16x8S` joins the existing binary-`v128`-op
  type-check arm (pop two `V128`, push `V128`) -- the Q15 rounding/
  saturating math is entirely a runtime concern, invisible to the type
  checker, same as every other `i16x8` binary op already in this arm.
- 2 new tests: a valid-module case, plus an invalid-module regression
  confirming `i16x8.q15mulr_sat_s` genuinely rejects an `i32` in one of
  its two `v128` operand slots, not just accepting whatever's on the
  stack.

## [0.2.35] - 2026-08-19 (task #180-182 — SIMD widen PR21: i64x2.extmul_i32x4 widening-multiply type rules)

### Added

- `SimdOpKind::ExtmulLowI64x2S | ExtmulHighI64x2S | ExtmulLowI64x2U |
  ExtmulHighI64x2U` join the existing binary-`v128`-op type-check arm
  (pop two `V128`, push `V128`) -- the third and final "extmul" rung,
  mirroring the already-implemented `ExtmulLowI16x8S`/`ExtmulLowI8x16S`
  entries in the same arm. The `i32x4` -> `i64x2` widening is entirely
  a runtime concern, invisible to the type checker (WASM's type system
  doesn't distinguish lane widths).
- 2 new tests: a valid-module case covering all 4 new ops, plus an
  invalid-module regression confirming `i64x2.extmul_low_i32x4_s`
  genuinely rejects an `i32` in one of its two `v128` operand slots,
  not just accepting whatever's on the stack.

## [0.2.34] - 2026-08-19 (task #177-179 — SIMD widen PR20: i32x4<->f32x4 trunc_sat/convert type rules)

### Added

- `SimdOpKind::TruncSatF32x4S | TruncSatF32x4U | ConvertI32x4S |
  ConvertI32x4U` join the existing unary-`v128`-op type-check arm (pop
  one `V128`, push `V128`) -- even though these change the LANE TYPE
  (f32 lanes <-> i32 lanes) at runtime, WASM's type system doesn't
  distinguish "i32-lane v128" from "f32-lane v128"; both are just the
  opaque `V128` type here, so no new type-checker machinery is needed.
- 2 new tests: a valid-module case covering all 4 new ops, plus an
  invalid-module regression confirming `f32x4.convert_i32x4_u`
  genuinely rejects an `i32` in the `v128` operand slot, not just
  accepting whatever's on the stack.

## [0.2.33] - 2026-08-19 (task #174-176 — SIMD widen PR19: f32x4.abs/f32x4.mul/f32x4.min type rules)

### Added

- `SimdOpKind::MulF32x4 | MinF32x4` join the existing binary-`v128`-op
  type-check arm (pop two `V128`, push `V128`) -- their NaN/signed-zero
  runtime subtlety (see `wasm-opcodes`'s own `SimdOpKind::MinF32x4` doc
  comment) is entirely invisible to the type checker.
- `SimdOpKind::AbsF32x4` joins the existing unary-`v128`-op type-check
  arm (pop one `V128`, push `V128`) -- a pure bit operation, no new
  type-checker machinery needed.
- 2 new tests: a valid-module case covering all 3 new ops, plus an
  invalid-module regression confirming `f32x4.mul` genuinely rejects
  an `i32` in a `v128` operand slot, not just accepting whatever's on
  the stack.

## [0.2.32] - 2026-08-19 (task #171-173 — SIMD widen PR18: i8x16 swizzle/extract_lane_s/extract_lane_u/replace_lane type rules)

### Added

- `SimdOpKind::Swizzle` joins the existing binary-`v128`-op type-check
  arm (pop two `V128`, push `V128`) -- an index-vector-driven
  permutation at the runtime level, but the same shape as
  `Add`/`AddI8x16`/etc. to the type checker.
- New `SimdOpKind::ExtractLaneI8x16S | ExtractLaneI8x16U` arm: same
  shape as the pre-existing `ExtractLane` arm (skip the raw lane-index
  immediate byte, pop `V128`, push `I32`) -- the 0-15 lane range and
  the signed/unsigned split are runtime concerns, invisible here.
- New `SimdOpKind::ReplaceLaneI8x16` arm: the GENUINELY NEW shape --
  skip the lane-index immediate byte, then pop `I32` (the replacement
  value, popped first, matching the shift family's own mixed-type
  pop order) then `V128` (the base operand), push `V128`.
- 4 new tests: valid-module cases for all 4 new ops (`swizzle`,
  `extract_lane_s`/`_u` together, `replace_lane`), plus an
  invalid-module regression confirming `replace_lane` genuinely
  rejects a `v128` in the `i32` value slot, not just accepting
  whatever's on the stack.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.31] - 2026-08-19 (task #168-170 — SIMD: float splat family type rules)

### Added

- New `SimdOpKind::SplatF32x4` arm (pop `F32`, push `V128`) and
  `SplatF64x2` arm (pop `F64`, push `V128`) -- the FIRST
  floating-point-typed SIMD ops in this crate's type rules.
- 2 new tests: a valid module exercising both new splat ops, and an
  invalid-module case confirming `f32x4.splat` genuinely rejects an
  `i32` operand (not just accepting whatever scalar type is on the
  stack).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.30] - 2026-08-19 (task #165-167 — SIMD: splat family widening type rules)

### Added

- `SimdOpKind::Splat | SplatI8x16 | SplatI16x8` now share one type-check
  arm (pop `I32`, push `V128`) alongside the pre-existing `i32x4.splat`
  rule. New separate arm for `SimdOpKind::SplatI64x2` (pop `I64`, push
  `V128`) -- the first splat whose popped operand type differs from
  `i32`, so it genuinely needed its own arm rather than joining the
  shared one.
- 2 new tests: a valid module exercising all three new splat ops, and
  an invalid-module case confirming `i64x2.splat` genuinely rejects an
  `i32` operand (not just accepting whatever scalar type is on the
  stack).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.29] - 2026-08-18 (task #162-164 — SIMD: v128.load/v128.store type rules)

### Added

- New `0xFD` SIMD type-check arm for `SimdOpKind::Load | SimdOpKind::
  Store` -- the FIRST SIMD ops in this crate's type rules that need a
  `memarg` immediate. Mirrors the existing scalar `0x28..=0x3E` memory
  arm's `MULTI_MEMORY_FLAG` (`0x40`) decode logic exactly (so byte
  consumption stays correct even under multi-memory encodings), then
  requires `ctx.has_memory` (erroring "v128.load/v128.store used, but
  module declares no memory" otherwise, same shape as every other
  memory-instruction error in this crate) before popping/pushing:
  `Load` pops `I32`, pushes `V128`; `Store` pops `V128` then `I32` --
  same pop order as `wasm-execution`'s own handler.
- 2 new tests: a valid module using both ops with a declared memory,
  and an invalid-module pair proving each op is rejected when the
  module declares no memory at all.

### Fixed

- `/security-review` finding: unlike the scalar `0x28..=0x3E` memory
  arm (which bounds-checks an explicit `memidx` against
  `ctx.memory_count`, since its executor genuinely honors any valid
  memory index), the new SIMD arm now REJECTS any explicit non-zero
  `memidx` outright rather than bounds-checking it -- because
  `wasm-execution`'s `v128.load`/`v128.store` handlers unconditionally
  target memory 0 for this first PR (see their own scope note).
  Bounds-checking alone would have let a module that declares 2+ real
  memories and explicitly encodes `v128.load memidx=1` validate
  successfully and then silently read/write memory 0 at execution time
  instead -- a cross-memory data-confusion path at a trust boundary,
  fixed by failing closed until multi-memory `v128.load`/`v128.store`
  is actually implemented. 1 new regression test builds a raw
  `WasmModule` directly (this crate's text-form parser has no
  leading-memidx syntax for `v128.load`/`v128.store`, so the only way
  to reach this path is hand-crafted bytecode) proving the explicit,
  in-bounds `memidx=1` case is rejected.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.28] - 2026-08-18 (task #159-161 — SIMD: shift family type rules)

### Added

- New `0xFD` SIMD type-check arm for `ShlI8x16 | ShrSI8x16 |
  ShrUI8x16 | ShlI16x8 | ShrSI16x8 | ShrUI16x8 | ShlI32x4 | ShrSI32x4
  | ShrUI32x4 | ShlI64x2 | ShrSI64x2 | ShrUI64x2` -- the FIRST
  mixed-type binary SIMD op family in this crate's type rules. Pops
  `I32` first (the shift amount is on top of stack, per
  `(ixNxM.shl (v128 $a) (i32 $amount))`'s push order), then `V128`,
  pushes `V128` -- matching wasm-execution's own pop order exactly.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.27] - 2026-08-18 (task #156-158 — SIMD: i64x2 arith+cmp family type rules)

### Added

- `0xFD` SIMD type-check match widened for i64x2's first REAL
  ARITHMETIC family: the pop-two-push-one binary arm extended to also
  cover `AddI64x2 | SubI64x2 | MulI64x2`; the comparison arm extended
  to also cover `EqI64x2 | NeI64x2 | LtSI64x2 | GtSI64x2 | LeSI64x2 |
  GeSI64x2`; the pop-one-push-one unary arm extended to also cover
  `AbsI64x2 | NegI64x2`. All reuse the same `v128,v128->v128`/
  `v128->v128` type shapes already used for every other lane width --
  this is a new LANE WIDTH, not a new operand shape, so no new
  type-checker plumbing.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.26] - 2026-08-18 (task #153-155 — SIMD: boolean-reduction/bitmask family type rules)

### Added

- New `0xFD` SIMD type-check arm for `AnyTrue | AllTrueI8x16 |
  AllTrueI16x8 | AllTrueI32x4 | AllTrueI64x2 | BitmaskI8x16 |
  BitmaskI16x8 | BitmaskI32x4 | BitmaskI64x2`: same `v128`-in/`i32`-out
  shape as the existing `ExtractLane` arm, but with NO lane-index
  immediate to consume (these reduce over ALL lanes, not select one).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.25] - 2026-08-18 (task #150-152 — SIMD: v128 bitwise family type rules)

### Added

- `0xFD` SIMD type-check match widened for the lane-width-agnostic
  raw-byte bitwise family: the pop-two-push-one binary arm extended
  to also cover `And | AndNot | Or | Xor`; the pop-one-push-one unary
  arm extended to also cover `Not`. A brand-new arm added for
  `Bitselect` -- the first TERNARY SIMD op in this crate -- which
  pops three `v128`s and pushes one `v128`; at the type level it's
  just three `V128` pops, the runtime's byte-level `(a AND c) OR (b
  AND (NOT c))` semantics are invisible to the type checker.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.24] - 2026-08-18 (task #147-149 — SIMD: i16x8-from-i8x16 widening type rules)

### Added

- `0xFD` SIMD type-check match widened for `i16x8`'s own widening
  family: the pop-two-push-one binary arm extended to also cover
  `ExtmulLowI8x16S | ExtmulHighI8x16S | ExtmulLowI8x16U |
  ExtmulHighI8x16U`; the pop-one-push-one unary arm extended to also
  cover `ExtaddPairwiseI8x16S | ExtaddPairwiseI8x16U`. Both stay
  `v128`-in/`v128`-out at the type level regardless of the narrower
  `i8`-in/`i16`-out lane interpretation the interpreter uses
  internally.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.23] - 2026-08-18 (task #144-146 — SIMD: i16x8 abs/min/max/avgr_u type rules)

### Added

- `0xFD` SIMD type-check match widened for `i16x8`'s own "arith2"
  family: the pop-two-push-one binary arm extended to also cover
  `MinSI16x8 | MinUI16x8 | MaxSI16x8 | MaxUI16x8 | AvgrUI16x8`; the
  pop-one-push-one unary arm extended to also cover `AbsI16x8`. Both
  stay `v128`-in/`v128`-out at the type level regardless of the
  narrower `i16` lane interpretation the interpreter uses internally.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.22] - 2026-08-18 (task #141-143 — SIMD: i8x16 abs/popcnt/min/max/avgr_u type rules)

### Added

- `0xFD` SIMD type-check match widened for `i8x16`'s own "arith2"
  family: the pop-two-push-one binary arm (`Add | Sub | Mul | ...`)
  extended to also cover `MinSI8x16 | MinUI8x16 | MaxSI8x16 |
  MaxUI8x16 | AvgrUI8x16`; the pop-one-push-one unary arm (`Neg | Abs
  | ...`) extended to also cover `AbsI8x16 | PopcntI8x16`. Both stay
  `v128`-in/`v128`-out at the type level regardless of the narrower
  `i8` lane interpretation the interpreter uses internally.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.21] - 2026-08-18 (task #137-140 — SIMD: i8x16 comparison family type rules)

### Added

- `0xFD` SIMD type-check match widened for `i8x16`'s own comparison
  family: `Eq | Ne | ... | GeUI16x8` arm extended to also cover
  `EqI8x16 | NeI8x16 | LtSI8x16 | LtUI8x16 | GtSI8x16 | GtUI8x16 |
  LeSI8x16 | LeUI8x16 | GeSI8x16 | GeUI8x16` (same pop-two-push-one
  `v128` shape -- WASM's SIMD comparison convention keeps the RESULT a
  `v128` boolean mask, not a plain `i32`, same as `i16x8`'s and
  `i32x4`'s own comparison families).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.20] - 2026-08-18 (task #133-136 — SIMD: i16x8 comparison family type rules)

### Added

- `0xFD` SIMD type-check match widened for `i16x8`'s own comparison
  family: `Eq | Ne | ... | GeU` arm extended to also cover
  `EqI16x8 | NeI16x8 | LtSI16x8 | LtUI16x8 | GtSI16x8 | GtUI16x8 |
  LeSI16x8 | LeUI16x8 | GeSI16x8 | GeUI16x8` (same pop-two-push-one
  `v128` shape -- WASM's SIMD comparison convention keeps the RESULT a
  `v128` boolean mask, not a plain `i32`, same as `i32x4`'s own
  comparison family).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.19] - 2026-08-18 (task #129-132 — SIMD: i16x8 first primary-lane slice type rules)

### Added

- `0xFD` SIMD type-check match widened for this crate's first opcodes
  where `i16x8` is a PRIMARY lane width: `Add | Sub | Mul | ... |
  SubI8x16` arm extended to also cover `AddI16x8 | SubI16x8 | MulI16x8`
  (same pop-two-push-one `v128` shape). `Neg | Abs | ... | NegI8x16` arm
  extended to also cover `NegI16x8` (same pop-one-push-one `v128`
  shape). Same "type checker only sees plain `v128`, never the narrower
  lane interpretation" pattern as every prior SIMD addition.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.18] - 2026-08-18 (task #125-128 — SIMD: i8x16 first slice type rules)

### Added

- `0xFD` SIMD type-check match widened for this crate's first
  `i8x16`-lane-width ops: `Add | Sub | Mul | ... | ExtmulHighI16x8U`
  arm extended to also cover `AddI8x16 | SubI8x16` (same pop-two-
  push-one `v128` shape). `Neg | Abs | ... | ExtaddPairwiseI16x8U` arm
  extended to also cover `NegI8x16` (same pop-one-push-one `v128`
  shape). Same "type checker only sees plain `v128`, never the
  narrower lane interpretation" pattern as every prior SIMD addition.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.17] - 2026-08-18 (task #121-124 — SIMD widening: i32x4-from-i16x8 family type rules)

### Added

- `0xFD` SIMD type-check match widened further: `Add | Sub | Mul | MinS |
  MinU | MaxS | MaxU` arm extended to also cover `DotI16x8S |
  ExtmulLowI16x8S | ExtmulHighI16x8S | ExtmulLowI16x8U |
  ExtmulHighI16x8U` (same pop-two-push-one `v128` shape -- these ops
  read their operands as `i16x8` internally, but the type checker only
  ever sees plain `v128`, never the narrower lane interpretation).
  `Neg | Abs` arm extended to `Neg | Abs | ExtaddPairwiseI16x8S |
  ExtaddPairwiseI16x8U` (same pop-one-push-one `v128` shape).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.16] - 2026-08-18 (task #118-120 — SIMD widening: i32x4 abs/min/max family type rules)

### Added

- `0xFD` SIMD type-check match widened further: `Add | Sub | Mul` arm
  extended to `Add | Sub | Mul | MinS | MinU | MaxS | MaxU` (same
  pop-two-push-one `v128` shape, result stays a plain `v128`, not a
  boolean mask like the comparison family). `Neg` arm extended to
  `Neg | Abs` (same pop-one-push-one `v128` shape).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.15] - 2026-08-18 (task #113-117 — SIMD widening: i32x4 arithmetic + comparison family type rules)

### Added

- `0xFD` SIMD type-check match widened with two new arms: `Add | Sub |
  Mul` (pop two `v128`, push one) and `Eq | Ne | LtS | LtU | GtS | GtU |
  LeS | LeU | GeS | GeU` (same pop-two-push-one shape, but the result is
  still a `v128` boolean mask, not a plain `i32` -- same rule `Eq` alone
  already established). New `Neg` arm: pop one `v128`, push one --
  UNARY, unlike every other kind in this match.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.14] - 2026-08-17 (task #92/#111 — real multi-memory memidx bounds check)

### Added

- `ModuleContext.memory_count: u32` (combined imported + module-defined
  memory count, same index-space convention as `table_count`), replacing
  `has_memory: bool`'s "is there at least one" check for anything that
  can now reference a SPECIFIC memory index.

### Fixed

- Every memarg-carrying load/store (`0x28`-`0x3E`) now decodes the align
  byte's real multi-memory flags bit (`0x40`) and an optional trailing
  memidx, bounds-checking it against `ctx.memory_count` -- previously
  this byte wasn't even read as a memidx at all.
- `memory.size`/`memory.grow` (`0x3F`/`0x40`): their memory-index byte
  was already treated as a REAL memidx at execution time since WASM17,
  but the validator still named it `_reserved` and discarded it --
  closed that validation-time gap to match, bounds-checking it the
  same way.
- `memory.init`/`memory.copy`/`memory.fill`: previously hard-rejected
  ANY nonzero memory index outright (`memory != 0 => err`), which is
  now the WRONG behavior once `wasm-execution` genuinely supports
  multi-memory (task #109) -- each memidx is now bounds-checked against
  `ctx.memory_count` instead, so a real, in-bounds nonzero index
  validates correctly and only an out-of-bounds one is rejected.

See `code/specs/W18-wasm-multi-memory-memarg.md`.

## [0.2.13] - 2026-08-17 (task #107 — call_indirect/return_call_indirect table-index bounds check)

### Fixed

- `call_indirect` (`0x11`) and `return_call_indirect` (`0x13`)'s
  `tableidx` immediate was decoded and then explicitly discarded via
  `let (_table_idx, ..)`, never bounds-checked. Now checked against
  `ctx.table_count`, same shape `table.grow`/`table.size`/`table.fill`
  (task #98) and `table.init`/`table.copy` (task #97) already use.

## [0.2.12] - 2026-08-17 (task #97 — table.init/table.copy/elem.drop type-checking)

### Added

- New `0xFC` sub-opcode type-check arms for `table.init` (`0x0C`),
  `elem.drop` (`0x0D`), and `table.copy` (`0x0E`) -- previously fell
  into the catch-all `"unsupported 0xFC sub-opcode"` error. `table.init`
  bounds-checks BOTH its elem segment index (against
  `ctx.module.elements.len()`, mirroring `memory.init`'s own data_idx
  check, task #95) and its table index (against `ctx.table_count`,
  mirroring `table.grow`'s own check, task #98); pops `[dest, src,
  len]` as three `ValueType::I32`. `elem.drop` bounds-checks only its
  elem segment index, no table requirement at all, mirroring
  `data.drop`'s own "no memory requirement" reasoning. `table.copy`
  bounds-checks both table indices independently (a self-copy, dst ==
  src, is valid and left to a runtime check, not rejected here).

### Fixed

- `Check 9`'s element-segment function-index bounds-check loop
  (`for func_idx in &elem.function_indices { if let Some(idx) = ...`)
  triggered clippy's `manual_flatten` lint after `function_indices`
  widened to `Vec<Option<u32>>` (task #97, `wasm-types`); rewritten as
  `for idx in elem.function_indices.iter().flatten()`.

## [0.2.11] - 2026-08-16 (task #98 — table.grow/table.size/table.fill type-checking)

### Added

- New `0xFC` sub-opcode type-check arms for `table.grow` (`0x0F`),
  `table.size` (`0x10`), and `table.fill` (`0x11`) -- previously
  unhandled. `table.grow`/`table.fill` type-check against the
  REFERENCED table's own declared element type (funcref vs externref),
  same per-table lookup `table.get`/`table.set` (task #96) already
  established, not a hardcoded assumption. All three bounds-check their
  `table_idx` against `ctx.table_count`, same real-index-check pattern
  `table.get`/`table.set` use.

## [0.2.10] - 2026-08-16 (task #95 — memory.init/data.drop type-checking)

### Added

- New `0xFC` sub-opcode type-check arms for `memory.init` (`0x08`) and
  `data.drop` (`0x09`) -- previously unhandled, falling into the generic
  "unsupported 0xFC sub-opcode" rejection. `memory.init` requires a
  declared memory (same as `memory.copy`/`memory.fill`) and pops the
  same `[dest, src, length]` i32 triple; `data.drop` has no stack effect
  and no memory requirement at all (a module with zero memories can
  still declare and drop a passive data segment it never gets to
  `memory.init` from). Both bounds-check their data-segment-index
  immediate against `ctx.module.data.len()` -- a real validation error
  for an out-of-bounds index, not deferred to a runtime trap, matching
  every other indexed immediate this type-checker validates.

## [0.2.9] - 2026-08-16 (task #100 — ValidatedModule.module made private)

### Changed (breaking)

- `ValidatedModule.module` is no longer a public field -- access it via
  the new `ValidatedModule::module()` accessor instead.

### Security

- Found via `/security-review` on `wasm-runtime`'s task #100 fix (making
  `instantiate()` require `&ValidatedModule` instead of `&WasmModule`,
  so its validation checks can't be skipped): that fix offered no real
  protection while `ValidatedModule.module` was a public field, since
  any crate depending on `wasm-validator` could construct
  `ValidatedModule { module: attacker_controlled }` directly with a
  struct literal, skipping `validate()` (and its memory/table allocation
  caps) entirely. Privatizing the field makes `validate()` succeeding
  the only way to obtain a `ValidatedModule` at all.

## [0.2.8] - 2026-08-16 (task #96 — multi-table)

### Changed (breaking)

- The table-count check ("Check 2") no longer rejects a module with more
  than 1 table outright -- the cap is now `wasm_execution::MAX_TABLES`
  (64), replacing WASM 1.0's hardcoded "at most 1".
- The element-segment table-index check ("Check 9") is now a real bounds
  check against the total table count, instead of hardcoding "must be
  0". Unlike W16's data-segment check (deliberately left at "must be 0"
  to avoid a silent-misapplication risk), this is safe to generalize:
  `wasm-runtime::instantiate()`'s element-segment application already
  indexes by the real `elem.table_index`.

### Fixed

- `table.get`/`table.set`'s instruction-level type check unconditionally
  assumed every table was `funcref` (a real, previously-deliberate WASM
  1.0-only limitation). Now looks up the REFERENCED table's own declared
  element type (funcref or externref) instead -- a multi-table module can
  freely mix both, and each `table.get $t`/`table.set $t` must type-check
  against `$t`'s own type, not a blanket assumption.

### Security

- **New Check 1b**: a memory's `min`/`max` must not exceed 2^16 pages --
  a real WASM spec structural-validation rule (not a heuristic), the
  identical bound `LinearMemory::grow()` already enforced at runtime, but
  previously never checked before an eager, unvalidated allocation at
  instantiation time. Found via `/security-review` while widening
  `MAX_TABLES`. Bonus: closes 6 previously-`NotYetSupported`
  `assert_invalid` cases in the vendored `memory.wast` for free.
- **New Check 2b**: a table's `min` must not exceed the new
  `wasm_execution::MAX_TABLE_ELEMENTS` (10,000,000) -- unlike Check 1b,
  this is an implementation-defined resource limit, not a spec
  requirement (real WASM allows a table `min` up to `2^32 - 1`), since
  `Table::new` allocates eagerly and raising `MAX_TABLES` from 1 to 64
  in this same release amplified an unvalidated `min`'s DoS blast radius
  64x.
- **Check 1b/2b, round 2**: a per-item cap alone doesn't bound the
  aggregate -- 64 memories (or tables) each individually under their
  per-item cap can still multiply out to ~256GB (memory) / ~5.1GB
  (table) of eager allocation from one small module, through the fully-
  intended `validate()` path, no bypass needed. Found via a second
  `/security-review` pass on this same diff. Both checks now also track
  a running total across every memory/table (imported + declared) and
  cap the SUM at the same per-item bound -- still permits any single
  memory/table at its full max, just not many of them simultaneously.
  Verified zero conformance-corpus impact (full baseline regen, byte-
  identical to before this fix).

## [0.2.7] - 2026-08-15 (W16, task #85 — multi-memory first slice)

### Changed (breaking)

- New `wasm-execution` dependency, for `MAX_MEMORIES`.
- The memory-count check ("Check 1") no longer rejects a module with more
  than 1 memory outright -- the cap is now `wasm_execution::MAX_MEMORIES`
  (64), replacing WASM 1.0's hardcoded "at most 1". `ValidationError::
  TooManyMemories`'s message is now generically worded ("too many"), not
  "more than 1", so it reads correctly regardless of where the cap sits.
- The data-segment memory-index check ("Check 8") stays exactly "must be
  0" -- deliberately NOT widened alongside the count cap. `wasm-runtime::
  instantiate()` only ever applies a data segment to memory 0 regardless
  of `seg.memory_index`; widening this check alone would let a module
  targeting a non-zero memory index PASS validation and then have its
  segment silently misapplied to the wrong memory at instantiation time.
  See `code/specs/W16-wasm-multi-memory-first-slice.md`'s implementation
  note for the full reasoning (this diverges from that spec's original
  design, found during implementation).

See `code/specs/W16-wasm-multi-memory-first-slice.md` for the full design.

## [0.2.6] - 2026-08-15 (task #81 — v128/funcref/externref single-value blocktypes)

### Fixed

- `decode_blocktype` only special-cased the 4 MVP scalar single-byte
  blocktypes (`i32`/`i64`/`f32`/`f64`) explicitly; `v128` (`0x7B`, SIMD)
  and `funcref`/`externref` (`0x70`/`0x6F`, WASM17) fell through to the
  type-index branch, where their raw byte read as signed LEB128 (`0x7B`
  → -5) always failed with `TypeIndexOutOfBounds` -- even for an
  ordinary, valid `(block (result v128) ...)`. Found vendoring the real
  `simd_const.wast` corpus (task #78); see `wasm-execution` 0.9.1's
  matching fix in `decode_function_body`/`block_arity` (the same
  representation gap on the runtime side).
- 1 new test proving all 3 single-value blocktypes now validate
  correctly.

## [0.2.5] - 2026-08-15 (SIMD PR1b-2 — type rules for the v128 first slice)

### Added

- A new `0xFD` arm in the per-instruction type-check `match`, mirroring
  the existing `0xFE` atomics arm's shape (a prefixed sub-opcode family
  looked up in a `wasm-opcodes` metadata table), but decoding the
  sub-opcode as a LEB128 `u32` (`wasm_opcodes::get_simd_op`), not a raw
  byte. Type rules for all 5 SIMD PR1a opcodes: `v128.const` pushes
  `V128` (also advancing past its 16-byte literal, which doesn't affect
  the type stack itself); `i32x4.splat` pops `I32` pushes `V128`;
  `i32x4.add` pops two `V128` pushes `V128`; `i32x4.eq` pops two `V128`
  pushes `V128` (the SIMD boolean-mask convention -- the comparison
  RESULT is still a v128, not a plain `i32`, unlike every other
  comparison opcode this validator handles); `i32x4.extract_lane` pops
  `V128` pushes `I32`, after advancing past its 1-byte raw lane-index
  immediate.
- 8 new end-to-end tests in `tests/type_check.rs`, built via
  `wasm-wast-parser`'s new SIMD text syntax (SIMD PR1b-2, same release):
  5 valid-shape cases (including `v128` as a local/global type, not just
  a param/result) and 3 type-mismatch rejections.

See `code/specs/W13-wasm-simd-v128-first-slice.md`'s follow-up scope.

## [0.2.4] - 2026-08-15 (WASM16 — return_call/return_call_indirect type rules)

### Added

- Type rules for `return_call`/`return_call_indirect`: same param-popping
  shape as `call`/`call_indirect`, plus the tail-call-specific rule the
  real spec requires -- the callee's declared result types must match
  the CURRENT FUNCTION's own declared result types exactly (nothing
  runs after a tail call, so its results become the caller's results
  directly), and everything textually after the instruction is
  unreachable/stack-polymorphic, the same handling `return` already has.
  See `code/specs/W11-wasm-tail-calls.md`.
- 5 new tests: valid self-contained + indirect cases, out-of-range
  function index, argument type mismatch, and (the real tail-call-
  specific check) result-type-mismatches-caller for both the direct and
  indirect forms.

## [0.2.3] - 2026-08-15 (WASM18 — atomic memory op type rules)

### Added

- Type rules for the entire `0xFE`-prefixed atomics family, looked up
  via `wasm_opcodes::get_atomic_op` and branching on `AtomicOpKind`:
  `Fence` is a pure no-op; every other kind requires `ctx.has_memory`
  and enforces its declared `align=` immediate matches the operation's
  natural alignment *exactly* (stricter than plain load/store's
  upper-bound-only check), then pops/pushes per its kind (`Load`,
  `Store`, `Rmw`, `Cmpxchg`, `Notify`, `Wait`).
- 9 new tests covering valid/invalid shapes for every `AtomicOpKind`,
  narrow-width `i64` variants, and the missing-memory error case.

### Corrected (implementation-time, vs. the merged W09 spec)

- Initially implemented a `has_shared_memory` requirement per the merged
  spec's literal wording ("atomic ops require the memory be shared").
  Directly contradicted by the real, pinned-commit `atomic.wast`
  testsuite file's own `;; unshared memory is OK` module, which
  exercises every atomic op against a non-shared `(memory 1 1)`
  expecting success. Removed the `has_shared_memory` check entirely
  (and the `ModuleContext` field backing it) -- only `has_memory` is
  required. The now-wrong `invalid_atomic_op_on_a_non_shared_memory`
  test was deleted and replaced with
  `valid_atomic_ops_on_a_non_shared_memory`, proving the correction.

## [0.2.2] - 2026-08-15 (WASM17 — funcref/externref type rules)

- Upgraded `ref.null`'s existing type rule: instead of unconditionally
  pushing `StackType::Unknown`, it now reads the heap-type byte and pushes
  a real static type -- `Funcref` (0x70), `Externref` (0x6F), `Anyref`
  (0x0F, this repo's own pre-existing bare-`ref.null` convention). Still
  not full subtyping (any other heap-type byte still falls back to
  `Unknown`), but enough to make `select`/`global.set`/etc.'s existing
  type-mismatch checks catch a funcref-vs-externref mixup, which they
  couldn't before since both looked like the same `Unknown`.
- Added type rules for `ref.func` (bounds-checks `funcidx` against the
  same `func_types` table `call`'s rule uses, pushes `Funcref`) and
  `table.get`/`table.set` (pop/push `I32`+`Funcref`, bounds-checked
  against a new `table_count` -- the REAL declared table count, not just
  a boolean "does any table exist", since (unlike memory ops, which
  hardcode index 0) these decode a real `tableidx` immediate that can be
  out of range even when *some* table exists).
- 3 new "valid" tests, 4 new "invalid" tests (including one proving the
  upgraded `ref.null` type now catches a funcref/externref mixup that
  type-checked before this release).

The instruction-level validator now decodes and type-checks `memory.copy` and
`memory.fill`, including their memory indices and three `i32` operands. This
closes a false rejection exposed when strict validation reached an existing
runtime string-concatenation module that uses `memory.copy`.

It also type-checks `ref.is_null` as consuming a reference and producing an
`i32`, closing the corresponding false rejection in existing WasmGC-backed
McCarthy Lisp output.

## [0.2.0] - 2026-08-14 (WASM06 -- instruction-level type checking, W02 Phase 2)

### Added -- a real per-instruction type checker

`validate()` previously only checked module-level structure (index bounds,
unique exports, memory/table counts). It now also runs a full
abstract-interpretation type check of every function body's instruction
sequence -- the algorithm `W02-wasm-validator.md`'s own §2 already
designed, now implemented in a new `type_check` module. Covers every WASM
1.0 MVP instruction family (control, parametric, variable, memory,
numeric, conversion), plus the sign-extension and non-trapping-conversion
proposals already supported elsewhere in this stack (WASM03), plus enough
of this repo's own small WasmGC opcode subset (struct/i31/ref.test) to
stay byte-in-sync and keep the abstract stack's height accurate without
implementing real reference-type subtyping (out of this phase's scope).

- Control-frame stack (`block`/`loop`/`if`, with the branch-target
  asymmetry a `loop`'s START vs. a `block`/`if`'s END needs -- same
  asymmetry `wasm-execution`'s `Label::param_arity` fix (WASM04) added to
  the interpreter side).
- `Unknown`-typed polymorphic dead code after `unreachable`/`br`/`return`:
  **deliberately diverges from `W02-wasm-validator.md`'s own literal
  pseudocode**, which only returns `Unknown` when `len(stack) <=
  frame.stack_height` -- that reading still strictly type-checks any real
  value sitting above the frame's floor at the moment reachability was
  lost, which rejects the spec doc's *own* worked example (`f32.const
  3.14` then `i64.add` in dead code). This implementation returns
  `Unknown` unconditionally while a frame is unreachable (discarding a
  real value if one happens to be there, but never comparing its type),
  which is what real engines implement and is the reading that makes that
  example type-check. `W02-wasm-validator.md` §2.5 updated to match.
- Multi-value blocktypes (WASM04/WASM06) resolve via the real type
  section, matching `wasm-execution`'s own `block_arity` fix.
- 38 new tests (`tests/type_check.rs`): one group that must validate, one
  that must be rejected, covering every instruction family plus the
  control-flow edge cases (`if` without `else` needing identical
  param/result types, `br_table` arity mismatches, dead-code
  polymorphism, memarg alignment limits via a hand-built binary fixture).
- **Bug found and fixed via the full `wasm-conformance` baseline regen**
  (the true integration test, not just hand-written cases): the `else`
  opcode handler reused the same `push_ctrl` helper `block`/`loop`/`if`'s
  initial entry uses, which pops the block's declared params off the
  *enclosing* scope -- correct for the original `if`, but wrong for
  `else`'s re-entry, which reuses the SAME already-consumed params rather
  than requiring the enclosing code to supply a second copy. Silently
  broke `if.wast`'s own top-level `(module ...)` validation, which
  cascaded into all 123 of that file's `assert_return` cases failing too
  (the module never registered) -- caught by a real regression, not
  inspection.
- Baseline effect (`wasm-conformance`): `assert_invalid` 15/838 (826
  `not_yet_supported`) -> 838/838 (100%, only 3 remaining
  `not_yet_supported`, both needing binary-format-level checks out of
  this phase's scope). Zero regressions elsewhere -- `assert_return`
  ended at the exact same 13775/13777 as before this change.

### Fixed -- `/security-review` found a reachable panic before this shipped

`control_stack` starts with exactly one frame (the function body's own
implicit outer block), meant to be closed by exactly one matching `end`
-- the LAST byte of a well-formed body. Nothing enforced that: a 2-byte
body `[0x0B, X]` for any function with empty declared results closed
that outer frame on the first byte, emptying `control_stack` while a
byte remained, and every later opcode handler's `frame!()`/`frame_mut!()`
macro (`.expect("control_stack never empties mid-body")`) -- or
`return`'s own unchecked `control_stack[0]` read -- panicked instead of
cleanly rejecting the module. A validator panicking on adversarial
bytecode is itself a denial-of-service: the one thing this code must
never do is crash on malformed input, only reject it. Fixed with two
layers: the `end` handler now rejects a premature top-level close
outright, and `frame!()`/`frame_mut!()` return a `ValidationError`
instead of panicking as defense in depth. Also fixed a related gap
found in the same review: `ref.null`'s heap-type immediate byte wasn't
bounds-checked (a truncated encoding was silently accepted rather than
rejected), and `br`/`br_if`/`br_table`'s branch-depth arithmetic used a
plain (non-`checked_add`) addition before the `checked_sub`, safe on
64-bit targets but not provably so. 4 new regression tests, verified via
TEMP-REVERT-CHECK to reproduce the exact real panics
(`index out of bounds` / `.expect()`) with the fix reverted.

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
