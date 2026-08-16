# Changelog — wasm-conformance

## 0.1.29 — 2026-08-16 — vendor memory_copy.wast/memory_fill.wast; baseline regen (task #94)

### Changed

- Baseline regen: vendored `memory_copy.wast`/`memory_fill.wast` -- the
  first vendored files from the bulk-memory proposal. Both fully pass --
  every directive kind at 100%. Zero regressions anywhere else in the
  now-66-file vendored corpus (full before/after per-file/per-kind diff).
  Surfaced a real bug in `wasm-execution::LinearMemory::copy()`/`fill()`
  (see that crate's own CHANGELOG): a zero-length copy/fill skipped
  bounds-checking entirely instead of still requiring `dest`/`src` to sit
  at or before the end of memory. Deliberately excludes their sibling
  `bulk.wast`: it mixes memory.copy/memory.fill with memory.init/
  data.drop (task #95) and table.init/elem.drop/table.copy (task #97) in
  the same file, all still unimplemented.

See `code/packages/rust/wasm-wast-parser/CHANGELOG.md` and `code/
packages/rust/wasm-execution/CHANGELOG.md` for the parsing and
interpreter-layer changes this vendoring pass needed and surfaced.

## 0.1.28 — 2026-08-16 — instantiate() call sites updated for ValidatedModule (task #100)

### Changed

- `wasm-runtime::WasmRuntime::instantiate()` now takes `&ValidatedModule`
  instead of `&WasmModule` (see `wasm-runtime`'s own CHANGELOG). This
  harness already called `validate()` before `instantiate()` -- it just
  discarded the `ValidatedModule` and re-passed `&validated.module`, so
  the fix is a one-line change per call site (`&validated` instead of
  `&validated.module`). No behavioral change; full baseline regen
  confirms byte-identical results.

## 0.1.27 — 2026-08-16 — vendor table_get.wast/table_set.wast; baseline regen (task #96)

### Changed

- Baseline regen: vendored `table_get.wast`/`table_set.wast` (real
  cross-table type-checking, funcref+externref mix). Both fully pass --
  every directive kind at 100%. Zero regressions anywhere else in the
  now-64-file vendored corpus (full before/after per-file/per-kind diff).
  Deliberately excludes `table.wast` (hex-literal table limits + a real
  `spectest` import, task #99) and `table_size.wast`/`table_grow.wast`/
  `table_fill.wast` (need entirely unimplemented opcodes, task #98).

See `code/packages/rust/wasm-wast-parser/CHANGELOG.md` and `code/
packages/rust/wasm-validator/CHANGELOG.md` for the real bugs this
surfaced and fixed (table reftype parsing, per-table type-checking).

## 0.1.26 — 2026-08-15 — vendor linking.wast; module registry resolves $id (task #93)

### Fixed

- The module registry (`Executor`) now registers each module under its
  own script-level `$id` too (in addition to the existing `None`/"current
  module" slot), using `wasm-wast-parser`'s newly-captured `Directive::
  Module.id`. `Action::Invoke`/`Action::Get`'s `module: Option<String>`
  field and `Directive::Register`'s `module_name: Option<String>` field
  already carried a `$id` reference -- they just had nothing to resolve
  it against. This was the SOLE root cause of every one of the real,
  vendored `linking.wast` corpus file's 65 `assert_return` failures
  (confirmed via direct diagnostic: all 65 failed with the identical "no
  module registered as Some($id)" message before this fix); 48 of them
  now pass for real. The remaining 17 trace to a real, separate,
  already-documented limitation (`RegistryHost::resolve_memory`/
  `resolve_table`'s clone-not-share semantics for cross-instance
  memory/table imports) -- out of scope here, a distinct future epic.
- `Directive::Register`'s handling of an explicit `$id` target (as
  opposed to "register the current module") is now real instead of
  silently ignored.

### Added

- Vendored `linking.wast` (task #93) -- real cross-module linking was
  originally excluded from this crate's corpus (W05's own scope note:
  "needs heavier module-linking semantics"), but WASM05's real
  `HostInterface` link-failure path already provides exactly that. Of its
  71 modules, 2 import from `spectest` (this crate has no `spectest`
  host, by design) and grade `NotYetSupported`/collateral-fail; the rest
  exercise real, already-supported machinery. Lands at `assert_return`
  48/65, `assert_trap` 18/18, `assert_unlinkable` 49/50, `module` 16/17,
  `register` 6/8 -- see `tests/fixtures/testsuite/NOTICE` for the full
  vendoring rationale.
- Baseline regenerated: zero regressions in any of the other 61
  pre-existing vendored files (full before/after per-file/per-kind diff).

## 0.1.25 — 2026-08-15 — multi-memory; baseline regen (W16, task #85)

### Fixed

- `RegistryHost::resolve_memory` discarded the resolved export's memory
  index and always cloned `instance.memory` (the single memory that
  existed before `wasm-runtime` 0.6.2) -- harmless before multi-memory,
  a real latent bug afterward. Now indexes `instance.memories` by the
  real export index, matching `resolve_global`'s existing pattern.

### Changed

- Baseline regen: `memory_grow.wast` moves from a file-level parse
  failure (`$mem1` unresolved) to fully passing -- `module` 933/934 ->
  936/936 (100%), `register` 1/2 -> 2/2 (100%), `assert_return` 15523/
  16029 -> 15570/16029 (the 47-directive gap moving from `not_yet_
  supported` to real `pass`). Zero regressions anywhere else in the
  61-file vendored corpus (full before/after per-file/per-kind diff).
  This is the last remaining conformance gap in the corpus -- every
  graded directive kind is now at 100% pass with zero `fail`/`trap`
  anywhere.

See `code/specs/W16-wasm-multi-memory-first-slice.md` for the full design.

## 0.1.24 — 2026-08-15 — v128 invoke arguments; baseline regen (task #86, W15 follow-up)

### Fixed

- `run_action`'s `Action::Invoke` arm rejected any `(v128.const ...)`
  invoke ARGUMENT with `NotYetSupported` -- at the time that code was
  written (SIMD PR1b-3), no live heap existed before a call started to
  allocate its handle into. Now that `WasmInstance.v128_heap` is
  persistent and exists from `instantiate()` onward (W15, task #79), a
  v128 argument allocates directly into it, the same "push and return
  the new index" shape `evaluate_const_expr`/`push_v128` already use --
  a real `WasmValue::V128(handle)`, not a synthesized/placeholder one.
  Bounds-checked against `wasm_execution::MAX_V128_HEAP_LEN` (now `pub`
  in `wasm-execution` 0.9.3 for exactly this reuse).
- Existing test `invoke_with_a_v128_argument_grades_not_yet_supported_
  not_a_silent_wrong_pass` renamed to `invoke_with_v128_arguments_
  passes_for_real` and rewritten to assert the new, correct outcome
  (`Pass`, byte-exact) instead of the old capability-gap `NotYetSupported`.

### Changed

- Baseline regen following `wasm-execution` 0.9.3: `simd_const.wast`'s
  `assert_return` tally moves from 235/240 (4 fails) to 243/243 (fully
  clean, 0 fails, 0 traps -- the file's directive count itself also
  changed slightly, since some `NotYetSupported` invoke-argument cases
  now grade as real `Pass`es instead). Aggregate `assert_return` across
  the 61-file vendored corpus moves to 15523/15523 (100%, zero fails
  anywhere). Zero regressions (full before/after diff of every file's
  per-kind tally).

## 0.1.23 — 2026-08-15 — v128 global reads resolved; baseline regen (W15, task #79)

### Fixed

- `run_action`'s `Action::Get` arm (a bare `(get "name")` action reading
  an exported global directly, not via a function call) used to always
  return `None` for a `WasmValue::V128` global's resolved bytes -- at
  the time that code was written (SIMD PR1b-3), there was no way to
  resolve a v128 handle outside of an active call's `ctx.v128_heap`.
  Now that `WasmInstance.v128_heap` is a persistent, directly-readable
  field (`wasm-runtime` 0.6.1, W15), a `Get` action resolves the handle
  against it exactly like a call's result does. No vendored corpus file
  currently exercises `(get ...)` against a v128 global, so this has no
  baseline-visible effect today, but closes the gap for real ahead of a
  future corpus file that does.

### Changed

- Baseline regen following `wasm-execution` 0.9.2 / `wasm-runtime` 0.6.1
  (W15, task #79 -- v128 persistent storage). `simd_const.wast`'s
  `module` tally moves from 308/309 (1 trap) to 309/309 (the module
  whose `(global (mut v128) (v128.const ...))` initializer previously
  failed to instantiate now builds); `assert_return` moves from 235/240
  (5 fails) to 235/240 (4 fails, `not_yet_supported` +1) -- one
  previously-collateral failure now correctly grades
  `NotYetSupported("a v128 invoke ARGUMENT is not yet supported...")`,
  a real, SEPARATE, already-documented gap (v128.const literals aren't
  yet supported as `invoke` arguments -- logged as task #86, NOT fixed
  by this PR). The remaining 4 fails are a direct, expected downstream
  consequence of that same NotYetSupported case (globals a skipped
  "set" call never actually set), not a new or masked bug. Zero
  regressions anywhere else in the 61-file vendored corpus (full
  before/after diff of every file's per-kind tally).

## 0.1.22 — 2026-08-15 — assert_malformed also validates; baseline regen (tasks #82-84)

### Fixed

- `grade_assert_malformed`'s binary path used to only call
  `WasmModuleParser::parse` -- a module that parses fine but is
  structurally malformed by a rule this crate's parser doesn't check at
  PARSE time (e.g. a memop's align flags with the reserved top bit set,
  which `wasm-module-parser` never decodes at all since code-section
  bytes are stored raw) went undetected even though `wasm-validator`'s
  instruction-level type-checker already rejects it (via its existing
  `align > max_align` check, just under a different error message than
  the spec's own "malformed memop flags" wording). Now also calls
  `self.runtime.validate(&built)` after a successful parse and grades
  `Pass` if THAT fails too -- same "outcome category, not the specific
  reason" precedent `grade_assert_unlinkable` already uses. Found via a
  prioritization scan after task #80 (PR #11844); fixes `align.wast`'s
  "memop flags" `assert_malformed` cases with zero new decode logic.
- Baseline regen following `wasm-module-parser` 0.2.4 (tasks #82/#84:
  malformed mutability bytes, data count section cross-check) and the
  `grade_assert_malformed` fix above: `align.wast` 0/2 -> 2/2,
  `custom.wast` 6/8 -> 8/8, `global.wast` 3/7 -> 7/7 (all
  `assert_malformed`), aggregate `assert_malformed` 208/216 -> 216/216
  (100%). Zero regressions anywhere else in the 61-file vendored corpus
  (verified via a full before/after diff of every file's per-kind tally).

## 0.1.21 — 2026-08-15 — baseline regen: correctly-rounded hex floats (task #80)

### Changed

- Baseline regen following `wasm-wast-parser` 0.1.14 (task #80 -- the
  hex-float literal parser now rounds correctly instead of double-
  rounding). `const.wast`'s `assert_return` tally moves from 260/300 to
  300/300 (fully clean) and `simd_const.wast` improves from 209/240 to
  235/240. Verified via a full before/after diff of every vendored
  file's per-directive-kind tally: zero regressions anywhere in the
  61-file corpus, confirming the rounding fix is a strict improvement.

## 0.1.20 — 2026-08-15 — baseline regen: blocktype fix (task #81)

### Changed

- Baseline regen following `wasm-execution` 0.9.1 / `wasm-validator`
  0.2.6 (task #81 -- `v128`/`funcref`/`externref` single-value blocktypes
  were being misdecoded as bogus negative type indices). `simd_const.wast`
  moves from `module: 307/309, assert_return: 189/240` to `module:
  308/309, assert_return: 209/240` -- the module declaring `(block
  (result v128) ...)`-shaped functions (a SEPARATE module from the
  still-open v128-global-initializer trap, task #79) now validates and
  instantiates for real, and the ~20 collateral "no module registered"
  failures its own callers previously hit are gone. Confirmed via a full
  JSON diff that this is the ONLY file affected; zero regressions
  elsewhere.

## 0.1.19 — 2026-08-15 — vendor simd_const.wast, the first post-MVP-proposal corpus file (task #78)

### Added

- `simd_const.wast` vendored at the same pinned commit SHA
  (`28864811cf03bdbf880733786148feaba339582d`) as the rest of the corpus,
  verbatim (confirmed byte-identical against a fresh independent fetch
  before committing). The narrowest real root-level `simd_*.wast` file --
  almost entirely tests `v128.const`'s own literal syntax across all 6
  shapes, which this repo's `wasm-wast-parser` already fully supports
  (SIMD PR1b-2/1b-3) -- and the first file this repo has ever vendored
  from a post-MVP proposal, made possible by W14's per-module graceful
  degradation (task #76): this file's few genuinely-unsupported opcodes
  (e.g. a single `i64x2.add` usage) no longer abort grading the other
  ~600 directives in the same file.

### Baseline regen: 61 files parsed, 0 failed to parse (up from 60/0)

- `simd_const.wast`: `module` 307/309 pass (99.4%, 3 not yet supported --
  genuinely-unimplemented opcodes/shapes correctly deferred), `assert_return`
  189/240 pass (78.8%, 25 not yet supported), `assert_malformed` 72/72 pass
  (100%, 109 not yet supported). Confirmed via a full JSON diff against the
  prior baseline that every OTHER file's tallies are byte-for-byte
  unchanged (zero regressions).
- **3 real, root-caused bugs surfaced by this file's real assert_return/
  module Fails and Trap — reported honestly, not hidden or miscategorized
  as `NotYetSupported`, and not fixed in this PR (each is a genuine,
  separately-scoped follow-up, logged as tasks #79/#80/#81):**
  1. **v128.const is rejected inside constant expressions**
     (`wasm-execution::evaluate_const_expr` has no `0xFD` arm at all), so
     any module declaring a `(global (mut v128) (v128.const ...))` fails
     to *instantiate* entirely (a `Trap`, cascading into ~20 collateral
     "no module registered" `Fail`s for every subsequent bare `invoke`
     against it) -- task #79.
  2. **v128 globals don't survive a round-trip across separate `invoke`
     calls** -- a `global.set` in one call followed by `global.get` in a
     later call returns the raw, now-meaningless handle instead of the
     real bytes, because `ctx.v128_heap` (what the handle indexes into)
     is scoped to a single call. Already flagged as a known gap in SIMD
     PR1b-3's own CHANGELOG; now concretely triggering real graded
     failures against real corpus data -- same root cause as #1 above
     (v128 needs persistent, not per-call, storage), folded into task #79.
  3. **A real hex-float literal rounding bug**: `parse_float_magnitude`
     accumulates a hex-float mantissa digit-by-digit via plain `f64`
     arithmetic, which double-rounds instead of computing round-to-
     nearest-even from the full-precision value -- fails on the corpus's
     own deliberately-crafted over-precision edge cases (e.g.
     `+0x1.000000000000080000000000p-600`, an exact-halfway tie-break
     case). General MVP-level bug, not SIMD-specific -- task #80.
  4. One additional, less-triaged `module` structural-validation `Fail`
     ("blocktype references type index -5") from a single `(module
     binary ...)` directive, possibly a signed-LEB128 blocktype decoding
     ambiguity -- task #81, not yet root-caused in the same depth as
     #1-3.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and
`code/specs/W14-wasm-conformance-lazy-module-build.md`.

## 0.1.18 — 2026-08-15 — lazy per-module build support (W14, task #76)

### Added

- `Executor` now handles `wasm-wast-parser` 0.1.13's `Directive::Module(
  Result<WasmModule, String>)`: a build failure grades `NotYetSupported`
  (a real capability gap, not a bug) instead of the directive not
  existing at all in a fully-aborted script.
- `current_link_failed: Option<String>` renamed and broadened to
  `current_module_status: Option<String>`, now covering BOTH build
  failures and link failures uniformly (previously link-failure-only).
  `run_action`'s two read sites simplified to surface the already-
  formatted reason directly. `Directive::Register`'s "no current module"
  arm now checks this field too: a broken current module (build or link
  failure) grades `NotYetSupported` on `register`, not the generic
  hardcoded `Fail` reserved for a genuine test-script-structure problem
  (no module directive ever ran at all).

### Fixed

- A real, previously rarely-exercised bug the same change surfaces and
  fixes: the module registry's `None` ("current module") slot was only
  ever WRITTEN on a successful `instantiate`, never CLEARED on any
  failure path -- so a module that failed structural validation or
  trapped during instantiation left the PREVIOUS module silently
  registered as "current," and a later bare `invoke`/`register` would
  operate on the wrong module instead of failing loudly. Fixed by
  unconditionally clearing the `None` registry slot at the top of every
  `Directive::Module` directive, before its outcome is even determined.
  Verified load-bearing via TEMP-REVERT-CHECK: reverting just the
  `registry.borrow_mut().remove(&None)` line reproduces the exact
  predicted false pass (a stale-module `Pass` where a `Fail` was
  expected) on a dedicated regression test, then restored.

### Baseline regen

- `tests/fixtures/testsuite-status.json` regenerated: 12 previously
  entirely-unparseable MVP corpus files (`br_table.wast`,
  `call_indirect.wast`, `const.wast`, `float_exprs.wast`,
  `float_memory.wast`, `global.wast`, `id.wast`, `memory.wast`,
  `memory_grow.wast`, `select.wast`, `stack.wast`,
  `unreached-valid.wast`) now parse and grade for real -- confirmed via
  a full JSON diff that every OTHER previously-present file's tallies
  are byte-for-byte unchanged (zero regressions, exactly these 12 files
  added, zero files removed). Aggregate: 60 files parsed, 0 failed to
  parse (up from 48 parsed / 12 failed).

See `code/specs/W14-wasm-conformance-lazy-module-build.md`.

## 0.1.17 — 2026-08-15 — v128 byte-exact assert_return grading (SIMD PR1b-3)

### Added

- `run_action` now returns each result's real, resolved v128 bytes
  alongside its `WasmValue` (via `wasm-runtime` 0.6.0's
  `call_typed_with_v128`, SIMD PR1b-1), and `value_matches_expected`
  compares a `(v128.const ...)` expected value byte-exact against those
  resolved bytes, not just "is this a `V128` result" — proven via a
  TEMP-REVERT-CHECK (stubbing the byte compare out reproduces the exact
  predicted false-pass on a deliberately wrong computed value, confirming
  the check is load-bearing).
- 5 new hand-written regression tests exercising this crate's real 5
  SIMD opcodes end to end (`v128.const` exact/mismatch, `i32x4.add`'s
  actual computation vs. "any v128", `i32x4.eq`'s boolean-mask result
  staying a `v128` not a plain `i32`, a `splat`/`extract_lane` round
  trip) plus one confirming a `v128` invoke ARGUMENT degrades loudly to
  `NotYetSupported` rather than silently substituting the zero vector
  (see "Deferred" below for why arguments can't be resolved the way
  results can).

### Deferred: real corpus vendoring

Investigated vendoring one of the 4 pinned-commit root-level
`simd_*.wast` files (`simd_const.wast`/`simd_splat.wast`/
`simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast`) per this task's original
scope. Concretely confirmed **none currently parse**: each exercises SIMD
opcodes well beyond this repo's 5-opcode first slice -- e.g.
`simd_const.wast`'s sole `i64x2.add` use (its `i64x2.inc_smin` test),
`simd_splat.wast`'s `i8x16.add`/`f32x4.min`/`v128.and`/`v128.load`/etc.
across ~20 opcode families. Because `Directive::Module` is built EAGERLY
at `wasm_wast_parser::parse_script` time (see that crate's own module doc
comment for the — separately valid — reason why), a SINGLE unsupported
instruction ANYWHERE in a file, even in a test that would never run,
aborts parsing the WHOLE FILE, not just that one directive — the "partial
opcode coverage, grade the rest `NotYetSupported`" pattern that worked for
every prior WASM epic's first PR doesn't apply here until opcode coverage
is wide enough for a real file to fully parse. Logged as two follow-up
backlog items (widen opcode coverage, or make per-module parse failures
degrade gracefully) rather than either faking a pass or silently dropping
the requirement.

### Why a v128 invoke ARGUMENT can't be resolved the way a RESULT can

`call_function_with_v128`'s resolution trick (SIMD PR1b-1) works because
it runs one statement before `ctx`/`ctx.v128_heap` drop, right after a
call already happened. An ARGUMENT is needed *before* any call starts —
no engine, no `ctx`, no heap exists yet to allocate a handle into. A
`(v128.const ...)` invoke argument now degrades to `NotYetSupported`
(loud, honest) rather than silently becoming the reserved zero-vector
placeholder the legacy `wasm-runtime::call()` i64 path uses for exactly
this situation (which would risk a false pass/fail for the wrong reason
here, where exact bytes are the whole point).

## 0.1.16 — 2026-08-15 — baseline regen: call.wast even/odd now pass (WASM10)

### Changed

- Baseline regen following `wasm-execution` 0.7.0 (WASM10 — `call_function`
  now runs on a dedicated thread with a re-bisected, much higher
  `MAX_CALL_DEPTH`): `call.wast`'s `assert_return` moves from `pass: 67,
  fail: 2` to `pass: 69, fail: 0` — the `even(100)`/`odd(200)` mutual-
  recursion cases that previously needed more than the old 80-depth
  ceiling now complete. Confirmed via a full baseline diff that this is
  the ONLY file affected; zero regressions elsewhere.

## 0.1.15 — 2026-08-15 — real assert_unlinkable grading via registry-backed HostInterface (WASM05)

### Added

- `RegistryHost`: a `HostInterface` backed by `Executor`'s own module
  registry, letting a module import a function/memory/table/global from
  a `register`ed sibling module -- the shape the real corpus's own
  `assert_unlinkable`/linking cases use. Function imports resolve to a
  real `CrossModuleFunction` wrapper whose `call()` re-enters
  `WasmRuntime::call_typed` against the *callee's own* instance state,
  reusing already-tested machinery for genuine cross-instance calls, not
  just link-time type declarations. See `code/specs/
  W10-wasm-real-linking-and-unlinkable.md`.
- `assert_unlinkable` is now graded for real (`grade_assert_unlinkable`)
  instead of unconditionally `NotYetSupported` -- build failure,
  structural-validation failure, or a genuine link failure via
  `RegistryHost` all count as the expected outcome (matching
  `grade_assert_invalid`'s own precedent: the harness only needs the
  OUTCOME category to match, not the specific reason).
- `Executor.current_link_failed` replaces the old blanket
  `current_has_imports` gate: a module that fails to LINK for a genuine
  capability gap (an import from a host module `RegistryHost` doesn't
  know about, e.g. `spectest`) now cascades to `NotYetSupported` for
  subsequent `invoke`/`get` actions targeting it -- same outcome as
  before for every currently-vendored file, but for the real, specific
  reason rather than "any import present at all."
- 7 new tests: totally-unknown-module/unknown-export/type-mismatch
  `assert_unlinkable` cases (all now real `Pass`es), and a genuine
  cross-instance function call round-trip proving the positive linking
  path works end to end.

### Known limitation, named not silent

- `RegistryHost::resolve_memory`/`resolve_table` return a real CLONE of
  the exporting instance's memory/table, not a live shared view (both
  `HostInterface` methods return owned values by their existing
  signature) -- link-time limits compatibility is still checked for
  real, but a write through the importing instance won't become visible
  to the exporting one. None of the corpus vendored so far exercises
  that; revisit if a future vendored file needs it.

### Deferred, not silently dropped

- The real, pinned-commit `imports.wast` (93 `assert_unlinkable` cases,
  mostly `register`-based sibling-module linking this PR's
  `RegistryHost` can grade for real) was fetched and attempted, but
  fails to PARSE entirely: its "auxiliary modules to import from"
  section uses `(tag ...)` declarations (WebAssembly exceptions
  proposal syntax) `wasm-wast-parser` has no grammar support for at all.
  Not vendored this PR -- see the new backlog item tracking this
  (needs at least minimal structural `tag` parsing support first). The
  real linking/`assert_unlinkable` machinery itself (`RegistryHost`,
  `CrossModuleFunction`, `wasm-runtime`'s link-failure path) is fully
  implemented and verified via hand-written in-crate test scripts
  instead, and remains ready to grade `imports.wast` for real the
  moment it can parse.

## 0.1.14 — 2026-08-15 — vendor atomic.wast + regen baseline (WASM18)

Vendors `proposals/threads/atomic.wast` from the same pinned commit as
the rest of the corpus (fetch script updated to handle the one file
living at an upstream subdirectory path different from its local flat
filename), and regenerates the baseline against `wasm-execution` 0.6.9 /
`wasm-validator` 0.2.3 / `wasm-wast-parser` 0.1.9's new atomics support.

Verified via a full structured diff of every already-parsing file's
tally against the pre-WASM18 baseline: every non-`atomic.wast` entry is
byte-for-byte UNCHANGED. The entire aggregate delta (module +3, action
+59, `assert_invalid` +48, `assert_return` +142, `assert_trap` +45) is
exactly `atomic.wast`'s own per-file contribution -- zero regressions
elsewhere.

`atomic.wast` itself reached 100% on every directive kind except
`assert_trap`, which was 0/45 on the first regen -- the real corpus's 45
`assert_trap ... "unaligned atomic"` cases test a RUNTIME alignment
check `wasm-execution` didn't have yet (only the declared `align=`
immediate was validated statically; the effective runtime address
wasn't checked at all). `wasm-execution` 0.6.9 adds that check; a second
regen brought `assert_trap` to 45/45 and this is the baseline committed
here.

## 0.1.13 — 2026-08-15 — regen baseline for named global inline-import fix (WASM19)

Regenerates the baseline against `wasm-wast-parser` 0.1.8's fix for the
named global inline-import shorthand. No vendored-file-list change (same
pinned commit). Aggregate tallies and every already-parsing file's
pass/fail counts are byte-for-byte UNCHANGED (verified via a full
structured diff) -- the only change is `global.wast`'s `parse_failures`
entry moving further, from `at byte 17077: ... found "$g0"` to `at byte
17335: ... found "funcref"`. That new failure point is the SAME extended
active-`elem`-segment syntax (`(elem (table $t) (global.get $g2) funcref
(ref.func $f))`) that already blocks `call_indirect.wast`, and is already
tracked as out-of-scope in `code/specs/W08-wasm-funcref-externref.md`.

## 0.1.12 — 2026-08-15 — Ref comparison + regen baseline for funcref/externref (WASM17)

Adds `WasmValue::Ref` handling to `const_value_to_wasm_value`/
`value_matches_expected` (exact `ref.extern n`/`ref.null` literals, plus
the bare `(ref.null)`/`(ref.func)` wildcard forms that only ever appear as
an `assert_return` expectation) and regenerates the baseline against
`wasm-wast-parser` 0.1.7's new reference-types grammar.

No vendored-file-list change (same pinned commit). Aggregate tallies are
UNCHANGED (`module` 108/108, `assert_return` 13874/13876, etc.) -- verified
via a full structured diff against the pre-change baseline: every
already-parsing file's pass/fail counts are byte-for-byte identical, zero
regressions.

Three previously-`failed_to_parse` entries moved to a DIFFERENT, deeper
failure point (real progress, not yet a full pass -- each blocked on its
own separate, already-scoped-out gap):
- `global.wast`: `at byte 840: ... found "externref"` -> `at byte 17077:
  ... found "$g0"` (a named global inline-import shorthand gap, unrelated
  to reference types -- logged as its own backlog item, WASM19).
- `br_table.wast`: `at byte 50812: ... found "externref"` -> `at byte
  51401: ... found "list"` (a concrete `(ref null $t)` heap type --
  deliberately out of scope, see `code/specs/
  W08-wasm-funcref-externref.md`).
- `unreached-valid.wast`: `unknown instruction "ref.is_null"` -> `unknown
  instruction "call_ref"` (a tail-calls/GC-proposal instruction, out of
  scope).

Two entries UNCHANGED, each blocked immediately by its own separate,
already-scoped-out gap (not touched by this PR):
- `select.wast`: `unknown instruction "result"` -- `select (result T)` is
  a SEPARATE opcode (0x1C) from plain `select` (0x1B), needed to
  disambiguate a reference-typed `select`'s result; genuinely out of this
  PR's scope (see `wasm-wast-parser`'s own
  `select_with_explicit_result_type_annotation_is_a_known_gap` test).
- `call_indirect.wast`: `expected an index, found "func"` -- the extended
  active-`elem`-segment syntax (`(elem (table $t) (i32.const 0) func $g
  $h)`), a bulk-memory-adjacent grammar this PR's spec already excludes.

## 0.1.11 — 2026-08-14 — vendor 11 more MVP-scope testsuite files (WASM08)

Extends the vendored slice (same pinned commit, `28864811cf03bdbf88073378
6148feaba339582d` -- no re-pin) with 11 more WASM 1.0 MVP-core `.wast`
files, chosen by fetching the full upstream file listing at that pin and
filtering out anything referencing `"spectest"` or a bulk-memory/
reference-type table op (`table.get/set/size/grow/copy/fill/init`,
`memory.copy/fill/init`, `elem.drop`, `data.drop`) -- the same exclusion
criteria the original W05/PR3 slice used, just applied to the files that
slice didn't cover yet:

- `unreached-invalid.wast`, `unreached-valid.wast` -- dead-code type
  checking, directly exercising WASM06's new instruction-level type
  checker from a different angle than this repo's own hand-written
  tests. `unreached-invalid.wast`: 71/71 `assert_invalid` (100%).
  `unreached-valid.wast` currently fails to parse (`ref.is_null`, a
  reference-types proposal instruction outside this repo's scope) --
  vendored anyway since that's the same honestly-tracked "failed to
  parse" outcome several original-slice files already have for the same
  reference-types reason (`global.wast`, `select.wast`, `br_table.wast`,
  `call_indirect.wast`), not a new category of gap.
- `left-to-right.wast` (operand evaluation order): 95/95 `assert_return`.
- `memory_redundancy.wast`: 4/4 `assert_return` + 3/3 `action` (100%).
- `type.wast` (type-section declaration syntax): 1/1 `assert_malformed`.
- `obsolete-keywords.wast` (renamed-instruction rejection):
  11/11 `assert_malformed`.
- `float_memory.wast` currently fails to parse -- the same pre-existing
  "expected 1 or 2 limit numbers" gap `memory.wast`/`memory_grow.wast`/
  `float_exprs.wast` already have in the original slice.
- `id.wast` currently fails to parse -- an entire file dedicated to
  quoted-string identifier syntax (`$"arbitrary bytes"`), which this
  repo's hand-rolled tokenizer only supports for bare `$name` identifiers.
  A real, actionable gap, but out of this PR's "vendor more corpus" scope
  to also implement.
- `stack.wast` currently fails to parse -- `if $label ... else $label
  ... end $label` (an optional matching label repeated after `else`/
  `end`), a real WAT syntax gap in `wasm-wast-parser`'s `if` handling.
  Same "out of scope for this PR" reasoning as `id.wast`.
- `custom.wast` (custom-section handling): 6/8 `assert_malformed`.
- `utf8-invalid-encoding.wast`: all 176 cases `not_yet_supported` (UTF-8
  string-encoding validation in the binary format isn't implemented yet)
  -- correctly graded as such, not `Fail`.

Baseline: `assert_invalid` 838/838 -> 909/909 (+71, all from
`unreached-invalid.wast`). `assert_return` 13775/13777 -> 13874/13876
(+99). `assert_malformed` 51/53 -> 69/73 (+18 pass, +199
`not_yet_supported`, mostly `utf8-invalid-encoding.wast`'s 176 cases).
`module` 102/102 -> 108/108 (+6, one per newly-parsing file). Verified
via a full per-file diff against the pre-change baseline: zero
regressions on any pre-existing file.

## 0.1.10 — 2026-08-14 — baseline regenerated after the instruction-level type checker landed (WASM06)

`wasm-validator` 0.2.0 added a real per-instruction type checker (W02 Phase
2) to `validate()`. Baseline regenerated: `assert_invalid` 15/838 (826
`not_yet_supported`) → 838/838 (100%, only 3 `not_yet_supported`
remaining). `assert_return`/`module`/every other kind ended at the exact
same counts as before this change — zero regressions, verified via a full
per-file diff against the pre-change baseline.

Also fixed one stale test in this crate: `assert_invalid_accepted_by_structural_validator_is_not_yet_supported`
asserted the old "we can't tell" behavior for a module that's structurally
fine but semantically ill-typed (`(func (result i32))` with an empty
body). Now that `validate()` catches this for real, the case is correctly
graded `Pass`, not `NotYetSupported` — renamed and updated to assert that.

## 0.1.9 — 2026-08-13 — baseline regenerated after multi-value blocktypes were implemented (WASM04)

No code changes in this crate — `wasm-wast-parser` 0.1.6 and `wasm-execution`
0.6.7 added support for multi-value `block`/`loop`/`if` blocktypes (a
blocktype that's a type-section index, not just the MVP's empty/single-valtype
byte). Baseline regenerated: `assert_return` 13512/13521 (99.9%) →
13775/13780 (99.98%, +263 pass, -4 fail). `module` 98/98 → 102/102 (+4).
Verified via a full per-file diff:

- `block.wast`, `if.wast`, and `loop.wast` — previously failed to parse
  at all — now parse and pass in full (`assert_return` 52/52, 123/123,
  78/78 respectively).
- `fac.wast` also newly parses (unrelated pre-existing gap this happened
  to close too).
- `br.wast` (75/76 → 76/76) and `func.wast` (93/96 → 96/96) each had a
  handful of previously-failing `assert_return` cases fixed by the same
  interpreter change.
- No regressions: every file that changed strictly gained passes; nothing
  that previously passed now fails or is newly unsupported.

See `wasm-wast-parser`'s `0.1.6` and `wasm-execution`'s `0.6.7` changelog
entries for the full bug writeups.

## 0.1.8 — 2026-08-13 — baseline regenerated after the f32 NaN payload bug was fixed (WASM13)

No code changes in this crate — `wasm-execution` 0.6.6 fixed a real bug
(every f32 value silently lost its exact NaN bit pattern passing through
the interpreter's typed operand stack, an arithmetic-cast round-trip
artifact, not just an issue for values an opcode computed on) surfaced
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 13495/13518 (99.8%) → 13512/13518 (100.0%,
+17). Verified via a full per-file diff that exactly 4 files changed
(`conversions.wast`, `float_literals.wast`, `float_misc.wast`,
`local_tee.wast`), every one going from some real fails to zero — no
regressions, no partial fixes. See `wasm-execution`'s own `0.6.6`
changelog entry for the full bug writeup.

## 0.1.7 — 2026-08-13 — baseline regenerated after sign-extension/trunc_sat opcodes were added (WASM03)

No code changes in this crate — `wasm-opcodes` 0.2.1, `wasm-wast-parser`
0.1.5, and `wasm-execution` 0.6.5 added the sign-extension and trunc_sat
opcode families (plus fixed a real, pre-existing boundary bug in the
trapping `trunc_*` handlers those additions exposed) surfaced running this
crate's own harness against the real testsuite. Baseline regenerated:
`i32.wast`/`i64.wast`/`conversions.wast` go from full parse failures to
parsing and running (98.9%+ passing across them); `assert_return`
12235/12254 (99.8%) → 13495/13518 (99.8%, +1260 across the newly-parseable
files); `assert_trap` 331/331 (100%) → 418/418 (100%). Verified via a full
per-file diff that these 3 newly-parseable files are the only ones whose
tally changed anywhere in the corpus. See `wasm-execution`'s own `0.6.5`
changelog entry for the full bug writeup, including the 4 remaining
`conversions.wast` fails (an unrelated, already-tracked NaN-payload gap,
WASM13).

## 0.1.6 — 2026-08-13 — baseline regenerated after inline-import shorthand was fixed (WASM02)

No code changes in this crate — `wasm-wast-parser` 0.1.4 fixed a real bug
(`func`/`table`/`memory`/`global` **inline-import shorthand** wasn't
recognized, and fixing it exposed a deeper pre-existing indexing bug once
a module could combine an import with a same-kind real definition)
surfaced running this crate's own harness against the real testsuite.
Baseline regenerated: `func_ptrs.wast` goes from a full parse failure to
100% passing every directive kind it has; `assert_return` 12219/12238
(99.8%) → 12235/12254 (99.8%, +16). Verified via a full per-file diff that
`func_ptrs.wast` is the only file whose tally changed anywhere in the
corpus. See `wasm-wast-parser`'s own `0.1.4` changelog entry for the full
bug writeup.

## 0.1.5 — 2026-08-13 — baseline regenerated after (module quote/binary ...) directives were fixed (WASM12)

No code changes in this crate — `wasm-wast-parser` 0.1.3 fixed a real bug
(`(module quote/binary ...)` **directives** silently built an empty
module instead of the module the source actually described) surfaced
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 12215/12238 (99.8%) → 12219/12238 (99.8%,
+4); `assert_malformed` 145/147 → 33/35 graded (46 → 158
`NotYetSupported`) — a real, understood reclassification, not a
regression: many quote-module `assert_malformed` cases were previously
`Pass` only because the quote text failed to parse for the WRONG reason
(the missing-wrapper bug, not the case's actual intended malformation);
now that it parses correctly, this repo's still-missing instruction-level
type-checker genuinely can't tell those specific cases apart from a valid
module, so they honestly report `NotYetSupported` instead. Verified via a
full per-file diff against the previous baseline: every changed file's
`fail` count went down or stayed the same, never up. See
`wasm-wast-parser`'s own `0.1.3` changelog entry for the full bug writeup.

## 0.1.4 — 2026-08-13 — baseline regenerated after a branch double-pop bug fix (WASM11)

No code changes in this crate — `wasm-execution` 0.6.4 fixed a real bug
(a branch to an outer block double-popped `label_stack`, corrupting
control flow for any branch that unwound past one or more already-open
outer blocks) surfaced by running this crate's own harness against the
real testsuite's `switch.wast`. Baseline regenerated: `assert_return`
12171/12238 (99.4%) → 12215/12238 (99.8%).

## 0.1.3 — 2026-08-13 — baseline regenerated after a local-index bug fix (WASM14)

No code changes in this crate — `wasm-wast-parser` 0.1.2 fixed a real bug
(a declared local aliasing parameter index 0 when a function references
its signature only via `(type $sig)`) surfaced by running this crate's
own harness against the real testsuite. Baseline regenerated:
`assert_return` 12169/12238 (99.4%) → 12171/12238 (99.5%).

## 0.1.2 — 2026-08-13 — baseline regenerated after 3 real assert_return bug fixes (WASM07)

No code changes in this crate — `wasm-execution` 0.6.3 and `wasm-runtime`
0.5.1 fixed 3 real bugs (an implicit function-body branch label, an
`instance.memory`/`tables` loss after any trapped call, and
`call_indirect` checking against the wrong index space) surfaced by
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 12030/12238 (98.3%) → 12169/12238 (99.4%).
See those crates' own changelogs for the full bug writeups.

## 0.1.1 — 2026-08-13 — assert_exhaustion is graded for real (WASM01)

`wasm-execution` 0.6.2 added a real call-depth guard, closing the exact
gap that forced this crate to never execute `assert_exhaustion`
directives at all (an unbounded-recursion host-crash risk, not just a
coverage gap). Both vendored `assert_exhaustion` cases (`call.wast`) now
run for real and pass. Updated the module doc comment and `Executor`'s
own reasoning to match; the old `assert_exhaustion_is_never_executed`
test replaced with `assert_exhaustion_passes_on_real_unbounded_recursion`
and a matching `_fails_if_the_action_returns_normally` case. Baseline
regenerated: `assert_exhaustion 2/2 (100%)`, up from `0/2
(NotYetSupported)`.

A security review of `wasm-execution`'s guard found its first chosen
depth (200) wasn't actually safe on small thread stacks in a debug
build; the corrected, safe value (80) is deliberately conservative
enough that 2 previously-"passing" `assert_return` cases in `call.wast`
(`even(100)`/`odd(200)`, genuinely bounded mutual recursion needing more
than 80 levels) now correctly trap instead — see `wasm-execution`'s own
`0.6.2` changelog entry for the full trade-off reasoning and the tracked
follow-up. `assert_return` moved from 12032/12238 to 12030/12238 as a
direct, understood consequence, not a new bug.

## 0.1.0 — 2026-08-13 — initial release (W05 PR-4)

New crate. Runs the official WebAssembly spec testsuite's `.wast` scripts
against `wasm-execution` (via `wasm-runtime`/`wasm-wast-parser`) and
reports a real, git-pinned conformance baseline. Phase A of the
`wasm-execution`-as-good-as-wasmtime arc; see
`code/specs/W05-wasm-conformance-harness.md`.

- **`report`**: `DirectiveKind`/`DirectiveOutcome`/`Tally`/`ConformanceReport`
  — pass/fail/trap/not-yet-supported tallies broken down by directive kind
  (so "the interpreter is wrong" is never confused with "we haven't built
  the type-checker yet"), per file and aggregated, serializable to the
  golden baseline manifest. `ConformanceReport::parse_failures` tracks
  files whose `.wast` SCRIPT itself failed to parse as a distinct field,
  not an indistinguishable all-zero tally.
- **`lib`**: the directive executor — walks a script's directives in file
  order, maintains a module registry (keyed by `register` name, `None` for
  "the current module", sharing live instances via `Rc<RefCell<..>>` since
  a registered module IS the same instance, not a copy — `WasmInstance`
  isn't `Clone` anyway), and does bit-exact `assert_return` grading
  (including `nan:canonical`/`nan:arithmetic` NaN-class comparison) via
  `wasm-runtime`'s new `call_typed`.
  - `assert_invalid` routes through `wasm_validator::validate()`
    regardless; a structural rejection is a real `Pass`, an accept is
    `NotYetSupported` (no instruction-level type-checker exists yet).
  - `assert_malformed`'s binary variant grades for real via
    `wasm-module-parser`'s existing error paths; the `quote` (text)
    variant now also attempts a real re-parse via `wasm-wast-parser`
    (a reject is `Pass`; an accept is `NotYetSupported`, since a missing
    type-checker could be the real reason either way).
  - `assert_unlinkable` is always `NotYetSupported`:
    `WasmRuntime::instantiate` never actually fails on an unresolved
    import today.
  - `assert_exhaustion` is **never executed** — `wasm-execution` has no
    call-depth guard, so the deliberately unbounded recursion these cases
    trigger would overflow the real host stack (an uncatchable process
    abort, not a gradeable trap). Always `NotYetSupported` without running
    the action at all — a safety requirement, not just an honesty one.
    A security review flagged that this guard is keyed on the
    directive's own spelling in the source, not anything semantic — a
    runaway-recursive function invoked through a plain `Action`/
    `AssertReturn`/`AssertTrap` gets no protection. Currently safe only
    because the two vendored files with such functions both fail to
    parse for unrelated reasons (an accident of corpus coverage, not a
    guarantee) — documented with a loud in-code warning for whoever
    widens `wasm-wast-parser`'s grammar coverage or vendors more files
    next, since the real fix is a call-depth guard in `wasm-execution`
    itself, out of scope here.
- **`bin/wasm_conformance_report`**: the day-to-day deliverable — walks
  the vendored corpus, prints a per-file/aggregate table, and (with
  `--write-baseline`) regenerates the golden manifest.
- **`tests/testsuite_conformance.rs`**: one data-driven test (not one per
  file) diffing a fresh run against the committed baseline — fails on ANY
  drift, regression or improvement, naming the exact file/kind that
  changed. Verified this actually catches drift (not just passes
  vacuously) by deliberately corrupting a baseline entry and confirming
  the test fails with a clear diff, then restoring it.
- 20 unit tests plus the 2 corpus-driven tests, ~95%+ line coverage on the
  hand-written logic.

### The baseline itself

32 of 48 vendored files parse and run today; 16 fail to parse entirely
(tracked in `parse_failures`, not folded into misleading all-zero
tallies) — all legitimate, out-of-scope gaps for this phase: multi-value
block signatures, reference-types' `externref` and generalized `elem`
syntax, post-MVP saturating-truncation/sign-extension opcodes, and the
`func`/`global` inline-import shorthand (linking-adjacent, sharing this
phase's already-documented `spectest` deferral). Among the 32 that do
parse: `assert_return` 12032/12238 (98.3%), `assert_trap` 325/325
(100%), `assert_invalid` 11/11 graded (100%) with 412 `NotYetSupported`,
`assert_malformed` 145/147 (98.6%) with 46 `NotYetSupported`,
`assert_exhaustion` 0/2 graded (both `NotYetSupported` by design). See
`tests/fixtures/testsuite-status.json` for the exact, current, per-file
numbers this changelog entry is a snapshot of.

### Building this required real `wasm-wast-parser` bug fixes

Running the actual corpus (not just `wasm-wast-parser`'s own hand-written
unit tests) surfaced 4 genuine grammar bugs in that crate, plus one more
found by security review of the fix for one of them (an empty-list
`(table funcref ())` panic), fixed as part of landing this baseline
(folded `br_table`'s label/operand order was backwards, `(table reftype
(elem e*))`'s implied-size form was unhandled and its own fix had a
reachable panic on an empty inline list, and two hex-float-literal
gaps) — see `code/packages/rust/wasm-wast-parser/CHANGELOG.md`'s `0.1.1`
entry for the full detail. File-level parse failures dropped from 33/48
to 16/48 as a direct result.

### It also found 3 real `wasm-execution` float correctness bugs

`wasm-execution` 0.6.1 fixes three float-NaN/sign-handling bugs this
harness's very first real run against the interpreter surfaced —
`min`/`max` not propagating NaN (the single largest source of
`assert_return` failures in the whole corpus: fixing it alone moved the
aggregate `assert_return` pass rate from 94.1% to 98.3%), `nearest` not
preserving the sign of a zero result, and `ceil`/`floor`/`trunc` not
reliably quieting a signaling NaN input. See that crate's own `0.6.1`
changelog entry for the full detail.

The third of those was caught pushing this exact PR: CI's `ubuntu-latest`
build failed the `corpus_matches_the_committed_baseline` gate — a
genuine, reproducible platform difference between macOS (where this
baseline was first generated) and Linux, not a flake. Diagnosed by
reproducing Ubuntu's exact behavior locally via a `linux/amd64` Docker
container and bisecting which specific `f64.wast` cases differed. This
is exactly the failure mode the baseline gate exists to catch — a
silent, un-reviewed drift in what "conformant" means would have shipped
unnoticed without it. The final baseline was verified identical on both
macOS and a Linux container before push.

### `wasm-runtime::call_typed`

This crate's bit-exact `assert_return` grading needed a non-lossy call
entry point — added as `wasm-runtime` `0.5.0`; see that crate's own
changelog.
