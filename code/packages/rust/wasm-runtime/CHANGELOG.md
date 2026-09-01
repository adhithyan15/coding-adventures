# Changelog

All notable changes to this package will be documented in this file.

## [0.6.26] — 2026-09-01 (W35 third slice — `func_identities`, `LocalFunctionRef`, `GlobalStorage`, `instance_identity`)

Slice 3 of 4 for `code/specs/W35-wasm-cross-instance-function-identity.md`.
Adds the "wasm-runtime instantiation/import-wiring" machinery the spec's
own design calls for — `WasmInstance::func_identities` (mirrors
`tag_identities`'s own construction loop exactly), `LocalFunctionRef` (the
"wrap MY OWN local function, called by raw index" counterpart to
`wasm-conformance::CrossModuleFunction`), `WasmRuntime::call_by_index`,
`GlobalStorage`-based globals, and a real `SelfFunctionResolver`
implementation (`InstanceSelfResolver`) — but with ONE MAJOR,
evidence-backed deviation from the spec's own literal design, described
in detail below, because the spec's own recommended construction-order
resolution is structurally unsound for the common case it was built to
handle.

### MAJOR deviation: `instantiate()` does NOT do the spec's own two-phase `Rc::try_unwrap` construction

The spec's own design (§6/"Recommended slice decomposition," item 3)
calls for: build `WasmInstance` with elem/global resolution deferred,
`Rc::new(RefCell::new(instance))` it, run an elem/global "fixup" pass that
resolves each entry into a real `FuncRefTarget` via a `LocalFunctionRef`
closing over that SAME `Rc`, then `Rc::try_unwrap` back to the plain
`WasmInstance` `instantiate()`'s own signature promises — claiming this
unwrap "MUST be infallible... nothing else holds a reference yet."

**This claim is false, and not merely for an edge case.** A
`LocalFunctionRef` resolved for an elem-segment entry (or a funcref-typed
global initializer) referencing the SAME module's OWN local function —
`linking.wast`'s own `$Mt`/`$g` example, this spec's own motivating case,
is exactly this shape — necessarily holds `instance:
instance_rc.clone()`, and that clone gets written into `instance`'s OWN
`tables`/`globals`, which are THEMSELVES part of the SAME `instance` the
`Rc` wraps. This is an unavoidable SELF-referential `Rc` cycle (not the
two-INSTANCE cycle the spec's own "Security and lifetime consideration"
section already anticipated and accepted for a bounded-lifetime registry)
— `instance_rc`'s strong count becomes `1 + (however many local functions
got referenced by this module's own elem/global entries)`, never `1`
again, so `Rc::try_unwrap` fails UNCONDITIONALLY the moment even ONE such
entry exists — which is the common, expected case for real WASM modules,
not a rare one.

