# Changelog

All notable changes to this package will be documented in this file.

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
