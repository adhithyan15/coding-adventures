# Changelog

All notable changes to this package will be documented in this file.

## [0.9.73] - 2026-08-26 (W28 — shared, live `LinearMemory`/`Table` storage across instances)

### Fixed

- **Real interpreter correctness bug: an imported memory or table was a
  CLONE, not a shared live view.** `LinearMemory` and `Table` both used
  `#[derive(Clone)]` over plain owned fields (`data: Vec<u8>`/
  `current_pages: u32` for memory, `elements: Vec<Option<u32>>` for
  tables). `wasm-runtime`'s `instantiate()` resolves a memory/table import
  through `HostInterface::resolve_memory`/`resolve_table`, which hands
  back an owned VALUE that gets pushed directly into the importing
  instance's own `WasmInstance::memories`/`tables` — so every clone was a
  full, independent copy. A write through the IMPORTING instance's memory/
  table was invisible when read back through the EXPORTING instance, and
  vice versa: wrong for any real multi-module WASM program using a
  shared-memory/shared-table import (a common pattern — a "libc"-style
  module sharing its memory with several consumer modules), not just a
  conformance-corpus gap. `wasm-conformance`'s own `RegistryHost::
  resolve_memory`/`resolve_table` had already named this exact limitation
  in their doc comments.
- **Fix: `LinearMemory`/`Table`'s mutable storage now lives behind
  `Rc<RefCell<..>>`.** New private `MemoryStorage { data, current_pages }`
  and `TableStorage { elements }` structs hold everything a `grow`/store/
  `table.set` can actually mutate; `LinearMemory`/`Table` each hold
  `inner: Rc<RefCell<..Storage>>` plus their immutable-after-construction
  fields (`max_pages`/`is64` for memory, `max_size`/`is64` for tables)
  outside the `RefCell`. `#[derive(Clone)]` on the outer struct is now
  EXACTLY the right shape: cloning clones the `Rc` pointer, giving a
  second handle onto the SAME underlying storage, not a second copy of
  the bytes/elements. Every public method signature is unchanged (`&self`/
  `&mut self` exactly as before) — only the body now goes through one
  `borrow()`/`borrow_mut()` per call, scoped to a single statement, never
  held across two calls. The two raw-pointer-based cross-object
  primitives (`LinearMemory::copy_between`, `Table::copy_between`, both
  pre-existing `unsafe fn`s for `memory.copy $dst $src`/`table.copy $dst
  $src` when `$dst`/`$src` may alias the SAME object) remain sound
  unchanged: they already read the source range into an owned temporary
  `Vec` via one scoped borrow BEFORE taking a separate borrow for the
  destination write, so the two `RefCell` borrows never overlap even when
  `dst`/`src` share the same underlying `Rc<RefCell<..>>` (self-copy, or
  now also a shared cross-instance import).