**Reproduced directly, not theorized**: implementing the spec's own
literal design made this crate's own pre-existing
`table_init_copy_elem_drop.rs::active_element_segment_on_an_is64_table_
applies_at_instantiation_time` test (`(elem (table $t0) (i64.const 1)
func $one $two)` — two of the module's OWN local functions, written by
the module's OWN active elem segment) panic on `Rc::try_unwrap`, on every
single run, with exactly the message the spec's own verification-plan
note anticipated for "a bug that leaks an extra `Rc` clone" — except this
isn't a leak-able bug, it's a structural certainty for this shape.

**Root cause**: `instantiate()`'s own signature promises to return a
bare, OWNED `WasmInstance` — it does not, and structurally CANNOT, own
that instance's long-term lifetime. A `LocalFunctionRef`'s
`Rc<RefCell<WasmInstance>>` is only ever sound to construct when
SOMETHING holds a genuinely long-lived, PERMANENT `Rc` for the instance's
whole real lifetime — exactly what `wasm-conformance`'s `ModuleRegistry`
already does (`Rc<RefCell<HashMap<..., Rc<RefCell<WasmInstance>>>>>`,
held for an entire script's lifetime, cycle-tolerant BY DESIGN per the
spec's own security section: "a cycle within one registry is harmless
there, since the WHOLE registry, cycle and all, is freed together").
`instantiate()` itself has no such permanent home to offer. A
`Weak`-based redesign doesn't rescue this either: `Rc::try_unwrap`'s own
`into_inner()` frees the allocation a `Weak` would need to `upgrade()`
from, forever, the moment `instantiate()` returns — there would be
nothing left to upgrade to, ever again, even after a caller re-wraps the
RETURNED `WasmInstance` in its own, unrelated, brand-new `Rc`.

**Resolution actually shipped**: `instantiate()` does NOT attempt real
cross-instance funcref resolution for its own elem-segment/global-
initializer entries — they stay exactly as slice 2 left them
(`TableElement::Raw`/an unresolved raw index in `GlobalStorage::value`),
resolved LAZILY, on read, against whichever ctx actually dispatches them
— which is EXACTLY CORRECT for the single-instance case (the only case
this slice's own corpus baseline is expected to move for) and a KNOWN,
PRE-EXISTING, still-open gap for the genuinely cross-instance case
(unchanged by this slice, exactly as it was before it — that gap is
slice 4's job, via `wasm-conformance`'s own `ModuleRegistry`, which CAN
safely sustain the resulting cycle). `LocalFunctionRef`/
`resolve_func_ref_for_instance`/`InstanceSelfResolver`/`WasmRuntime::
call_by_index` remain fully implemented, `pub`, and directly unit-tested
— genuinely additive, tested infrastructure, ready for slice 4 to invoke
SAFELY from `ModuleRegistry`'s own permanent `Rc`.

### Further deviation: `build_engine` does not install a `SelfFunctionResolver` either

For the identical structural reason: `build_engine(&self, instance: &mut
WasmInstance)` only ever has a plain `&mut WasmInstance`, never a live
`Rc<RefCell<WasmInstance>>`, for ordinary per-call execution (`call`/
`call_typed`). Making one available would require either (a) breaking
`call`/`call_typed`'s own public signature to require callers to already
hold the instance behind an `Rc<RefCell<..>>` (an out-of-scope, much
larger API change — this slice's own scope is explicit that `call`/
`call_typed` stay unchanged), or (b) new cross-module wiring in
`wasm-conformance`'s `Executor`/`ModuleRegistry` to re-establish a
self-reference on every `Rc`-wrap it performs (explicitly slice 4's job).
`build_engine` DOES call the new `set_func_identities`/
`set_instance_identity` unconditionally — both are plain `u64`/`Vec<u64>`
values, no `Rc` involved, no structural issue.

### New types / fields

- **`WasmInstance::func_identities: Vec<u64>`** (new) — mirrors
  `tag_identities`'s own construction loop in `instantiate()` EXACTLY,
  sharing the SAME `NEXT_TAG_IDENTITY` counter (the spec's own explicit
  "tags and functions are never compared against each other, so sharing
  one counter is harmless" reasoning): an imported function adopts
  `host_func.identity()` verbatim; a module-defined function mints a
  fresh identity.
- **`WasmInstance::instance_identity: u64`** (new, further deviation from
  the spec's own literal text — not named anywhere in its design) —
  minted once per `instantiate()` call from a NEW, dedicated
  `NEXT_INSTANCE_IDENTITY` counter (kept separate from
  `NEXT_TAG_IDENTITY`, unlike `func_identities` — there's no "spec
  explicitly calls for sharing" reasoning for an instance identity).
  Exists purely to support `wasm-execution`'s new
  `FuncRefTarget::owner_instance_identity`/`effective_local_index`
  mechanism — see that crate's own CHANGELOG for the concrete recursion-
  depth regression it avoids.
- **`LocalFunctionRef`** (new, private struct, `HostFunction` impl) —
  wraps one of an instance's own functions (exported or not) for by-index
  dispatch, mirroring `wasm-conformance::CrossModuleFunction`'s own
  snapshot-at-construction pattern (`func_type`/`identity`/`group_shape`/
  `is_final`/`canonical_type`, all snapshotted from the owning instance at
  resolution time). `call` dispatches via the new `WasmRuntime::
  call_by_index`, not `call_typed` (works for non-exported functions too).
- **`resolve_func_ref_for_instance`** (new, `pub fn`) — the shared
  resolution logic `InstanceSelfResolver` delegates to; mirrors
  `wasm_execution::WasmExecutionContext::resolve_function_ref`'s own
  import/local split, usable before any `WasmExecutionContext` exists.
  `pub` (not merely `pub(crate)`) so a future slice-4 embedder can call it
  directly against its OWN permanent `Rc`.
- **`InstanceSelfResolver`** (new, `pub struct { pub instance:
  Rc<RefCell<WasmInstance>> }`, implements `wasm_execution::
  SelfFunctionResolver`) — `pub` for the identical forward-looking reason.
- **`WasmRuntime::call_by_index`** (new, `pub fn`) — exposes the
  already-existing private `call_engine` under a by-index (not
  by-export-name) contract. Purely additive: `call`/`call_typed`
  unchanged, both still delegate to the same `call_engine` internally.
- **`WasmInstance::globals: Vec<Rc<RefCell<GlobalStorage>>>`** (was
  `Vec<Rc<RefCell<WasmValue>>>`) — see `wasm-execution`'s own CHANGELOG
  (`GlobalStorage`, design §7) for the full rationale. `instantiate()`'s
  own global-construction loop wraps each computed value as `GlobalStorage
  { value, func_ref: None }` — a funcref-typed global's `ref.func`-
  produced initial value stays UNRESOLVED here too (same reasoning as
  elem segments above: no `Rc<RefCell<WasmInstance>>` exists yet at this
  point in construction, and even if it did, eagerly resolving it would
  hit the identical self-referential-cycle problem this slice's own
  headline deviation documents).

### Downstream ripple (mechanical)

- Every `HostInterface` impl in this crate's own module (`WasiStub`,
  `WasiEnv`) and test module (`TestHost`, `GroupShapeHost`,
  `CanonicalHost`, `TagTestHost`, `LinkingTestHost`, plus
  `tests/shared_memory_table_import.rs`, `tests/elem_before_data_
  ordering.rs`, `tests/shared_global_import.rs`'s own test-only hosts) —
  `resolve_global`'s return type follows `wasm-execution`'s
  `GlobalStorage` change.
- `tests/call_typed_with_v128.rs`'s own hand-built `WasmInstance` literal
  gains `func_identities`/`instance_identity` fields.
- `wasm-conformance::RegistryHost::resolve_global` and its own global-read
  `Action::Get` handler — see that crate's own CHANGELOG.
- `lang-aot`'s own test-only `HostInterface` doubles (`tests/lang_matrix.
  rs`, `tests/wasm_emit.rs`) needed the identical mechanical fix.

### New tests (verification plan)

- `instantiate_builds_func_identities_mirroring_tag_identities_imported_
  adopts_verbatim_module_defined_mints_fresh` — verification plan (a).
- `instantiate_mints_a_fresh_instance_identity_per_call_never_reused` —
  the `instance_identity` counterpart to the pre-existing tag-identity
  test of the same shape.
- `local_function_ref_dispatches_to_the_right_function_body_via_a_raw_
  index_unrelated_to_any_export` — verification plan (b): a genuinely
  UNEXPORTED function, resolved and dispatched purely by raw index.
- `resolve_func_ref_for_instance_of_an_imported_function_reuses_its_
  existing_identity_and_is_owner_agnostic` — the import-branch
  counterpart.
- `instance_self_resolver_installed_on_a_hand_built_engine_dispatches_a_
  local_function_via_ref_func_and_call_ref` — proves `InstanceSelfResolver`/
  `set_self_resolver` work end-to-end over a long-lived, test-owned `Rc`
  (never unwrapped — the self-cycle problem this slice's own headline
  deviation documents doesn't apply to a caller who never needs to
  recover a plain, owned value back out).
- `instantiate_never_panics_on_a_module_whose_own_active_elem_segment_
  references_its_own_local_functions` — verification plan (c), reworked:
  the ORIGINAL ask ("prove the two-phase `Rc::try_unwrap` construction
  never panics") no longer applies verbatim since that construction was
  removed; this test instead pins the CONCRETE regression that removal
  fixes — a real module in exactly the shape that broke the spec's own
  literal design instantiates cleanly and dispatches correctly.

### Verification

- `cargo build --workspace`: clean (see `wasm-execution`'s own CHANGELOG
  for the full downstream-consumer sweep).
- `cargo test -p wasm-runtime`: 66 lib tests (60 pre-existing + 6 new) +
  33 integration tests across 9 files, all passing. Verified via `git
  stash` A/B: every pre-existing test still passes unchanged.
- `cargo run --release --bin wasm_conformance_report -p wasm-conformance
  -- --write-baseline`: regenerated baseline byte-for-byte identical to
  the pre-slice-3 committed one — no corpus regression, and (per this
  slice's own scope: the real cross-instance fix is deferred to slice 4)
  no corpus improvement either, exactly as expected.
- `cargo clippy -p wasm-execution -p wasm-runtime -p wasm-validator -p
  wasm-conformance --all-targets -- -D warnings`: clean (required making
  `LocalFunctionRef`'s two supporting items `pub` — see above — since a
  private item genuinely unused by any non-test code trips `dead_code`
  under `-D warnings` without `--all-targets`, and this slice's own
  production code deliberately never constructs one).

## [0.6.25] — 2026-09-01 (W35 second slice — mechanical `TableElement` fallout)

Mechanical ripple from `wasm-execution` 0.9.88 (slice 2 of 4 for
`code/specs/W35-wasm-cross-instance-function-identity.md` — see that
crate's own CHANGELOG for the full rationale, including the deliberate
deviations this slice's design needed). No NEW cross-instance logic in
this crate — this slice's own scope explicitly excludes `WasmInstance::
func_identities`, `LocalFunctionRef`, `WasmRuntime::call_by_index`,
`GlobalStorage`, and active-elem-segment application's real declaring-
instance resolution (all W35 third slice).

- **`instantiate()`'s active-elem-segment application loop**: the one
  real call site in this crate that calls `Table::set` directly (applying
  an active elem segment's `Element.function_indices` entries) now wraps
  each entry as `TableElement::Raw(func_idx)` instead of passing the raw
  `Option<u32>` straight through — `Table::set`'s signature changed to
  `Option<TableElement>` (`wasm-execution`'s own §6 design, adapted for
  this codebase's genuine externref-table support — see that crate's
  CHANGELOG). Purely mechanical: the entry is stored UNRESOLVED
  (`Raw`), exactly matching this slice's own `table.init` opcode
  handler's identical choice — real resolution happens lazily, at
  `call_indirect`'s own read site, once a real `WasmExecutionContext`
  exists (it doesn't yet, here in `instantiate()`).

### Verification

See `wasm-execution`'s own CHANGELOG entry for this slice — `cargo test`/
`cargo clippy`/conformance-baseline verification covers both crates
together.

## [0.6.24] — 2026-09-01 (W35 first slice — `host_functions` moves from `Box` to `Rc`)

Mechanical ripple from `wasm-execution` 0.9.87 (slice 1 of 4 for
`code/specs/W35-wasm-cross-instance-function-identity.md` — see that
crate's own CHANGELOG for the full rationale). No behavior change in this
crate either.

- **`WasmInstance::host_functions`** and `WasmRuntime::instantiate`'s own
  local `host_functions` builder Vec: `Vec<Option<Box<dyn HostFunction>>>`
  → `Vec<Option<Rc<dyn HostFunction>>>` — round-trips through
  `wasm-execution::WasmEngineConfig`/`WasmEngineState`, both now `Rc`-based
  too, so this crate's own field had to follow.
- **`HostInterface::resolve_function`'s signature is UNCHANGED** — it
  still returns `Option<Box<dyn HostFunction>>` (out of this slice's
  scope per the spec's own call-site audit). The one real conversion this
  ripple needed: `instantiate()`'s import-resolution loop now does
  `host_functions.push(Some(Rc::from(host_func)))` instead of `Some
  (host_func)` — `Rc::from(Box<dyn HostFunction>)` is a standard, safe,
  allocation-cheap conversion (no re-boxing; `Rc` takes ownership of the
  same heap allocation `Box` already had).
- `tests/call_typed_with_v128.rs` hand-builds a `WasmInstance` directly
  (every field `pub`) — its own `host_functions: Vec<Option<Box<dyn
  HostFunction>>>` type annotation updated to `Rc` to match.

### Verification

See `wasm-execution`'s own CHANGELOG entry for this slice — it covers
`cargo test`/`cargo clippy`/conformance-baseline verification for both
crates together (run and diffed jointly, since the change ripples across
both).

## [0.6.23] — 2026-09-01 (test fixture update for `wasm-types`' new `missing_data_count_section` field)

No functional change in this crate. `wasm-types` 0.1.23 added
`WasmModule::missing_data_count_section` (see that crate's own
CHANGELOG). Two hand-built `WasmModule` struct literals in `tests/
v128_persistent_storage.rs` named every field explicitly (no `..Default::
default()`) — updated to add the new field (`false`, matching every
other field's already-default-equivalent value there).

## [0.6.22] — 2026-09-01 (W-next — cross-module linking value-corruption bug-hunt: two real, distinct bugs)

A fresh prioritization pass over the conformance corpus's module-linking
cluster (`elem.wast`/`instance.wast`/`linking.wast`/`linking0.wast`/
`linking3.wast`) surfaced a set of wrong-VALUE `assert_return` failures —
not `NotYetSupported` gaps — and one spurious trap. Investigated from
first principles (per this session's own working discipline: verify, do
not assume it's the same shape as a prior bug just because the symptom
looks similar). Two genuinely distinct, real bugs were found and fixed;
a third, pre-existing, already-documented gap was confirmed (not fixed)
as the sole remaining cause of everything else in this cluster.

### Fixed

- **Mutable globals were never made cross-instance-shared the way W28
  already made memory/table.** `instantiate()`'s `ImportTypeInfo::Global`
  arm pushed whatever `HostInterface::resolve_global` returned straight
  into `globals: Vec<WasmValue>` — correct when that was a plain value
  copy of an IMMUTABLE global, silently wrong for a MUTABLE one (see
  `wasm-execution`'s own CHANGELOG for the full field-level fix:
  `WasmInstance::globals` is `Vec<Rc<RefCell<WasmValue>>>` now). No
  change was needed at this exact push site — once `resolve_global`
  itself hands back a shared cell, pushing it as-is is already correct —
  but the global-initializer loop (`evaluate_const_expr_gc`) and the
  element/data segment offset-expression evaluation both needed a fresh
  `Vec<WasmValue>` SNAPSHOT derived from the (now cell-backed) `globals`
  at each point they read "the globals defined so far," since
  `evaluate_const_expr`/`_gc` themselves still take a plain `&[WasmValue]`
  slice (unchanged, deliberately — see that function's own doc comment).
- **Active element segments were applied AFTER active data segments, not
  before** — the exact opposite of the official spec's own instantiation
  algorithm order (element-segment initializers execute strictly before
  data-segment initializers; verified against the spec text directly, not
  assumed). Invisible until a module combines an in-bounds active element
  segment with a data segment that traps: the elem write, which must
  already be applied and PERSIST past the later data-segment trap (the
  same "earlier segments persist past a later trap" rule this crate's own
  per-segment atomicity fix — W28, CHANGELOG 0.6.13 — already established
  WITHIN one kind, here needed ACROSS kinds), was silently lost because
  the wrongly-ordered data loop's `?` returned before the elem loop ever
  ran. Fixed by swapping the two loops in `instantiate()` — no change to
  either loop's own internal per-segment-atomicity/bounds-checking logic,
  only their relative order. See `code/specs/
  W10-wasm-real-linking-and-unlinkable.md`'s second addendum for the full
  writeup.

### Confirmed, not fixed (pre-existing, already-documented gap)

`elem.wast`, `linking.wast`, and `linking3.wast` each still have wrong-
value `assert_return` failures after both fixes above — traced
individually (not assumed) to the SAME already-documented, deliberately
out-of-scope gap: a table entry is a bare `u32` function index, resolved
against whichever instance's OWN function-index space is CURRENTLY
executing `call_indirect`/`table.get`, regardless of which instance's
active elem segment (or `table.set`) originally wrote it. A funcref
written into a genuinely-SHARED table (W28) by one module and read back
through a DIFFERENT module's own function-index space resolves to
whichever LOCAL function happens to sit at that same raw index in the
READING instance — sometimes numerically identical by coincidence
(explaining `linking.wast`'s "4 and -4 swapped" symptom this bug-hunt
started from: two functions at colliding raw indices in two different
instances' own local function tables). Needs real cross-instance function
IDENTITY for table entries (the same class of problem `WasmInstance::
tag_identities`, W23, already solves for exception tags, but requiring
genuine cross-instance CALL DISPATCH, not just equality comparison) — see
`Table`'s own doc comment in `wasm-execution` and this crate's own W10
spec addendum for the full design gap. Not designed or implemented here,
consistent with this repo's standing "no broad rewrites" discipline: it
would touch `WasmValue::Ref`'s representation and every table/`ref.func`/
`call_indirect` consumer, not just the specific wrong-clone/wrong-index
site this pass was scoped to fix.

`linking0.wast`'s own one remaining failure, by contrast, was fully
explained and fixed by the elem/data ordering bug above — it no longer
exhibits the table-identity gap at all after this fix.

### Tests

- `tests/shared_global_import.rs` (new): pins the mutable-global-sharing
  fix directly (bypassing the `.wast` harness, same style as `tests/
  shared_memory_table_import.rs`) — a cross-instance import case
  (`global_set_through_an_imported_mutable_global_is_visible_in_the_
  exporting_instance`) and a same-instance double-import case
  (`two_imports_of_the_same_mutable_global_are_the_same_cell_not_
  independent_copies`, `instance.wast`'s own "Import is not generative"
  shape reduced to its essence — no cross-instance re-export chain even
  needed to reproduce it).
- `tests/elem_before_data_ordering.rs` (new): pins the elem/data ordering
  fix directly — an in-bounds active element segment into a shared,
  imported table must persist even though a separately-declared,
  out-of-bounds active data segment traps the rest of that same
  `instantiate()` call. Deliberately asserts on the table's raw entry
  (`Table::get`) rather than `call_indirect`-ing through it: WHICH
  function ends up at that slot is the separate, still-open
  cross-instance identity gap above, not this fix's own concern.

### Corpus impact — real, measured, programmatically diffed, not asserted

`--write-baseline` was re-run and diffed programmatically (Python,
comparing the `files` dict in `tests/fixtures/testsuite-status.json`
keyed by filename) against the pre-fix baseline across all 257 files.
Exactly 2 files changed, both strict improvements, zero regressions
anywhere in the corpus:

- **`instance.wast`**: `assert_return` 10/12 (2 fail) → 12/12 (0 fail).
  Both previously-failing cases are the "Import is not generative" tests
  (two imports of the same mutable global under different local names)
  — the global-sharing fix above.
- **`linking.wast`**: `assert_return` 54/65 (11 fail) → 55/65 (10 fail).
  The one newly-passing case is the `mut_glob` re-export chain
  (`$Mg`/`$Ng`, `(assert_return (get $Ng "Mg.mut_glob") (i32.const
  241))`) — the SAME global-sharing fix, this time through an actual
  cross-instance re-export rather than a same-instance double import.
  `linking.wast`'s remaining 10 `assert_return` failures are every one
  the confirmed, not-fixed table-identity gap above (verified
  individually by tracing each one's own module/elem-segment shape, not
  assumed from the tally alone).
- **`elem.wast`, `linking0.wast`, `linking3.wast`**: tallies UNCHANGED.
  `elem.wast`/`linking3.wast`'s failures were already, and remain, the
  table-identity gap (confirmed unaffected by either fix — direct
  `wasm_conformance::run_wast_source()` probing of each specific failing
  directive, not just the aggregate count, before AND after). `linking0.
  wast`'s tally is ALSO unchanged (`assert_return` stays 0/1) despite its
  underlying bug being genuinely fixed — before this fix its one failing
  case was a spurious `TrapError` ("uninitialized table element"); after,
  it fails differently, now on a WRONG VALUE from the confirmed
  table-identity gap (traced directly: the failing module's own elem
  write is a raw local-function index that happens to collide with an
  unrelated local function's index in the READING instance) — the tally
  bucket (`fail`) doesn't distinguish "why" it failed, only "did it
  fail," so this is invisible in the JSON diff and was caught only by
  re-probing the specific directive's error message before/after.

## [0.6.21] — 2026-09-01 (W34 fourth slice — cross-module canonical equivalence, epic closed)

`instantiate`'s function-import compatibility check (`ImportTypeInfo::
Function` arm) now uses real canonical type-group equivalence for
CROSS-MODULE linking, closing this epic's final gap (`code/specs/
W34-wasm-gc-canonical-type-equivalence.md`): the last remaining decision
point in this crate's WASM type system that still compared two
independently-validated modules' types with no canonical-equivalence
awareness at all.

- When BOTH the importing module's own declared type
  (`canonical_types[type_idx]`, already computed once at the top of
  `instantiate` from `ValidatedModule::canonical_types()`) and the
  exporting `HostFunction`'s own `canonical_type()` report a real
  canonical identity, the check now calls `host_func.canonically_matches
  (&module_ct)` — the real GC-proposal rule (canonical equivalence OR a
  declared nominal subtype, climbing the EXPORTER's own local `sub`
  chain) — and REPLACES the pre-existing three-part conservative guard
  entirely for that import. This SUBSUMES the old guard's own `(rec_
  group_size, rec_group_position)`/`is_final`/raw-`FuncType`-equality
  checks (all three are already folded into a `CanonicalSubtype`'s own
  fields) while additionally: (a) ACCEPTING an isomorphic-but-differently-
  numbered `rec` group import the old guard couldn't recognize as the
  same type (`type-equivalence.wast`'s own "Semantic types (link time)"
  section), (b) ACCEPTING a genuine nominal-subtype import — an export
  whose declared type is a `sub`-chain ancestor's canonical match, not its
  own exact type (`type-subtyping.wast`'s own `M6`/`M7` "Linking" cases —
  see the note below on why this needed a SECOND `HostFunction` method,
  not just the first), and (c) STILL REJECTING a genuine topology mismatch
  the old guard was blind to (`type-subtyping.wast`'s `M5`/`M10`/`M11`
  cases — see "Corpus impact" below).
- When EITHER side reports no real canonical identity (`None` — e.g. any
  WASI-shim host import, or a type this crate's canonicalizer couldn't
  resolve), the check falls back to the pre-existing three-part
  conservative guard, UNCHANGED, verbatim — every such import is
  byte-for-byte unaffected by this slice.
- **A real regression found and fixed mid-implementation, not assumed
  correct from the design sketch**: the spec's own Design §4 described the
  cross-module check as a plain canonical-equivalence comparison. Building
  it that way and re-running the full conformance baseline (this
  campaign's own "diff after every change" discipline) surfaced two new
  `NotYetSupported` regressions in `type-subtyping.wast` (`M6`/`M7`, whose
  own "Linking" cases rely on a func import being satisfied by an export
  whose declared type is a NOMINAL SUBTYPE of the import's declared type,
  not merely canonically equivalent to it — real WASM func-import matching
  is a subtyping relation, per MVP.md's own external-type-matching rule).
  Root-caused by tracing exactly which corpus directive regressed and why,
  fixed by adding `HostFunction::canonically_matches` (see `wasm-
  execution`'s own CHANGELOG) instead of comparing `canonical_type()`
  values directly — re-running the baseline confirmed both cases restored
  to `Pass` with no new regressions anywhere else across all 257 files.

### Corpus impact — real, measured, individually investigated, not
asserted

`--write-baseline` was re-run and diffed programmatically against the
pre-slice baseline across all 257 files. Exactly 2 files changed, 5
directive-level improvements, ZERO regressions:

| File | Category | Before | After |
|---|---|---|---|
| `type-equivalence.wast` | `module` | 20 pass / 1 NYS | 21 pass / 0 NYS |
| `type-subtyping.wast` | `assert_unlinkable` | 5 pass / 3 fail | 8 pass / 0 fail |
| `type-subtyping.wast` | `module` | 36 pass / 10 NYS | 37 pass / 9 NYS |

Every changed tally individually investigated, not just summed:

- `type-equivalence.wast`'s one flipped `module` directive is the
  "Indirect types" link-time case (`N.f1` imported at type `$t2`, whose
  params/results reference `$s1`/`$s2` — themselves canonically identical,
  byte-for-byte, but at swapped raw indices relative to `$t1`): plain raw
  `FuncType` equality (comparing `$s1`'s vs `$s2`'s raw index INSIDE the
  param/result types) rejected this before; real canonical equivalence,
  which resolves those inner references to their tied form before
  comparing, correctly accepts it.
- `type-subtyping.wast`'s 3 flipped `assert_unlinkable` cases are exactly
  the `M5`/`M10`/`M11` "Linking" cases this epic's THIRD slice's own
  addendum named as present "since the first slice" (traced individually
  via a throwaway per-directive debug harness, confirmed to fail with
  reason `"module linked successfully; expected unlinkable"` before this
  slice — i.e. the OLD three-part guard coincidentally accepted every one
  of them, since their `(rec_group_size, rec_group_position)`/`is_final`/
  raw-`FuncType`-shape all happen to match despite a genuine internal
  topology mismatch the old guard structurally cannot see). All three are
  now correctly rejected via real canonical equivalence.
- `type-subtyping.wast`'s one flipped `module` directive is the `M`-
  registration block's own `f1`/`f2` imports (lines 551-562): these rely
  on `$t0 <: $t1 <: $t2`'s declared nominal chain, which is exactly what
  `canonically_matches`'s chain-climbing (not plain equivalence) newly
  supports for cross-module imports.
- The other 9 `not_yet_supported` `module` entries in `type-subtyping.wast`
  are UNCHANGED and individually re-confirmed unrelated to this slice: 7
  are the pre-existing non-null-abstract-heap-type-as-result-type
  `wasm-wast-parser` gap (predates W34 entirely), and 2 are genuinely
  unresolvable structural gaps this slice's own scope does not touch.
- The 2 pre-existing `module.fail` entries (unchanged, both before and
  after) live in `br_table.wast`/`table.wast` — confirmed unrelated to
  type/canonicalization at all.
- No other file's tally changed at all, confirmed by diffing all 257
  files' full tallies programmatically.

### Security review

A dedicated security-review sub-agent was briefed on this slice's diff
(`git diff origin/main...HEAD`), specifically asked about the NEW trust
boundary this slice opens (the first time this epic compares two
INDEPENDENTLY-ATTACKER-CONTROLLED modules' type data against each other,
rather than one module's own data against itself): whether a maliciously
large or deeply-structured type in ONE module could cause disproportionate
comparison cost against a module importing from it (a cross-module
amplification distinct from the third slice's already-fixed single-module
one), and whether the comparison correctly handles a module importing
from itself or a cyclic import chain without infinite recursion.

**Result: one real HIGH-severity finding, fixed before push** — this
epic's FOURTH consecutive slice with a genuine finding in its own review.
`MAX_CANONICAL_TREE_WEIGHT` bounds any ONE `CanonicalGroup` tree's shape
at construction time, so any ONE full structural comparison between two
such trees is itself bounded — but nothing previously bounded how many
times a full, near-max-weight comparison could be ATTEMPTED across an
entire `instantiate()` call's whole import-resolution loop. Unlike the
WITHIN-module case (where `canonicalize_types`'s own interning makes
every SAME-shape comparison an O(1) `Rc::ptr_eq` hit, and triggering many
DIFFERENT expensive comparisons requires declaring that many expensive
types — itself bounded by module size), the cross-module case can never
hit `Rc::ptr_eq` at all (two different modules' `canonicalize_types`
calls never intern into the same allocation), and an attacker who
controls both the importing and exporting module can multiply one
expensive-but-capped comparison by an arbitrary, BYTE-CHEAP import count
— each `(func (import "M" "f") (type $expensive))` costs only a few
bytes to declare, unlike the expensive type itself. Worst case: `imports
× hops (≤1,000) × per-comparison cost (≤1,000,000 nodes)`, each factor
individually capped but their PRODUCT unbounded by anything that existed
before this fix.

Fixed by introducing `wasm_types::CrossModuleComparisonBudget`: a shared,
mutable work counter created ONCE per `instantiate()` call (before the
import-resolution loop starts, not per import) and threaded `&mut`
through the whole loop via a new parameter on `HostFunction::
canonically_matches`. A hand-written, budget-aware structural-equality
walk (`canonical_type_entries_equivalent_budgeted` and its per-field/
per-variant helpers in `wasm-types`) replaces derived `PartialEq`
specifically on the cross-module comparison path, charging one unit per
tree node visited (budget checked BEFORE recursing into any child, so
exhaustion can never be preceded by uncharged work) and failing CLOSED
(reports "not equivalent," never a false accept) once exhausted — the
same direction every other cap in this mechanism already takes. See
`wasm-types`'s own CHANGELOG for the full account, including the
re-review that confirmed the fix's completeness (every field/variant of
`CanonicalGroup`'s tree is covered by the budgeted walk, budget-before-
work ordering holds in all seven helper functions, and the budget
arithmetic uses `checked_sub` with no overflow/panic path). Full 257-file
conformance baseline re-confirmed byte-for-byte identical before and
after this fix, since it changes worst-case performance only, never
behavior.

The self-import/cyclic-import-chain question came back clean on first
review: `wasm-conformance`'s own sequential register-after-instantiate
model (a module is only inserted into the registry AFTER its own
`instantiate()` call returns successfully) makes both a self-import and a
genuine A↔B import cycle structurally impossible to reach — a module
being instantiated cannot yet resolve an import back to itself or to a
not-yet-completed sibling, so this fails as an ordinary "unknown import"
link error, never a `RefCell` double-borrow or infinite recursion.

## [0.6.20] — 2026-09-01 (W34 third slice — thread canonical equivalence into wasm-execution)

- `WasmInstance` gains a `canonical_types` field, cloned once at
  `instantiate()` time from the new `ValidatedModule::canonical_types()`
  accessor (`wasm-validator`'s own already-computed cache — nothing new
  computed here). Threaded into `wasm-execution` by `build_engine` via
  `WasmExecutionEngine::set_canonical_types`, the same optional-setter
  pattern `type_subtyping`/`set_type_subtyping` already use, so `call_
  indirect`/`ref.cast`/`ref.test`'s real runtime dispatch (see
  `wasm-execution`'s own CHANGELOG for the dispatch-side fix this unlocks)
  has real canonical data to work with for every module this crate
  instantiates.
- One test fixture (`tests/call_typed_with_v128.rs`, which hand-builds a
  `WasmInstance` directly rather than parsing WAT) updated for the new
  field.

## [0.6.19] — 2026-09-01 (W33 fourth slice — struct/array runtime wiring + persistent GC heap)

- `struct_array_runtime_tables` (a new shared helper) rebuilds
  `struct_field_counts`/`struct_field_storage`/`array_element_storage`
  on top of `WasmModule::struct_type_at`/`array_type_at`
  (`type_kinds`-aware) instead of the old "pad `func_type_count` zeros,
  then append every struct's field count in `struct_types` order"
  scheme, which assumed struct types always follow ALL function types —
  exactly what a TEXT-format module (via `wasm-wast-parser`'s now-real
  struct/array declarations) is free to violate.
- `WasmInstance` gains a persistent `gc_heap` field, threaded through
  instantiation/`build_engine`/post-call writeback exactly like
  `v128_heap` already is — needed because a GLOBAL initializer's
  `struct.new`/`array.new` (evaluated via the new
  `evaluate_const_expr_gc`) must survive past the instantiation call
  that created it, into a later, separate `call()` that reads it back
  via `global.get` (`struct.wast`'s own "Packed field instructions"
  module does exactly this).

All existing tests pass; one test fixture (`call_typed_with_v128.rs`)
updated for the new `WasmInstance` field.

## [0.6.18] — 2026-08-31 (W33 second slice — thread subtyping info into the engine)

### Added

- **`WasmInstance::func_type_indices: Vec<u32>`**: each function's own
  declared type-SECTION index, combined imported+module-defined index
  space (parallel to `func_types`, which holds the resolved `FuncType`
  SHAPE instead). Populated in `instantiate()` alongside `func_types`
  itself (same two loops, same order).
- **`build_engine` now calls `wasm-execution`'s new `set_type_subtyping`/
  `set_func_type_indices`** (threading `instance.module.type_subtyping`
  and the new `instance.func_type_indices` through), alongside the
  pre-existing `set_type_section` call — giving `call_indirect`'s real
  subtype check and `ref.cast`/`ref.test`'s dynamic type check
  (`wasm-execution` 0.9.81, W33 second slice, item 4) what they need for
  every real instantiated module. See that crate's own changelog for the
  runtime-side consumer.

## [0.6.17] — 2026-08-31 (W33 first slice — cross-module `rec`-group guard)

Adding `(rec ...)` group parsing (`wasm-wast-parser` 0.1.89) makes modules
that use it newly BUILDABLE — including ones whose cross-module
import/tag linking `instantiate()` now actually reaches. This crate's
pre-existing import-compatibility check compares only a plain `FuncType`
shape, which is blind to `rec`-group POSITION: two structurally-identical
members of a `rec` group at different positions (e.g. `tag.wast`'s own
`(rec (type $t1 (func)) (type $t2 (func)))`) are DISTINCT types under the
real GC canonicalization algorithm, but would wrongly compare equal here.
Full canonical type-group equivalence (`code/specs/
W33-wasm-gc-recursive-type-subtyping.md`'s own item 3b) is out of scope
for this slice — but leaving the gap unguarded would have let some newly-
reachable `assert_unlinkable` corpus cases wrongly LINK once `rec` groups
parse, a real regression the full-corpus baseline diff would have caught.

### Added

- **Function and tag import compatibility now also requires a matching
  `(rec_group_size, rec_group_position)`** (`wasm_types::WasmModule::
  type_group_shape`, ANDed onto the pre-existing `FuncType`/tag-type
  structural equality check, never replacing it): `instantiate()`'s
  `ImportTypeInfo::Function`/`ImportTypeInfo::Tag` arms now ask the host
  (`HostFunction::type_group_shape`/`HostInterface::
  resolve_tag_group_shape`, both new `wasm-execution` 0.9.80 default
  methods returning `(1, 0)`) and compare against the importing module's
  own declared shape. Safe for every PRE-EXISTING import (both sides
  trivially report the singleton-group default): can only ADD a
  rejection on top of the existing check, never remove one — verified via
  the full-corpus baseline diff showing zero regressions (see
  `wasm-conformance`'s own changelog for the exact tallies).
- **Function and tag import compatibility ALSO requires matching
  finality** (`HostFunction::is_final`/`HostInterface::
  resolve_tag_is_final`, both new `wasm-execution` 0.9.80 default methods
  returning `true`): `(sub (func))` (open) and `(sub final (func))`
  (final) are structurally identical `FuncType`s yet distinct canonical
  types — this fixed 2 of the 4 new `assert_unlinkable` fails the initial
  `rec`-group-shape-only guard left behind (`type-subtyping.wast` lines
  594-617's finality-mismatch pair); see `wasm-conformance`'s own
  changelog for the full before/after accounting, including the 2
  remaining fails (`type-subtyping.wast`'s M10/M11 linking pair) this
  guard does NOT catch — confirmed to need real cross-module canonical
  type-group equivalence (item 3b), not a shallow shape/finality check.
- Four new unit tests (two for the `rec`-group-shape guard, two for the
  finality guard) exercising both directly, independent of the full
  corpus.

## [0.6.16] — 2026-08-31 (W32 second slice — non-null concrete reference types)

### Added

- `call()`'s argument-conversion match gained arms for the two new
  non-null concrete-ref variants (`NonNullStructRef`/
  `NonNullConcreteFuncRef`, `wasm-types` 0.1.14) — required just to keep
  this exhaustive match compiling; joins the same lossy-legacy-path
  placeholder group as the pre-existing GC/funcref/externref/exnref arms.
  See `code/specs/W32-wasm-non-null-concrete-reference-types.md`.

## [0.6.15] — 2026-08-31 (W32 first slice — bottom reference types)

### Added

- `call()`'s argument-conversion match gained arms for the four new
  bottom reference types (`NullFuncref`/`NullExternref`/`NullExnref`/
  `NullRef`, `wasm-types` 0.1.13) — required just to keep this exhaustive
  match compiling; joins the same lossy-legacy-path placeholder group as
  the pre-existing GC/funcref/externref/exnref arms (no vendored corpus
  directive passes one as a top-level `invoke` argument either). See
  `code/specs/W32-wasm-non-null-concrete-reference-types.md`.

## [0.6.14] — 2026-08-26 (W26 follow-up — real table64 operations)

### Fixed

- **Real, pre-existing bug: active ELEMENT segment application ignored
  the target table's own `is64`.** `instantiate()`'s active
  element-segment-application loop unconditionally evaluated the
  segment's offset expression as `i32`, even though the sibling active
  DATA segment branch immediately above it was already correctly
  `is64`-aware (W25) — an active element segment targeting an `is64`
  table trapped instantiation (`expected I64, found I32`) instead of
  applying. Found via the real `call_indirect64.wast` corpus (its
  `(table $t64 i64 funcref (elem $const-i32))` shorthand hit exactly
  this). Fixed to branch on the target table's own `Table::is64()`,
  matching the data-segment code exactly; kept in `u64` throughout
  (narrowing to `u32` only AFTER the upfront whole-segment bounds check),
  same "never narrow before checking" discipline `wasm-execution`'s own
  `table_u64_to_u32` helper documents.

### Tests

- New `active_element_segment_on_an_is64_table_applies_at_instantiation_time`
  end-to-end test (`tests/table_init_copy_elem_drop.rs`): instantiates a
  module with an active element segment on an `is64` table and confirms
  `call_indirect` reaches the right functions.

## [0.6.13] — 2026-08-26 (W28 — real cross-instance shared memory/table + atomic elem-segment application)

### Fixed

- **Imported memory/table is now a genuine shared live view, not a clone**
  — depends on `wasm-execution` 0.9.73's `LinearMemory`/`Table` becoming
  `Rc<RefCell<..>>`-backed (see that crate's own CHANGELOG for the full
  rationale). This crate's own `instantiate()` needed NO code change for
  this half of the fix: it already just pushes whatever `HostInterface::
  resolve_memory`/`resolve_table` hands back straight into `WasmInstance::
  memories`/`tables`, so once those values genuinely share storage on
  `.clone()`, this crate's existing import-resolution path is correct for
  free.
- **Active element-segment application is now atomic per segment.** The
  `for elem in &module.elements` loop in `instantiate()` used to call
  `table.set(offset + j, func_idx)?` once per entry, propagating the
  first out-of-bounds error via `?` as soon as ONE index in the segment
  was out of range — but by then, every EARLIER entry in that same
  segment had already been written. This is a real, spec-violating bug
  (a single active segment must be all-or-nothing; only *earlier,
  already-applied* segments persist past a *later* segment's trap, not
  partial entries WITHIN one segment) that was completely unobservable
  before this same PR's shared-storage fix: a failed `instantiate()`
  call's local `tables` Vec was simply dropped on error, so a partial
  write vanished regardless of whether the table was a fresh local one or
  an independently-CLONED import. It became observable the moment a
  table's storage started being genuinely SHARED across instances — a
  partial write to a shared table now persists in the exporting
  instance's own storage even though the importing instance's
  `instantiate()` call fails, exactly the shape `linking.wast`'s own
  already-vendored `assert_trap` directives probe. Fixed by
  bounds-checking the WHOLE segment (`offset + segment.len() <=
  table.size()`) BEFORE writing any entry, mirroring `LinearMemory::
  write_bytes`'s existing upfront-bounds-check-then-one-write shape.
  Confirmed via `linking.wast`'s own tally: without this second fix, its
  `assert_trap` count regressed 18/18 -> 17/18 the moment the storage-
  sharing fix alone landed; with both fixes together, it's back to 18/18
  AND `assert_return` improved 48/65 -> 54/65, with zero regressions
  anywhere else in the 216-file corpus (programmatically diffed
  baseline-to-baseline).
- **Known, deliberately out-of-scope remaining gap:** `call_indirect`
  still resolves a table entry (a bare `u32` function index) against the
  CALLING instance's own function-index space — correct within one
  instance, but not a genuine cross-instance funcref identity. See
  `wasm-execution`'s `Table` doc comment for the full explanation; two
  `assert_return` directives in the newly-vendored `linking0.wast`/
  `linking3.wast` (see `wasm-conformance`'s CHANGELOG) hit exactly this
  gap and are the only real `fail`s introduced by vendoring those files.

### Added

- **`wasm-runtime/tests/shared_memory_table_import.rs`** — three new
  integration tests, each building a two-instance import scenario (module
  A exports a memory/table, module B imports it) directly against this
  crate's own `WasmRuntime`/`HostInterface`, bypassing the `.wast` corpus
  entirely: a write through B's imported memory is visible via A's own
  `read` export; a `memory.grow` through B's import is visible via A's
  own `memory.size`; a `table.grow` through B's imported table is visible
  via A's own `table.size`. All three FAIL against the pre-fix code (the
  clone-not-share bug reproduced directly, not just inferred from corpus
  numbers) and PASS after it.

## [0.6.12] — 2026-08-26 (W27 — census batch: multi-memory data segments + start function)

### Fixed

- **Active data segments now apply to their OWN `seg.memory_index`, not
  unconditionally memory 0.** `instantiate()`'s data-segment loop used
  to grab `memories.first_mut()` once and apply every non-passive
  segment to it; it now looks up `memories.get_mut(seg.memory_index as
  usize)` per segment, and resolves that segment's `i32.const`-vs-
  `i64.const` offset-expression width from the TARGET memory's own
  `is64`-ness (previously always memory 0's). `wasm-validator` 0.2.71
  bounds-checks `seg.memory_index` before this ever runs, so the
  `continue` fallback for a not-found index is defensive only.
- **A module's `start` function is now actually invoked.** `module.start`
  (parsed and carried on `WasmModule` since `wasm-wast-parser`'s own
  `"start"` build arm) was never read anywhere in this crate —
  `instantiate()` now calls it, via the same `call_engine` plumbing an
  ordinary export call uses, as the LAST step of instantiation, exactly
  once, only if present. A start-function trap surfaces through
  `instantiate()`'s existing `Err` path, same as any other
  instantiation-time fault.
- Real corpus impact: unblocks `start.wast`/`start0.wast` outright;
  `linking.wast` (already vendored) has exercised the missing-start-
  invocation gap all along in its own `assert_return` tally, though a
  full before/after baseline diff confirms neither this fix nor the
  multi-memory one above moved that specific file's numbers (its
  remaining fails share cross-instance-import root causes tracked
  separately — see `wasm-conformance`'s own CHANGELOG skip list).

## [0.6.11] — 2026-08-26 (W11 addendum — concrete function-type refs)

### Changed

- `call()`'s legacy i64 param-conversion path gained a
  `ValueType::ConcreteFuncRef(_)` arm (same lossy `WasmValue::I32`
  placeholder every other reference type already gets there — see
  `wasm-types` 0.1.12's `ConcreteFuncRef`). No behavior change for any
  existing type; needed only to keep this exhaustive match compiling.

## [0.6.10] — 2026-08-26 (W26 — table64 proposal, first slice)

### Changed

- `instantiate()`'s module-declared-table allocation now calls
  `Table::new_with_is64` (fallible) instead of an outright truncating `as
  u32` cast on `table_type.limits.min` — the cast was a real, previously
  latent correctness bug: an `is64` table's spec-valid `min` (up to
  `u64::MAX`, per W26) is now reachable past `u32::MAX` for the first
  time, and would have silently produced a wrong-sized table instead of
  failing loudly. Returns a real, graceful `TrapError` (never a panic) if
  an `is64` table's `min` exceeds `wasm_execution::MAX_TABLE_ELEMENTS`,
  this interpreter's own practical instantiation-time cap.
- Table-import linking gains an `is64` mismatch check (`if
  imported_table.is64() != table_type.is64 { ... }`), checked before
  `limits_compatible`, mirroring the existing memory-import arm's own
  `is64` check exactly (W25).
- **Security review**: `instantiate()`'s table-allocation loop gains a
  `total_is64_table_elements` aggregate cap across every `is64` table in
  the module, mirroring `total_is64_pages` (memory64, W25) — without it,
  a module could declare up to `MAX_TABLES` (64) separate `is64` tables
  each individually AT the per-table `MAX_TABLE_ELEMENTS` cap (10,000,000)
  and still instantiate all of them, ~5.1GB of eager allocation from one
  small module (the exact "many individually-under-cap tables still
  totaling too much" shape `wasm-validator`'s own Check 2b comment already
  names as the reason its 32-bit aggregate exists — `wasm-validator`
  deliberately excludes `is64` tables from THAT aggregate, since an
  `is64` table's real spec ceiling has no useful per-item bound to
  aggregate from at validation time, so the aggregate has to live here,
  at instantiation, instead). Uses `saturating_add`, not `+=`: unlike
  `total_is64_pages` (whose addends are already capped at memory64's much
  smaller `2^48`-page validator ceiling), an `is64` table's `min` is
  validator-uncapped up to `u64::MAX` itself — a plain `+=` could wrap the
  running total back under the cap in a release build and defeat the
  check outright.

See `code/specs/W26-wasm-table64-first-slice.md`.

## [0.6.9] — 2026-08-26 (W25 — memory64 proposal, first slice)

### Changed

- `instantiate()`'s module-defined-memory allocation now calls
  `LinearMemory::new_with_is64` (fallible), tracking a running total of
  every `is64` memory's declared `min` pages and rejecting (a real,
  graceful `TrapError`, not a panic) if the total exceeds
  `wasm_execution::MAX_MEMORY64_INITIAL_PAGES` — the same "many
  individually-under-cap memories still summing to too much" aggregate
  reasoning `wasm-validator`'s Check 1b already applies to 32-bit
  memories, applied here for `is64` ones at the point where real
  allocation actually happens (`wasm-validator`'s own spec-conformance
  ceiling for `is64` is `2^48` pages — far larger than any real system
  will actually back with allocated bytes, so this repo's OWN practical
  resource limit lives here, at instantiation, not at validation).
- The active-data-segment offset evaluation (previously hardcoded
  `.as_i32()`) now checks memory 0's `is64` and calls `.as_i64()`
  instead, matching `wasm-wast-parser` emitting an `i64.const` offset
  expression for a 64-bit memory's data segments.
- Import-compatibility checking for a memory import now also rejects an
  `is64` mismatch between the actual memory and the declared import type
  (previously uncheckable at all, since both sides' `Limits` are `u64`
  regardless of `is64` — a mismatch wouldn't otherwise be caught).
- `wasm_types::Limits.min`/`max` widened to `u64` (`wasm-types` 0.1.10):
  `limits_compatible` and every `Limits`/`Table::new` construction site
  updated to match (tables narrow back to `u32` — safe, since `table64`
  is a separate, out-of-scope proposal and no real `TableType` this
  crate builds sets a value outside `u32`'s range).

See `code/specs/W25-wasm-memory64-first-slice.md`.

## [0.6.8] — 2026-08-26 (W23 — exceptions proposal, cross-instance tag identity)

### Added

- `WasmInstance::tag_identities: Vec<u64>` (new field): a canonical,
  cross-instance-safe identity per tag, same combined imported+defined
  index space as `tags`. A module-DEFINED tag gets a freshly minted,
  never-repeating identity (the new process-wide `NEXT_TAG_IDENTITY`
  counter) exactly once, at `instantiate()` time — persists across every
  later call on the same instance, unlike `wasm_execution::
  WasmExecutionContext::instance_id` (reminted every top-level call). An
  IMPORTED tag adopts the identity `HostInterface::resolve_tag` returns
  for it verbatim, rather than minting an unrelated new one.
- `build_engine` threads it into the execution engine via the new
  `wasm_execution::WasmExecutionEngine::set_tag_identities`, mirroring
  `set_tags` exactly.

### Changed

- `HostInterface::resolve_tag`'s return type changes from
  `Option<FuncType>` to `Option<(FuncType, u64)>` (see `wasm-execution`'s
  own changelog) — `instantiate()`'s `ImportTypeInfo::Tag` arm now reads
  both the type (link-compatibility check, unchanged) and the identity
  (adopted into `tag_identities`).

### Fixed

- This is what makes a `throw` in one module instance catchable by a
  `try_table` in another instance that imported the SAME tag (via
  `register`/module linking) — see `code/specs/
  W23-wasm-exceptions-cross-instance-tag-identity.md` for the full
  investigation and `wasm-conformance`'s changelog for the measured
  corpus win.

## [0.6.7] — 2026-08-25 (W22 — exceptions proposal, real catch/catch_all matching)

### Added

- `HostInterface::resolve_tag` (new, default `None` so existing
  implementors keep compiling unchanged) — resolves an imported tag's
  real declared type. `instantiate()`'s `ImportTypeInfo::Tag` arm
  (previously an unconditional link failure, W21) now asks the host for
  it and checks compatibility against the importing module's own
  declaration, exactly like `Function` imports already do.
- `WasmInstance::tags: Vec<u32>` (new field): the COMBINED
  imported+defined tag index space ("imports first, then declared"),
  built during `instantiate()` the same way `func_types` already is.

### Fixed

- A real, previously-latent bug: `build_engine` was passing
  `instance.module.tags` (module-DEFINED tags only — like
  `module.functions`, imports live separately in `module.imports`) to
  `wasm-execution::WasmExecutionEngine::set_tags`, which expects the
  COMBINED index space `throw`/`catch` actually encode. Any module
  declaring at least one tag import got every LOCAL tag's type looked up
  at the wrong (off-by-import-count) slot. `wasm-validator` already built
  its own correctly-combined `tag_types`; this crate did not. Silent
  until W22's real payload-popping became the first code path to
  actually read it — reproduced directly against the real testsuite's
  own `try_table.wast` (`catch-complex-1`/`catch-complex-2`, which
  declare tag imports before several differently-typed local tags). See
  `code/specs/W22-wasm-exceptions-catch-clause-matching.md`.

## [0.6.6] — 2026-08-25 (W21 — exceptions proposal, tag/throw first slice)

### Added

- `instantiate`'s import-resolution match gained an
  `ImportTypeInfo::Tag(_)` arm: cleanly link-fails with an "unknown
  import"-classified message (`HostInterface` has no `resolve_tag`
  method — a real, separate generalization this slice doesn't need,
  since its own corpus's tag-importing module has no subsequent
  `invoke`/`assert_return` exercising it at all). Exists purely so the
  workspace keeps compiling now that `ImportTypeInfo` (`wasm-types`
  0.1.7) has a 5th variant, and so a module with a tag import grades a
  real, gradeable `NotYetSupported` rather than crashing.

See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.

## [0.6.5] — 2026-08-17 (task #97 — table.init/table.copy/elem.drop instance-state threading)

### Added

- `WasmInstance.dropped_elements: Vec<bool>` -- persists for the
  instance's whole lifetime, same shape/reasoning as
  `dropped_data_segments` (task #95). Initialized all-`false`, one
  entry per `module.elements`, at `instantiate()` time; `build_engine`/
  `call_engine`/`call_engine_with_v128` thread it into/out of
  `wasm_execution::WasmExecutionEngine` via the new
  `set_elements`/`set_dropped_elements` setters, exactly like
  `dropped_data_segments` already is.
- New end-to-end test `table_init_copy_elem_drop.rs`: confirms
  `table.init`/`table.copy`/`elem.drop` survive THIS crate's own
  instance-state threading, not just a single `call_function` at the
  bare `wasm-execution` layer -- caught a real bug (see
  `wasm-execution`'s own CHANGELOG entry for this version): the
  interpreter's post-call state restore never wrote `dropped_elements`
  back, so an `elem.drop` from one `call()` silently reverted the
  moment that call returned, invisible to a later, separate `call()`
  on the same instance.

## [0.6.4] — 2026-08-16 (task #95 — memory.init/data.drop instance-state threading)

### Added

- `WasmInstance.dropped_data_segments: Vec<bool>` -- persists for the
  instance's whole lifetime, same shape as `v128_heap`. Initialized
  all-`false` (one entry per `module.data`) at instantiation time;
  `data.drop`'s effect from one `call()` is visible in a LATER, separate
  `call()` on the same instance, not just within the call that ran it
  (`build_engine`/`call_engine`/`call_engine_with_v128` thread it into/
  out of `wasm_execution::WasmExecutionEngine` exactly like `v128_heap`
  is).

### Changed

- `instantiate()`'s data-segment application loop now skips PASSIVE
  segments (`seg.is_passive`) -- applying one automatically at
  instantiation time would defeat the entire point of `memory.init`
  (the whole reason a segment is passive is that it stays resident,
  untouched, until an explicit `memory.init` copies from it, possibly
  more than once). A passive segment's bytes are instead threaded into
  the execution engine via the new `set_data_segments` call in
  `build_engine`.

## [0.6.3] — 2026-08-16 (task #100 — instantiate() requires a validated module)

### Changed (breaking)

- `WasmRuntime::instantiate()` now takes `&ValidatedModule` instead of
  `&WasmModule`. This crate's own `ValidatedModule` doc comment always
  documented the intent that "downstream code (the runtime) can accept
  `ValidatedModule` instead of `WasmModule` to ensure validation is
  never accidentally skipped", but `instantiate()` never actually
  enforced it: it took a plain `&WasmModule` and never called
  `validate()` itself, so every `validate()` check -- including the
  memory/table allocation caps added for task #96's security review --
  was silently bypassable by any caller who called `instantiate()`
  directly instead of going through `WasmRuntime::validate()` first.
  Callers now call `validate()` (or `load_and_run()`, fixed to actually
  thread its own `validate()` result through instead of discarding it
  and re-passing the raw module) and pass the resulting
  `ValidatedModule` -- the guarantee is now a compile-time fact instead
  of a caller convention.
- Confined blast radius: outside this crate's own test suite, only
  `wasm-conformance`'s harness calls `instantiate()`, and it already
  called `validate()` first (it just threw away the `ValidatedModule`
  and re-passed `&validated.module` -- trivially updated to pass
  `&validated` instead). No other crate in the workspace calls
  `instantiate()` directly.

### Security

- Found via `/security-review` as a follow-up to task #96's memory/
  table allocation caps: those caps (and every other `validate()`
  check) were bypassable by any embedder calling `instantiate()`
  directly. Closed by making `ValidatedModule` the only way to reach
  `instantiate()` at all.
- A second `/security-review` round on this same diff found that
  `instantiate(&ValidatedModule)` alone wasn't the compile-time
  guarantee it claimed to be: `ValidatedModule.module` was still a
  public field in `wasm-validator`, so any crate could construct one
  directly with a struct literal, skipping `validate()` entirely. See
  `wasm-validator`'s own CHANGELOG (0.2.9) for that companion fix --
  this crate's `instantiate()` needed no further change, since it
  already only reads through the (now-private) field via `wasm_
  validator`'s public accessor.

## [0.6.2] — 2026-08-15 (W16, task #85 — multi-memory first slice)

### Changed (breaking)

- `WasmInstance.memory: Option<LinearMemory>` is now `memories:
  Vec<LinearMemory>`. Import resolution now accumulates (`memories.
  push(...)`) instead of overwriting, so a module importing more than one
  memory keeps all of them instead of silently retaining only the last.
  `instantiate()` allocates every entry in `module.memories`, not just
  `module.memories[0]`.
- `build_engine`/`call_engine`/`call_engine_with_v128` thread the
  `Vec<LinearMemory>` through via the same unconditional-even-on-trap
  restore discipline the singular field already used.
- Data-segment application still only ever targets memory 0 regardless of
  `seg.memory_index` -- a deliberate scope boundary, not an oversight; see
  `code/specs/W16-wasm-multi-memory-first-slice.md`'s "What does NOT
  change".

### Fixed

- `RegistryHost::resolve_memory` (`wasm-conformance`) discarded the
  resolved export's memory INDEX and always cloned "the" single memory --
  harmless before this change (an instance had at most one memory), but a
  real latent bug once an exporting instance can have more than one:
  importing memory export #1 from a 2-memory module would have silently
  returned memory #0 instead. Fixed alongside this crate's own change
  since it shares the same root field.

See `code/specs/W16-wasm-multi-memory-first-slice.md` for the full design.

## [0.6.1] — 2026-08-15 (W15, task #79 — v128 persistent storage)

### Fixed (breaking)

- `WasmInstance` gains a `pub v128_heap: Vec<[u8; 16]>` field -- the
  instance's own persistent v128 (SIMD) value storage, replacing the old
  per-call-only `wasm_execution::WasmExecutionContext::v128_heap`. A
  v128-typed global's `WasmValue::V128(handle)` used to go stale the
  moment one `call`/`call_typed` invocation ended (the heap it indexed
  into was thrown away and rebuilt fresh every call); it now survives
  across separate invocations on the same instance, exactly like
  `globals`/`memory`/`tables` already do. `instantiate()` builds this
  field up directly (starting from the reserved all-zero entry) as it
  evaluates global/data/element initializers, so a `v128.const` inside
  one of those (previously a hard instantiation failure -- see the
  companion `wasm-execution` 0.9.2 release) now allocates straight into
  the instance's own long-lived heap.
- `build_engine`/`call_engine`/`call_engine_with_v128` thread
  `v128_heap` through the exact clone/restore shape `globals` already
  uses (`build_engine` calls the new
  `WasmExecutionEngine::set_v128_heap`; both `call_engine` variants
  restore `instance.v128_heap = state.v128_heap` after the call,
  unconditionally, matching the existing regardless-of-trap discipline
  documented on `call_engine` itself).
- This is a breaking change to `WasmInstance`'s public field list -- the
  one hand-built test construction site in this crate's own integration
  tests was updated in the same PR; per this repo's stated preference
  (break compatibility freely, no back-compat shims), no deprecated
  alias or default was added.

See `code/specs/W15-wasm-v128-persistent-storage.md` for the full design
and motivating corpus evidence.

## [0.6.0] — 2026-08-15 (SIMD PR1b-1 — call_typed_with_v128, real v128 results end to end)

### Added

- `WasmRuntime::call_typed_with_v128(&mut WasmInstance, name, args) ->
  Result<(Vec<WasmValue>, Vec<Option<wasm_execution::V128Bytes>>),
  TrapError>` — the host-facing sibling of `call_typed` for functions that
  return real v128 values. Thin-wraps `wasm-execution` 0.9.0's new
  `WasmExecutionEngine::call_function_with_v128`, resolving each `V128`
  result to its actual 16 bytes rather than leaving it as an
  already-meaningless handle once the engine's internal context has been
  torn down.
- Internal refactor to support this without duplicating the run+restore
  bookkeeping: the engine-construction half of `call_engine` (memory/
  tables/host-function ownership transfer into a fresh
  `WasmExecutionEngine`) is extracted into a private `build_engine`
  helper, shared by both the existing `call_engine` (unchanged behavior,
  confirmed via the full existing test suite) and a new sibling
  `call_engine_with_v128`.

### Why `wasm_wast_parser` couldn't test this directly

`wasm-wast-parser` doesn't yet support `v128.const`'s text literal syntax
(deferred to SIMD PR1b-2), so this crate's new integration test
(`tests/call_typed_with_v128.rs`) hand-constructs a `WasmInstance`
directly with raw SIMD bytecode rather than going through
`wasm_wast_parser::parse_module` — every `WasmInstance` field is already
public, so this needed no new test-only surface.

### Added

- `call()`'s existing lossy i64-round-trip conversions (both directions)
  gained a `ValueType::V128`/`WasmValue::V128` arm, matching the
  established pattern for reference types: a deterministic, non-panicking
  placeholder (handle `0`, the reserved all-zero v128), not a real
  conversion — `call()`'s own `i64`-only signature cannot represent a
  128-bit value at all. `call_typed()` should be used for real v128
  arguments/results, same guidance the existing `Ref` comment already
  gives.

## [0.5.3] — 2026-08-15 (WASM05 — real instantiate() link-failure path)

### Changed (breaking behavior, deliberate)

- `instantiate` now returns a real `Err(TrapError)` when any import
  can't be resolved by the host, or resolves to something whose actual
  type doesn't satisfy the module's declared import type. Previously it
  never failed on an import at all: an unresolved function got pushed
  as `None` (failing later, at *call* time, only if that specific
  import was ever invoked), and an unresolved memory/table/global
  silently fabricated a default value from the *declared* type instead
  of erroring. See `code/specs/W10-wasm-real-linking-and-unlinkable.md`.
- Function imports are now checked against `HostFunction::func_type()`;
  memory/table imports are checked via the real spec's limits-
  compatibility rule (actual min ≥ declared min; if declared has a max,
  actual must too, and not exceed it) — `Table` doesn't track its
  declared element type at runtime, so table element-type mismatches
  aren't caught here (a real, named limitation, not silently ignored;
  every table this repo can currently construct is funcref anyway, so
  this doesn't lose real coverage against the vendored corpus).
- **Verified safe for existing real callers**: confirmed by reading
  `WasiEnv::resolve_function` directly that it never actually returns
  `None` for its own module — every unimplemented WASI function falls
  through to a real `EnosysFunc` stub, not `None`. So
  `brainfuck-wasm-compiler`/`nib-wasm-compiler`/`twig-to-wasm`/
  `twig-demo`/`lang-aot`'s existing WASI-based execution paths cannot
  regress from this change; confirmed empirically too, full workspace +
  downstream consumer test suite unchanged.
- 12 new tests covering unresolved/type-mismatched/compatible imports
  for each of function, memory, table, and global.

## [0.5.2] — 2026-08-15 (WASM17 — exhaustive-match fix for Funcref/Externref)

### Fixed

- `call()`'s param-type-to-`WasmValue` conversion match was non-exhaustive
  after `wasm-types` 0.1.1 added `ValueType::Funcref`/`Externref` (a
  compile error, not a behavior change). Added both to the same lossy
  "pass the raw i64 as a null-pointer sentinel" arm the existing GC
  reference types (`Anyref`/`I31ref`/`StructRef`) already use -- this is
  `call()`'s pre-existing legacy behavior; `call_typed()` should be used
  instead when a real `WasmValue::Ref` needs to be passed. No behavior
  change for any existing `ValueType`.

## [0.5.1] — 2026-08-13 (WASM07 — a trapped call must not lose an instance's state)

`call_engine` (shared by `call()` and `call_typed()`) temporarily
`take()`s `instance.memory`/`mem::take`s `instance.tables`/
`instance.host_functions` into a fresh `WasmExecutionEngine` for the
duration of one call, then writes the engine's post-call state back onto
`instance`. That write-back used `let results =
engine.call_function(func_index, wasm_args)?;` — the `?` early-returns on
ANY trap, before the write-back lines ever run. Since the fields were
already taken, `instance.memory` was left `None` (and `instance.tables`/
`instance.host_functions` left empty) **forever after** — not just for
that one trapped call, but for every subsequent call on the same
instance, since nothing else ever puts them back.

This is exactly the shape of `wasm-conformance`'s own module-registry
model: a script's module registry holds one `WasmInstance` per `(module
...)` directive and runs every `invoke`/`assert_*` against it in order.
The moment ANY of those directives trapped for any reason — an
intentionally-trapping `assert_trap`, or a genuine bug in an unrelated
function — every LATER directive against that same module silently and
permanently failed with a spurious "no memory available"/"undefined
table", masking whatever those later directives were actually checking.
This affected dozens of real testsuite cases across
`load.wast`/`local_tee.wast`/`nop.wast`/`memory_trap.wast`/`call.wast`
(their common `as-call_indirect-*`/`as-load-*`/`as-store-*` cases always
follow an earlier intentionally-trapping case in the same module).

Fixed by capturing the `Result` instead of `?`-ing it immediately, always
restoring `instance`'s fields from `engine.into_state()` (safe regardless
of whether the call trapped — `call_function` takes `&mut self`, so
`engine`, and everything moved into it, is fully intact either way), and
only then returning the captured result.

Also wires the module's real type section into the engine
(`engine.set_type_section(instance.module.types.clone())`) so
`call_indirect` gets real type-checking — see `wasm-execution` 0.6.3's
own changelog for why that was a separate, necessary fix and not
something this crate could paper over on its own.

2 new regression tests (`tests/wasm07_regression.rs`): memory and a
table each independently confirmed to survive an earlier trapped call on
the same instance and remain usable by a later one.

## [0.5.0] — 2026-08-13

### Added — `WasmRuntime::call_typed`, a bit-exact sibling of `call()` (W05 PR-4)

`call()` is the crate's only public execution entry point, and it round-trips
every argument and result through `i64` — lossy for floats. Its result
conversion does `WasmValue::F32(v) => *v as i64` / `F64(v) => *v as i64`, a
numeric *truncation* (Rust's `as` cast), not a bit reinterpretation: a
`3.5f64` result comes back as `3i64`, and a NaN's payload/sign bits are not
preserved at all. This is fine for `call()`'s existing callers (integer-only
WASI/Lisp-value-model workloads), but it means `call()` cannot support
anything that needs the *exact* result the interpreter produced — most
directly, a conformance harness grading the official testsuite's
`assert_return` directives, some of which assert an exact
`nan:0x<payload>` bit pattern.

`call_typed(&self, instance: &mut WasmInstance, name: &str, args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError>`
is a purely additive sibling: same export-lookup and engine-execution
plumbing as `call()` (now factored into a shared private `call_engine`
helper so neither duplicates the memory/tables/host-functions ownership
transfer and WasmGC struct-field-count wiring), but it takes and returns
typed `WasmValue`s directly, with no `i64` round trip at all. `call()`
itself, its behavior, and its existing callers/tests are unchanged — this
refactor was verified against the existing WASI Tier 3 test suite (17
tests, all still passing) before and after.

New tests in `tests/call_typed.rs` empirically confirm the bug `call_typed`
fixes: one asserts `call()` really does truncate `3.5` to `3`, and a
sibling assertion on the same call confirms `call_typed` returns the exact
`f64` bits instead; another constructs a NaN with a specific,
non-canonical payload via `f64.reinterpret_i64` and asserts `call_typed`'s
result preserves that exact bit pattern, not just "is NaN".

## [0.4.0] — 2026-07-13

### Fixed — struct field counts indexed by deduplicated function-type count (LANG-FULL E6d-5)

`WasmRuntime::call` registered WasmGC struct field counts by padding the front of
the `struct_field_counts` vec with one filler slot per **function** (`instance.
func_types.len()`) before appending the struct counts. But function and struct
types share one wasm type-index space and the encoder **deduplicates** function
types, so the per-function count over-counts whenever two functions share a
signature — and the struct's field-count entry then landed at a type index higher
than the one the emitted `struct.new`/`struct.set` actually reference, leaving the
real struct index registered as a zero-field filler.

The symptom: any module whose functions include duplicate signatures trapped
`struct.set: field 0 out of range`. This is exactly the shape a Twig `record`
produces — a constructor plus N same-shape accessors plus a predicate collapse to
a few distinct function types — so records never ran on the WASM column despite
compiling and validating. Single-function cons programs and the list-op helpers
were unaffected because their function types happened to all be distinct, so the
per-function and deduplicated counts coincided.

Fix: pad by `instance.module.types.len()` (the type section's deduplicated
function-type count, i.e. the exact count the encoder used to place the struct
types) instead of `instance.func_types.len()`. One-line change; no API change; 47
existing wasm-runtime tests still pass.

## [0.3.0] — 2026-06-08

### Added — run WasmGC struct (cons) modules end-to-end (LANG77 / McCarthy L3b-3a-3c-2)

`WasmRuntime::call` now derives each WasmGC struct type's field count from the
parsed module's `struct_types` and registers it with the execution engine
(`set_struct_field_counts`), so a module that uses `struct.new`/`struct.get`
runs **without the embedder calling `set_struct_field_counts` by hand**. Field
counts are placed at their wasm *type index* (function types first, then struct
types — matching the encoder's layout).

With this, a hand-assembled `$LispyPair` cons module computing `(CAR (CONS 7 9))`
parses, instantiates, and **runs to `7`** on the in-repo runtime (both via the
explicit `load`→`instantiate`→`call` path and the all-in-one `load_and_run`).
Before this slice the same module trapped with "no field count registered for
struct type 1".

Note: assumes struct types follow all function types (true for the cons modules
we emit today, which declare no host imports). A module that interleaved
imported-function types after the struct types would need order-preserving type
parsing — not yet emitted or consumed. The reference-return placeholder from
earlier (a returned `Ref` → its handle / `0`) is unchanged; the cons return
boundary unboxes to `i32`, so it isn't exercised here.

2 new tests: the `(CAR (CONS 7 9))` → 7 end-to-end run and a `load_and_run`
regression guard.

## [0.2.0] - 2026-04-06

### Added

- **WASI Tier 3**: 8 new WASI host functions via the new `WasiEnv` struct:
  - `args_sizes_get` — write argc and argv buffer size to WASM memory
  - `args_get` — write argv pointer array and null-terminated strings to WASM memory
  - `environ_sizes_get` — write envc and environ buffer size to WASM memory
  - `environ_get` — write environ pointer array and null-terminated strings to WASM memory
  - `clock_res_get` — write clock resolution (nanoseconds) as i64 little-endian
  - `clock_time_get` — write current clock time (nanoseconds) as i64 little-endian
  - `random_get` — fill a WASM memory region with random bytes
  - `sched_yield` — no-op yield returning errno 0

- **`WasiClock` trait** — injectable clock interface for deterministic testing; `SystemClock` uses `std::time::SystemTime` and a lazy `Instant` for monotonic time

- **`WasiRandom` trait** — injectable random interface for deterministic testing; `SystemRandom` uses a hash-based fallback (NOT crypto-secure; documented and swappable)

- **`WasiConfig` struct** — configuration bundle for args, env, stdout/stderr callbacks, clock, and random; implements `Default`

- **`WasiEnv` struct** — full `HostInterface` implementation that resolves all Tier 3 WASI functions; uses `Arc<Mutex<LinearMemory>>` to share memory between host functions and the runtime

- **Integration tests** in `tests/wasi_tier3.rs` — 14 tests covering all 8 new functions with `FakeClock` and `FakeRandom` for deterministic verification

### Changed

- No breaking changes to existing `WasiStub`, `WasmRuntime`, or `WasmInstance` APIs

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
