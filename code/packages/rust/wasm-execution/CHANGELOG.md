# Changelog

All notable changes to this package will be documented in this file.

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
