# Changelog

All notable changes to this package will be documented in this file.

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