- **Also fixed, surfaced BY this change: active element-segment
  application was not atomic per segment.** `wasm-runtime::instantiate()`
  applied each active element segment's entries one `Table::set` call at
  a time, propagating the first out-of-bounds error immediately — correct
  for a segment that's entirely out of bounds, but wrong for one that's
  only PARTIALLY out of bounds: earlier entries in that same segment had
  already been written by the time the trap fired, violating the real
  spec's per-segment atomicity (a single active segment is all-or-nothing;
  only *earlier, already-fully-applied* segments are guaranteed to persist
  past a *later* segment's trap — see `linking.wast`'s own "unlike the v1
  spec" comment). This was unobservable before this same PR's storage fix:
  a failed `instantiate()` call's local `tables` Vec (holding an
  independent CLONE of any imported table) was simply dropped on error, so
  a partial write vanished along with it regardless. It stopped being
  unobservable the moment table storage became genuinely shared — a
  partial write to a SHARED table now persists in the exporting instance's
  own storage even though the importing instance's `instantiate()` call
  fails. Fixed in `wasm-runtime` (see that crate's own CHANGELOG) by
  bounds-checking the WHOLE segment against `table.size()` before writing
  any entry, matching `LinearMemory::write_bytes`'s existing upfront-
  bounds-check shape (a single check before one `copy_from_slice`, never a
  byte-at-a-time loop that could partially write before trapping).
- **Known, deliberately out-of-scope remaining gap:** this fix makes a
  table's raw entries (bare `u32` function indices) and size genuinely
  shared and observable cross-instance, but `call_indirect` still resolves
  a table entry against the CALLING instance's own `func_bodies`/
  `host_functions` index space. A funcref written into a SHARED table by
  one module and `call_indirect`-invoked through a DIFFERENT module needs
  real cross-instance function IDENTITY (the same class of problem
  `WasmInstance::tag_identities` already solves for exception tags, W23,
  but requiring genuine cross-instance CALL DISPATCH, not just equality
  comparison) — a separate, larger follow-on. See `Table`'s own doc
  comment. `linking0.wast`/`linking3.wast` (newly vendored, see `wasm-
  conformance`'s CHANGELOG) each have exactly one `assert_return` that
  hits this remaining gap.
- Real corpus impact (`wasm-conformance`, same date): the already-vendored
  `linking.wast`'s `assert_return` tally improved from 48/65 to 54/65 with
  ZERO new failures anywhere else in the 216-file corpus (programmatically
  diffed baseline-to-baseline before pushing). Five new files vendored:
  `elem.wast`, `linking0.wast`, `linking1.wast`, `linking3.wast`,
  `load1.wast` — see that crate's own CHANGELOG for per-file numbers.

### Added

- New unit tests directly exercising `LinearMemory`/`Table`'s shared-clone
  semantics stay in `wasm-runtime` (see that crate's own CHANGELOG) since
  they need a real `WasmInstance`/`HostInterface` to build the import
  scenario; this crate's own existing memory/table unit tests were updated
  in place (not added to) where they previously reached into the
  now-relocated `data`/`elements`/`current_pages` fields directly (e.g.
  `mem.data[..]` -> `mem.inner.borrow().data[..]`) — no behavioral change,
  same assertions.

## [0.9.72] - 2026-08-26 (W27 — census batch: `ref.null` in a global's const-expr)

### Fixed

- **`evaluate_const_expr` gained a `0xD0` (`ref.null <heap_type>`) case.**
  This crate already handled `ref.null` fine as an ordinary instruction
  (the main decode loop) and as an element-segment entry, but never as a
  GLOBAL's init expression — any `(global <reftype> (ref.null ...))`
  simply trapped ("illegal opcode 0xD0 in constant expression") at
  instantiation instead of producing the null reference it always
  evaluates to. Same "heap-type byte doesn't change runtime behavior"
  reasoning as the main decode loop's own `0xD0` handler: skip the
  immediate (one byte for an abstract heap type, or `0x63` + a LEB128
  type index for a concrete `(ref.null $t)`) and push `WasmValue::Ref
  (None)`.
- Real corpus impact: this was reaching for `ref_null.wast`'s own
  `(global anyref (ref.null any))`-shaped cases — see `wasm-wast-
  parser`'s own CHANGELOG (same date) for the heap-type-keyword
  recognition this fix pairs with, and `wasm-conformance`'s CHANGELOG
  for why that specific file still isn't vendored despite both fixes.

## [0.9.71] - 2026-08-26 (W11 addendum — concrete function-type refs)

### Fixed

- **`ref.null`'s bytecode decode loop no longer hardcodes a 1-byte
  heap-type immediate.** `wasm-wast-parser` can now emit a MULTI-byte
  heap-type immediate for a concrete function-type ref (`0x63` tag +
  `LEB128(idx)`, `wasm-types` 0.1.12's `ValueType::ConcreteFuncRef`) —
  the old unconditional `offset += 1` would have consumed only the tag
  byte and mis-decoded the trailing LEB128 index byte(s) as the START of
  the next instruction. The decoder now checks for the `0x63` tag and, if
  present, skips the LEB128 sequence via `wasm_leb128::decode_unsigned`
  instead of a flat single-byte skip; every other heap type (the
  pre-existing single-byte abstract ones) is unaffected. Runtime
  semantics are unchanged either way (every null is still the same null
  in this repo's value model) — only the byte-accounting needed fixing.
- `WasmValue::default_for` gained a `ValueType::ConcreteFuncRef(_)` arm
  (same `Ref(None)` default every other nullable reference type gets).

## [0.9.70] - 2026-08-26 (W26 — table64 proposal, first slice)

### Added

- `Table::new_with_is64(initial_size: u64, max_size: Option<u64>, is64:
  bool) -> Result<Self, TrapError>`: a fallible constructor for a table
  that may be 64-bit-indexed, mirroring `LinearMemory::new_with_is64`
  (W25) exactly. The plain, infallible `new(u32, Option<u32>)` constructor
  is unchanged (always `is64: false`) — every existing call site needed no
  changes.
- `Table::is64(&self) -> bool`.
- `MAX_TABLE_ELEMENTS` (the existing 10,000,000-element resource limit) is
  now ALSO reused as the practical `is64` table instantiation-time cap —
  the same "reuse the existing 32-bit-shaped bound" move W25 made with
  `MAX_MEMORY64_INITIAL_PAGES`. An `is64` table's real spec ceiling
  (`u64::MAX`) is validation-time-acceptable but has no relation to what
  this interpreter can actually allocate; `new_with_is64` returns a real,
  gracefully-propagated `TrapError` (never a panic/allocator abort) if
  `initial_size` exceeds it — checked UNCONDITIONALLY, not only when
  `is64` (security review: a 32-bit table's `min` is already
  validator-capped at this exact same bound, so this is a pure,
  behavior-preserving no-op for every validated module, and removes this
  `pub` constructor's own safety from depending entirely on an invariant
  living in a different crate, `wasm-validator`).

See `code/specs/W26-wasm-table64-first-slice.md`.

## [0.9.69] - 2026-08-26 (W25 — memory64 proposal, first slice)

### Added

- `LinearMemory::new_with_is64(initial_pages: u64, max_pages: Option<u64>,
  is64: bool) -> Result<Self, TrapError>`: a fallible constructor for a
  memory that may be 64-bit-addressed. The plain, infallible `new(u32,
  Option<u32>)` constructor is unchanged (always `is64: false`) — every
  existing call site (55 of them, mostly unit tests) needed no changes.
- `LinearMemory::is64(&self) -> bool`.
- `MAX_MEMORY64_INITIAL_PAGES: u64 = 65536` — this interpreter's own
  practical, implementation-defined resource limit on a 64-bit memory's
  ACTUAL allocation, deliberately separate from (and far smaller than)
  `wasm-validator`'s spec-conformance ceiling for a 64-bit memory's
  DECLARED `min`/`max` (`2^48` pages — not a safe allocation target for
  any real system: `2^48 * 65536 = 2^64` bytes, overflowing a 64-bit
  byte-count multiplication outright). A module that only ever declares
  such a memory still validates successfully; only an actual
  instantiation attempt hits this cap, as a real `TrapError`, never a
  panic or allocator abort.
- `decode_leb_u64`: decodes an unsigned LEB128 `u64` for the `memarg`
  immediate's `offset` field, which the real spec encodes as `u64`
  unconditionally (verified live against `https://webassembly.github.io/
  spec/core/binary/instructions.html`), not just for a 64-bit memory.

### Changed

- `DecodedOperand::MemArg.offset` widened `u32` → `u64` (see above).
- The shared `pop_effective_addr` helper (new, in `register_memory`) and
  every one of the ~23 scalar load/store opcode handlers (`i32.load`...
  `i64.store32`, `0x28`-`0x3E`) now pop `WasmValue::I64` instead of
  `WasmValue::I32` for the address operand when the target memory is
  `is64`. `memory.size`/`memory.grow` (`0x3F`/`0x40`) push/pop `I64`
  instead of `I32` under the same condition.
- `LinearMemory::grow`'s hard page-count ceiling is `is64`-aware
  (`MAX_MEMORY64_INITIAL_PAGES` instead of the 32-bit spec's own 65536).
- **Security fix**: `LinearMemory::bounds_check`'s `offset + width` used
  unchecked `usize` addition — harmless while every caller's `offset`
  came from a 32-bit address (at most `2 * u32::MAX`, nowhere near
  `usize::MAX`). An `is64` memory's address can be a full `u64`, making
  this genuinely, adversarially reachable (e.g. `i32.load (i64.const
  -1)` against a 64-bit memory) — an unchecked overflow would panic in a
  debug build or silently WRAP to a small, incorrectly-in-bounds sum in
  release. Fixed with `checked_add`, the same overflow-proof shape
  `copy`/`fill` already use.

See `code/specs/W25-wasm-memory64-first-slice.md`.

## [0.9.68] - 2026-08-26 (Exceptions proposal, fourth slice W24: real exnref, catch_ref/catch_all_ref, throw_ref)

### Added

- `WasmExecutionContext::exception_heap: Vec<ExceptionPayload>` — a real
  `exnref` heap, the same "handle into a per-call heap" shape `gc_heap`/
  `v128_heap` already use for their own non-numeric reference kinds. A
  `catch_ref`/`catch_all_ref` clause that matches now pushes a genuine
  `WasmValue::Ref(Some(handle))` (via new `push_caught_exception`,
  bounded by new `MAX_EXCEPTION_HEAP_LEN` = 1,000,000, mirroring
  `push_v128`/`MAX_V128_HEAP_LEN`'s own security-review shape) instead of
  never being selected as a match at all.
- `throw_ref` (`0x0A`) opcode handler: pops a `WasmValue::Ref` (an
  `exnref`), traps `"null exception reference"` if null, otherwise looks
  up the full `ExceptionPayload` it names in `ctx.exception_heap` and
  re-raises it verbatim (same tag, tag identity, and argument values) via
  `TrapError::exception_with_payload` — a real re-throw, not a fresh
  unrelated exception.

### Changed

- `try_catch_exception`: `CatchClauseKind::CatchRef`/`CatchAllRef` now
  match under the EXACT same rule as their non-`_ref` counterparts
  (`catch_clause_tag_matches`/unconditional, respectively) instead of
  never matching. On a match, `CatchRef` pushes the tag's argument values
  THEN a reified `exnref`; `CatchAllRef` pushes only the reified `exnref`.
- `ValueType::default_for(Exnref)` and `CatchClauseKind`'s own doc
  comments updated to describe the new real semantics (previously
  documented as permanently inert).
- Blocktype decoding (`decode_function_body`'s `"blocktype"` operand
  decoder and `block_arity`) now recognizes `0x69` (`exnref`) as a
  single-value shorthand blocktype byte, alongside the existing `0x7B`/
  `0x70`/`0x6F` (v128/funcref/externref) special cases — the same real,
  previously-undetected gap those three closed once already (WASM17/SIMD),
  now hit by `throw_ref.wast`'s own `(block $h (result exnref) ...)`
  shape: without this, the byte fell through to the type-index branch and
  misread trailing bytes as a bogus type index.

### Fixed

- **Security review finding**: the `exnref` blocktype byte above was
  originally `0xE9` (matching this crate's — incorrect — pre-existing
  `ValueType::Exnref` wire byte), which has its LEB128 continuation bit
  SET, making it indistinguishable from the leading byte of a genuine
  multi-byte type index: a module declaring 234+ types could trigger a
  silent blocktype misparse (attacker-reachable, since type-section size
  is entirely module-controlled). Fixed at the source — `wasm-types`
  0.1.9 corrects `ValueType::Exnref`'s wire encoding to `0x69` (the real
  spec-correct single-byte SLEB128 encoding of `-0x17`, continuation bit
  clear) — and this crate's two blocktype-decode sites updated to match.

See `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`.

## [0.9.67] - 2026-08-26 (Exceptions proposal, third slice W23: cross-instance tag identity)

### Added

- `ExceptionPayload` gains `tag_identity: u64` — a canonical, globally-
  unique tag identity (`0` = "none configured"), alongside the existing
  `instance_id`/`tag_idx`. `TrapError::exception_with_payload` takes it as
  a new parameter; the sentinel-prefixed `VMError` wire format
  (`encode_exception_payload`/`decode_exception_payload`) carries it as a
  new length-delimited field.
- `WasmExecutionContext`/`WasmExecutionEngine` gain `tag_identities: Vec<u64>`
  (same combined tag-index space as `tags`), threaded via a new optional
  setter `WasmExecutionEngine::set_tag_identities`, mirroring `set_tags`
  exactly. `throw` now also carries `ctx.tag_identities[tag_idx]` (or `0`
  if unconfigured) in the exception it raises.
- `HostInterface::resolve_tag`'s return type changes from `Option<FuncType>`
  to `Option<(FuncType, u64)>` — the exporting instance's own canonical
  tag identity, adopted verbatim by an importer (see `wasm-runtime`'s own
  changelog for how it's minted/threaded).

### Changed

- `try_catch_exception`'s old `instance_id == 0 || instance_id !=
  ctx.instance_id` early-return gate is REMOVED. A `catch` clause now
  matches via real tag-identity comparison (`catch_clause_tag_matches`,
  new), falling back to the old same-context raw-index comparison only
  when no real identity was configured on either side (every pre-existing
  hand-built unit test that never calls `set_tag_identities`) — this keeps
  every W21/W22 unit test passing unmodified. A `catch_all` clause now
  matches UNCONDITIONALLY, including a foreign exception that crossed a
  cross-instance host-function call boundary, matching the real spec's
  own "`try_table` catches foreign exceptions... as well" rule (previously
  wrongly refused by the same gate this removes).
- `ExceptionPayload::instance_id`/`WasmExecutionContext::instance_id` are
  no longer read by any matching logic — still minted and carried
  (nothing depended on removing them), doc comments updated to say so.

### Fixed

- Real corpus: `try_table.wast`'s `catch-imported`, `catch-imported-alias`,
  and `imported-mismatch` `assert_return` directives — the exact three
  cross-instance cases W22 named and deliberately deferred — now pass for
  real (`assert_return` 25/13/5 pass/fail/not-yet-supported → 28/10/5; no
  other file's stats moved). See `code/specs/
  W23-wasm-exceptions-cross-instance-tag-identity.md`.

## [0.9.66] - 2026-08-25 (Exceptions proposal, second slice W22: real catch/catch_all matching)

### Added

- Real, same-instance `catch`/`catch_all` matching: `try_table` no longer
  unconditionally propagates an exception uncaught. `TrapError` gains
  `exception: Option<ExceptionPayload>` (`instance_id`, `tag_idx`,
  `values`) alongside `is_exception`; `TrapError::exception_with_payload`
  is the new constructor real `throw` uses (pops the tag's real declared
  param values via a new `WasmExecutionContext::tags: Vec<u32>` field,
  wired the same optional-setter way `type_section` already is).
- `decode_function_body`'s `try_table` (`0x1F`) branch now builds a real
  `TryTableInfo { block_type, catches: Vec<CatchClause> }` (previously
  decoded and discarded, W21) via a new per-function side-table
  (`ctx.try_table_infos`), mirroring the existing `br_table_targets`/
  `gc_ops` precedent. `Label` gains `catches: Vec<CatchClause>`.
- `try_catch_exception` (new): the ONE choke point every instruction
  handler's `Result` passes through searches the current frame's
  `label_stack` innermost-to-outermost for a matching `catch`/`catch_all`
  clause (first listed wins), pushes the tag's payload for `catch`, and
  branches via the existing `execute_branch` — `catch_ref`/
  `catch_all_ref` clauses are structurally present but never selected (no
  `exnref` value is ever produced this slice).
- Cross-instance exceptions are deliberately never matched: a fresh
  `instance_id` per `WasmExecutionEngine::new` gates every match attempt,
  so an exception that crossed a nested cross-module host-function call
  boundary never produces a false-positive match on a coincidentally-equal
  raw tag index.
- `WasmExecutionEngine::set_tags` (new optional setter, mirrors
  `set_type_section`).
- `ValueType::Exnref` (new, deliberately inert — see `wasm-types`'s own
  changelog) plumbed through `WasmValue::default_for` and
  `wasm-runtime`'s legacy `call()` i64 round-trip.
- `DEDICATED_STACK_SIZE` doubled 8 MiB → 16 MiB: adding per-frame state
  to the recursive `call_function_inner` dispatch path measurably eroded
  `MAX_CALL_DEPTH`'s existing ~1.5x safety margin, reproducibly
  overflowing the real thread stack on the vendored `call_indirect.wast`.
  `MAX_CALL_DEPTH` itself is deliberately left unchanged (see
  `code/specs/W22-wasm-exceptions-catch-clause-matching.md`).

### Fixed

- `call_function_inner`'s dispatch loop keeps its catch-or-propagate
  logic inlined (not a call to the new shared `run_dispatch_loop`, which
  `call_function_impl`'s non-recursive top-level entry uses instead) —
  the recursive path is stack-depth-sensitive; see `DEDICATED_STACK_SIZE`
  above for why this distinction is load-bearing, not stylistic.

## [0.9.65] - 2026-08-25 (Exceptions proposal, first slice W21: tag/throw real conformance)

### Added

- `throw` (`0x08`): unconditionally raises an uncaught WASM exception --
  this repo implements no catch-clause matching (see `try_table` below),
  so every `throw` propagates all the way out to the top-level
  `call_function` caller, exactly like any other trap already does via
  `?`-based `Result` propagation.
- `try_table` (`0x1F`): a near-verbatim copy of `block` (`0x02`)'s own
  execution handler -- pushes a `Label`, closes on the matching `end`.
  Its catch-clause list is decoded and DISCARDED by `decode_function_body`
  (never retained), so an exception (or an ordinary trap) raised inside
  its body propagates straight through uncaught -- the real spec's own
  defined behavior for "no catch clause matched," which is ALWAYS true
  here since this slice never looks for a match. `build_control_flow_map`
  treats `0x1F` as an opener alongside `0x02..=0x04` so its matching `end`
  resolves the same way.
- `TrapError.is_exception: bool` (new field, defaults `false` via the
  existing `TrapError::new` constructor) and `TrapError::exception(msg)`
  (sets it `true`) -- distinguishes an uncaught WASM **exception** from an
  ordinary trap, a real spec distinction (`try_table` never catches a
  trap, only an exception) `wasm-conformance`'s `assert_exception`/
  `assert_trap` grading now depends on.
- `EXCEPTION_SENTINEL` + `vm_error_to_trap_error`: `VMError`
  (`virtual-machine`, a generic cross-language-frontend shared type) has
  no room for a real boolean flag, so `is_exception` is round-tripped
  through the message string itself via a sentinel prefix, recovered at
  the one place a `VMError` crosses back into this crate's own public
  `Result<_, TrapError>` API for a top-level `call_function`.
- `"tagidx"` recognized as a generic immediate kind in `decode_immediates`
  (plain LEB128 index, same shape as `funcidx`/`localidx`/etc.), backing
  `throw`'s decode.

### Security

- Security review found `decode_function_body`'s new `try_table`
  catch-clause decode loop trusted the attacker-controlled `catch_count`
  LEB128 immediate alone (up to ~4.3 billion), with no bound tied to the
  function body's real remaining byte count -- a ~10-byte truncated body
  naming a huge count could spin the loop billions of times before ever
  reaching `decode_leb_u32`'s own silent past-the-end default. Fixed:
  the loop now breaks the moment real bytes run out, bounding total cost
  by the body's actual length, never by the fabricated count -- same
  discipline `wasm-validator`'s sibling `0x1F` decode already had for
  free (its `decode_idx` errors on truncation instead of defaulting).
  New regression test proves a wildly-large `catch_count` over a
  truncated body completes instantly rather than hanging.
- Hardened `vm_error_to_trap_error`'s `EXCEPTION_SENTINEL` recovery to
  anchor on the sentinel appearing immediately after `VMError::
  GenericError`'s own fixed `"Error: "` `Display` prefix (`.strip_prefix`
  chained), not a generic `.contains`/`.split_once` search anywhere in
  the message -- eliminates any theoretical future risk of a message
  that merely contains the sentinel substring elsewhere being
  misclassified as an exception.

See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.

## [0.9.64] - 2026-08-25 (GC epic, first slice W20: real i31ref box/unbox semantics + i31.get_u)

### Fixed

- **`ref.i31`/`i31.get_s` (WasmGC `0xFB` sub-opcodes `0x1C`/`0x1D`) were
  literal stack-identity no-ops**, correct only by coincidence for the
  small, always-positive, bit-30-clear integers this repo's own LANG77
  Lisp-compiler tests happened to box (7, 9, 42, ...). Fixed to real
  spec semantics: `ref.i31` masks its `i32` operand to the low 31 bits
  (`& 0x7FFF_FFFF`); `i31.get_s` sign-extends from bit 30
  (`(v << 1) as i32 >> 1`) after checking for a null reference. Verified
  against `i31.wast`'s own real test vectors (e.g.
  `i31.get_s(0x7fff_ffff)` must be `-1`, which the old no-op behavior got
  wrong). See `code/specs/W20-wasm-gc-i31-conformance.md`.

### Added

- `i31.get_u` (WasmGC `0xFB` sub-opcode `0x1E`, new): pops a non-null
  `i31ref`, zero-extends (masks to 31 bits), pushes the result. Same
  null-reference trap (`"null i31 reference"`) as `i31.get_s`.
- `evaluate_const_expr` (the separate, restricted constant-expression
  evaluator used for global initializers): added `0xFB 0x1C` (`ref.i31`)
  support, needed by `i31.wast`'s `(global $i (ref i31) (ref.i31
  (i32.const 2)))`. Any other `0xFB` sub-opcode in a constant expression
  is still a clean error.
- New `pop_i31_payload` helper: pops a value expected to be a plain `i32`
  i31ref payload, trapping cleanly on a null reference (`WasmValue::
  Ref(None)`, which `ref.null i31` still produces, exactly like every
  other heap type's null) or any other type mismatch.
- Unit tests: `ref.i31`'s masking, `i31.get_s`'s sign-extension and
  `i31.get_u`'s zero-extension (using the exact `0xaaaa_aaaa`/
  `0xcaaa_aaaa`/`0x7fff_ffff` vectors `i31.wast` itself uses), both
  opcodes' null-reference trap, and `evaluate_const_expr`'s new
  `ref.i31` support (including rejecting an unrelated `0xFB` sub-opcode).

## [0.9.63] - 2026-08-25 (Relaxed SIMD epic PR6: i16x8/i32x4.relaxed_dot_i8x16_i7x16_s/_add_s)

### Added

- `SimdOpKind::RelaxedDotI8x16I7x16S` execution arm: BINARY, reinterprets
  both popped `v128`s as 16 signed `i8` lanes each; for each of the 8
  result lanes, computes `a[2i] * b[2i] + a[2i+1] * b[2i+1]` in `i32`
  arithmetic, truncating down to `i16` for the result -- same
  per-pair multiply-accumulate shape as `DotI16x8S`, one lane width
  narrower.
- `SimdOpKind::RelaxedDotI8x16I7x16AddS` execution arm: TERNARY -- the
  first two operands feed the same per-pair signed-`i8`
  multiply-accumulate, widened and pairwise-folded four-at-a-time into
  4 `i32` lanes, then added into the third operand's `i32x4`
  accumulator. The first ternary SIMD op in this crate whose third
  operand is a genuine numeric accumulator.
- Both ops read their operands as plain signed `i8` throughout ("signed
  * signed") -- hand-verified against every `either` alternative in the
  real vendored `relaxed_dot_product.wast` corpus (pinned
  `WebAssembly/testsuite` SHA `28864811cf03bdbf880733786148feaba339582d`)
  to land on one literal alternative in each ambiguous case, and to
  match every exact (non-`either`) case bit-for-bit.
- New tests: `i16x8_relaxed_dot_i8x16_i7x16_s_matches_the_real_corpus_exact_cases`,
  `i16x8_relaxed_dot_i8x16_i7x16_s_matches_the_signed_signed_either_alternative`,
  `i32x4_relaxed_dot_i8x16_i7x16_add_s_matches_the_real_corpus_exact_cases`,
  `i32x4_relaxed_dot_i8x16_i7x16_add_s_matches_the_signed_signed_either_alternative`,
  `relaxed_dot_product_family_is_self_consistent_across_repeated_invocations`.
- Verified bit-for-bit identical under Linux/x86_64 via Docker
  (`rust:latest`, `--platform linux/amd64`) in addition to native
  macOS/ARM64 -- the implementation is pure integer arithmetic
  (`wrapping_mul`/`wrapping_add` over `i32`), with no floating-point
  rounding or platform-dependent SIMD intrinsics involved, so no
  platform divergence is possible.

## [0.9.62] - 2026-08-25 (Relaxed SIMD epic PR5: f32x4/f64x2.relaxed_madd/relaxed_nmadd)

### Added

- `SimdOpKind::RelaxedMaddF32x4`/`RelaxedNmaddF32x4` and
  `SimdOpKind::RelaxedMaddF64x2`/`RelaxedNmaddF64x2` execution arms
  (sub-opcodes `0x105`-`0x108`) -- the ELEVENTH/TWELFTH/THIRTEENTH/
  FOURTEENTH relaxed-simd opcodes (see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`). TERNARY: pops three v128s
  (`a`, `b`, `c`), pushes one -- the FIRST relaxed-simd family whose
  body is genuine per-lane FLOATING-POINT arithmetic rather than a
  bitwise blend (`Bitselect`/`RelaxedLaneselectI8x16` etc. are also
  ternary but bitwise, lane-width-agnostic).
- Semantics: `madd(a,b,c) = a*b+c`; `nmadd(a,b,c) = -(a*b)+c`,
  implemented per-lane as `a.mul_add(b, c)` and `(-a).mul_add(b, c)`
  respectively. The relaxed-simd spec leaves the rounding
  implementation-defined (fused, single rounding, OR unfused, double
  rounding); this repo picks FUSED via Rust's `f32::mul_add`/
  `f64::mul_add`, which the language guarantees is single-rounding
  regardless of whether the target has hardware FMA (a software
  fallback is used otherwise, but it is still single-rounding) -- so
  this choice is deterministic and platform-independent, unlike a
  naive `a * b + c` in source. Hand-verified against every `either`
  pair in the real vendored `relaxed_madd_nmadd.wast` corpus that this
  choice lands on the FIRST alternative in every fused-vs-unfused test
  case (e.g. `x=0x1.000004p+0, y=0x1.0002p+0, z=-0x1.000204p+0`:
  `x.mul_add(y, z)` = `0x1p-37`, the corpus's first alternative; an
  unfused `x*y+z` would give `0`, the second).
- Confirmed the `nmadd(a,b,c) == madd(-a,b,c) == madd(a,-b,c)` identity
  by hand against the corpus's own "nmadd tests with negated x/y, same
  answers are expected [as madd]" cases.
- Verified this platform-independence claim by also running the
  conformance suite under Linux/x86_64 via Docker (`rust:latest`,
  `--platform linux/amd64`), in addition to native macOS/ARM64 -- both
  agree bit-for-bit, matching the corpus's baseline exactly.
- New tests: `f32x4_relaxed_madd_matches_the_real_corpus_first_either_alternative_flt_max`,
  `f32x4_relaxed_madd_nmadd_match_the_real_corpus_special_values_precision_case`,
  `f64x2_relaxed_madd_nmadd_match_the_real_corpus_special_values_precision_case`,
  `f32x4_relaxed_madd_is_self_consistent_across_repeated_invocations` --
  the last one mirrors the real corpus's own `*_cmp`/"test-consistent-
  nondeterminism" assertions, confirming this repo's fixed FUSED choice
  produces bit-identical results across repeated invocations with
  identical operands.

## [0.9.61] - 2026-08-25 (Relaxed SIMD epic PR4: i8x16/i16x8/i32x4/i64x2.relaxed_laneselect)

### Added

- `SimdOpKind::RelaxedLaneselectI8x16`/`RelaxedLaneselectI16x8`/
  `RelaxedLaneselectI32x4`/`RelaxedLaneselectI64x2` execution arm
  (sub-opcodes `0x109`-`0x10c`) -- the SEVENTH/EIGHTH/NINTH/TENTH
  relaxed-simd opcodes (see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`). TERNARY: pops three v128s
  (`a`, `b`, mask `c`), pushes one -- the same shape as this crate's
  existing `Bitselect` arm, whose body (bytewise `(a AND c) OR (b AND
  (NOT c))`) this new arm reuses verbatim, hand-verified against every
  `either` group in the real vendored `relaxed_laneselect.wast` corpus
  to be an exact, literal match to the first alternative in every case
  -- including the file's own "impure mask" pblendvb special case (a
  THREE-alternative `either`). The first relaxed-simd family to reuse a
  TERNARY base opcode's body rather than a binary/unary one.
- 5 new unit tests: a cross-lane-width pure-mask test mirroring the
  existing `v128_bitselect_selects_bits_per_mask` test, plus one
  dedicated test per opcode reproducing the real corpus's exact first
  `either` alternative byte-for-byte.

## [0.9.60] - 2026-08-25 (Relaxed SIMD epic PR3: f32x4/f64x2 relaxed_min/relaxed_max)

### Added

- `SimdOpKind::RelaxedMinF32x4`/`RelaxedMaxF32x4` execution arm
  (sub-opcodes `0x10d`/`0x10e`) and `SimdOpKind::RelaxedMinF64x2`/
  `RelaxedMaxF64x2` execution arm (sub-opcodes `0x10f`/`0x110`) -- the
  THIRD/FOURTH/FIFTH/SIXTH relaxed-simd opcodes (see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`): byte-for-byte copies of the
  existing `PminF32x4`/`PmaxF32x4`/`PminF64x2`/`PmaxF64x2` arms' bodies
  (`lane_b < lane_a ? lane_b : lane_a` for min, `lane_a < lane_b ?
  lane_b : lane_a` for max -- IEEE-754 `<` directly, always `false` when
  either operand is NaN, so the result is the first/lower-pushed
  operand UNCHANGED on a NaN or a signed-zero tie). The relaxed-simd
  spec permits EITHER this behavior OR the stricter NaN-propagating/
  signed-zero-canonicalizing `MinF32x4`/`MaxF32x4`/`MinF64x2`/
  `MaxF64x2` behavior; hand-verified against the real upstream
  `relaxed_min_max.wast` corpus's own `either` groups (the FIRST
  relaxed-simd file whose `either` groups carry FOUR alternatives, not
  two): this repo's chosen `pmin`/`pmax`-style behavior computes, for
  every test case in that file, an EXACT, literal match to the corpus's
  second `either` alternative -- not merely "a member of some looser
  equivalence" -- so no new numeric logic was needed, only reuse of the
  existing `Pmin`/`Pmax` bodies.
- New unit tests: `f32x4_relaxed_min_max_return_the_first_operand_
  unchanged_when_either_operand_is_nan`, `f32x4_relaxed_min_max_normal_
  case_matches_plain_less_than_select`, and their `f64x2` mirrors.

## [0.9.59] - 2026-08-25 (Relaxed SIMD epic PR2: i16x8.relaxed_q15mulr_s)

### Added

- `SimdOpKind::RelaxedQ15mulrI16x8S` execution arm (sub-opcode `0x111`,
  the SECOND relaxed-simd opcode -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`): a byte-for-byte copy of the
  existing `Q15mulrSatI16x8S` arm's body (per-lane Q15 rounding fixed-
  point multiply: sign-extend both `i16`s to `i32`, add the rounding
  constant `0x4000`, arithmetic-shift right by 15, clamp to `i16::MIN..
  =i16::MAX`). The relaxed-simd spec permits EITHER saturating OR
  wrapping for the single overflow lane pattern (both operand lanes
  `i16::MIN`); hand-verified against the real upstream
  `i16x8_relaxed_q15mulr_s.wast` corpus's own `either`-wrapped expected
  values that this repo's existing saturating body computes the file's
  overflow test case's EXACT expected vector, `[32767, 32767, 32766,
  0,0,0,0,0]` -- a literal match to that `either` pair's second
  alternative -- so no new numeric semantics were needed. Own match arm
  rather than merging into `Q15mulrSatI16x8S`'s pattern, matching this
  match's existing convention of one arm per `SimdOpKind`.
- Confirms no new decoder infrastructure was needed for this opcode:
  `0x111` LEB128-encodes as the 2-byte sequence `[0x91, 0x02]`, the same
  2-byte-continuation shape this crate already decodes for
  `RelaxedSwizzle`/base-SIMD values `>= 0x80`.
- New tests: `i16x8_relaxed_q15mulr_s_matches_strict_op_in_the_non_
  overflow_case`, `i16x8_relaxed_q15mulr_s_saturates_the_min_min_
  overflow_lane_to_i16_max`.

## [0.9.58] - 2026-08-25 (Relaxed SIMD epic PR1: i8x16.relaxed_swizzle)

### Added

- `SimdOpKind::RelaxedSwizzle` execution arm (sub-opcode `0x100`, the
  first opcode of the relaxed-simd epic that follows the now-complete
  base SIMD epic, PR1-PR47 -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`): a byte-for-byte copy of the
  existing `Swizzle` arm's body (pop index vector `s`, pop data vector
  `a`, `result[i] = a[s[i]]` if `s[i] < 16` else `0`). The relaxed-simd
  spec deliberately leaves out-of-range-index behavior implementation-
  defined; hand-verified against the real upstream
  `i8x16_relaxed_swizzle.wast` corpus's own `either`-wrapped expected
  values that this repo's existing clamp-to-zero choice is a literal
  member of every `either` pair in that file, so no new numeric
  semantics were needed -- own match arm rather than merging into
  `Swizzle`'s pattern, matching this match's convention of one arm per
  `SimdOpKind`.
- Confirms no new decoder infrastructure was needed for this opcode:
  `0x100` is the first sub-opcode in this table `>= 0x100`, and
  LEB128-encodes as the 2-byte sequence `[0x80, 0x02]` -- the same
  2-byte-continuation shape this crate already decodes for base-SIMD
  values `>= 0x80` (e.g. `i32x4.add`'s `0xAE` -> `[0xAE, 0x01]`).
- New tests: `i8x16_relaxed_swizzle_permutes_lanes_by_the_index_vector`,
  `i8x16_relaxed_swizzle_out_of_range_index_lane_produces_zero`.

## [0.9.57] - 2026-08-25 (SIMD PR47: v128.load64_lane/store64_lane)

### Added

- `decode_function_body`'s `0xFD` branch: the `sub_opcode == 0x54 ||
  sub_opcode == 0x58 || sub_opcode == 0x55 || sub_opcode == 0x59 ||
  sub_opcode == 0x56 || sub_opcode == 0x5A` gate (from PR44/PR45/PR46)
  widened to also intercept `sub_opcode == 0x57 || sub_opcode == 0x5B`
  -- `v128.load64_lane`/`v128.store64_lane`, one width up from PR46's
  32-bit pair, reuse the IDENTICAL `DecodedOperand::SimdMemLane` shape
  unchanged (`ImmLaneIdx2` is also a single raw byte per BinarySIMD.md,
  so the one-byte lane read needs no change) -- no new decoder
  infrastructure needed this time, just the widened condition.
- `register_simd`: a new `sub_opcode == 0x57 || sub_opcode == 0x5B`
  early-dispatch branch (same "intercept before the generic `SimdOpKind`
  lookup" pattern PR44/PR45/PR46's own branches use), implementing:
  - `v128.load64_lane` (`0x57`): pop the existing `v128` (top of stack),
    pop the `i32` base address, bounds-checked 8-byte (little-endian)
    read from memory 0 at `base + memarg offset` (via the existing
    full-width `load_i64`), overwrite ONLY the selected lane's 8 bytes
    of the popped `v128` (the other lane passes through unchanged),
    push the result.
  - `v128.store64_lane` (`0x5B`): pop the `v128` to read a lane from,
    pop the `i32` base address, bounds-checked 8-byte write of the
    selected lane's 8 bytes to memory 0 at `base + memarg offset`.
  - Lane-index bounds check is `>= 2`, NOT the 32-bit pair's `>= 4` --
    an `i64x2` v128 holds only 2 lanes (8 bytes each), not `i32x4`'s 4
    (4 bytes each); reusing the wider bound would silently accept an
    invalid lane index 2-3.
- `SimdOpKind::Load64Lane | SimdOpKind::Store64Lane` added to the
  `unreachable!` match arm (both are always intercepted by the early
  dispatch above, before the generic `SimdOpKind` lookup runs).
- 5 new unit tests: lane-preservation (load), memarg-offset honoring
  (load), neighboring-byte isolation (store), past-end-of-memory
  trapping (both directions), and out-of-range (`>= 2`) lane-index
  rejection (both directions) -- direct mirrors of PR46's own 32-bit
  test suite, at the 64-bit width. Closes the entire lane-load/store
  family (PR44-47) and, with it, the larger load-extend/splat/zero/lane
  epic started in PR40.

## [0.9.56] - 2026-08-25 (SIMD PR46: v128.load32_lane/store32_lane)

### Added

- `decode_function_body`'s `0xFD` branch: the `sub_opcode == 0x54 ||
  sub_opcode == 0x58 || sub_opcode == 0x55 || sub_opcode == 0x59` gate
  (from PR44/PR45) widened to also intercept `sub_opcode == 0x56 ||
  sub_opcode == 0x5A` -- `v128.load32_lane`/`v128.store32_lane`, one
  width up from PR45's 16-bit pair, reuse the IDENTICAL
  `DecodedOperand::SimdMemLane` shape unchanged (`ImmLaneIdx4` is also a
  single raw byte per BinarySIMD.md, so the one-byte lane read needs no
  change) -- no new decoder infrastructure needed this time, just the
  widened condition.
- `register_simd`: a new `sub_opcode == 0x56 || sub_opcode == 0x5A`
  early-dispatch branch (same "intercept before the generic `SimdOpKind`
  lookup" pattern PR44/PR45's own branches use), implementing:
  - `v128.load32_lane` (`0x56`): pop the existing `v128` (top of stack),
    pop the `i32` base address, bounds-checked 4-byte (little-endian)
    read from memory 0 at `base + memarg offset` (via the existing
    full-width `load_i32`), overwrite ONLY the selected lane's 4 bytes
    of the popped `v128` (every other lane passes through unchanged),
    push the result.
  - `v128.store32_lane` (`0x5A`): pop the `v128` to read a lane from,
    pop the `i32` base address, bounds-checked 4-byte write of the
    selected lane's 4 bytes to memory 0 at `base + memarg offset`.
  - Lane-index bounds check is `>= 4`, NOT the 16-bit pair's `>= 8` --
    an `i32x4` v128 holds 4 lanes (4 bytes each), not `i16x8`'s 8 (2
    bytes each); reusing the wider bound would silently accept an
    invalid lane index 4-7.
- `SimdOpKind::Load32Lane | SimdOpKind::Store32Lane` added to the
  `unreachable!` match arm (both are always intercepted by the early
  dispatch above, before the generic `SimdOpKind` lookup runs).
- 5 new unit tests: lane-preservation (load), memarg-offset honoring
  (load), neighboring-byte isolation (store), past-end-of-memory
  trapping (both directions), and out-of-range (`>= 4`) lane-index
  rejection (both directions) -- direct mirrors of PR45's own 16-bit
  test suite, at the 32-bit width.

## [0.9.55] - 2026-08-25 (SIMD PR45: v128.load16_lane/store16_lane)

### Added

- `decode_function_body`'s `0xFD` branch: the `sub_opcode == 0x54 ||
  sub_opcode == 0x58` gate (from PR44) widened to also intercept
  `sub_opcode == 0x55 || sub_opcode == 0x59` -- `v128.load16_lane`/
  `v128.store16_lane`, one width up from PR44's 8-bit pair, reuse the
  IDENTICAL `DecodedOperand::SimdMemLane` shape unchanged (both
  `ImmLaneIdx16` and `ImmLaneIdx8` are single raw bytes per
  BinarySIMD.md, so the one-byte lane read needs no change) -- no new
  decoder infrastructure needed this time, just the widened condition.
- `register_simd`: a new `sub_opcode == 0x55 || sub_opcode == 0x59`
  early-dispatch branch (same "intercept before the generic `SimdOpKind`
  lookup" pattern PR44's own `0x54`/`0x58` branch uses), implementing:
  - `v128.load16_lane` (`0x55`): pop the existing `v128` (top of stack),
    pop the `i32` base address, bounds-checked 2-byte (little-endian)
    read from memory 0 at `base + memarg offset`, overwrite ONLY the
    selected lane's 2 bytes of the popped `v128` (every other lane
    passes through unchanged), push the result.
  - `v128.store16_lane` (`0x59`): pop the `v128` to read a lane from,
    pop the `i32` base address, bounds-checked 2-byte write of the
    selected lane's 2 bytes to memory 0 at `base + memarg offset`.
  - Lane-index bounds check is `>= 8`, NOT the 8-bit pair's `>= 16` --
    an `i16x8` v128 holds 8 lanes (2 bytes each), not `i8x16`'s 16 (1
    byte each); reusing the wider bound would silently accept an
    invalid lane index 8-15.
- `SimdOpKind::Load16Lane | SimdOpKind::Store16Lane` added to the
  `unreachable!` match arm (both are always intercepted by the early
  dispatch above, before the generic `SimdOpKind` lookup runs).
- 8 new unit tests: lane-preservation (load), memarg-offset honoring
  (load), neighboring-byte isolation (store), past-end-of-memory
  trapping (both directions), and out-of-range (`>= 8`) lane-index
  rejection (both directions) -- direct mirrors of PR44's own 8-bit
  test suite, at the 16-bit width.

## [0.9.54] - 2026-08-25 (SIMD PR44: v128.load8_lane/store8_lane)

### Added

- New `DecodedOperand::SimdMemLane { sub_opcode, offset, lane }` variant:
  the FIRST SIMD operand shape needing THREE things together (a memarg,
  a lane-index immediate, and -- read off the stack at execute time, not
  carried in the operand itself -- an existing `v128`). No existing
  variant fits: `MemArg` has no lane-index field; `Simd{sub_opcode,aux}`
  (the shape every OTHER memarg-carrying SIMD op reuses) has only one
  spare `u32` slot, already fully spoken for by the memarg offset.
  `convert_operand` packs this via the SAME `simd_consts` const-pool
  side-table `V128Const`/`Shuffle` already use for their own oversized
  immediates (offset packed into slot bytes 0-3, lane into byte 4) --
  sound for the identical reason `Shuffle`'s own doc comment gives: no
  new field to thread through every call-frame save/restore site
  `simd_consts` is already wired through.
- `decode_function_body`'s `0xFD` branch: a new `sub_opcode == 0x54 ||
  sub_opcode == 0x58` arm, checked BEFORE the generic memarg-detection
  gate (every load/store-shaped SIMD opcode with an offset/align
  immediate must be in this gate, per PR40's own decoder-desync lesson)
  -- these two sub-opcodes carry ONE MORE immediate (the lane index)
  than that gate knows to consume, so routing them through it would
  silently drop the lane-index byte and desync the rest of the function
  body.
- `register_simd`: a new `sub_opcode == 0x54 || sub_opcode == 0x58`
  early-dispatch branch (same "intercept before the generic `SimdOpKind`
  lookup" pattern `v128.const`/`i8x16.shuffle` already use), implementing:
  - `v128.load8_lane` (`0x54`): pop the existing `v128` (top of stack),
    pop the `i32` base address, bounds-checked 1-byte read from memory 0
    at `base + memarg offset`, overwrite ONLY the selected lane of the
    popped `v128` with that byte (every other lane passes through
    unchanged), push the result.
  - `v128.store8_lane` (`0x58`): pop the `v128` to read a lane from, pop
    the `i32` base address, bounds-checked 1-byte write of the selected
    lane's byte to memory 0 at `base + memarg offset`.
  - Defense-in-depth lane-index bounds check (`>= 16` errors cleanly,
    same discipline as `i8x16.shuffle`'s own `>= 32` check) -- real
    validation-time rejection lives in `wasm-validator` (see that
    crate's own changelog), this is only a backstop against a hand-built
    instruction stream that skipped validation.
- 6 new dedicated unit tests: lane-preservation (every OTHER lane of the
  base `v128` unchanged after `load8_lane`), nonzero memarg offset
  honored, `store8_lane` writes only the selected lane with no
  neighboring-byte corruption, out-of-bounds-memory traps cleanly (not a
  panic) for both directions, and out-of-range lane index (`>= 16`)
  errors cleanly (not a panic) for both directions.

## [0.9.53] - 2026-08-25 (SIMD PR42: v128.load_extend family)

### Added

- `v128.load8x8_s`/`_u`, `v128.load16x4_s`/`_u`, `v128.load32x2_s`/`_u`
  (`SimdOpKind::Load8x8S`/`Load8x8U`/`Load16x4S`/`Load16x4U`/
  `Load32x2S`/`Load32x2U`, sub-opcodes `0x01`-`0x06`): pop the `i32` base
  address, add the instruction's own `memarg` offset, then read 8 raw
  bytes total from memory 0 as a set of narrow lanes (8x1, 4x2, or 2x4
  bytes depending on width) via the existing scalar `load_i32_8s`/
  `load_i32_8u`/`load_i32_16s`/`load_i32_16u`/`load_i64_32s`/
  `load_i64_32u` narrow loaders -- each already bounds-checked and
  already implementing the correct sign/zero-extension semantics reused
  unchanged from the scalar `iNN.load8_s`/etc. family. Each loaded lane
  is placed independently (little-endian) into the corresponding, WIDER
  lane of a new `v128`: `load8x8` produces `i16x8`, `load16x4` produces
  `i32x4`, `load32x2` produces `i64x2`. The FIRST opcodes in this table
  that widen EACH loaded lane independently, rather than broadcasting one
  value (`Load8Splat` etc., PR40) or zero-filling the unused lanes
  (`Load32Zero`/`Load64Zero`, PR41).

### Fixed

- The `0xFD`-prefixed SIMD instruction decoder's memarg-detection gate
  (widened in PR40/PR41 to cover `0x07..=0x0A` and `0x5C`/`0x5D`) did not
  yet recognize `0x01..=0x06`. Folded into the existing splat-family
  range, now `0x01..=0x0A` -- without this, every `v128.load_extend` op
  with a non-zero `offset=` immediate would have silently fallen through
  to the "no immediate" decode arm (leaving `aux` at 0) and read from the
  wrong address. Caught by `v128_load8x8_s_honors_a_nonzero_memarg_offset`
  and the upstream `simd_load_extend.wast` corpus's own offset-variant
  `assert_return` directives, all of which would otherwise silently
  mis-grade. Same lesson PR40/PR41's own decoder fixes already
  established -- every new memarg-carrying SIMD opcode must be added to
  this gate, not just to the executor's own `match`; re-verified this gate
  explicitly for this PR's own new sub-opcode range rather than assuming
  a prior widening still covers it.

### Tests

- `v128_load8x8_s_sign_extends_each_byte_into_its_own_i16_lane`,
  `v128_load8x8_u_zero_extends_each_byte_into_its_own_i16_lane`,
  `v128_load16x4_s_sign_extends_each_halfword_into_its_own_i32_lane`,
  `v128_load16x4_u_zero_extends_each_halfword_into_its_own_i32_lane`,
  `v128_load32x2_s_sign_extends_each_word_into_its_own_i64_lane`,
  `v128_load32x2_u_zero_extends_each_word_into_its_own_i64_lane`: each
  writes real bytes to memory via plain scalar `i32.store`s (with the
  LAST narrow lane's high bit deliberately set, e.g. byte `0x80`) and
  confirms the exact lane-by-lane sign- or zero-extended result -- the
  correctness-critical distinction this whole family exists to test (a
  byte/halfword/word with its high bit set must sign-extend to a
  NEGATIVE lane for `_s` but zero-extend to a small POSITIVE lane for
  `_u`).
- `v128_load8x8_s_honors_a_nonzero_memarg_offset`: same "prove the
  `memarg` offset, not just the base address, is honored" discipline
  PR40/PR41 established, this PR's own regression test for the
  decoder-gate fix above.
- `v128_load_extend_family_past_the_end_of_memory_traps_cleanly_not_panic`:
  same "verify bounds guards adversarially" discipline as the
  `load_splat`/`load_zero` families, checked for all 6 new opcodes (every
  variant reads exactly 8 bytes total regardless of lane width, so one
  shared width applies to all of them).

## [0.9.52] - 2026-08-25 (SIMD PR41: v128.loadN_zero family)

### Added

- `v128.load32_zero`/`load64_zero` (`SimdOpKind::Load32Zero`/
  `Load64Zero`, sub-opcodes `0x5C`/`0x5D`): pop the `i32` base address,
  add the instruction's own `memarg` offset, bounds-checked read of 4/8
  raw bytes (little-endian) from memory 0 via the existing full-width
  `load_i32`/`load_i64` loaders, place those bytes in the LOW 32/64 bits
  of a new `v128` and ZERO the remaining bytes. Same "load then fill a
  v128" shape as `Load32Splat`/`Load64Splat` (SIMD PR40), but zeroed
  instead of repeated.

### Fixed

- The `0xFD`-prefixed SIMD instruction decoder's memarg-detection gate
  (widened in PR40 to cover `0x07..=0x0A`) only recognized `v128.load`/
  `v128.store`/the `load_splat` family. Widened again to also cover
  `0x5C`/`0x5D` -- without this, every `v128.loadN_zero` with a non-zero
  `offset=` immediate would have silently fallen through to the "no
  immediate" decode arm (leaving `aux` at 0) and read from the wrong
  address. Caught by `v128_load32_zero_honors_a_nonzero_memarg_offset`
  and the upstream `simd_load_zero.wast` corpus's own offset-variant
  `assert_return` directives, all of which would otherwise silently
  mis-grade. Same lesson PR40's own decoder fix already established --
  every new memarg-carrying SIMD opcode must be added to this gate, not
  just to the executor's own `match`.

### Tests

- `v128_load32_zero_places_four_bytes_in_the_low_lane_and_zeroes_the_
  rest`, `v128_load64_zero_places_eight_bytes_in_the_low_lane_and_
  zeroes_the_rest`: each reads back the exact bytes a plain scalar
  `i32.store`/`i64.store` wrote to real memory, placed in the low lane
  with the rest ZEROED -- same "prove it reads genuine `LinearMemory`
  content" discipline as the existing `v128.load`/`load_splat` tests.
- `v128_load32_zero_honors_a_nonzero_memarg_offset`: pins the
  decoder-gate fix above.
- `v128_load_zero_family_past_the_end_of_memory_traps_cleanly_not_panic`:
  both widths trap, not panic, when `address + width` overruns memory.

## [0.9.51] - 2026-08-25 (SIMD PR40: v128.loadN_splat family)

### Added

- `v128.load8_splat`/`load16_splat`/`load32_splat`/`load64_splat`
  (`SimdOpKind::Load8Splat`/`Load16Splat`/`Load32Splat`/`Load64Splat`,
  sub-opcodes `0x07`-`0x0A`): pop the `i32` base address, add the
  instruction's own `memarg` offset, bounds-checked read of 1/2/4/8 raw
  bytes (little-endian) from memory 0, broadcast into all 16/8/4/2 lanes
  of a new `v128`. First opcodes in this crate that fuse a real
  linear-memory read with a lane broadcast in one instruction. Reuses
  the existing narrow scalar loaders (`load_i32_8u`/`load_i32_16u`) and
  full-width loaders (`load_i32`/`load_i64`) purely for their
  bounds-checked reads -- same memory-0-only scope as `v128.load`/
  `v128.store` (SIMD widen PR15).

### Fixed

- The `0xFD`-prefixed SIMD instruction decoder's memarg-detection gate
  (`sub_opcode == 0x00 || sub_opcode == 0x0B`, deciding whether an
  instruction's `align`/`offset` immediate gets decoded into `aux` at
  all) only recognized `v128.load`/`v128.store`. Widened to also cover
  `0x07..=0x0A` -- without this, every `v128.loadN_splat` with a
  non-zero `offset=` immediate would have silently fallen through to the
  "no immediate" decode arm (leaving `aux` at 0) and read from the wrong
  address. Caught by `v128_load8_splat_honors_a_nonzero_memarg_offset`
  and the upstream `simd_load_splat.wast` corpus's own offset-variant
  `assert_return` directives, all of which would otherwise silently
  mis-grade.

### Tests

- `v128_load8_splat_broadcasts_one_byte_into_all_16_lanes`,
  `v128_load16_splat_broadcasts_two_bytes_into_all_8_lanes`,
  `v128_load32_splat_broadcasts_four_bytes_into_all_4_lanes`,
  `v128_load64_splat_broadcasts_eight_bytes_into_both_lanes`: each reads
  back the exact bytes a plain scalar `i32.store`/`i64.store` wrote to
  real memory, broadcast into every lane -- same "prove it reads genuine
  `LinearMemory` content" discipline as the existing `v128.load` tests.
- `v128_load8_splat_honors_a_nonzero_memarg_offset`: pins the decoder-gate
  fix above.
- `v128_load_splat_family_past_the_end_of_memory_traps_cleanly_not_panic`:
  all 4 widths trap, not panic, when `address + width` overruns memory.

## [0.9.50] - 2026-08-25 (PR39 CI fix: f32x4/f64x2 rounding family cross-platform NaN quieting)

### Fixed

- `f32x4.ceil`/`floor`/`trunc`/`nearest` and `f64x2.ceil`/`floor`/
  `trunc`/`nearest` (SIMD widen PR39, added in 0.9.49) now canonicalize
  a NaN lane to the platform-independent quiet NaN (`f32::NAN`/
  `f64::NAN`) instead of falling through to `f32::ceil()`/`floor()`/
  `trunc()`/`round_ties_even()` (or the `f64` equivalents) on a NaN
  input. This is the exact same cross-platform gap the scalar
  `f32.ceil`/`floor`/`trunc` opcodes (`0x8D`/`0x8E`/`0x8F`) already hit
  and fixed -- a SIGNALING NaN input's quiet bit through these
  functions is platform-dependent (macOS passed, Linux CI failed with
  8 real `assert_return` mismatches in `simd_f32x4_rounding.wast`,
  confirmed by reproducing under a Linux/x86_64 container), but WASM's
  `nan:arithmetic` result class always requires the quiet bit SET
  regardless of the input NaN's own bit pattern. New tests
  `f32x4_rounding_family_quiets_a_signaling_nan_lane` and
  `f64x2_rounding_family_quiets_a_signaling_nan_lane` pin this down
  the same way `test_f32_ceil_floor_trunc_quiets_signaling_nan`/
  `test_f64_ceil_floor_trunc_quiets_signaling_nan` do for the scalar
  opcodes.

## [0.9.49] - 2026-08-25 (SIMD widen PR39: f32x4/f64x2 ceil/floor/trunc/nearest rounding family)

### Added

- `f32x4.ceil`/`floor`/`trunc`/`nearest` and `f64x2.ceil`/`floor`/
  `trunc`/`nearest` (8 new opcodes, sub-opcodes `0x67`/`0x68`/`0x69`/
  `0x6A` and `0x74`/`0x75`/`0x7A`/`0x94`): two new combined match arms
  in `run_simd_op`, `CeilF32x4 | FloorF32x4 | TruncF32x4 | NearestF32x4`
  and `CeilF64x2 | FloorF64x2 | TruncF64x2 | NearestF64x2`, same UNARY
  "pop one v128, push one" shape as the existing `AbsF32x4`/`SqrtF32x4`/
  `AbsF64x2` arms. `ceil`/`floor`/`trunc` use Rust's native
  `f32::ceil()`/`floor()`/`trunc()` (`f64` equivalents for the `f64x2`
  arm) directly -- already IEEE-754 compliant (`roundToIntegralToward
  Positive`/`Negative`/`Zero`), NaN payload/sign preserved, signed zero
  preserved, infinities pass through unchanged, no bespoke handling
  needed. `nearest` uses `round_ties_even()` (IEEE-754
  `roundToIntegralTiesToEven`), DELIBERATELY NOT `round()` -- `round()`
  breaks ties away from zero (`2.5 -> 3.0`), which is the WRONG answer
  for WASM's `nearest`; `round_ties_even()` breaks ties toward even
  (`2.5 -> 2.0`), the spec-correct behavior.
- 7 new unit tests covering both directions of rounding for ordinary
  fractional values, `nearest`'s ties-to-even behavior specifically
  (including a genuine tie case in each direction), and NaN/signed-zero/
  infinity preservation across all four rounding modes for both shapes.

## [0.9.48] - 2026-08-24 (task #229-231 — SIMD widen PR38: i8x16.shuffle, unlocks 268 stuck directives in the already-vendored simd_lane.wast)

### Added

- `i8x16.shuffle` (sub-opcode `0x0D`): the most structurally complex
  SIMD opcode implemented in this crate so far. Decoded as a new
  `DecodedOperand::Shuffle([u8; 16])` variant (mirrors `V128Const`'s
  16-byte-raw-immediate decode arm exactly, since the binary shape is
  identical -- 16 raw, non-LEB128 bytes right after the sub-opcode), and
  packed into the SAME `ctx.simd_consts` const-pool `convert_operand`
  already uses for `v128.const`, tagged with sub-opcode `0x0D` instead
  of `0x0C` so `register_simd` can tell the two apart. Reusing the pool
  (rather than adding a second `Vec<[u8; 16]>` field to
  `WasmExecutionContext`) avoided touching every call-frame save/restore
  site that field would otherwise need threading through.
- `register_simd`'s dispatch: a new `sub_opcode == 0x0D` early
  special-case (mirroring the existing `sub_opcode == 0x0C` v128.const
  case, both intercepted before the generic `SimdOpKind` lookup). Pops
  TWO v128 operands (rhs on top, popped first, per the usual WASM
  binary-op convention), conceptually concatenates them into a 32-byte
  `combined` array (first-popped/bottom operand = lanes 0-15,
  second-popped/top operand = lanes 16-31), then for each of the 16
  output lanes reads `combined[immediate[i]]`.
- Security: every one of the 16 immediate bytes is guaranteed `0..=31`
  for any module that passed `wasm-validator`'s new validation-time
  check (see that crate's own changelog) -- this executor's own
  `idx >= 32` guard is real bounds-checking kept as DEFENSE IN DEPTH
  (this crate has no validation pass of its own; a hand-built
  instruction stream, as this crate's own unit tests build directly,
  can still reach it), not a claim that a validated module could ever
  trip it. On that path it returns a clean `VMError`, never a panic or
  an out-of-bounds read of the 32-byte `combined` array -- no `%`-wrap,
  no unchecked index.
- 5 new unit tests: identity shuffle (indices 0-15, copies the first
  operand unchanged), pure-second-operand shuffle (indices 16-31),
  reverse-second-operand shuffle, an interleaving shuffle that reads
  from BOTH operands' halves of the combined space in one instruction,
  and the out-of-range-immediate-byte defense-in-depth case above.

## [0.9.47] - 2026-08-24 (task #226-228 — SIMD widen PR37: extract_lane/replace_lane family, remaining shapes)

### Added

- New dispatch arms for the 10 remaining extract_lane/replace_lane
  opcodes: `i16x8.extract_lane_s`/`_u` (sign-/zero-extend the 2-byte
  lane to `i32`), `i16x8.replace_lane` (truncate the popped `i32` to its
  low 16 bits), `i32x4.replace_lane` (full-width, no truncation),
  `i64x2.extract_lane`/`replace_lane` (native `i64`, no widening),
  `f32x4.extract_lane`/`replace_lane`, `f64x2.extract_lane`/
  `replace_lane`. Every arm bounds-checks the lane index BEFORE indexing
  the 16-byte lane buffer (0-7 for `i16x8`, 0-3 for `i32x4`/`f32x4`,
  0-1 for `i64x2`/`f64x2`), same discipline as the existing
  `i8x16.extract_lane_s`/`_u`/`replace_lane` handlers -- a clean
  `VMError`, never an out-of-bounds panic.
- The `0xFD`-prefix decoder's lane-immediate sub-opcode check widened
  from an explicit 4-value list (`0x15`/`0x16`/`0x17`/`0x1B`) to the now
  fully-contiguous `0x15..=0x22` range -- every one of the 14
  extract_lane/replace_lane opcodes carries a single raw (non-LEB128)
  lane-index byte immediate.

### Changed

- New unit tests: sign/zero-extension for `i16x8.extract_lane_s`/`_u`,
  replace-then-extract round-trips for every new `replace_lane` variant
  (proving the write actually landed, not just that SOME v128 came
  back), out-of-range lane index rejection for all 10 new opcodes (a
  clean `Result::Err`, not a panic).

## [0.9.46] - 2026-08-24 (task #223-225 — SIMD widen PR36: i64x2.extend_low/high_i32x4_s/u)

### Added

- New dispatch arm for `ExtendLowI32x4S`/`ExtendHighI32x4S`/
  `ExtendLowI32x4U`/`ExtendHighI32x4U`: UNARY -- pop ONE `v128`, reinterpret
  it as 4 `i32` lanes, take the LOW (indices 0-1) or HIGH (indices 2-3) 2
  lanes, sign- or zero-extend each to `i64`, producing an `i64x2` result.
  The THIRD and FINAL rung of the "extend" family, one lane width up from
  `ExtendLowI16x8S`/etc. (PR26) -- EXACTLY the lane-selection + extend
  half of the already-implemented `ExtmulLowI64x2S`/etc. handlers, minus
  the multiply.
- New tests:
  `i64x2_extend_low_high_i32x4_preserves_lane_order_and_selects_the_correct_half`
  (a sequential `0..4` operand proves lanes 0-1 go to `extend_low`,
  lanes 2-3 go to `extend_high`, in order, not reversed),
  `i64x2_extend_low_high_i32x4_distinguishes_signed_from_unsigned_at_the_i32_boundary`
  (`i32::MIN`/`i32::MAX` at the sign boundary, mirroring the existing
  i16x8/i32x4 boundary tests one lane width up), and
  `i64x2_extend_low_i32x4_sign_and_zero_extends_ordinary_positive_and_negative_values`
  (ordinary in-range positive/negative values, not just the extremes --
  confirms `_u` on a negative operand produces a large POSITIVE `i64`,
  never a negative one).

## [0.9.45] - 2026-08-24 (task #220-222 — SIMD widen PR35: f64x2.abs/min/max/pmin/pmax)

### Added

- New dispatch arm for `AbsF64x2`: UNARY, direct 2-lane mirror of
  `AbsF32x4` -- a pure bit operation (clear the sign bit of each of the
  2 `f64` lanes), no NaN/signed-zero subtlety, `f64::abs()` is directly
  correct.
- New dispatch arm for `MinF64x2`: BINARY, direct 2-lane mirror of
  `MinF32x4`'s exact NaN-canonicalization boilerplate -- if either lane
  is NaN the result lane is NaN; for a `-0.0`/`+0.0` tie, `-0.0` wins.
- New dispatch arm for `MaxF64x2`: BINARY, direct 2-lane mirror of
  `MaxF32x4`, same pop-order/lane shape as `MinF64x2` -- if either lane
  is NaN the result lane is NaN; for a `-0.0`/`+0.0` tie, `+0.0` wins
  (the mirror-image tie-break from `MinF64x2`'s `-0.0`).
- New dispatch arm for `PminF64x2`/`PmaxF64x2`: BINARY, direct 2-lane
  mirror of `PminF32x4`/`PmaxF32x4` -- a genuinely DIFFERENT and
  SIMPLER code path than `MaxF64x2`/`MinF64x2` -- a plain IEEE-754
  `<`-based conditional select (`pmin(a, b) = b < a ? b : a`,
  `pmax(a, b) = a < b ? b : a`), no NaN canonicalization. Since
  IEEE-754 `<` is always `false` when either operand is NaN, this
  returns the FIRST operand (`a`) unchanged whenever either operand is
  NaN -- NOT a canonicalized NaN the way `MaxF64x2`/`MinF64x2` would
  produce.
- New tests: `f64x2_abs_clears_sign_bit_and_leaves_nan_lane_nan`,
  `f64x2_min_propagates_nan_in_either_lane_regardless_of_operand_order`,
  `f64x2_min_signed_zero_tie_returns_negative_zero`,
  `f64x2_min_normal_case_picks_the_smaller_value`,
  `f64x2_max_propagates_nan_in_either_lane_regardless_of_operand_order`,
  `f64x2_max_signed_zero_tie_returns_positive_zero`,
  `f64x2_max_normal_case_picks_the_larger_value`,
  `f64x2_pmin_pmax_return_the_first_operand_unchanged_when_either_
  operand_is_nan` (the highest-risk correctness case in this PR, tested
  at both operand positions for both `pmin` and `pmax`),
  `f64x2_pmin_pmax_normal_case_matches_plain_less_than_select`.

## [0.9.44] - 2026-08-24 (task #217-219 — SIMD widen PR34: f32x4.max/pmin/pmax)

### Added

- New dispatch arm for `MaxF32x4`: BINARY, same pop-order/lane shape as
  `MinF32x4`, mirroring its exact NaN-canonicalization boilerplate --
  if either lane is NaN the result lane is NaN; for a `-0.0`/`+0.0`
  tie, `+0.0` wins (the mirror-image tie-break from `MinF32x4`'s
  `-0.0`).
- New dispatch arm for `PminF32x4`/`PmaxF32x4`: BINARY, a genuinely
  DIFFERENT and SIMPLER code path than `MaxF32x4`/`MinF32x4` -- a plain
  IEEE-754 `<`-based conditional select (`pmin(a, b) = b < a ? b : a`,
  `pmax(a, b) = a < b ? b : a`), no NaN canonicalization. Since IEEE-754
  `<` is always `false` when either operand is NaN, this returns the
  FIRST operand (`a`) unchanged whenever either operand is NaN -- NOT a
  canonicalized NaN the way `MaxF32x4`/`MinF32x4` would produce.
- New tests: `f32x4_max_propagates_nan_in_either_lane_regardless_of_
  operand_order`, `f32x4_max_signed_zero_tie_returns_positive_zero`,
  `f32x4_max_normal_case_picks_the_larger_value`,
  `f32x4_pmin_pmax_return_the_first_operand_unchanged_when_either_
  operand_is_nan` (the highest-risk correctness case in this PR, tested
  at both operand positions for both `pmin` and `pmax`),
  `f32x4_pmin_pmax_normal_case_matches_plain_less_than_select`.

### Added

- Two new dispatch arms covering `AddSatI8x16S`/`AddSatI8x16U`/
  `SubSatI8x16S`/`SubSatI8x16U` and `AddSatI16x8S`/`AddSatI16x8U`/
  `SubSatI16x8S`/`SubSatI16x8U`: BINARY, same pop-order/lane-count shape
  as `AddI8x16`/`SubI8x16`/`AddI16x8`/`SubI16x8`, but the result is
  CLAMPED to the lane type's range instead of wrapped. Signed lanes are
  computed in a wider intermediate type (`i16` for `i8x16`, `i32` for
  `i16x8` -- both provably wide enough that the widened add/sub itself
  never overflows) then `.clamp()`-ed to the target signed range before
  the narrowing cast back down. Unsigned lanes are computed in a signed
  wider type (`i32` for `i8x16`, `i64` for `i16x8`) specifically so a
  negative intermediate difference clamps to `0` instead of wrapping or
  panicking, then `.clamp()`-ed to `0..=<unsigned MAX>`.
- New tests: `i8x16_add_sat_s_and_sub_sat_s_saturate_signed_overflow_and_underflow`,
  `i8x16_add_sat_u_and_sub_sat_u_saturate_unsigned_overflow_and_underflow_to_zero_not_wrap`,
  `i16x8_add_sat_s_and_sub_sat_s_saturate_signed_overflow_and_underflow`,
  `i16x8_add_sat_u_and_sub_sat_u_saturate_unsigned_overflow_and_underflow_to_zero_not_wrap`
  -- each covers normal in-range results, overflow saturation, underflow
  saturation (including the classic unsigned-subtraction-goes-negative
  case, e.g. `3u8 - 10u8` must be `0`, not `249`), and exact boundary
  values at `MIN`/`MAX`.

## [0.9.42] - 2026-08-24 (task #211-213 — SIMD widen PR32: f64x2 eq/ne/lt/gt/le/ge)

### Added

- `register_simd` gains one new dispatch arm covering `EqF64x2`/
  `NeF64x2`/`LtF64x2`/`GtF64x2`/`LeF64x2`/`GeF64x2` (`match op.kind`
  inside): BINARY, pops two `v128`s, compares each of the 2 `f64` lane
  pairs with ordinary IEEE-754 comparison, pushes one `v128` boolean
  mask (all-1s/all-0s per lane) -- a direct 2-lane mirror of PR30's
  `f32x4` comparison family.
- Rust's native `f64` `==`/`!=`/`<`/`>`/`<=`/`>=` operators are already
  IEEE-754 compliant, so no bespoke NaN-detection logic is needed:
  `NaN == x` and `NaN <op> x` for any ordered `<op>` are already false,
  `NaN != x` (including `x == NaN`) is already true, and `+0.0 == -0.0`.
- New tests: `f64x2_cmp_family_uses_the_mask_convention_on_ordinary_values`,
  `f64x2_ordered_cmp_family_is_false_when_either_operand_is_nan`,
  `f64x2_ne_is_true_when_either_operand_is_nan_including_nan_vs_itself`,
  `f64x2_eq_ne_treat_positive_and_negative_zero_as_equal`,
  `f64x2_ordered_cmp_family_orders_negative_and_positive_values_correctly`.

## [0.9.41] - 2026-08-24 (task #208-210 — SIMD widen PR31: f64x2 neg/sqrt/add/sub/mul/div)

### Added

- `register_simd` gains three new dispatch arms, a direct structural
  mirror of PR29's `f32x4` arithmetic family at `f64x2`'s 2-lane width:
  - `NegF64x2`: UNARY, pops one `v128`, flips the sign bit of each of
    the 2 `f64` lanes.
  - `SqrtF64x2`: UNARY, pops one `v128`, IEEE-754 square root of each
    of the 2 `f64` lanes.
  - `AddF64x2`/`SubF64x2`/`MulF64x2`/`DivF64x2` (one combined arm,
    `match op.kind` inside): BINARY, pops two `v128`s, applies standard
    IEEE-754 arithmetic to each of the 2 `f64` lane pairs. `mul` is new
    (`f32x4.mul` predates PR29; `f64x2.mul` did not exist before this
    PR).
- Rust's native `f64` `-`/`sqrt()`/`+`/`-`/`*`/`/` operators are already
  IEEE-754 compliant, so no bespoke NaN/signed-zero handling is needed:
  `sqrt(negative) == NaN`, `sqrt(-0.0) == -0.0`, and `/`'s TOTAL
  behavior on a zero divisor (finite/`0.0` -> `+/-infinity`, `0.0/0.0`
  -> `NaN`, no trap, no panic).
- New tests: `f64x2_neg_flips_sign_bit_and_leaves_nan_lane_nan`,
  `f64x2_sqrt_computes_ieee754_square_root_per_lane`,
  `f64x2_add_adds_each_lane_pair`, `f64x2_sub_subtracts_each_lane_pair`,
  `f64x2_mul_multiplies_each_lane_pair_and_overflows_to_infinity`,
  `f64x2_div_divides_each_lane_pair`,
  `f64x2_div_by_zero_produces_signed_infinity_not_a_trap`,
  `f64x2_div_zero_by_zero_produces_nan`,
  `f64x2_add_sub_mul_div_propagate_nan_in_either_operand`.

## [0.9.40] - 2026-08-24 (task #205-207 — SIMD widen PR30: f32x4 eq/ne/lt/gt/le/ge)

### Added

- `register_simd` gains one new dispatch arm (`EqF32x4`/`NeF32x4`/
  `LtF32x4`/`GtF32x4`/`LeF32x4`/`GeF32x4` together, `match op.kind`
  inside), closing the `f32x4` comparison family gap -- the arithmetic
  family completed in PR29:
  - BINARY, pops two `v128`s, compares each of the 4 `f32` lane pairs
    with a native Rust `f32` comparison operator, pushes one `v128`
    boolean mask (all-1s/all-0s per lane) -- same lane-wise shape and
    mask convention as the integer comparison families (`Eq`/`EqI16x8`/
    `EqI8x16`/`EqI64x2` etc.), just at `f32x4`'s width with float
    operands and no signed/unsigned split.
  - Rust's native `f32` `==`/`!=`/`<`/`>`/`<=`/`>=` are already
    IEEE-754 compliant, so no bespoke NaN handling is needed: `NaN ==
    x`/`NaN <op> x` for any ordered `<op>` are already false, and
    `NaN != x` (including `x == NaN`) is already true -- same "native
    operator is already correct" discipline as PR29's `add`/`sub`/`div`,
    unlike `MinF32x4`'s bespoke tie-break logic.
- New unit tests:
  `f32x4_cmp_family_uses_the_mask_convention_on_ordinary_values`,
  `f32x4_ordered_cmp_family_is_false_when_either_operand_is_nan`,
  `f32x4_ne_is_true_when_either_operand_is_nan_including_nan_vs_itself`,
  `f32x4_eq_ne_treat_positive_and_negative_zero_as_equal`,
  `f32x4_ordered_cmp_family_orders_negative_and_positive_values_correctly`.

## [0.9.39] - 2026-08-24 (task #202-204 — SIMD widen PR29: f32x4 add/sub/div/neg/sqrt)

### Added

- `register_simd` gains five new dispatch arms, closing the last
  remaining gap in `f32x4`'s core arithmetic family (`abs`/`mul`/`min`
  landed in PR19):
  - `SimdOpKind::NegF32x4` (`f32x4.neg`): UNARY, flips the sign bit of
    each of the 4 `f32` lanes (`-v`) -- same shape as `AbsF32x4`, no
    bespoke NaN handling needed (`-NaN` is still NaN, just sign-flipped).
  - `SimdOpKind::SqrtF32x4` (`f32x4.sqrt`): UNARY, IEEE-754 square root
    per lane via Rust's native `f32::sqrt()`, already spec-compliant
    (`sqrt(negative) == NaN`, `sqrt(-0.0) == -0.0`), no bespoke handling
    needed.
  - `SimdOpKind::AddF32x4` / `SubF32x4` / `DivF32x4` (`f32x4.add`/`sub`/
    `div`): BINARY, ordinary IEEE-754 `+`/`-`/`/` per lane pair, same
    shape as `MulF32x4`. `div` is TOTAL, not partial -- a finite lane
    divided by `0.0` produces `+/-infinity` (sign per the usual
    sign-of-quotient rule), `0.0/0.0` produces `NaN`, and there is NO
    trap and NO panic on a zero divisor, unlike this crate's integer
    division opcodes.
- New unit tests: `f32x4_neg_flips_sign_bit_and_leaves_nan_lane_nan`,
  `f32x4_sqrt_computes_ieee754_square_root_per_lane`,
  `f32x4_add_adds_each_lane_pair`, `f32x4_sub_subtracts_each_lane_pair`,
  `f32x4_div_divides_each_lane_pair`,
  `f32x4_div_by_zero_produces_signed_infinity_not_a_trap`,
  `f32x4_div_zero_by_zero_produces_nan`,
  `f32x4_add_sub_div_propagate_nan_in_either_operand`.

## [0.9.38] - 2026-08-19 (task #199-201 — SIMD widen PR28: promote/demote/convert_low family)

### Added

- `register_simd` gains four new dispatch arms:
  - `SimdOpKind::DemoteF64x2Zero` (`f32x4.demote_f64x2_zero`): pops one
    `v128`, reads 2 `f64` lanes, demotes each to `f32` via the exact
    same plain `as f32` cast the scalar `f32.demote_f64` handler
    (`0xB6`) already uses -- no hand-rolled saturation or NaN handling,
    so an out-of-range magnitude correctly overflows to `f32::INFINITY`/
    `f32::NEG_INFINITY` per IEEE-754 (expected, not an error) and a NaN
    lane demotes to a NaN (payload canonicalization is Rust/LLVM's
    call, same as every other float-narrowing path in this crate).
    Writes 4 `f32` lanes: 0-1 demoted, 2-3 ALWAYS zero (mirrors PR25's
    `TruncSatF64x2SZero`/`UZero` zero-fill shape).
  - `SimdOpKind::PromoteLowF32x4` (`f64x2.promote_low_f32x4`): pops one
    `v128`, reads 4 `f32` lanes but uses ONLY the LOW 2 (indices 0-1;
    2-3 are DROPPED, never read at all -- the opposite discipline from
    `DemoteF64x2Zero`'s zero-FILL, since promoting from 4 lanes to 2
    can't invent extra output lanes to zero), promotes each to `f64`
    via the same plain `as f64` cast the scalar `f64.promote_f32`
    handler (`0xBB`) uses (exact, lossless for every finite value).
    Writes 2 `f64` lanes.
  - `SimdOpKind::ConvertLowI32x4S | ConvertLowI32x4U`
    (`f64x2.convert_low_i32x4_s`/`_u`): pops one `v128`, reads 4 `i32`
    lanes, uses only the LOW 2 (same lane-dropping discipline as
    `PromoteLowF32x4`), converts each to `f64` -- signed (`v as f64`)
    or, for `_u`, the lane's bit pattern reinterpreted as `u32` first
    (`(v as u32) as f64`), same signed/unsigned split as
    `ConvertI32x4S`/`ConvertI32x4U` (PR20). Both directions are exact
    and lossless (every `i32`/`u32` fits precisely in `f64`'s 52-bit
    mantissa) -- no rounding or NaN case to consider. Writes 2 `f64`
    lanes. The reverse direction of PR25's `TruncSatF64x2SZero`/
    `UZero` (that went `f64x2` -> `i32x4` with zero-padding; this goes
    `i32x4` -> `f64x2` with lane-dropping).
- New test helper `f64x2_lanes` (decode a `V128Bytes` result back into
  its 2 `f64` lanes), the `f64x2` counterpart of the existing
  `f32x4_lanes`.
- 11 new unit tests: `demote_f64x2_zero`'s ordinary-value + zero-fill
  proof, a NaN-lane proof, and BOTH signed-overflow-to-infinity
  directions (huge positive -> `+infinity`, huge negative ->
  `-infinity`); `promote_low_f32x4`'s ordinary-value proof, a
  dedicated LANE-DROPPING proof (distinguishing values in the high
  half that must NOT influence the result), and a NaN-lane proof;
  `convert_low_i32x4_s`'s ordinary-value proof and its own
  lane-dropping proof; `convert_low_i32x4_u`'s high-bit-set
  (`0xFFFFFFFF` must read as `u32::MAX`, not sign-extend to `-1.0`) and
  ordinary-positive-value proofs.

### Notes

- Mirrors PR25's `i32x4.trunc_sat_f64x2_s/u_zero` (zero-padding
  `f64x2` -> `i32x4`) in reverse, and PR20's `f32x4.convert_i32x4_s/u`
  (int -> float, same signed/unsigned discipline) one lane width up.
- **Campaign complete, corpus now vendored.** With this PR, all 16
  opcodes needed by the upstream `simd_conversions.wast` corpus file
  exist (`extend_low`/`high` from PR26, `narrow` from PR27, this PR's
  4), so `wasm-conformance` now vendors that file for the first time --
  100% pass on every directive. See `wasm-conformance`'s own CHANGELOG.

## [0.9.37] - 2026-08-19 (task #196-198 — SIMD widen PR27: narrow saturating family)

### Added

- `register_simd` gains two new dispatch arms:
  - `SimdOpKind::NarrowI16x8S | NarrowI16x8U` pops TWO `v128`s, reads
    each as 8 `i16` lanes, saturates every lane to the `i8` range
    (signed: `i8::MIN..=i8::MAX`; unsigned: `0..=u8::MAX`, negative
    saturates to 0, does NOT wrap), and writes an `i8x16` result: the
    FIRST operand's 8 saturated lanes become the LOW half (indices
    0-7), the SECOND operand's 8 saturated lanes become the HIGH half
    (indices 8-15). The saturating-demote OPPOSITE of the `ExtendLow/
    HighI8x16S/U` arm (PR26): BINARY where extend is UNARY.
  - `SimdOpKind::NarrowI32x4S | NarrowI32x4U` is the same pattern one
    lane width up: 4 `i32` lanes per operand, saturated to the `i16`
    range, LOW (0-3) / HIGH (4-7), `i16x8` result.
- 6 new unit tests: two operand-ordering proofs (distinct in-range
  values per operand, confirming FIRST operand -> LOW half, SECOND
  operand -> HIGH half — the classic bug spot for this opcode family),
  two SIGNED-saturation boundary tests (mirror-image operand arrays so
  an ordering bug is ALSO caught), and two UNSIGNED-saturation tests
  specifically confirming a negative source lane (`-1` is the sharpest
  case) saturates to `0`, not a wrapped large-unsigned bit pattern
  (`0xFF`/`0xFFFF`) — the classic gotcha for `narrow_u`.

### Notes

- **Staged campaign, no corpus vendoring yet.** These 4 opcodes are the
  second of a 3-PR sequence (`extend_low`/`high` done in PR26, `narrow`
  here, `promote`/`demote`/`convert_low` in a future PR) needed to
  unlock the upstream `simd_conversions.wast` corpus file. This PR is
  opcode-only.

## [0.9.36] - 2026-08-19 (task #193-195 — SIMD widen PR26: extend_low/high family)

### Added

- `register_simd` gains two new dispatch arms:
  - `SimdOpKind::ExtendLowI8x16S | ExtendHighI8x16S | ExtendLowI8x16U |
    ExtendHighI8x16U` pops one `v128`, reads it as 16 `i8` lanes, takes
    only the LOW (indices 0-7) or HIGH (indices 8-15) 8 lanes,
    sign-/zero-extends each to `i16`, and writes an `i16x8` result.
    Exactly the lane-selection + extend half of the already-implemented
    `ExtmulLowI8x16S`/`ExtmulHighI8x16S`/etc. arm, minus the multiply.
  - `SimdOpKind::ExtendLowI16x8S | ExtendHighI16x8S | ExtendLowI16x8U |
    ExtendHighI16x8U` is the same pattern one lane width up: 8 `i16`
    lanes in, LOW (0-3) or HIGH (4-7) 4 lanes selected, extended to
    `i32`, `i32x4` result.
- 4 new unit tests: two "lane order preserved, correct half selected"
  tests (sequential `0..16`/`0..8` operands make each source lane's
  position visible in the result) and two "signed vs. unsigned disagree
  at the sign boundary" tests (`i8::MIN`/`i8::MAX` and
  `i16::MIN`/`i16::MAX`), one pair per lane-width rung.

### Notes

- **Staged campaign, no corpus vendoring yet.** Part of the 16-opcode
  set (`extend_low`/`high` here, `narrow` and `promote`/`demote`/
  `convert_low` in future PRs) needed to unlock the upstream
  `simd_conversions.wast` corpus file. This PR is opcode-only.

## [0.9.35] - 2026-08-19 (task #190-192 — SIMD widen PR25: i32x4.trunc_sat_f64x2_s/u_zero)

### Added

- `register_simd` gains a new dispatch arm:
  `SimdOpKind::TruncSatF64x2SZero | SimdOpKind::TruncSatF64x2UZero` pops
  one `v128`, reads it as 2 `f64` lanes (unlike `TruncSatF32x4S`/`_U`'s
  4 `f32` lanes), converts each to a SATURATING `i32` (signed for
  `_s_zero`, unsigned bit pattern for `_u_zero`) via the same Rust `as`
  cast discipline as `TruncSatF32x4S`/`_U` (saturating + NaN-safe since
  Rust 1.45, no hand-rolled bounds checking), and writes a `v128` with 4
  `i32` lanes: lanes 0-1 hold the two truncated results (same order as
  the source `f64` lanes), lanes 2-3 are always `0` -- the "_zero" half
  of this op's semantics, since `f64x2` only has 2 lanes to widen
  `i32x4`'s 4.
- New test helpers `v128_const_bytes_f64x2`/`trunc_sat_f64x2_zero_code`.
- 9 new unit tests: ordinary in-range values truncate toward zero with
  lanes 2-3 zero-filled; NaN saturates to 0 (never traps) for both `_s`
  and `_u`; +/-infinity saturate to `i32::MIN`/`MAX` (`_s`) or `0`/
  `u32::MAX` (`_u`); a huge finite value (`1e20`) saturates to
  `i32::MAX` without wrapping or panicking; a negative value saturates
  to `0` for `_u_zero` (not wrapped); and a dedicated test confirming
  lanes 2-3 are always zero even for ordinary non-saturating,
  non-zero-producing input values.

## [0.9.34] - 2026-08-19 (task #183-185 — SIMD widen PR22: i16x8.q15mulr_sat_s)

### Added

- `register_simd` gains a new dispatch arm: `SimdOpKind::Q15mulrSatI16x8S`
  pops two `v128`s, reads each of the 8 `i16` lane pairs, and computes a
  Q15 fixed-point ROUNDING SATURATING multiply per lane: sign-extend
  both lanes to `i32` (`l as i32 * r as i32` -- never overflows `i32`,
  max magnitude is `32768 * 32768 == 2^30`), add the rounding constant
  `0x4000`, arithmetic-shift right by 15 (Rust's `>>` on `i32` is
  already arithmetic), then `.clamp(i16::MIN as i32, i16::MAX as i32)`
  -- a REAL saturating clamp, not a wrapping `as i16` cast. The clamp
  only ever fires for the single `(i16::MIN, i16::MIN)` lane pair, where
  the unsaturated formula computes `32768`, one past `i16::MAX`.
- 1 new unit test covering all 4 hand-verified reference cases
  (`q15mulr_sat_s(0, 0) == 0`, `q15mulr_sat_s(32767, 32767) == 32766`,
  `q15mulr_sat_s(i16::MIN, i16::MIN) == i16::MAX` -- the saturating
  edge case, the whole point of this op -- and
  `q15mulr_sat_s(i16::MIN, i16::MAX) == -32767`), plus a full 8-lane
  vector sanity check proving every lane computes independently.

## [0.9.33] - 2026-08-19 (task #180-182 — SIMD widen PR21: i64x2.extmul_i32x4 widening-multiply family)

### Added

- `register_simd` gains a new dispatch arm:
  `SimdOpKind::ExtmulLowI64x2S | ExtmulHighI64x2S | ExtmulLowI64x2U |
  ExtmulHighI64x2U` pop two `v128`s, reinterpret both as 4 `i32` lanes
  each, take only the LOW (indices 0-1) or HIGH (indices 2-3) 2 lanes
  of each operand, sign- or zero-extend every value to `i64`, multiply
  the corresponding pairs lane-wise, and push one `v128` holding the
  2 `i64` results. Mirrors the already-implemented
  `ExtmulLowI16x8S`/`ExtmulLowI8x16S`/etc. arms one and two lane
  widths up respectively -- same narrow-input (32-bit)/wide-output
  (64-bit) BINARY shape, no summation (unlike `DotI16x8S`). This is
  the third and final rung of this crate's "extmul" widening-multiply
  family.
- 1 new unit test covering all 4 variants: proves the LOW 2 lanes and
  HIGH 2 lanes of each `i32x4` operand are read independently (not
  aliased), and that `_s`/`_u` disagree on `-1 * 1` the same way every
  other signed/unsigned pair in this interpreter does, including the
  unsigned-widening edge case (`0xFFFFFFFF` zero-extended to `i64` is
  `4294967295`, not `-1`).

## [0.9.32] - 2026-08-19 (task #177-179 — SIMD widen PR20: i32x4<->f32x4 trunc_sat/convert conversion family)

### Added

- `register_simd` gains four new dispatch arms:
  - `SimdOpKind::TruncSatF32x4S`/`TruncSatF32x4U` pop one `v128`,
    convert each of the 4 `f32` lanes to a SATURATING `i32` (signed
    for `_s`, unsigned bit pattern for `_u`), push one `v128`. NEVER
    TRAPS -- unlike this crate's TRAPPING scalar `i32.trunc_f32_s`/
    `_u` handlers (`0xA8`/`0xA9`) just above, deliberately not reused
    here: a NaN lane saturates to `0`, an out-of-range lane saturates
    to the target bound. Rust's `as` cast from `f32` to `i32`/`u32`
    has implemented exactly this saturating semantic since Rust 1.45,
    same discipline this crate's own `0xFC`-prefixed scalar
    `trunc_sat` handlers already use -- no hand-rolled bounds
    checking needed.
  - `SimdOpKind::ConvertI32x4S`/`ConvertI32x4U` pop one `v128`,
    convert each of the 4 `i32` lanes to `f32` (signed directly for
    `_s`; for `_u`, the lane's bit pattern is reinterpreted as `u32`
    BEFORE the cast -- `(v as u32) as f32`, never `v as f32` directly,
    which would sign-extend a high-bit-set lane into the wrong float
    value), push one `v128`.
- 12 new tests: `trunc_sat_f32x4_s` (ordinary value, NaN saturates to
  0, +/-infinity saturate to `i32::MIN`/`MAX`, a huge finite value
  (`1e20`) saturates to `i32::MAX`), `trunc_sat_f32x4_u` (ordinary
  value, NaN saturates to 0, a negative value saturates to 0 -- NOT
  wrapped/reinterpreted, +infinity saturates to `u32::MAX`'s bit
  pattern), `convert_i32x4_s` (positive/negative values),
  `convert_i32x4_u` (an ordinary positive value, and -- the most
  important test in this PR -- a lane with the high bit set
  (`0xFFFFFFFF`/`-1i32`) converting to `4294967295.0f32`, NOT
  `-1.0f32`, the exact bug class `ConvertI32x4U`'s own doc comment
  warns about).

## [0.9.31] - 2026-08-19 (task #174-176 — SIMD widen PR19: f32x4.abs/f32x4.mul/f32x4.min)

### Added

- `register_simd` gains three new dispatch arms:
  - `SimdOpKind::AbsF32x4` pops one `v128`, clears the sign bit of
    each of the 4 `f32` lanes (`f32::abs()`), pushes one `v128`. A
    pure bit operation -- unlike `MinF32x4` below, no NaN/signed-zero
    subtlety.
  - `SimdOpKind::MulF32x4` pops two `v128`s, multiplies each of the 4
    `f32` lane pairs with ordinary IEEE-754 float multiply (`*`),
    pushes one `v128`.
  - `SimdOpKind::MinF32x4` pops two `v128`s, takes the WASM-spec
    `fmin` of each of the 4 `f32` lane pairs, pushes one `v128`. NOT
    `f32::min()`/IEEE `minNum`: if either lane is NaN the result lane
    is NaN (propagated in either operand order); for a `-0.0`/`+0.0`
    tie, `-0.0` wins. This is the exact per-lane transplant of this
    crate's own scalar `f32.min` (sub-opcode `0x96`, registered in
    `register_numeric_f32`) NaN-propagating, signed-zero-aware logic --
    see that handler's own comment for the original scalar bug this
    mirrors (`min(NaN, -0.0)` silently returning `-0.0` under Rust's
    native `.min()`).
- `v128_const_bytes_f32x4`/`f32x4_lanes` test helpers for building/
  decoding 4-lane `f32` v128 literals.
- 5 new tests: `f32x4.abs` (sign bit cleared, NaN lane stays NaN),
  `f32x4.mul` (a normal lane-wise product), and three `f32x4.min`
  cases -- NaN propagation in BOTH operand orders, the `-0.0`/`+0.0`
  signed-zero tie (checked via `is_sign_negative()`, not `== 0.0`),
  and a normal non-edge-case minimum.

## [0.9.30] - 2026-08-19 (task #171-173 — SIMD widen PR18: i8x16 swizzle/extract_lane_s/extract_lane_u/replace_lane)

### Added

- `register_simd` gains four new dispatch arms:
  - `SimdOpKind::Swizzle` pops two `v128`s in the usual binary order
    (index vector `s` on top, popped first; data vector `a` popped
    second); for each of the 16 result lanes, looks up `a[s[i]]` if
    `s[i] < 16`, else `0`. The `< 16` bounds check runs BEFORE
    indexing `a` -- the index byte `s[i]` is an unconstrained `u8`
    (`0..=255`), so without the check an adversarial/malformed index
    vector could index `a` out of bounds.
  - `SimdOpKind::ExtractLaneI8x16S`/`ExtractLaneI8x16U` pop a `v128`,
    read the `aux`-selected `i8` lane (sign- or zero-extended to
    `i32`), same shape as the pre-existing `ExtractLane` arm but at
    `i8x16`'s 0-15 lane range. Bounds-checked (`lane_idx >= 16`
    rejected with a clean `Err`) BEFORE indexing the 16-byte lane
    array, same discipline as `ExtractLane`'s own 0-3 check.
  - `SimdOpKind::ReplaceLaneI8x16` pops an `i32` (the replacement
    value, only its low byte used) then a `v128` (pop order matches
    the shift family's own "scalar pushed last, popped first"
    convention), overwrites the `aux`-selected lane, pushes the result.
    Same bounds-check discipline as the extract-lane arms above.
- 10 new tests in `mod tests`: `i8x16.swizzle` permutation (a real
  lane-reversal, not just identity) and its out-of-range-index-produces-
  zero case; `extract_lane_s` sign-extension and `extract_lane_u`
  zero-extension of the SAME `0x80` byte (proving they genuinely
  differ, not just each "doing something"); both extract variants'
  out-of-range-lane clean-error case; `replace_lane`'s
  only-target-lane-changes case, its only-low-byte-of-i32-used case,
  and its own out-of-range-lane clean-error case.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.29] - 2026-08-19 (task #168-170 — SIMD: float splat family, first float-lane ops)

### Added

- `register_simd` gains two new dispatch arms: `SimdOpKind::SplatF32x4`
  pops an `f32` and broadcasts its 4 little-endian bytes into all 4
  lanes; `SplatF64x2` pops an `f64` and broadcasts its 8 little-endian
  bytes into both lanes. A pure bit-pattern broadcast via
  `to_le_bytes()` (not a numeric conversion), so no rounding or NaN
  handling is needed -- the FIRST floating-point-typed SIMD ops in
  this crate.
- 2 new tests, each verifying the EXACT IEEE-754 bit pattern is
  broadcast into every lane (using `3.5`, a value whose bit pattern
  couldn't accidentally match a broken implementation the way `0.0`
  could).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.28] - 2026-08-19 (task #165-167 — SIMD: splat family widening)

### Added

- `register_simd` gains three new dispatch arms: `SimdOpKind::SplatI8x16`
  pops an `i32` and broadcasts its LOW byte into all 16 lanes;
  `SplatI16x8` pops an `i32` and broadcasts its LOW 16 bits into all 8
  lanes; `SplatI64x2` pops a real `i64` (not `i32`) and broadcasts all
  8 bytes into both lanes. Same shape as the pre-existing `Splat`
  (`i32x4.splat`) arm, just at narrower/wider lane widths.
- 3 new tests, each proving the SPECIFIC bytes broadcast, not just
  "returns some v128": `i8x16.splat`/`i16x8.splat` verify only the low
  byte/16 bits of a deliberately-oversized `i32` operand end up in the
  lanes (the high bits must be silently dropped, not carried through or
  trapped on); `i64x2.splat` verifies the FULL 64-bit value is
  broadcast, not just its low 32 bits.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.27] - 2026-08-18 (task #162-164 — SIMD: v128.load/v128.store)

### Added

- `LinearMemory::load_v128(&self, offset: usize) -> Result<[u8; 16],
  TrapError>` -- bounds-checks 16 bytes starting at `offset`, then
  copies them out. Same shape as the existing `load_f64`/etc, just 16
  bytes instead of 4/8.
- Raw-bytecode decode gains a new `0xFD` branch for `sub_opcode == 0x00
  || sub_opcode == 0x0B` (`v128.load`/`v128.store`) that decodes a full
  `memarg` immediate (respecting the multi-memory `0x40` flag bit for
  correct byte-consumption even if a producer combines SIMD with
  multi-memory) but keeps only `offset`, packed into the existing
  `DecodedOperand::Simd{sub_opcode, aux}` shape -- reused rather than
  inventing a new packing scheme, since the top-level `0xFD` dispatch
  has no way to tell which packing scheme an already-packed
  `Operand::Index` used without inspecting the original
  `DecodedOperand` before conversion. This scopes execution to memory
  index 0 only for this first PR; multi-memory `v128.load`/`v128.store`
  is deferred to a later PR, same as WASM92 later widened the scalar
  load/store family.
- `register_simd` gains two new dispatch arms: `SimdOpKind::Load` pops
  an `i32` base address, adds the memarg offset, calls
  `load_v128`/pushes a new `v128_heap` handle; `SimdOpKind::Store` pops
  the `v128` value FIRST (it's pushed last, so on top of stack), then
  the `i32` base address, and writes the 16 bytes via the existing
  `write_bytes` -- same pop order as the scalar `i32.store` (`0x36`)
  handler.
- 3 new tests: a `v128.store`-then-`v128.load` round trip through real
  memory; a cross-check proving `v128.load` reads the SAME bytes a
  plain scalar `i32.store` wrote (not just an internal-only round trip
  with its own sibling `v128.store`); and an adversarial bounds test
  proving both ops trap cleanly (not panic) when `address + 16`
  overruns the memory's actual byte length.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.26] - 2026-08-18 (task #159-161 — SIMD: shift family)

### Added

- `register_simd` gains four new dispatch arms (one per lane width)
  for the FIRST mixed-type binary SIMD op family:
  `ShlI8x16 | ShrSI8x16 | ShrUI8x16`, `ShlI16x8 | ShrSI16x8 |
  ShrUI16x8`, `ShlI32x4 | ShrSI32x4 | ShrUI32x4`, and `ShlI64x2 |
  ShrSI64x2 | ShrUI64x2`. Each pops the scalar `i32` shift amount
  FIRST (it's pushed LAST per `(ixNxM.shl (v128 $a) (i32 $amount))`,
  so it's on top of stack), then the `v128` operand, and masks the
  shift amount MODULO the lane's bit width (8/16/32/64 respectively)
  before shifting -- both spec-mandated and required for Rust safety,
  since shifting a primitive by >= its bit width panics. `shl` is a
  plain logical left shift (signedness-independent); `shr_s` reads
  each lane as its signed type before shifting (sign-extending);
  `shr_u` reads each lane as its unsigned type (zero-extending).
  Verified with dedicated tests proving the shift amount is correctly
  masked (shifting by exactly the lane's bit width, or bit-width + k,
  must behave identically to shifting by 0, or k, not panic) and that
  `shr_s`/`shr_u` disagree on sign extension across all 4 lane widths
  (an operand with the sign bit set, shifted right by 1, keeps
  propagating the sign bit under `shr_s` but shifts in a zero under
  `shr_u`).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.25] - 2026-08-18 (task #156-158 — SIMD: i64x2 arith+cmp family)

### Added

- `register_simd` gains four new dispatch arms for i64x2's first REAL
  ARITHMETIC family: `SimdOpKind::AbsI64x2` (UNARY, `wrapping_abs` so
  `i64::MIN` maps to itself, not a panic), `NegI64x2` (UNARY,
  `wrapping_neg`), `AddI64x2 | SubI64x2 | MulI64x2` (BINARY, wrapping
  arithmetic over 2 `i64` lanes), and `EqI64x2 | NeI64x2 | LtSI64x2 |
  GtSI64x2 | LeSI64x2 | GeSI64x2` (BINARY, boolean-mask-per-lane,
  `-1i64`/`0i64` -- SIGNED ONLY, since the SIMD proposal never defines
  unsigned `i64x2` comparisons). Verified with dedicated tests proving
  `abs`/`neg` wrap `i64::MIN` instead of panicking,
  `add`/`sub`/`mul` wrap on overflow (`i64::MAX + 1` -> `i64::MIN`),
  and the comparison family reads lanes as SIGNED (using `i64::MIN` vs
  a small positive value, where an unsigned mix-up would flip the
  result since `i64::MIN`'s bit pattern is the largest unsigned
  value).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.24] - 2026-08-18 (task #153-155 — SIMD: boolean-reduction/bitmask family)

### Added

- `register_simd` gains three new dispatch arms: `SimdOpKind::AnyTrue`
  (pops one v128, pushes i32 `1` if ANY of the 128 bits is set, else
  `0` -- reduces over the WHOLE operand, no lane interpretation
  needed), `AllTrueI8x16 | AllTrueI16x8 | AllTrueI32x4 | AllTrueI64x2`
  (pops one v128, pushes i32 `1` only if EVERY lane at that width is
  nonzero -- chunks the 16 raw bytes into 1/2/4/8-byte lanes and
  checks each chunk for any nonzero byte), and `BitmaskI8x16 |
  BitmaskI16x8 | BitmaskI32x4 | BitmaskI64x2` (pops one v128, pushes
  an i32 whose bit `i` is the sign bit of lane `i`, packed low-to-high
  -- a lane's sign bit is the MSB of its last, most-significant,
  little-endian byte). `i64x2`'s two variants are the first opcodes in
  this crate to read the operand as 8-byte lanes. Verified with
  dedicated tests proving `any_true` distinguishes all-zero from a
  single nonzero byte anywhere, `all_true` flips to false when exactly
  one lane (at that op's own width) is zeroed out even though the
  surrounding bytes are nonzero, and `bitmask` packs an alternating
  sign-bit pattern correctly at every width.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.23] - 2026-08-18 (task #150-152 — SIMD: v128 bitwise family)

### Added

- `register_simd` gains three new dispatch arms for the
  lane-width-agnostic raw-byte bitwise family: `SimdOpKind::Not`
  (UNARY -- flips every bit of the popped v128), `And | AndNot | Or |
  Xor` (BINARY -- pops rhs then lhs, computes the bytewise operation
  lane-by-lane over all 16 bytes, `AndNot` being `lhs & !rhs`), and
  `Bitselect` (TERNARY -- the first three-operand SIMD op in this
  interpreter: pops `c` then `b` then `a`, computes `(a[i] & c[i]) |
  (b[i] & !c[i])` per byte, i.e. select bits from `a` where the
  corresponding `c` bit is 1, else from `b`). Every handler resolves
  its `v128_heap` handle(s) via `.get(...).ok_or_else(...)` --
  never raw indexing -- so a malformed handle produces a clean typed
  `VMError`, not a panic. Verified with dedicated tests covering
  `not`'s full-bit-flip, each of `and`/`andnot`/`or`/`xor`'s real
  boundary-value semantics, and `bitselect` selecting an exact mask
  pattern from two maximally-different operands.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.22] - 2026-08-18 (task #147-149 — SIMD: i16x8-from-i8x16 widening family)

### Added

- `register_simd` gains two new dispatch arms mirroring the
  already-implemented `i32x4`-from-`i16x8` widening family one lane
  width down: `ExtaddPairwiseI8x16S | ExtaddPairwiseI8x16U` (UNARY --
  reinterpret the popped v128 as 16 `i8` lanes, pairwise-add adjacent
  lanes after sign-/zero-extending each to `i16`, producing an 8-lane
  `i16x8` result) and `ExtmulLowI8x16S | ExtmulHighI8x16S |
  ExtmulLowI8x16U | ExtmulHighI8x16U` (BINARY -- take only the low
  (indices 0-7) or high (indices 8-15) 8 `i8` lanes of each operand,
  sign-/zero-extend to `i16`, and multiply lane-wise, no summation).
  No `i16x8.dot_i8x16_s` handler -- WASM SIMD does not define a
  dot-product for this pair. Verified with dedicated tests proving
  the low/high halves are read independently (distinct operand values
  in each half produce distinct results) and that `_s`/`_u` disagree
  on `-1 * 1` the same way every other signed/unsigned pair in this
  interpreter does.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.21] - 2026-08-18 (task #144-146 — SIMD: i16x8 abs/min/max/avgr_u family)

### Added

- `register_simd` gains two new dispatch arms for `i16x8`'s own
  "arith2" family: `AbsI16x8` (UNARY, same shape as `i16x8.neg`/
  `i8x16.abs`) and `MinSI16x8 | MinUI16x8 | MaxSI16x8 | MaxUI16x8 |
  AvgrUI16x8` (BINARY, same shape as `i8x16`'s own `min_s`/`min_u`/
  `max_s`/`max_u`/`avgr_u`, just at `i16x8`'s wider lane width). `abs`
  uses the same two's-complement wrapping discipline `i8x16.abs`'s own
  test already established (`i16::MIN.wrapping_abs() == i16::MIN`).
  `avgr_u` computes `(a + b + 1) >> 1` widened to `u32` so the `+1`
  cannot overflow the lane width. Verified with a dedicated test
  proving `avgr_u(0xFFFF, 0)` rounds UP to `32768`, not down to
  `32767` -- the one case that would silently hide a missing `+1`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.20] - 2026-08-18 (task #141-143 — SIMD: i8x16 abs/popcnt/min/max/avgr_u family)

### Added

- `register_simd` gains two new dispatch arms for `i8x16`'s own
  "arith2" family: `AbsI8x16 | PopcntI8x16` (UNARY, same shape as
  `i8x16.neg`/`i32x4.abs`) and `MinSI8x16 | MinUI8x16 | MaxSI8x16 |
  MaxUI8x16 | AvgrUI8x16` (BINARY, same shape as `i32x4`'s own
  `min_s`/`min_u`/`max_s`/`max_u`). `abs` uses the same
  two's-complement wrapping discipline `i32x4.abs`'s own test already
  established (`i8::MIN.wrapping_abs() == i8::MIN`). `popcnt` and
  `avgr_u` are genuinely NEW op shapes with no `i32x4`/`i16x8`
  precedent in this interpreter: `popcnt` counts set bits per lane
  (Hamming weight, bit-pattern-only -- no signed/unsigned split
  needed); `avgr_u` computes `(a + b + 1) >> 1` widened to `u16` so
  the `+1` cannot overflow the lane width. Verified with a dedicated
  test proving `avgr_u(0xFF, 0)` rounds UP to `128`, not down to
  `127` -- the one case that would silently hide a missing `+1`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.19] - 2026-08-18 (task #137-140 — SIMD: i8x16 comparison family)

### Added

- `register_simd` gains a new dispatch arm for `i8x16`'s own comparison
  family (`EqI8x16 | NeI8x16 | LtSI8x16 | LtUI8x16 | GtSI8x16 |
  GtUI8x16 | LeSI8x16 | LeUI8x16 | GeSI8x16 | GeUI8x16`): same
  lane-wise BINARY shape and boolean-mask convention as `i16x8`'s and
  `i32x4`'s own comparison families, but over all 16 `i8` lanes -- each
  lane becomes all-1s (`-1i8`, i.e. `0xFF`) if true, all-0s otherwise.
  Verified with a dedicated test proving `-1 <_s 1` (true, signed) and
  `-1 <_u 1` i.e. `0xFF <_u 1` (false, unsigned) actually disagree --
  the same signed/unsigned discipline `i16x8`'s own comparison test
  already established.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.18] - 2026-08-18 (task #133-136 — SIMD: i16x8 comparison family)

### Added

- `register_simd` gains a new dispatch arm for `i16x8`'s own comparison
  family (`EqI16x8 | NeI16x8 | LtSI16x8 | LtUI16x8 | GtSI16x8 |
  GtUI16x8 | LeSI16x8 | LeUI16x8 | GeSI16x8 | GeUI16x8`): same
  lane-wise BINARY shape and boolean-mask convention as `i32x4`'s own
  comparison family, but over 8 `i16` lanes instead of 4 `i32` lanes --
  each lane becomes all-1s (`-1i16`, i.e. `0xFFFF`) if true, all-0s
  otherwise (the mask width tracks the LANE width, not a fixed `i32`).
  Verified with a dedicated test proving `-1 <_s 1` (true, signed) and
  `-1 <_u 1` i.e. `0xFFFF <_u 1` (false, unsigned) actually disagree --
  the same signed/unsigned discipline `i32x4`'s own comparison test
  already established.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.17] - 2026-08-18 (task #129-132 — SIMD: i16x8 first primary-lane slice)

### Added

- `register_simd` gains two new dispatch arms for this crate's first
  opcodes where `i16x8` is a PRIMARY lane width: `AddI16x8 | SubI16x8 |
  MulI16x8` (BINARY, 8 `i16` lanes instead of the 4 `i32` lanes or 16
  `i8` lanes every prior binary SIMD op here reads/writes, same
  wrapping-arithmetic shape as `i32x4`'s own `Add`/`Sub`/`Mul`) and
  `NegI16x8` (UNARY, same shape as `i8x16.neg`/`i32x4.neg`/`.abs`).
  Verified with a dedicated test proving `i16::MAX + 1` wraps to
  `i16::MIN` and `i16::MIN.wrapping_neg() == i16::MIN` -- the same
  two's-complement wrapping edge cases `i8x16`/`i32x4`'s own tests
  already established at their respective widths.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.16] - 2026-08-18 (task #125-128 — SIMD: i8x16 first slice)

### Added

- `register_simd` gains two new dispatch arms for this crate's first
  `i8x16`-lane-width ops: `AddI8x16 | SubI8x16` (BINARY, 16 `i8` lanes
  instead of the 4 `i32` lanes every prior binary SIMD op here reads/
  writes, same wrapping-arithmetic shape as `i32x4`'s own `Add`/`Sub`
  otherwise) and `NegI8x16` (UNARY, same shape as `i32x4.neg`/`.abs`).
  Verified with a dedicated test proving `i8::MAX + 1` wraps to
  `i8::MIN` and `i8::MIN.wrapping_neg() == i8::MIN` -- the same
  two's-complement wrapping edge cases `i32x4.abs`'s own test already
  established for `i32`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.15] - 2026-08-18 (task #121-124 — SIMD widening: i32x4-from-i16x8 family)

### Added

- `register_simd` gains two new dispatch arms, both reading their v128
  operand(s) as 8 `i16` lanes (`i16x8`) instead of the 4 `i32` lanes
  (`i32x4`) every prior SIMD op in this crate reads/writes -- the first
  opcodes here whose input and output lane widths differ:
  - `ExtaddPairwiseI16x8S | ExtaddPairwiseI16x8U` (UNARY): pairwise-adds
    adjacent `i16x8` lanes (0+1, 2+3, 4+5, 6+7), each extended to `i32`
    first (sign- or zero-extend depending on `_s`/`_u`), producing an
    `i32x4` result.
  - `DotI16x8S | ExtmulLowI16x8S | ExtmulHighI16x8S | ExtmulLowI16x8U
    | ExtmulHighI16x8U` (BINARY): `DotI16x8S` pairwise
    multiply-accumulates ALL 8 lanes of both operands into 4 `i32x4`
    results; `ExtmulLow`/`ExtmulHigh` instead multiply only the low
    (indices 0-3) or high (indices 4-7) 4 lanes of each operand
    pairwise, no summation.
  - Both arms extend each `i16` lane to `i32` FIRST (correctly, per
    `_s`/`_u`), then use plain `i32` `wrapping_mul`/`wrapping_add` --
    verified this is bit-for-bit correct for the unsigned variants too
    (Rust's wrapping arithmetic on a fixed-width integer is identical
    regardless of the signed/unsigned interpretation of the result), via
    a dedicated test proving `0xFFFF * 0xFFFF` (which doesn't fit `i32`'s
    positive range) still wraps to the correct bit pattern. Also covers
    `dot_i16x8_s`'s own two's-complement overflow edge case
    (`i16::MIN * i16::MIN` summed twice overflows `i32` by exactly 1 and
    must wrap to `i32::MIN`).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.14] - 2026-08-18 (task #118-120 — SIMD widening: i32x4 abs/min/max family)

### Added

- `register_simd`'s binary `v128,v128->v128` dispatch arm widened further
  to add `MinS | MinU | MaxS | MaxU`: `MinS`/`MaxS` compare lanes as
  signed `i32` (plain `.min`/`.max`), `MinU`/`MaxU` cast each lane to
  `u32` first -- same signed/unsigned split already proven necessary for
  the comparison family, verified with a dedicated test showing
  `min_s(-1, 1) == -1` but `min_u(-1, 1)` (i.e. `min_u(0xFFFFFFFF, 1)`)
  `== 1` actually disagree.
- `SimdOpKind::Neg | SimdOpKind::Abs` now share the unary dispatch arm
  (previously `Neg`-only), computing `wrapping_neg`/`wrapping_abs`
  per-kind inside -- `i32::MIN.wrapping_abs() == i32::MIN` (the classic
  two's-complement absolute-value overflow edge case) is covered by a
  dedicated test.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.13] - 2026-08-18 (task #113-117 — SIMD widening: i32x4 arithmetic + comparison family)

### Added

- `register_simd`'s `SimdOpKind::Add | SimdOpKind::Eq` dispatch arm
  widened to `Add | Sub | Mul | Eq | Ne | LtS | LtU | GtS | GtU | LeS |
  LeU | GeS | GeU` -- all 13 kinds share the identical binary
  `v128,v128->v128` shape (pop rhs, pop lhs, lane-wise op, push result),
  differing only in the per-lane operation (`wrapping_add`/`sub`/`mul`,
  or one of 10 comparison predicates producing WASM's boolean-mask
  convention). Unsigned comparisons (`LtU`/`GtU`/`LeU`/`GeU`) cast each
  lane to `u32` before comparing -- verified with a dedicated test
  proving `-1 <_s 1` (true) and `-1 <_u 1` i.e. `0xFFFFFFFF <_u 1`
  (false) actually disagree, the one place a missing/wrong cast would
  silently hide.
- New `SimdOpKind::Neg` arm: the first UNARY SIMD op in this interpreter
  (pops exactly one `v128`, negates each lane with `wrapping_neg`,
  pushes one) -- a genuinely different shape from every op above, kept
  in its own match arm rather than folded into the binary-ops one.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.9.12] - 2026-08-17 (task #92/#109 — real multi-memory memarg)

### Fixed

- `DecodedOperand::MemArg` widened from `{ _align, offset }` to
  `{ _align, offset, memidx }`. The decode arm now checks the align
  byte's top bit (`0x40`, the real multi-memory flags bit), masks it
  back out for the real alignment hint, and conditionally decodes a
  third LEB128 into `memidx`. All 23 memarg-carrying load/store
  handlers (`0x28`-`0x3E`) switched from a hardcoded `get_memory(ctx)`
  (always memory 0) to `get_memory_at(ctx, memarg_memidx(instr))`, via
  a new `unpack_memarg_operand` helper (same packed-`usize` shape as
  `unpack_call_indirect_operand`: memidx in the high 32 bits, offset in
  the low 32).
- `memory.init`/`memory.copy`/`memory.fill` (`0xFC` `0x08`/`0x0A`/`0x0B`)
  assumed their memory-index immediate(s) were a FIXED byte width (1/2/1
  bytes respectively) instead of real LEB128s -- this only worked
  because an MVP-only (always-0) index happens to be exactly one byte;
  a genuine multi-memory-encoded non-zero index would silently misalign
  every subsequent decode. Fixed by decoding real LEB128s for all three,
  repurposing the existing `(data_idx, aux)` decode-tuple slots (`aux`
  for `memory.init`'s memidx, `data_idx`/`aux` for `memory.copy`'s
  dst/src memidx pair, mirroring `table.copy`'s own shape).
- New `LinearMemory::copy_between` (mirrors `Table::copy_between`'s own
  security-reviewed raw-pointer pattern from task #97 exactly: raw
  pointers, not `&mut`/`&` references, to avoid aliasing UB on a
  same-memory self-copy; every field access scoped to a single,
  explicit -- never autoref'd -- statement) wires `memory.copy`'s
  handler to actually copy BETWEEN two distinct memories instead of
  always operating within memory 0.
- New end-to-end regression tests proving cross-memory `i32.load`,
  `memory.fill`, `memory.copy`, and `memory.init` each operate on the
  NAMED memory, not memory 0.

See `code/specs/W18-wasm-multi-memory-memarg.md`.

## [0.9.11] - 2026-08-17 (task #107 — call_indirect/return_call_indirect real table index)

### Fixed

- `call_indirect` (`0x11`) and `return_call_indirect` (`0x13`) always ran
  against table 0 regardless of what their `tableidx` immediate actually
  named. The immediate was already decoded correctly (`wasm-opcodes`
  already modeled both opcodes with `["typeidx","tableidx"]`, and
  `decode_immediates` already read both LEB128s into `DecodedOperand::
  CallIndirect { type_idx, table_idx }`) -- `table_idx` was just silently
  dropped in `convert_operand`, the actual bug. Fixed by packing both
  indices into the operand's `usize` (table_idx high 32 bits, type_idx
  low 32, same shape as `Atomic`/`Simd`'s own packing) via a new
  `unpack_call_indirect_operand` helper, and both handlers now call
  `get_table(ctx, table_idx as usize)` instead of the hardcoded
  `get_table(ctx, 0)`. Discovered while vendoring `table_init.wast`/
  `table_copy.wast` (task #97): most modules in both files use this
  explicit-table-index form pervasively, and every one of them failed
  to build entirely under the old text-encoder (see `wasm-wast-parser`'s
  own CHANGELOG entry for this version).

## [0.9.10] - 2026-08-17 (task #97 — table.init/table.copy/elem.drop)

### Added

- `table.init` (`0xFC 0x0C`), `elem.drop` (`0xFC 0x0D`), `table.copy`
  (`0xFC 0x0E`) interpreter handlers -- entirely unimplemented before
  this pass, mirroring `memory.init`/`data.drop`'s own shape (task
  #95): `WasmExecutionContext.elements: Vec<Vec<Option<u32>>>` and
  `dropped_elements: Vec<bool>` fields threaded through
  `WasmExecutionEngine` (`set_elements`/`set_dropped_elements`,
  `into_state()`, context construction), a real `elem_idx` bounds check
  as a hard error (not deferred to a runtime trap) matching every
  other indexed immediate, and a soft "already dropped -> treat as
  empty" degrade for `table.init` matching `memory.init`'s own.
- `DecodedOperand::BulkMemory` gained a third field, `aux: u8`, packed
  into the same `usize` operand alongside `sub`/`data_idx` (bits 40-47
  -- 24 bits were free) to carry the SECOND index `table.init`/
  `table.copy` need (elemidx+tableidx, or dst_table+src_table).
- `Table::copy_between(dst_table: &mut Table, src_table: &Table, ...)`
  -- a free function (not a `&mut self` method) taking two SEPARATE
  references so a same-table self-copy still works, via two
  independently-obtained raw pointers from `ctx.tables`. Reads the
  source range into an owned `Vec` before any write (overlap-safe,
  same atomicity discipline as `LinearMemory::copy`), bounds-checks
  BOTH tables before any write.

### Fixed

- **`/security-review` finding (HIGH)**: `Table::copy_between`
  originally took `dst_table: &mut Table, src_table: &Table` -- when
  `table.copy`'s two table operands name the SAME table (`table.copy
  $t $t ...`, a legal, attacker-reachable self-copy), the caller's two
  independently-resolved raw pointers into that one `Table` were both
  dereferenced into a `&mut` and a `&` bound as function parameters,
  live for the whole call -- an aliasing violation under Rust's memory
  model regardless of whether any actual read/write hazard existed.
  Rewritten to take raw pointers (`*mut Table`/`*const Table`)
  throughout instead, with every access scoped to a short-lived,
  explicit (not autoref'd -- `dangerous_implicit_autorefs` catches the
  autoref'd form even through a raw-pointer deref) reference that ends
  before the next one is formed, so no `&mut`/`&` pair is ever
  simultaneously live over the same `Table`, self-copy or not.

- **Real, load-bearing bug caught by `wasm-runtime`'s own persistence
  test, not this crate's unit tests**: the shared `call_function`/
  `call_function_with_v128` post-call state-restore block wrote
  `self.dropped_data_segments = ctx.dropped_data_segments.clone()`
  (task #95) but had no equivalent line for `dropped_elements` -- an
  `elem.drop` from one call never stuck for a LATER, separate call on
  the same engine (it silently reverted to all-`false` the moment the
  call returned). This crate's own single-call tests couldn't catch it
  (they only ever call `call_function` once); `wasm-runtime`'s
  `elem_drop_persists_across_separate_calls_on_the_same_instance` test
  (mirroring task #95's own `data_drop_persists_across_separate_calls_
  on_the_same_instance`) failed immediately until this line was added,
  confirming the fix was load-bearing.

## [0.9.9] - 2026-08-17 (task #101 — memory.grow cross-memory aggregate cap)

### Security

- `memory.grow`'s interpreter handler (`0x40`) now rejects growth that
  would push the SUM of every memory's page count -- across ALL
  memories, not just the target -- past `MAX_TOTAL_MEMORY_PAGES`
  (65536, a new constant). Mirrors `table.grow`'s own task #98 round-2
  fix: `LinearMemory::grow`'s per-memory 65536-page cap alone still
  permitted an aggregate DoS across `MAX_MEMORIES` (64) memories --
  each individually grown to the per-memory cap would total ~256GB from
  one small module, reintroducing at RUNTIME exactly what `wasm-
  validator`'s existing declare-time "Check 1b" already closes for
  every memory's DECLARED minimum. Reuses that same 65536-page bound
  (not a new arbitrary constant) so "total pages across every memory,
  declared or grown, never exceeds 65536" is one consistent invariant
  at both points in a module's lifecycle. The threshold arithmetic is
  factored into a pure `memory_grow_would_exceed_aggregate_cap`
  function specifically so it's cheaply unit-testable with small
  synthetic page counts -- the real cap is 65536 pages (4GB), far too
  large to actually allocate in a unit test. Confirmed load-bearing by
  TEMP-REVERT-CHECK (stubbing the check to always return `false`
  reproduces the exact predicted failures across all three threshold
  tests, while the positive-path wiring test still passes).

## [0.9.8] - 2026-08-16 (task #98 — table.grow/table.size/table.fill)

### Security

- `Table::grow` now rejects growth past `MAX_TABLE_ELEMENTS`
  (10,000,000), independent of whether the module declared its own
  `max_size`. Caught by `/security-review` round 1: with no declared
  max (entirely legal WASM), the only prior ceiling was `i32::MAX`
  entries -- since `Table::elements` is `Vec<Option<u32>>` (8 bytes/
  entry, no niche optimization for `u32`), a single `table.grow` call
  from one attacker-controlled `.wasm` module could resize to ~17GB,
  and Rust's default allocation-failure handler aborts the whole
  process, not just the offending instance. `MAX_TABLE_ELEMENTS`
  already existed (task #96, security review) to bound a table's
  DECLARED `min` for this exact resource-exhaustion reason, but that
  check never applied to runtime growth -- mirrors `LinearMemory::
  grow`'s own two-tier shape (module-declared max, THEN a hard
  implementation ceiling independent of it). New tests pin both the
  zero-cap-declared and grown-to-cap-then-grow-again shapes; confirmed
  load-bearing by TEMP-REVERT-CHECK (reverting the check reproduces
  the exact predicted failure: `assertion left == right failed, left:
  1, right: -1`).
- `table.grow`'s interpreter handler (`0xFC 0x0F`) now also rejects
  growth that would exceed `MAX_TABLE_ELEMENTS` SUMMED ACROSS ALL
  TABLES, not just the target table alone. Caught by `/security-review`
  round 2: the per-table cap above still permitted an aggregate DoS --
  `MAX_TABLES` (64) tables, each individually grown to
  `MAX_TABLE_ELEMENTS`, would total ~4.77GB from one small module,
  reintroducing at RUNTIME exactly the aggregate gap `wasm-validator`'s
  own "Check 2b" already closes at DECLARE-time for a table's declared
  `min` (task #96). Sums every other table's current size plus the
  target's prospective new size and rejects BEFORE calling `Table::
  grow` at all, so a rejected growth leaves every table untouched. A
  follow-up task (#101) tracks the identical, larger, pre-existing gap
  in `LinearMemory::grow` (64 memories × the 65536-page cap ≈ 256GB),
  out of this PR's scope but the same bug class. Confirmed load-bearing
  by TEMP-REVERT-CHECK.

### Added

- `table.grow`/`table.size`/`table.fill` (`0xFC 0x0F`/`0x10`/`0x11`)
  interpreter handlers -- entirely unimplemented before this pass.
  `table.grow` grows by `delta` entries filled with `init`, returning the
  old size on success or `-1` on failure (a spec-mandated normal return
  value, never a trap) -- failure cases are exceeding the table's own
  declared `max_size`, or exceeding what fits in `table.size`'s own
  `i32` result type (real engines cap table size for exactly this
  reason). `table.size` pushes the current size. `table.fill` fills a
  range with a reference value, same overflow-proof, zero-length-
  still-bounds-checked discipline `LinearMemory::fill` established in
  task #94 (`dest` must be `<= size()` even when `len == 0`).
- `Table::grow`/`Table::fill` methods, modeled directly on
  `LinearMemory::grow`/`fill`'s own contracts.
- `table.grow`/`table.size`/`table.fill`'s decoded table index reuses
  `DecodedOperand::BulkMemory`'s existing `data_idx` slot (renamed in
  spirit, not in code -- see the variant's own updated doc comment) --
  the same generic-index-slot-reused-by-`sub` pattern `Simd`'s `aux`
  field already establishes, so no new packed-operand shape was needed.

## [0.9.7] - 2026-08-16 (task #95 — memory.init/data.drop, passive data segments)

### Security

- `memory.init`'s handler now hard-errors on an out-of-range `data_idx`
  instead of silently degrading it to an empty segment. The first pass
  of this fix (caught by `/security-review` round 1) removed a direct
  `ctx.data_segments[idx]` panic by resolving `segment_bytes` once via
  `.get(idx).unwrap_or(&[])` -- correct for the panic, but it also
  collapsed "index out of range" and "segment was dropped" into the
  same fallback. That let a zero-length `memory.init` with an
  out-of-range `data_idx` return `Ok(())` (silently "succeed") instead
  of trapping, which the interpreter's own defensive posture doesn't
  allow elsewhere -- `data.drop`'s handler, just below, has always
  hard-errored on an out-of-range index unconditionally. Fixed by
  checking `idx` against both `Vec` lengths FIRST, returning
  `VMError::GenericError` immediately if out of range, before ever
  touching `dropped_data_segments`/`data_segments`; only a genuinely
  in-range index is allowed to soft-degrade via `dropped`. New test
  `memory_init_with_an_out_of_range_data_idx_traps_cleanly_even_at_zero_length`
  pins both the zero-length and nonzero-length out-of-range shapes.
  Confirmed load-bearing by TEMP-REVERT-CHECK (reverting to the
  `.get(idx).unwrap_or(true/&[])` fallback reproduces the exact
  predicted regression: the new test fails with "out-of-range data_idx
  must trap, not silently succeed: Ok([])").

### Added

- `memory.init`/`data.drop` (`0xFC 0x08`/`0x09`) interpreter handlers --
  entirely unimplemented before this pass. `memory.init` copies from a
  data segment (bounds-checked against the segment's own length, same
  zero-length-still-checked discipline task #94 established for
  `copy`/`fill`) into memory (bounds-checked the normal way via
  `LinearMemory::write_bytes`); `data.drop` permanently marks a segment
  dropped. A dropped segment behaves as length-0 for `memory.init`'s
  bounds check -- a zero-length `memory.init` on it still succeeds, but
  any nonzero-length one traps, matching the real spec's "a dropped
  segment can never be initialized from again" rule.
- `WasmExecutionContext::data_segments: Vec<Vec<u8>>` (immutable
  content, one entry per data segment) and `dropped_data_segments:
  Vec<bool>` (mutable, persistent across calls within one instance's
  lifetime -- same shape as `v128_heap`, NOT reset per call, since
  `data.drop`'s effect must survive into later calls). Set via new
  `WasmExecutionEngine::set_data_segments`/`set_dropped_data_segments`,
  same optional-setter pattern as `set_struct_field_counts`/
  `set_v128_heap`; `dropped_data_segments` round-trips through the new
  `WasmEngineState::dropped_data_segments` field the same way
  `v128_heap` does.
- `DecodedOperand::BulkMemory { sub: u8, data_idx: u32 }` -- replaces the
  bare `Int(sub)` operand every `0xFC` sub-opcode previously used.
  `memory.init`'s `data_idx` needed a REAL immediate carried through
  decoding (unlike `memory.copy`/`memory.fill`'s discarded memory-index
  bytes, which sub-opcode is being executed was the only thing that
  mattered), so every `0xFC` sub-opcode now packs `(sub, data_idx)`
  uniformly into one `Operand::Index`, mirroring the existing `Atomic`/
  `Simd` packed-operand shapes (`data_idx` is simply `0`, unused, for
  the sub-opcodes that don't need it).

## [0.9.6] - 2026-08-16 (task #94 — bulk memory: memory.fill + a real bounds-check bug)

### Added

- `LinearMemory::fill(dest, value, len)` -- the `memory.fill` bulk-memory
  primitive. Wired into the `0xFC 0x0B` interpreter handler (`memory.copy`,
  `0xFC 0x0A`, already existed for E4-dyn's runtime string concat).

### Fixed

- **Real bug, found vendoring `memory_copy.wast`**: `LinearMemory::copy()`
  (and the new `fill()`, written the same way before this fix) special-
  cased `len == 0` to always return `Ok(())` BEFORE the bounds check ran
  at all -- so a zero-length copy/fill with a wildly out-of-range `dest`/
  `src` (e.g. `memory.copy $dest=0x10001 $src=0 $len=0` on a 1-page/
  0x10000-byte memory) silently succeeded instead of trapping. The real
  spec only exempts a zero-length op from bounds failure when `dest`/
  `src` sits at EXACTLY `data.len()` (one-past-the-end, the same
  convention as a Rust slice's exclusive upper bound) -- anything past
  that must still trap, zero-length or not. Removed the special case
  entirely: the existing `checked_add`/`<=` bounds check already handles
  `len == 0` correctly on its own (`x.checked_add(0) == Some(x)`, and
  `data[x..x]` is a valid empty slice for any `x <= data.len()`), so no
  separate branch was ever needed. TEMP-REVERT-CHECK confirmed the fix is
  load-bearing (reverting reproduces `memory_copy.wast`'s exact 3 failing
  `assert_trap` directives + `memory_fill.wast`'s 1).

## [0.9.5] - 2026-08-16 (task #96 — multi-table)

### Added

- New `pub const MAX_TABLES: usize = 64` -- the real, bounded cap on
  total table count (imported + declared), replacing WASM 1.0's
  hardcoded "at most 1". Unlike W16's multi-memory work, this needed no
  companion storage-layer changes: `Table` storage (`WasmInstance.
  tables`/`WasmExecutionContext.tables`) has been a `Vec` all along, and
  `table.get`/`table.set` plus element-segment application already
  indexed by a real, decoded table index rather than assuming table 0.
- New `pub const MAX_TABLE_ELEMENTS: u32 = 10_000_000` (security review):
  a single table's declared `min` was never bounds-checked before this
  interpreter's own eager `Table::new` allocation, and raising
  `MAX_TABLES` from 1 to 64 in this same change amplified that gap's
  blast radius 64x. Unlike memory's real spec-mandated 65536-page
  ceiling, this is an implementation-defined resource limit (real WASM
  allows a table `min` up to `2^32 - 1`), enforced by `wasm-validator` at
  validation time.

## [0.9.4] - 2026-08-15 (W16, task #85 — multi-memory first slice)

### Changed (breaking)

- `WasmExecutionContext.memory: Option<*mut LinearMemory>` is now
  `memories: Vec<*mut LinearMemory>`. Same shape for
  `WasmEngineConfig.memory`/`WasmEngineState.memory`/
  `WasmExecutionEngine`'s private `memory` field (all `Option<...>` ->
  `Vec<...>`/`Vec<Box<...>>`). Every existing load/store/bulk-memory
  opcode handler still only ever targets memory 0 (`get_memory()` is now
  a thin `get_memory_at(ctx, 0)` wrapper; new `get_memory_at(ctx, memidx)`
  added for the two handlers that need a real index) -- this is purely a
  representation widening, not a behavior change, for every instruction
  except the two below.
- `memory.size`/`memory.grow` (0x3F/0x40) now actually read the memory
  index `wasm-opcodes` already declared an immediate for and this crate's
  own generic operand decoder already decoded -- previously discarded,
  always targeting memory 0 regardless of the real encoded index.
- New `pub const MAX_MEMORIES: usize = 64` -- the real, bounded cap on
  total memory count (imported + declared) a module may have, replacing
  WASM 1.0's hardcoded "at most 1". `pub` so `wasm-validator` enforces the
  identical cap, matching `MAX_V128_HEAP_LEN`'s existing cross-crate reuse
  pattern.

See `code/specs/W16-wasm-multi-memory-first-slice.md` for the full design
and root-cause writeup. Closes the last remaining conformance gap in the
61-file vendored corpus: `memory_grow.wast` (declares 4 memories) was the
only file keeping `module` (933/934) and `register` (1/2) below 100% --
both now at 100%, with zero regressions anywhere else in the corpus
(verified via a full before/after baseline diff).

## [0.9.3] - 2026-08-15 (task #86, W15 follow-up — v128 invoke arguments)

### Changed

- `MAX_V128_HEAP_LEN` is now `pub`. An embedder allocating directly into
  `WasmInstance.v128_heap` from OUTSIDE this crate's own execution loop
  (e.g. `wasm-conformance` converting a `v128.const` `invoke` argument
  into a real handle before a call even starts, now that `WasmInstance.
  v128_heap` is persistent -- W15) needs the identical cap this crate's
  own `push_v128`/`evaluate_const_expr` already enforce, not a
  separately-chosen or unbounded one.

## [0.9.2] - 2026-08-15 (W15, task #79 — v128 persistent storage)

### Fixed (breaking)

- `evaluate_const_expr`'s signature grows a third parameter,
  `v128_heap: &mut Vec<[u8; 16]>`, and gains a `0xFD`/`v128.const` match
  arm. Previously any `v128.const` inside a constant expression (a
  global initializer, data-segment offset, or element-segment offset)
  hit the catch-all `"illegal opcode 0x{opcode:02X} in constant
  expression"` -- real, spec-legal WAT like
  `(global (mut v128) (v128.const ...))` failed to instantiate
  outright. Confirmed against the real, pinned-commit `simd_const.wast`
  corpus.
- `WasmExecutionContext::v128_heap` is no longer unconditionally reseeded
  to `vec![[0u8; 16]]` on every `call_function`/`call_function_with_v128`
  invocation -- it now clones from a new `WasmExecutionEngine` field
  (also `v128_heap`, defaulting to the same reserved-zero-entry `Vec`),
  set via a new optional setter, `set_v128_heap` (same pattern as the
  existing `set_struct_field_counts`/`set_type_section`, chosen
  specifically so the ~50 existing `WasmEngineConfig`-construction unit
  tests that don't care about v128 at all don't need updating). Any v128
  value a caller wants to persist across separate calls on the same
  engine (e.g. `wasm-runtime`'s `WasmInstance.v128_heap`, see the
  companion `wasm-runtime` 0.6.1 release) now round-trips correctly
  instead of the handle going stale the moment one call ends.
- `WasmEngineState` gains a `v128_heap` field, written back from
  `WasmExecutionEngine::into_state()`. `call_function_impl` clones
  `ctx.v128_heap` into `self.v128_heap` UNCONDITIONALLY, alongside
  `self.globals`/`self.host_functions`, before the trap-check that can
  return early -- a clone rather than a move because the per-result
  `V128Bytes` resolution loop later in the same function still needs a
  live borrow of `ctx.v128_heap`. Security review (round 1) caught an
  earlier version of this that wrote back only AFTER that resolution
  loop, which is reachable only on the success path: a call that pushed
  a new `v128.const` entry and then trapped silently lost that heap
  growth -- the exact class of bug `wasm-runtime::call_engine`'s own
  doc comment already warns about for memory/tables. New regression
  test (`v128_heap_growth_survives_a_call_that_traps`,
  TEMP-REVERT-CHECK confirmed load-bearing) proves heap growth from
  before a trap is retained.

See `code/specs/W15-wasm-v128-persistent-storage.md` for the full design
and motivating corpus evidence.

## [0.9.1] - 2026-08-15 (task #81 — v128/funcref/externref single-value blocktypes)

### Fixed

- `decode_function_body`'s `"blocktype"` operand decoder only special-cased
  the 4 MVP scalar single-byte blocktypes (`0x40`/`0x7F`/`0x7E`/`0x7D`/
  `0x7C`) as raw bytes; `v128` (`0x7B`, SIMD) and `funcref`/`externref`
  (`0x70`/`0x6F`, WASM17) fell through to the signed-LEB128 type-index
  branch instead, producing a bogus negative "type index" (`0x7B` decodes
  to signed value -5) for a completely ordinary `(block (result v128)
  ...)` or similar. Found vendoring the real `simd_const.wast` corpus
  (task #78) -- confirmed a genuine bug, not a capability gap.
- `block_arity`'s matching range (`0x7C..=0x7F`) had the identical gap on
  the runtime side -- a `br` out of a `(block (result v128) ...)` would
  have silently dropped the branched value (arity `(0, 0)` instead of
  `(0, 1)`) even once the decoder above is fixed, since this function
  independently re-derives arity from the same raw byte. Verified
  load-bearing via TEMP-REVERT-CHECK: reverting just this arm reproduces
  a `StackUnderflow` trap on a dedicated regression test, then restored.
- 3 new tests: the decoder now carries `0x7B`/`0x70`/`0x6F` as their raw
  byte (not a signed-LEB128 type index), and a real end-to-end `br` out
  of a `(block (result v128) ...)` correctly carries its value across
  the branch.
- Security review (round 1) found a pre-existing, adjacent issue: the
  `"blocktype"` operand decoder's `let byte = code[offset];` was an
  UNCHECKED index, unlike `f32`/`f64` immediately above it (which already
  guard truncated input with a length check and a safe default). Reachable
  without going through `wasm-validator::validate()` first --
  `wasm-runtime::instantiate()`/`call()` don't call it themselves, only
  the separate `load_and_run()` convenience wrapper does -- so a real
  embedder could panic the process on a function body truncated right
  after a `block`/`loop`/`if` opcode. Fixed to `code.get(offset).copied()
  .unwrap_or(0x40)` (empty blocktype, the same safe-default convention
  `f32`/`f64` already use on truncation). Verified load-bearing via
  TEMP-REVERT-CHECK: reverting reproduces the exact predicted panic
  (`index out of bounds`) on a dedicated regression test, then restored.

## [0.9.0] - 2026-08-15 (SIMD PR1b-1 — v128 cross-call-boundary materialization)

### Added

- `V128Bytes(pub [u8; 16])` — a new public type, deliberately separate
  from `WasmValue::V128(u32)` rather than a second meaning layered onto
  that variant: internally, `V128(u32)` always means "a handle into the
  currently live `WasmExecutionContext::v128_heap`", which stops being
  meaningful the instant the context that owns that heap is dropped.
  `V128Bytes` is what a v128 result becomes once it has actually escaped
  the engine.
- `WasmExecutionEngine::call_function_with_v128(func_index, args) ->
  Result<(Vec<WasmValue>, Vec<Option<V128Bytes>>), TrapError>` — the new
  entry point host code needs to observe real v128 *contents* returned
  from a call. `WasmExecutionContext` (and its `v128_heap`) is scoped to
  a single `call_function` invocation and is dropped when it returns, so
  a bare `WasmValue::V128(handle)` result is already meaningless to the
  caller by the time they'd see it — this resolves each `V128` result
  into its real bytes one statement before `ctx` drops, via a partial
  move (`self.globals = ctx.globals; ...` moves only specific fields out
  of `ctx`, leaving `ctx.v128_heap` fully readable afterward since `ctx`
  itself is never consumed as a whole value before this point).
- The previously-public `call_function` is now a thin wrapper over a new
  private `call_function_impl`, which returns the `(results, v128_bytes)`
  pair `call_function` and `call_function_with_v128` each expose one half
  of. `call_function`'s existing signature and behavior are unchanged —
  every existing caller keeps working with no changes.

### Why not a wider fix

An earlier design (threading a `ctx` reference through `WasmValue::
to_typed`/`from_typed` so `V128` could resolve itself lazily wherever a
`WasmValue` is observed) was investigated and rejected: those two methods
alone have ~250 call sites across `push_wasm`/`pop_wasm`/`peek_wasm`/etc.,
real mass-rewrite territory for a problem that only actually matters at
one seam — a function's *results*, the one place a v128 value crosses
out of the engine's control. Internal v128 values living entirely within
one call's execution never need this treatment; only what escapes does.

### Added

- `WasmValue::V128(u32)` — a handle into a new `WasmExecutionContext::
  v128_heap: Vec<[u8; 16]>`, mirroring `WasmValue::Ref`'s existing
  GC-heap-handle shape exactly (tagged `0x7B` on the typed stack, riding
  the existing `Value::Int` payload — the shared `virtual-machine::Value`
  enum is untouched, per `code/specs/
  W13-wasm-simd-v128-first-slice.md`'s design decision). Handle `0` is
  permanently reserved as the all-zero vector, seeded once per top-level
  `call_function` call — the value `WasmValue::default_for(ValueType::
  V128)` returns for an uninitialized `(local $x v128)`.
- Real `0xFD`-prefix decoding in `decode_function_body`: a genuinely new
  LEB128-based sub-opcode read, distinct from the existing single-byte
  `0xFB`/`0xFC`/`0xFE` pattern (SIMD's sub-opcode space runs past 127 —
  `i32x4.add`'s real value, 174, needs the 2-byte LEB128 continuation
  encoding, and this decoder is verified to actually take that path via
  a dedicated regression test, not just the single-byte-safe happy
  path). `v128.const`'s own 16-byte literal is read as raw (non-LEB128)
  bytes, per the real spec.
- New per-function side-table `WasmExecutionContext::simd_consts: Vec<[u8;
  16]>` for `v128.const` literals — same shape and same SavedFrame-
  threaded save/restore-across-nested-calls treatment `br_table_targets`/
  `gc_ops` already have (a `v128.const`'s 16-byte immediate doesn't fit
  in a plain `Operand::Index(usize)`, so it's spilled into this table at
  decode time, exactly like `Gc`'s existing shape).
- 5 opcodes implemented: `v128.const`, `i32x4.splat`, `i32x4.add`,
  `i32x4.eq` (WASM's boolean-mask convention: all-1s/-1 if equal, all-0s
  if not — NOT a plain scalar 0/1), and `i32x4.extract_lane` (added
  beyond the original 4-opcode spec scope — the only way to observe a
  `v128`'s contents as a plain scalar, genuinely required to make this
  slice's OWN correctness verifiable via a real test rather than "it
  compiles and doesn't panic").
- 6 new hand-built-bytecode regression tests verifying actual COMPUTED
  values (not just "returns some v128"): per-lane round-trip through all
  4 lanes, splat broadcast, real lane-wise wrapping addition (including
  the `i32::MAX + 1` wraparound case specifically), the eq boolean-mask
  convention (both the equal and not-equal cases), multiple `v128.const`s
  in one function body staying distinct (a real regression risk given
  the const pool is a per-function, decode-order-indexed side-table),
  and an out-of-range lane index trapping cleanly rather than panicking.

### Fixed — 1 finding from this PR's own `/security-review`, before shipping

- **Unbounded `v128_heap` growth (memory-exhaustion DoS).** Every SIMD op
  that produces a new v128 (`v128.const`/`splat`/`add`/`eq`)
  unconditionally pushed a new entry with no reclamation. This crate's
  own threat model treats WASM bytecode as untrusted -- a `loop` with a
  backward `br` executing e.g. `i32x4.splat` on every iteration needs NO
  recursion at all (so `MAX_CALL_DEPTH` never engages) and would grow
  `v128_heap` without bound until the process OOMs -- exactly the
  failure mode `gc_heap` had before W04 added real mark-sweep collection,
  which `v128_heap` doesn't get (see its own doc comment for why: v128
  values are immutable-once-created `Copy` data, no cycles to collect,
  but still no upper bound without an explicit one). Fixed with a new
  `MAX_V128_HEAP_LEN` (1,000,000 entries / 16 MiB) checked in a new
  `push_v128` helper every v128-creating opcode now routes through,
  mirroring `MAX_CALL_DEPTH`'s existing guard shape. New regression test
  runs the exact adversarial shape (an infinite `loop` creating a v128
  every iteration) against the REAL production constant (not a reduced
  test-only value) and confirms it traps cleanly in ~1.2s rather than
  hanging or exhausting memory.

### Deferred to a follow-up PR (task #72)

- `wasm-wast-parser` support for `v128.const`'s real text-literal syntax
  — this slice is only exercisable via hand-built raw bytecode so far,
  not the `.wast` text format.
- `wasm-validator` type rules for the 5 new opcodes.
- `wasm-conformance` real V128 bit-exact comparison in `assert_return`
  grading, and vendoring any `simd_*.wast` corpus file. (`ctx.v128_heap`
  does not persist past `call_function` returning — the same limitation
  `WasmValue::Ref`'s GC-heap handles already have in this codebase — so
  designing real cross-call v128 value comparison needs its own
  investigation, not assumed to be a trivial addition.)
- Splitting this PR further than originally scoped in `code/specs/
  W13-wasm-simd-v128-first-slice.md`'s own PR-1 plan (which expected the
  conformance-graded pass in the same PR) was a deliberate, reported
  judgment call made during implementation, not a silent scope cut — the
  interpreter core alone already required a genuinely new decoder shape,
  a new per-function side-table with full `SavedFrame` threading, and a
  real design gap discovered and fixed (see the "one real gotcha" note
  below) — substantial enough to ship and verify on its own before
  taking on the corpus-grading layer's own separate design questions.

### Fixed (implementation-time, before this shipped)

- The first draft of the `simd_consts`/`v128_heap` side-table threading
  made `ctx` a `&mut WasmExecutionContext` REFERENCE inside the spawned-
  thread closure (via a raw-pointer deref, following `vm_ptr`'s existing
  WASM10 pattern) rather than an owned value — every existing `&mut ctx`
  call site (e.g. `vm.execute_with_context(&code, &mut ctx)`) then
  silently built a `&mut &mut WasmExecutionContext` (double reference),
  which type-checked but broke the opcode dispatcher's runtime downcast
  at runtime ("context must be WasmExecutionContext"). 128 of 213 unit
  tests caught this immediately when the full suite was run — fixed by
  relying on Rust's implicit reborrow (passing `ctx` directly, not
  `&mut ctx`) at the one call site that needed it. A reminder that a
  clean `cargo build` is necessary but not sufficient — the full test
  suite is what actually catches a working-as-designed-on-paper change
  that's subtly wrong.

## [0.7.0] - 2026-08-15 (WASM10 — dedicated-thread `call_function`, raised `MAX_CALL_DEPTH`)

### Changed

- `call_function` (the top-level, public entry point) now spawns one
  dedicated OS thread per call, via `Builder::stack_size(DEDICATED_STACK_SIZE)
  .spawn_scoped(...)`, and runs its entire recursive decode/dispatch loop —
  including every nested `call`/`call_indirect` through `call_function_inner`
  — on that thread, joining synchronously before returning. `MAX_CALL_DEPTH`'s
  safety margin no longer depends on whatever stack the CALLER happens to
  provide; the calling thread only spawns and blocks in `.join()`, doing
  none of the recursive work itself. See `code/specs/
  W12-wasm-dedicated-thread-call-depth.md` for the full design and the
  reasoning for rejecting a `Send`-bound change to `HostFunction`/
  `HostInterface` in favor of this approach.
- New private `AssertSend<T>` newtype (with an explicit, in-code safety
  argument) crosses the thread boundary for the call-local raw memory/table
  pointers and `Box<dyn HostFunction>` entries — this is the mechanism that
  avoids the breaking trait change. One real gotcha hit and fixed during
  implementation: Rust 2021's disjoint-closure-capture analysis reaches
  straight through a `let AssertSend(inner) = x;` destructure at the top of
  a spawned closure, capturing `x`'s non-`Send` INNER fields individually
  rather than `x` itself -- silently defeating the wrapper (the closure then
  fails to compile, requiring `Send` on the raw pointers directly, the exact
  error this type exists to avoid). Fixed by adding `AssertSend::into_inner`,
  a method call (not a field-projection/destructure), which forces
  whole-value capture of the wrapper instead.
- `MAX_CALL_DEPTH` re-bisected directly against the new `DEDICATED_STACK_SIZE`
  (8 MiB) rather than scaled by assumption from the old 512-KiB-based value:
  a real bounded countdown-recursion WASM module, run at increasing depths
  through `call_function` (so through the real dedicated-thread path), one
  depth per subprocess (a stack overflow aborts the whole process). Measured,
  reproducible (3 repeats, identical result) debug-build floor: safe at
  depth 1820, overflows at depth 1830. Applying the same ~33%-margin-below-
  the-safe-floor convention the original 80 used: raised from **80 to
  1200**.
- `call.wast`'s `even(100)`/`odd(200)` mutual-recursion cases -- previously
  the only 2 `assert_return` failures in that file, a known, honestly-
  documented trade-off since the original `MAX_CALL_DEPTH` guard shipped --
  now both pass. `wasm-conformance` baseline regen: `call.wast`
  `assert_return` moves from `pass: 67, fail: 2` to `pass: 69, fail: 0`;
  confirmed via a full baseline diff that this is the ONLY file affected,
  zero regressions elsewhere.
- New `tests/wasm10_dedicated_thread.rs`: the exact `call.wast` even/odd
  shape as a standalone regression guard; a caller-thread-with-a-256-KiB-
  stack test proving `call_function` now completes ~1000 levels of
  recursion that would overflow a Rust stack that size directly, since the
  heavy work runs elsewhere (verified this is load-bearing via
  TEMP-REVERT-CHECK: temporarily shrinking `DEDICATED_STACK_SIZE` to 64 KiB
  reproduced the exact same crash the dedicated thread exists to prevent);
  an unbounded-recursion-still-traps-cleanly test at the new ceiling.
  `tests/call_depth_guard.rs`'s existing
  `depth_guard_trips_before_overflow_on_the_documented_minimum_stack` test
  doc comment updated to reflect its changed meaning under WASM10 (it now
  proves the caller-stack decoupling, not the original caller-stack-size
  claim, which no longer applies).

### Non-goals (unchanged from the spec)

- `HostFunction`/`HostInterface` — no `Send` bound added, no signature
  change; `wasm-conformance`'s real `Rc<RefCell<..>>`-based cross-module
  linking (WASM05) is untouched.
- Real concurrent/multi-threaded WASM execution — this dedicated thread is
  purely an implementation detail for stack-size control; `call_function`
  remains synchronous and single-result from the outside, identical to
  before.
- Thread pooling/reuse for performance — explicitly deferred; the known
  trade-off (per-call OS thread spawn overhead, most visible in
  `wasm-conformance`'s full baseline run) is accepted for this first slice.

### Fixed — 2 findings from this PR's own `/security-review`, before shipping

- **Panic during the dedicated thread skipped the mandatory state
  write-back.** A panic reached through `Box<dyn HostFunction>::call`
  (e.g. `wasm-conformance`'s real `CrossModuleFunction` hitting its own
  documented `RefCell` double-borrow panic on a circular cross-module
  import) used to `resume_unwind` BEFORE `self.host_functions`/
  `self.globals` were restored from `ctx` — the exact bug class WASM07's
  security review already fixed once for TRAPS, reintroduced here for
  PANICS (`self.host_functions`, emptied via `mem::take` for the
  panicking call, would stay empty for every LATER, unrelated call on the
  same engine). Fixed by wrapping the dedicated thread's loop in
  `std::panic::catch_unwind(AssertUnwindSafe(...))` and restoring engine
  state UNCONDITIONALLY before ever propagating the caught panic via
  `resume_unwind`. New regression test
  (`a_panic_inside_the_dedicated_thread_restores_engine_state_before_propagating`),
  verified via TEMP-REVERT-CHECK to reproduce the exact bug (a later call
  to an unrelated, previously-working host-imported function fails with
  `"no body for function 1"`) when the restore is reordered back to after
  the panic check.
- **Unbounded OS-thread nesting via cross-module host calls.** Every
  top-level `call_function` now spawns a dedicated 8 MiB-stack thread —
  including calls reached through `HostFunction::call` re-entering a
  DIFFERENT engine's own `call_function` (exactly `wasm-conformance`'s
  real cross-module linking, WASM05). `MAX_CALL_DEPTH`/`ctx.call_depth`
  resets to 0 per top-level call and does not see across this boundary at
  all, so an ordinary, non-circular chain of N linked module instances
  would spawn N nested OS threads with no bound — a materially larger,
  more exhaustible resource than the old unbounded-Rust-stack-recursion
  version of this same reentrancy pattern. Fixed with a new
  `MAX_DEDICATED_THREAD_DEPTH` (64) guard, tracked via a `thread_local!`
  counter explicitly propagated parent-thread → child-thread at each
  `call_function` invocation (deliberately NOT a single process-global
  counter, which would incorrectly conflate unrelated, genuinely-
  concurrent top-level call chains from separate caller threads). New
  regression tests: a white-box pair in `lib.rs` proving the guard trips
  exactly at the max and not below it, and a black-box
  `a_long_but_finite_cross_module_chain_traps_cleanly_before_exhausting_os_threads`
  integration test (a real 100-engine `Rc<RefCell<..>>` chain), verified
  via TEMP-REVERT-CHECK to reproduce the unbounded-spawn behavior (the
  full 100-deep chain completes instead of trapping) with the guard
  disabled.

### Fixed — round 2: 1 more finding from re-reviewing the fixes above

- **A thread-spawn failure could ALSO skip the state write-back.** The
  first version of the panic-safety fix above still moved `ctx` BY VALUE
  into the closure passed to `Builder::spawn_scoped`. If spawning the
  dedicated thread itself failed (a real possibility under OS thread/
  resource exhaustion — this feature can spawn up to
  `MAX_DEDICATED_THREAD_DEPTH` nested 8 MiB-stack threads per call chain,
  on top of one thread per concurrent top-level call from a multi-
  threaded host), the closure — and `ctx`, and `self.host_functions`
  moved into it — was dropped without ever running, and the `.expect(...)`
  on the failed `spawn_scoped` call panicked immediately, unwinding
  straight out of `call_function` BEFORE the `let AssertSend((ctx, ...))
  = ...` binding (and the restoration after it) was ever reached — the
  same WASM07 bug class, reintroduced via a THIRD trigger the first round
  of fixes didn't cover. Fixed by keeping `ctx` OWNED in `call_function`'s
  own (spawning) stack frame for the entire call — only a raw pointer to
  it (`ctx_ptr`, given the exact same `AssertSend`/safety treatment
  `vm_ptr` already had) crosses into the spawned closure, so `ctx` is
  provably still there to restore from regardless of whether the thread
  spawns, panics, or completes normally. Restructured the three possible
  outcomes (success / panic / spawn failure) into a small local
  `DedicatedThreadFailure` enum so the mandatory restoration runs exactly
  once, unconditionally, before any of the three is handled. This
  invariant is now also compiler-enforced, not just test-enforced: since
  `ctx` is never moved into the closure, any future regression that tried
  to move it there again would fail to compile (`self.host_functions =
  ctx.host_functions;` after the `thread::scope(...)` call requires `ctx`
  to still be a valid, non-moved binding).
- Fixing the above surfaced a real, independent bug in the FIRST draft of
  this restructuring (caught by the full test suite, not by the security
  review): `ctx` inside the spawned closure became `&mut
  WasmExecutionContext` (a reference, via `ctx_ptr`) instead of an owned
  value, so every existing `&mut ctx` call site (e.g. `vm.
  execute_with_context(&code, &mut ctx)`) silently built a `&mut &mut
  WasmExecutionContext` instead of `&mut WasmExecutionContext` — type-
  checks fine, but breaks the opcode dispatcher's downcast at runtime
  ("context must be WasmExecutionContext"), which 128 of 213 unit tests
  caught immediately. Fixed by relying on Rust's implicit reborrow
  (passing `ctx` directly, not `&mut ctx`, since `ctx` is already a `&mut`
  reference) at the one call site that needed it.

## [0.6.12] - 2026-08-15 (security fix — pre-existing `call_indirect` panic)

### Fixed

- `call_indirect` (0x11)'s type-check block indexed `ctx.func_types[func_index]`
  directly, where `func_index` comes from `table.get(elem_index)` -- DATA, not
  a static part of the bytecode a validator necessarily already checked (this
  crate's own tests, and real embedders that skip `wasm-validator`, can drive
  this engine directly). A crafted or corrupt table entry pointing past
  `func_types.len()` panicked instead of trapping cleanly, whenever the
  embedder had set a type section via `set_type_section` (the common case --
  `wasm-runtime` always does). This is the exact bug class WASM16's own
  security review fixed in the new `return_call_indirect` (0x13) handler
  (0.6.11); that review explicitly flagged this pre-existing, untouched
  occurrence in `call_indirect` as a follow-up rather than in-scope for that
  PR. Fixed with the same `.get().ok_or_else(...)` pattern. New regression
  test (`test_call_indirect_through_a_table_entry_referencing_an_undefined_
  function_is_a_clean_error_not_a_panic`), verified via TEMP-REVERT-CHECK to
  reproduce the exact real panic (`index out of bounds: the len is 1 but the
  index is 99`) with the fix reverted.

## [0.6.11] - 2026-08-15 (WASM16 — real tail calls, genuinely constant Rust-stack space)

### Added

- Real `return_call`/`return_call_indirect` handlers. The naive
  implementation ("call the target, then return its result") would
  still recurse through `call_function`/`call_function_inner` exactly
  like an ordinary `call` -- defeating the entire point of the
  instruction, and silently failing its own primary use case (an
  unbounded tail-recursive loop would still hit `MAX_CALL_DEPTH` and
  trap on exactly the pattern the instruction exists to make
  unbounded). The real fix restructures BOTH of this crate's
  "run a function body" implementations (`call_function_inner`, used
  for nested calls, and `WasmExecutionEngine::call_function`, the
  separate top-level entry point with its own independent dispatch
  loop) with an outer loop: a tail call swaps in the callee's state
  and continues the SAME Rust stack frame, pushing no new `SavedFrame`
  and not incrementing `call_depth` -- the mechanical reason an
  unbounded `return_call` chain runs in genuinely constant Rust-stack
  space. See `code/specs/W11-wasm-tail-calls.md`.
- New `WasmExecutionContext::pending_tail_call: Option<(usize,
  Vec<WasmValue>)>` field -- set by the `return_call`/
  `return_call_indirect` opcode handlers to signal the enclosing
  "run a function body" loop, checked once per outer-loop iteration
  right after the inner instruction-dispatch loop halts.
- `code/packages/rust/wasm-execution/tests/wasm16_tail_calls.rs`: 5 new
  integration tests built from real WAT via `wasm-wast-parser`,
  including the load-bearing proof -- a self-tail-recursive accumulator
  running 20,000 iterations deep (250x `MAX_CALL_DEPTH`'s value of 80)
  succeeding cleanly, plus a companion test proving the SAME depth
  written with plain `call` still correctly traps (this PR doesn't
  weaken `MAX_CALL_DEPTH`'s existing guard), mutual tail recursion
  across two distinct functions at the same depth, `return_call_indirect`
  through a table, and a single non-recursive `return_call`.
- Full existing test suite (227 tests, spanning both "run a function
  body" implementations) passes completely unchanged by this
  restructuring -- strong evidence the refactor preserves every
  existing non-tail-call code path exactly.

### Investigated, not fixed this release

- The real, pinned-commit `return_call.wast`/`return_call_indirect.wast`
  each fail to parse (see `wasm-wast-parser` 0.1.10's CHANGELOG for
  why, and why a minimal fix was investigated and rejected as
  incorrect, not just skipped). Not vendored this release; the real
  tail-call machinery is fully verified via the hand-written
  integration tests above instead.

### Fixed

- Security review caught the new `return_call_indirect` (`0x13`)
  handler directly indexing `ctx.func_types[func_index]` where
  `func_index` comes from a TABLE ENTRY (data, not a static part of
  the bytecode a validator necessarily already checked -- this engine
  can be, and in this crate's own tests sometimes is, driven without
  `wasm-validator` running first) -- a table slot pointing past
  `func_types.len()` panicked the host process instead of trapping
  cleanly. Fixed with `.get()`, matching the sibling `return_call`
  (`0x12`) handler's already-safe pattern. 1 new regression test,
  confirmed to panic without the fix via a temporary revert.

## [0.6.10] — 2026-08-15 (WASM05 — Table gains Clone, LinearMemory/Table gain limit accessors)

### Added

- `Table` now derives `Clone` (matching `LinearMemory`, which already
  did) -- `wasm-conformance`'s new registry-backed `HostInterface`
  (WASM05/W10) needs to hand back an owned `Table`/`LinearMemory` when
  resolving a cross-module import, since `HostInterface::resolve_table`/
  `resolve_memory` return owned values, not references.
- `LinearMemory::max_pages()` and `Table::max_size()` -- both already
  had `size()`; the declared maximum was tracked internally but had no
  public getter. `wasm-runtime`'s new link-time limits-compatibility
  check needs both halves of a `Limits` pair, not just the current size.

## [0.6.9] — 2026-08-15 (WASM18 — plain atomic memory op handlers, no real concurrency)

### Added

- Real opcode handlers for the entire `0xFE`-prefixed atomics family
  (load/store/RMW/cmpxchg/fence/notify/wait), dispatched via
  `wasm_opcodes::ATOMIC_OPS` -- one `register_context_opcode(0xFE, ...)`
  handler matching on `AtomicOpKind` rather than 67 individual handlers.
  RMW ops (`add`/`sub`/`and`/`or`/`xor`/`xchg`) and `cmpxchg` reuse the
  existing narrow-width `LinearMemory` load/store methods already built
  for MVP `i32`/`i64` load/store, so no new memory-access code paths
  were needed -- only new dispatch and arithmetic.
- `memory.atomic.notify` always returns 0 woken; `wait32`/`wait64`
  return 1 ("not-equal") or 2 ("timed-out") based on a plain memory
  comparison against `expected` -- both fully deterministic without any
  real threading, since a single-threaded VM can never have a second
  agent blocked in `wait`. See `wasm-opcodes` 0.2.3's CHANGELOG for why
  this contradicts the merged W09 spec's original "meaningless without
  real threads" framing.
- Runtime alignment trap: atomic instructions now check that the
  *effective* address (`base + offset`, where `base` can be a runtime
  value) is a multiple of the operation's natural alignment, trapping
  with `"unaligned atomic"` if not. This is distinct from (and in
  addition to) `wasm-validator`'s existing check of the *declared*
  `align=` immediate, which is a static property of the bytecode and
  can't see a runtime-computed address. Confirmed against the real,
  pinned-commit `atomic.wast` testsuite's own 45 `assert_trap
  ... "unaligned atomic"` cases -- all 45 now pass (previously 0/45,
  since this check didn't exist at all).
- 13 new tests, including a misalignment/naturally-aligned pair proving
  the new runtime check traps exactly where it should and doesn't
  over-trigger on valid accesses.

### Fixed

- Security review caught the `AtomicOpKind::Wait` handler (`memory.
  atomic.wait32`/`wait64`) missing the `check_atomic_alignment` call
  every sibling atomic-op arm has -- a misaligned wait address would
  silently proceed instead of trapping with `"unaligned atomic"` like
  the spec requires. Not a memory-safety issue (still routes through
  bounds-checked `LinearMemory` accessors), but a real spec-conformance
  gap the vendored `atomic.wast` corpus doesn't happen to exercise
  (its own `"unaligned atomic"` cases only cover load/store/RMW/
  cmpxchg, not wait). Fixed with a dedicated regression test, confirmed
  to fail without the fix via a temporary revert.

## [0.6.8] — 2026-08-15 (WASM17 — ref.func, table.get, table.set opcode handlers)

### Added

- `ref.func` (0xD2): bounds-checks the `funcidx` immediate against
  `ctx.func_types` (same source of truth `call_function_inner`'s own
  bounds check uses) and pushes `WasmValue::Ref(Some(func_index))`.
- `table.get`/`table.set` (0x25/0x26): thin wrappers around the
  already-existing, already-tested `Table::get`/`Table::set` *methods* --
  only `call_indirect`'s hardcoded table-0 lookup and element-segment
  initialization reached them directly before this; no WASM *instruction*
  did. Honors the real decoded `tableidx` (unlike `call_indirect`, which
  hardcodes table 0) so a named `$t` resolves to whichever index
  `wasm-wast-parser` actually emitted.
- `ref.null` (0xD0)/`ref.is_null` (0xD1) needed NO changes -- both already
  had real, tested handlers from before WASM17 (this repo's existing GC
  slice already needed them); they just become *reachable* now that
  `wasm-wast-parser` can emit them from text. See `code/specs/
  W08-wasm-funcref-externref.md`.
- `WasmValue::default_for` and `wasm-runtime`'s lossy `call()` argument
  conversion updated to cover the two new `ValueType` variants
  (`Funcref`/`Externref` default to the null reference, same as every
  other nullable reference type).
- 5 new opcode-level unit tests (`ref.func` valid + out-of-range,
  `table.get`/`table.set` round-trip, uninitialized-slot, out-of-bounds).

## [0.6.7] — 2026-08-13 (WASM04 — multi-value block/loop/if blocktypes)

### Fixed — `block_arity` resolved a type-index blocktype against the wrong table

`block`/`loop`/`if`'s blocktype immediate can now be a signed LEB128
type-section index, not just the MVP's single-byte empty/valtype encoding
(paired with `wasm-wast-parser` 0.1.6, which now emits this form). Wiring
it into the interpreter surfaced two bugs, both latent until a multi-value
blocktype could ever be decoded at all:

- `block_arity` resolved a type-index blocktype against `ctx.func_types`
  (indexed by FUNCTION index — one entry per function, sized to the
  function count) instead of `ctx.types` (the module's real, deduplicated
  TYPE SECTION, the actual index space a blocktype's type-index refers
  to). `call_indirect`'s handler already had this exact wrong-table bug
  fixed once; `block_arity` had the same bug, just never reachable
  before now. Fixed by changing its signature to take the real type
  section and return `(param_arity, result_arity)` instead of a bare
  result-only arity.
- `execute_branch` hardcoded a branch-to-a-loop's arity to `0`, with an
  explicit `// MVP` comment — correct then, since a loop's blocktype could
  never declare params before the multi-value extension. A branch to a
  loop's label re-enters its START, which needs the loop's declared PARAM
  arity preserved on the stack (re-entry re-consumes them), not its result
  arity (which is what a branch to a block/`if`'s END needs instead).
  Added `Label::param_arity` and threaded it through every `Label`
  construction site (`block`/`loop`/`if`'s handlers, the implicit
  function-body label in both `call_function_inner` and the top-level
  entry point), and fixed `execute_branch`'s loop-vs-block arity split.
- 4 new regression tests, including a hand-built case (not lifted from the
  testsuite) that forces a branch to a loop's label through an
  intervening inner scope with real scratch data that must be discarded —
  the shape that actually exercises `param_arity`, since the official
  testsuite's own `params`/`params-break` cases happen to never need real
  stack surgery on this interpreter's unwind step. Verified via
  TEMP-REVERT-CHECK that all 4 fail (one cleanly, one by hanging) with
  either bug reintroduced.
- Baseline effect: `block.wast`, `if.wast`, and `loop.wast` previously
  failed to PARSE AT ALL (multi-value blocktypes were unrecognized
  syntax); all three now pass in full (`assert_return` 52/52, 123/123,
  78/78). `br.wast` and `func.wast` also gained 1 and 3 previously-failing
  `assert_return` cases respectively. `fac.wast` newly parses too. Zero
  regressions — see `wasm-conformance`'s own `0.1.9` changelog entry for
  the full per-file diff.

## [0.6.6] — 2026-08-13 (WASM13 — f32 NaN payloads silently canonicalized on every stack push/pop)

### Fixed — `WasmValue::to_typed`/`from_typed` destroyed f32 NaN bit patterns

`GenericVM`'s typed operand stack has exactly ONE float slot
(`Value::Float(f64)`), shared by both WASM float widths — so every `f32`
value that's merely pushed or popped (locals, params, results, operands;
not just values an opcode actually computed on) round-trips through this
f64 box via `to_typed`/`from_typed`. Those conversions used an ARITHMETIC
`as f64` widen on the way in and `as f32` narrow on the way back out.
Confirmed empirically that Rust's `as` cast between float widths does
**not** guarantee NaN payload preservation on the narrowing leg: `f32::
from_bits(0x7fa00000) as f64 as f32` produces `0x7fc00000` — LLVM's
`fpext`/`fptrunc` canonicalize the payload to the target type's generic
quiet NaN. So ANY f32 NaN merely sitting on the stack silently lost its
exact bit pattern by the time it came back off, independent of which
opcode touched it.

This was invisible for most NaN-producing operations because the WASM
spec itself only requires them to produce a value in the `nan:arithmetic`
CLASS (any quiet NaN, exact payload unspecified) — `wasm-conformance`'s
grading already accepts that loosely, so canonicalization to `0x7fc00000`
still graded `Pass`. It was NOT invisible for `f32.reinterpret_i32`/
`i32.reinterpret_f32` (pure bit reinterpretation, where the testsuite
asserts an EXACT value, not a class) and for any case that round-trips a
NaN through a `local.tee`/param/result boundary without touching it
arithmetically at all — both are supposed to preserve the bits exactly by
construction, and both went through `to_typed`/`from_typed` regardless.

Fixed by making both conversions bit-preserving reinterpretations
(`f64::from_bits(v.to_bits() as u64)` / `f32::from_bits(v.to_bits() as
u32)`) instead of arithmetic casts — lossless for every case (NaN,
normal, ±0.0, ±inf), not just NaN, since it does no rounding at all.
Confirmed `virtual-machine`'s own `GenericVM` never interprets
`Value::Float` numerically outside a `Display` impl (used for debug
printing only), so re-purposing that f64 slot as an opaque bit-carrier
for f32 values has no effect on anything else built on `GenericVM`.

Verified via TEMP-REVERT-CHECK: reverting the fix (restoring the
arithmetic casts) makes all 3 new regression tests fail with exactly the
`0x7fc00000` canonicalization this changelog describes; restoring the fix
makes them pass again.

Baseline: `assert_return` 13495/13518 (99.8%) → 13512/13518 (100.0%,
+17) — closes 4 files' worth of previously-tracked NaN-payload gaps in
one fix: `conversions.wast` (the 4 `reinterpret` cases WASM03 surfaced),
`float_literals.wast`, `float_misc.wast`, and `local_tee.wast`'s
"as-unary-operand" case — this WASM13 backlog item's own two originally-
named repro cases. Verified via a full per-file diff against the previous
baseline that these 4 files are the ONLY ones whose tally changed, and
every one of their fails went to exactly 0 (not just down).

New tests: a direct `to_typed`/`from_typed` round-trip over 6 real NaN bit
patterns (distinct payloads, both signs, quiet and would-be-signaling,
plus the canonical NaN itself as a sanity check that the fix doesn't
break the ALREADY-canonical case); end-to-end `f32.reinterpret_i32`/
`i32.reinterpret_f32` tests through the actual interpreter using the
exact bit patterns the real testsuite asserts against.

## [0.6.5] — 2026-08-13 (WASM03 — sign-extension, trunc_sat, and a real trapping-trunc boundary bug)

### Added

- The 5 sign-extension opcodes (0xC0-0xC4): each pops an int, sign-extends
  its low 8/16/32 bits to the full width via Rust's own `as i8 as i32`-style
  truncate-then-sign-extend cast (exactly matching the spec's `signed_N`
  definition), pushes the result.
- The 8 `trunc_sat` sub-opcodes (`0xFC 0x00`-`0x07`, decoding already
  existed for the `0xFC` prefix from bulk-memory's `memory.copy`/
  `memory.fill` — only the sub-opcode dispatch needed extending): the
  non-trapping float-to-int conversions. Implemented as a straight `as`
  cast with no bounds checking at all, because Rust's own float→int `as`
  cast has used SATURATING semantics (NaN → 0, out-of-range → the nearest
  bound) since Rust 1.45 — a direct, built-in match for the spec's
  definition, needing no hand-rolled boundary logic.

### Fixed — the TRAPPING `trunc_f32/f64_s/u` handlers (0xA8-0xB1) had real, pre-existing boundary bugs

Investigating why `conversions.wast` (only parseable for the first time
after this release's own opcode additions) still had `assert_trap`/
`assert_return` failures after the two additions above found these
**already-existing**, entirely unrelated bugs, invisible until now because
`conversions.wast` was the only vendored file exercising these boundary
cases and it could never parse before:

- `i32.trunc_f32_u`/`i32.trunc_f64_u` (0xA9/0xAB) used an inclusive `0.0..`
  lower bound, so any negative input — even a tiny one that truncates
  toward zero to a perfectly valid `0` — incorrectly trapped.
- `i32.trunc_f64_s` (0xAA) used an inclusive lower bound (`-2147483649.0..`)
  where the spec requires a STRICT exclusion — `-2147483649.0` itself (one
  past the valid range) was wrongly accepted instead of trapping.
- All four i64-destination handlers — `i64.trunc_f32_s`/`i64.trunc_f32_u`/
  `i64.trunc_f64_s`/`i64.trunc_f64_u` (0xAE/0xAF/0xB0/0xB1) — had **no
  overflow check at all**, only a NaN check — `a as i64`/`a as u64 as i64`
  alone is that same Rust-1.45+ SATURATING cast the new `trunc_sat`
  handlers correctly rely on above, so these TRAPPING opcodes were silently
  behaving like their non-trapping `trunc_sat` counterparts instead of
  trapping on overflow, the opposite of their contract.

Fixed with 4 new shared boundary-check functions (`trunc_s_i32_in_range`,
`trunc_u_i32_in_range`, `trunc_s_i64_in_range`, `trunc_u_i64_in_range`),
each doc-commented with the exact spec inequality and why the chosen f64
literal constants are exact (not approximations) for that specific
boundary — see their own doc comments for the precision reasoning, which
differs between the i32 case (plain strict inequalities against
exactly-representable `f64` constants) and the i64 case (the true boundary
constant itself isn't exactly representable in `f64`, but no representable
`f64` value exists in the gap where it would matter, so a carefully chosen
inclusive/exclusive form is still exact). `f32` sources are widened to
`f64` first (lossless) before applying the same checks, avoiding a second,
separate set of `f32`-precision boundary constants.

Baseline: `i32.wast`/`i64.wast` go from full parse failures to 100% passing
every directive kind they have; `conversions.wast` goes from a full parse
failure to `assert_trap` 67/67 (100%) and `assert_return` 522/526 (99.2%,
+1253 across the three newly-parseable files against the pre-WASM03
baseline). The 4 remaining `conversions.wast` `assert_return` fails are
`f32.reinterpret_i32`/`i32.reinterpret_f32` NaN-bit-pattern cases —
unrelated to this release, corroborating evidence for the already-tracked
WASM13 (NaN payload preservation) backlog item, not fixed here. Verified
via a full per-file diff against the previous baseline that these 3 newly-
parseable files are the ONLY files whose tally changed anywhere in the
corpus.

New tests: 5 sign-extension round-trips, 8 `trunc_sat` cases (ordinary
value, NaN-saturates-not-traps, overflow/underflow saturation for both
signed and unsigned, both `i32` and `i64` destinations), and 7 regression
tests for the trapping-trunc boundary fix (the exact tiny-negative,
exact-lower-boundary, and previously-never-trapping-on-overflow cases the
old code got wrong, plus the i64::MIN exact-boundary acceptance case).

## [0.6.4] — 2026-08-13 (WASM11 — a real branch double-pop bug)

`execute_branch` (the shared handler behind `br`/`br_if`/`br_table`) used
to `ctx.label_stack.truncate(label_stack_index)` — removing the TARGET
label — and then jump to `label.target_pc`. For a `block`/`if` label,
`target_pc` is the literal position of that block's own `end` opcode (not
one past it — see `block`'s handler and `build_control_flow_map`), and
the `end` handler unconditionally pops one label whenever it runs,
whether reached by ordinary fall-through or landed on by a branch.
Removing the target via `truncate` and THEN landing on its own `end`
byte popped it a SECOND time — a genuine double-pop that silently
removed one extra label (belonging to whatever the next enclosing block
happened to be) on any branch that unwound past one or more already-open
outer blocks. This was invisible for the extremely common "the
branched-into block is effectively the last thing in the function"
shape (the accidental extra pop just triggered the function-end path a
little early, with no observable difference), but produced a real
`StackUnderflow` trap for anything with real code still to run after the
target block closes — found running the official WebAssembly spec
testsuite's own `switch.wast` (a `br_table` dispatching through 10
levels of nested named blocks — some targets land in the MIDDLE of the
nesting, not just the innermost or outermost), not by inspection.

Fixed by keeping a `block`/`if` target's label ON `label_stack` when
branching to it (`truncate(label_stack_index + 1)` instead of
`truncate(label_stack_index)`), so landing on its own `end` byte pops it
EXACTLY once — identical to what ordinary, non-branching fall-through to
that same `end` already does. A `loop` target keeps the ORIGINAL
behavior (`truncate(label_stack_index)`, no `+ 1`): a loop's
`target_pc` is the position of the `loop` OPCODE ITSELF (not an `end`
byte), so branching back to it re-executes that opcode, which
unconditionally re-pushes a fresh label — keeping the old one too, as an
early draft of this fix did uniformly for both kinds, left both the
retained old label and the freshly re-pushed one on the stack every
iteration: an unbounded per-iteration duplicate that hung (an
effectively infinite loop, not a clean trap) instead of terminating,
caught by hand-testing a simple bounded `loop`+`br_if`-break before ever
reaching the testsuite (no vendored `.wast` file with a simple bounded
loop currently parses).

This ALSO surfaced a real, since-fixed bug in `iir-to-wasm`'s own
"dispatch-loop" codegen strategy (used for any IIR function with
control flow — COND/if-chains lower to labels/jumps, compiled to one
outer exit `block` wrapping a `loop` wrapping N nested per-block
`block`s + a `br_table`): its branch-depth formulas for "re-enter the
LOOP to redispatch" were empirically tuned against THIS crate's old
double-pop bug, not real WASM semantics — so fixing `execute_branch`
alone made every such branch land one label too shallow, silently
falling into the wrong basic block (confirmed via `lang-aot`'s
`mccarthy_is_uniform_across_every_backend`: the WASM backend computed
`0` instead of `22` for a 2-clause COND). See `iir-to-wasm`'s own
CHANGELOG for that fix's detail; both fixes are needed together for this
PR to be safe to ship.

4 new regression tests (`tests/wasm11_regression.rs`): the exact
`switch.wast` "stmt" shape reproduced in isolation (all 9 real
assert_return cases), a minimal out-of-depth-order `br_table` case, a
bounded `loop`+`br_if`-break that must terminate (not hang), and two
sequential loops on the same call confirming a loop's own label count
stays stable across iterations. Baseline: `assert_return` 12171/12238
(99.4%) → 12215/12238 (99.8%).

## [0.6.3] — 2026-08-13 (WASM07 — two real assert_return correctness bugs + a security-review fix)

Investigating why `wasm-conformance`'s `assert_return` pass rate sat at
98.3% (208 real failures, not opcode-coverage gaps) surfaced two genuine
bugs in this crate, found by running the official spec testsuite, not by
inspection.

- **A WASM function body is itself an implicit outer `block`, whose label
  is the function's own end — this crate never modeled that.** `br`/
  `br_if`/`br_table` at a depth that walks out of every *explicit* block
  (including a completely ordinary, spec-legal bare top-level `(br 0)`,
  meaning "return" — `func.wast`'s own `break-empty`/`break-i32`/etc.
  cases are exactly this) had no label on `ctx.label_stack` to resolve
  against, and traps with a spurious "branch target N out of range"
  instead of returning. Fixed by pushing an implicit label at call entry
  (`arity` = the function's own result count, `target_pc` one past the
  last instruction) — in **both** independent call-entry code paths this
  crate has (`call_function_inner`, used for nested `call`/
  `call_indirect`, and the separate, duplicated dispatch loop inside the
  public `WasmExecutionEngine::call_function`, the one true top-level
  entry point) since neither reuses the other's instruction-decode-and-
  dispatch logic.
- **`call_indirect $type`'s immediate indexes the module's TYPE SECTION —
  a completely different index space from `ctx.func_types`, which is
  indexed by FUNCTION index** (one entry per function, resolved to
  whichever type that function happens to declare; two functions can
  easily share a type, or a type can go unused by any function at all).
  The type check compared the callee's real type against
  `func_types[type_idx]` — an arbitrary, usually-unrelated function's
  type, not the type the call site actually declared — so legitimate
  `call_indirect` calls across dozens of real testsuite cases
  (`load.wast`/`local_tee.wast`/`nop.wast`/`call.wast`'s many
  `as-call_indirect-*` cases, `func.wast`'s `signature-*-duplicate`
  cases) spuriously trapped "indirect call type mismatch" even though the
  callee's real type matched exactly. Fixed the same way
  `struct_field_counts` already solves an analogous "the parser doesn't
  yet surface this to the engine" gap: a new engine-level
  `type_section: Vec<FuncType>` field (empty by default — deliberately
  **not** added to `WasmEngineConfig`, which would have forced every one
  of this crate's ~40 hand-built single/few-function unit-test modules to
  supply it) with a `set_type_section` setter, threaded into
  `WasmExecutionContext.types`. Left unset, the check is skipped
  (permissive — "no type info available" is not the same claim as "the
  type section is empty"); `wasm-runtime`'s real embedding path always
  sets it now (see that crate's own changelog).
- **A security review of this PR's `wasm-runtime` fix (a trapped call must
  not permanently lose an instance's memory/tables — see that crate's own
  changelog) found the identical bug pattern one layer further in.**
  `WasmExecutionEngine::call_function` (the public, top-level entry point)
  also `mem::take`s `self.host_functions` before running, and its own
  restore line (`self.host_functions = ctx.host_functions;`) used to sit
  AFTER `execute_with_context(...)?` — skipped on any trap, exactly the
  same bug the `wasm-runtime` fix addressed one call frame further out.
  Since `wasm-runtime::instantiate()` wires real WASI imports (`fd_write`,
  `random_get`, `clock_time_get`, `environ_get`, `proc_exit`, ...)
  through this exact field, ANY instance that trapped even once would
  silently and permanently lose every WASI import for the rest of its
  life — the `wasm-runtime` fix alone only restored `instance.memory`/
  `instance.tables` (pointer-aliased into the engine, never moved, so
  they survived a trap fine even with the old code) but not
  `host_functions` (genuinely moved via `mem::take`, one layer further
  in). Fixed the same way: capture the `Result` from
  `execute_with_context` first, restore `self.globals`/
  `self.host_functions`/`self.last_gc_state` unconditionally, THEN
  propagate the trap.
- 5 new regression tests (`tests/wasm07_regression.rs`): a bare top-level
  `br 0` returning correctly through both call-entry paths, a
  `call_indirect` case specifically shaped so the old bug (grabbing an
  unrelated function's type) is unmissable, verified both unset
  (permissive) and set (the real check) against the module's own actual
  parsed type section, a case confirming a genuine type mismatch still
  traps once the real type section is wired in, and a host-imported
  function confirmed to survive an unrelated trapped call and remain
  callable afterward.

Together with the matching `wasm-runtime` 0.5.1 fix (a trapped call
losing an instance's memory/tables forever after — see that crate's own
changelog), these three bugs closed 139 of the 208 real `assert_return`
failures the pre-fix baseline had: 12030/12238 (98.3%) → 12169/12238
(99.4%). The remaining 69 are tracked in the session backlog, not blindly
chased further in this PR: a distinct StackUnderflow bug in named-label
depth resolution through 3+ levels of nested blocks (`switch.wast`,
`labels.wast`'s `if`/`if2`, `func.wast`'s `break-br_table-nested-*`),
`comments.wast`'s CR/CRLF line-comment-terminator handling in the `quote`
re-parse path, a handful of NaN-payload-preservation gaps beyond the
0.6.1 fixes, `call.wast`'s `even`/`odd` (the already-documented, accepted
WASM01 trade-off), and `br.wast`'s one remaining case (a genuine
multi-value block signature, already tracked as WASM04) — each is its
own investigation, not a continuation of these three.

178 tests passing (up from 173: 5 new `wasm07_regression.rs` cases),
clippy clean.

## [0.6.2] — 2026-08-13 (WASM01 — a real call-depth guard)

`call_function` had NO limit on WASM call nesting: `call`/`call_indirect`
recurse through this crate's own Rust call stack one level per nested
WASM call, with no counter anywhere. A WASM program that recurses
without bound — the official spec testsuite's own
`call.wast`/`call_indirect.wast`/`fac.wast` deliberately test exactly
this, expecting a clean "call stack exhausted" trap — used to overflow
the REAL host thread stack: an uncatchable process abort, not a WASM
trap any caller could observe or recover from. `wasm-conformance` had to
route around this entirely (never executing `assert_exhaustion`
directives at all) rather than test it, for exactly this reason.

- **New `MAX_CALL_DEPTH` constant and `WasmExecutionContext::call_depth`
  field.** `call_function` is now a thin wrapper enforcing the limit
  around the previously-unguarded body (renamed `call_function_inner`):
  increments on entry, decrements on exit, traps with `"call stack
  exhausted"` instead of recursing further once the limit is hit.
- **A security review of this PR's first version of the constant (200)
  found its justification was wrong, and reproduced a real crash.** It
  reasoned from a *different* crate's measured overflow floor on a
  *different*, lighter recursive path, rather than measuring THIS
  crate's own (heavier) recursion directly — 200 reliably overflowed the
  real host stack in a **debug build** on any thread stack at or below
  ~1 MiB, reproduced with the PR's own regression test. Corrected by
  directly measuring this crate's own debug-build crash floor at a
  documented minimum assumed caller stack (512 KiB — chosen well under
  Rust's own 2 MiB default spawned-thread stack): safe at 120, crashes at
  130. **`MAX_CALL_DEPTH` is 80**, a ~33% margin below that measured
  floor, matching this repo's other recursive-descent crates' own
  25-45%-margin convention (e.g. `mccarthy-lisp-parser`).
- **Known, deliberate trade-off, not swept under the rug**: 80 is
  genuinely too low for 2 legitimate, bounded (terminating) recursion
  cases in the official testsuite's `call.wast` (`even(100)`/`odd(200)`,
  mutual recursion) — they now correctly-but-unfortunately trap "call
  stack exhausted" instead of completing (before this fix, they only
  "passed" by relying on the previously-unguarded, unsafe recursion
  path). The real fix for both safety AND depth capacity is running WASM
  execution on a dedicated thread with a guaranteed larger stack
  (tracked as its own follow-up; blocked on this crate's `*mut
  LinearMemory`/`*mut Table` raw pointers not being `Send`) — shipping
  the safe, conservative value now rather than leaving the host-crash
  risk in place while that larger change is pending.
- 5 new integration tests (`tests/call_depth_guard.rs`): unbounded
  self-recursion, unbounded mutual recursion, ordinary bounded recursion
  well under the limit still works, a depth-counter-leak regression guard
  (a trapped recursive call must not corrupt `call_depth` for a later,
  unrelated top-level call on the same engine), and a committed
  regression test running the exact overflow scenario on a real 512 KiB
  `Builder::stack_size` thread (no `RUST_MIN_STACK` simulation) so this
  can never silently regress back to an unsafe value.
- `wasm-conformance`'s `assert_exhaustion` directives are now genuinely
  executed and graded (previously always `NotYetSupported`, unconditionally,
  specifically because of this gap) — both vendored cases now pass for
  real. See that crate's own changelog.
- **A second security-review round** found the 512 KiB minimum-stack
  assumption behind `MAX_CALL_DEPTH` lived only in a doc comment on a
  *private* `const` — invisible to any downstream consumer reading just
  the public API. Surfaced it directly on the public
  `WasmExecutionEngine::call_function`'s own doc comment instead. Making
  `MAX_CALL_DEPTH` itself caller-configurable (for embedders who know
  they're running on a smaller stack) was judged out of scope for this
  PR — folded into the WASM10 follow-up alongside the dedicated-thread
  work, rather than widening this PR's surface further.

173 tests passing (up from 168), clippy clean. Downstream consumers
(`lang-aot` and the WASM-compiler crates) re-checked: no new failures
attributable to this change.

## [0.6.1] — 2026-08-13 (W05 PR-4 — three float NaN/sign correctness bugs)

Found running the official WebAssembly spec testsuite against this crate
for the first time, via the new `wasm-conformance` harness — these are
exactly the kind of gap that harness exists to surface. All three follow
the same shape: `f32`/`f64` opcode handlers used a Rust `std` float method
directly, whose semantics turned out not to match what the WASM spec
actually requires for that opcode.

- **`f32.min`/`f32.max`/`f64.min`/`f64.max` didn't propagate NaN.** WASM's
  `min`/`max` MUST return NaN if EITHER operand is NaN. Rust's native
  `f32::min`/`max` follow IEEE 754-2008 minNum/maxNum semantics instead:
  "if one of the arguments is NaN, then the OTHER argument is returned."
  `min(NaN, -0.0)` was silently returning `-0.0`. This was, by a wide
  margin, the single largest source of `assert_return` failures in the
  vendored corpus — fixing it alone moved the aggregate `assert_return`
  pass rate from 94.1% to 98.3%. Fixed with explicit NaN and signed-zero
  handling (`min(+0.0, -0.0)` must be `-0.0`; `max` the reverse).
- **`f32.nearest`/`f64.nearest` (round-ties-to-even) didn't preserve the
  sign of a result that rounds to zero.** `nearest(-0.25)` must be `-0.0`,
  not `0.0`, per IEEE 754's own roundTiesToEven rule. Fixed with an
  explicit `copysign` fixup when the rounded result is exactly zero.
- **`f32.ceil`/`floor`/`trunc`/`f64.ceil`/`floor`/`trunc` didn't
  reliably quiet a signaling NaN input.** WASM's spec requires any NaN
  propagated through these to have its quiet bit set, unconditionally.
  The platform libm's own behavior for a signaling-NaN input to
  `ceil`/`floor`/`trunc` turned out to genuinely differ between macOS and
  Linux — confirmed empirically by running the exact same conformance
  suite against both (via a Linux container) and diffing the results bit
  for bit. Fixed by explicitly checking `is_nan()` and returning the
  canonical `f32::NAN`/`f64::NAN` instead of relying on the platform's
  native rounding-function NaN handling at all.

9 new regression tests (2 min/max NaN-propagation, 2 min/max signed-zero,
2 nearest sign-of-zero, 2 ceil/floor/trunc signaling-NaN-quieting, plus
an f64 min/max pair the crate didn't have coverage for at all). 168 tests
passing (up from 159), clippy clean, verified against both macOS and a
Linux container (the platform that originally surfaced bug #3).

## [0.6.0] — 2026-08-03 (W04 — real garbage collection for `gc_heap`)

The WasmGC struct heap (`gc_heap`) was, until now, an append-only arena with
no reclamation — its own doc comment justified this as "bounded by the VM's
instruction budget," a budget that does not actually exist anywhere in this
crate. A long-running WASM-compiled program could allocate without bound.

- **New `gc` module**: a real mark-and-sweep collector over a **tombstone +
  free-list slot arena** — `gc_heap: Vec<GcStruct>` becomes
  `Vec<Option<GcStruct>>` (`Some` = live, `None` = a reclaimed slot ready
  for reuse). Compaction is out of scope: a `WasmValue::Ref(Some(handle))`
  is a `Vec` index (a WASM-spec-mandated representation), so shrinking the
  arena would silently invalidate every other live handle past the removed
  index. No generation tag is needed to guard a reused slot against
  aliasing a stale reference — mark-sweep's ordinary soundness argument
  (nothing left unmarked is reachable, given an exhaustive root walk)
  already rules that out, since every handle a program can hold either sat
  in a scanned root or was freshly minted.
- **Root set**: every `WasmValue::Ref(Some(_))` in `ctx.globals`,
  `ctx.typed_locals`, **every** `ctx.saved_frames[*].locals` (a paused
  caller's locals — missing this would free something a suspended caller
  still references), and the interpreter's operand stack
  (`GenericVM::typed_stack`, via the existing `REF_TAG` convention),
  traced transitively through each object's own fields — cycle-safe by
  construction (a worklist walk with a `marked` visited set, not naive
  recursion). Precise, with no `HeapKind`-style schema needed: a
  `GcStruct` field is a tagged `WasmValue`, self-describing as a reference
  or not.
- **Checked at two chokepoints**: `execute_branch` (every taken
  `br`/`br_if`/`br_table` — a loop's back-edge is a branch to its own loop
  label) and the internal `call_function` helper (every `call`/
  `call_indirect`, nested or not) — both of this crate's independent
  dispatch loops (`GenericVM::execute_with_context` and the hand-inlined
  loop inside `call_function`) route through these two shared functions, so
  instrumenting them covers both loops without adding anything WASM-specific
  to the generic `virtual-machine` crate's own dispatch code. Mirrors the
  existing "safepoints at back-edges and calls" convention rather than a
  per-instruction counter.
- **Adaptive object-count threshold**, mirroring `gc-core::FlatHeap::should_collect`/
  `adapt_threshold` verbatim (just in units of live objects rather than
  bytes, since this heap has no byte-size concept). New `gc-core` path
  dependency, reusing its representation-agnostic `GcProfile`/`GcCycleStats`
  for diagnostic consistency with the native-AOT and `vm-core` GC paths —
  exposed via new `WasmExecutionEngine::gc_live_object_count()` /
  `gc_profile()` accessors (`gc_heap` itself still resets every call, as it
  always has; only the counters are written back).
- Tests: `gc`'s own unit tests (`mark`/`sweep`/`alloc`/threshold-adaptation
  in isolation, including cycle-safety and a saved-caller-frame root) plus
  `end_to_end_loop_reclaims_garbage_and_preserves_kept_object` — a real WASM
  loop, driven through actual bytecode dispatch, that allocates 2000
  objects while keeping exactly one alive, proving both that the kept
  object survives with its field intact and that the live count is
  reclaimed mid-run rather than left to accumulate.
- **Security-review fixes, before this landed**:
  - `sweep` now also drops `gc_heap`'s trailing run of tombstoned slots
    (removing their indices from `free_list` too). Without this, the
    arena's length — and therefore `mark`/`sweep`'s O(len) cost — was a
    monotonically non-decreasing high-water mark for the life of a call: a
    program that transiently spiked the live count high, then settled into
    low-retention churn, would keep paying that peak cost on every
    subsequent collection. Not compaction (no live object moves or is
    renumbered) — only provably-all-garbage trailing capacity is dropped.
  - `gc::alloc`'s new-handle assignment now uses a checked `u32` conversion
    (clean trap on failure) instead of an unchecked `as u32` cast, so an
    eventual overflow (practically unreachable — it implies 100+GB of
    live, uncollected heap) can't silently alias a fresh object's handle
    onto an already-occupied lower index.
  - Tests: `sweep_shrinks_arena_when_everything_is_reclaimed`,
    `sweep_does_not_truncate_past_a_live_object_in_the_middle`.
- See `code/specs/W04-wasm-gc.md` for the full design rationale.

## [0.5.0] — 2026-07-07

### Added — `memory.copy` bulk-memory op (E4-dyn runtime string concat)

The engine now runs the `memory.copy` bulk-memory instruction, which
`iir-to-wasm` emits for runtime `str_concat` (splicing two operands' bytes into a
freshly allocated `[i32 len][bytes]` block).

- **Decoder**: a new `0xFC` two-byte-prefix branch (mirroring the `0xFB` GC prefix)
  reads the sub-opcode and its memory-index immediates, carrying the sub-opcode in a
  plain `Int` operand. `memory.copy` (`0x0A`, two index bytes) and `memory.fill`
  (`0x0B`, one index byte) are decoded; only `memory.copy` is executed.
- **Execution**: a `0xFC` handler pops `size`, `src`, `dest` and delegates to the new
  `LinearMemory::copy`, which bounds-checks both ranges (either out of range traps —
  never panics) and uses `copy_within` for overlap-safe (memmove) semantics. A
  zero-length copy is a no-op even at the end of memory.
- Tests: `memory_copy_decodes_as_one_0xfc_instruction`,
  `linear_memory_copy_moves_bytes_overlap_safe`.

## [0.4.0] — 2026-06-08

### Added — `ref.test` execution (LANG77 / McCarthy L3b-3a-4)

The engine now runs the WasmGC **`ref.test`** type-test op — the primitive
McCarthy `pair?` lowers to ("is this lisp value a cons cell?"):

- The decoder reads the heap-type immediate of `ref.test` (`0xFB 0x14`) and the
  nullable `ref.test null` (`0xFB 0x15`).
- The engine executes them: pop a reference, push `i32 1` if it is a (non-null)
  struct reference, else `0`. Our value model has exactly one struct type
  (`$LispyPair`), so a `struct.new` result — the only `Ref(Some(_))` — is that
  type; a boxed integer (`i31` payload / `I32`) or the null reference yields `0`.
  The `0x15` variant additionally accepts the null reference.

So `pair?(cons)` → 1, `pair?(atom)` → 0, `pair?(nil)` → 0. 3 new tests.

## [0.3.0] — 2026-06-04

### Added — WasmGC struct heap + references (LANG77 / McCarthy L3b-3a-3b)

The engine now *runs* the WasmGC object opcodes, so a lisp **cons cell**
(`$LispyPair`) can be allocated, read, and mutated in-repo — `(CAR (CONS 7 9))`
executes to `7` on the engine.

- **`WasmValue::Ref(Option<u32>)`** — a new value variant for GC references:
  `None` is the null reference (`ref.null` / lisp `nil`), `Some(handle)` indexes
  the engine's GC object heap. Round-trips through the typed stack tagged as
  `anyref` (`0x6E`). (An `i31ref` stays an `I32` payload, as in 0.2.0.)
- **GC object heap** — an append-only arena of `GcStruct { type_idx, fields }`
  on the execution context. `struct.new` allocates and returns a handle; the
  heap persists across calls within a run (a cons built in a callee survives).
  No reclamation: total allocations are bounded by the VM instruction budget.
- **Opcodes executed:**
  - `struct.new <type>` (`0xFB 0x00`) — pops the registered field count of
    values, allocates a `GcStruct`, pushes `Ref(Some(handle))`.
  - `struct.get <type> <field>` (`0xFB 0x02`) — reads a field of a non-null ref.
  - `struct.set <type> <field>` (`0xFB 0x04`) — writes a field of a non-null ref.
  - `ref.null` (`0xD0 0x0F`) — pushes the null reference.
  - `ref.is_null` (`0xD1`) — pops an anyref, pushes `1`/`0`.
- **Decoder** — the `0xFB` block now reads the struct ops' index immediates
  (type/field) into a `Gc { sub, type_idx, field_idx }` operand; `0xD0` consumes
  its one-byte heap-type immediate so it isn't mis-decoded.
- **`WasmExecutionEngine::set_struct_field_counts`** — registers struct type
  field counts (the parser doesn't yet surface struct types to the engine; this
  is populated from the parsed module in L3b-3a-3c).
- **Defaults** — nullable reference locals (`anyref`, `structref`) now default
  to `Ref(None)`; `i31ref` stays `I32(0)`.

Every failure mode (unknown type/field index, null dereference, missing arity,
type mismatch, unknown sub-opcode) is a **clean trap**, never a panic. 10 new
tests (cons/car/cdr round-trips, `struct.set` mutation, `ref.is_null`, and the
null-deref / out-of-range / missing-arity traps).

Also removed a pre-existing dead no-op branch in `decode_immediates`.

## [0.2.0] — 2026-06-05

### Added — WasmGC `i31` execution (LANG77 / McCarthy L3b-3a-3a)

First step of executing WebAssembly **GC** opcodes (so the McCarthy-Lisp → wasm
value model can be run in-repo, not just emitted):

- **`decode_function_body`** now decodes the two-byte `0xFB` GC prefix: it reads
  the sub-opcode byte and carries it in the instruction's operand (the MVP
  opcode table is single-byte and doesn't know `0xFB`). Previously a `0xFB`
  stream would be mis-decoded as separate single-byte instructions.
- The engine executes the `i31` boxing pair: **`i31.new`** (`0xFB 0x1C`) and
  **`i31.get_s`** (`0xFB 0x1D`). An `i31ref` is represented as its plain `i32`
  payload on the value stack (the small lisp integers we box never need the
  reference identity), so both are stack-identity no-ops — the integer passes
  straight through. `i32.const 42 → i31.new → i31.get_s` returns `42`.
- Unimplemented GC sub-opcodes (`struct.*`, `ref.*`) are a clean `Err`, not a
  panic — they land with the GC-object-heap slice (L3b-3a-3b).

3 new tests: the two-byte decode, the i31 box/unbox round-trip executed on the
engine, and the unsupported-opcode clean error.

## [0.1.1] — 2026-05-13

### Fixed

- **`br_table` handler** — corrected out-of-range index handling: when the
  branch index exceeds the target table length, the default label is now
  used (as per the WASM spec).  Previously the handler indexed past the
  end of the table, causing a panic.
- **`call_indirect` stub** — added a graceful error message for
  `call_indirect` (not yet supported) so the engine reports the opcode name
  instead of panicking on an unknown byte.
- **`i32.const` sign extension** — `i32.const` operands are now
  LEB128-decoded as signed (i32) and zero-extended to i64, matching the WASM
  spec for negative literal values.
- Various opcode stubs improved to match the WASM binary format byte layout.

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
