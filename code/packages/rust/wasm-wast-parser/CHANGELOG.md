# Changelog — wasm-wast-parser

## 0.1.38 — 2026-08-18 — SIMD: shift family text-form (task #159-161)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `ixNxM.shl`/`shr_s`/`shr_u`
  across all 4 lane widths -- same "no immediate beyond the opcode
  byte itself" shape every prior SIMD op has, so no new parsing logic;
  the mixed `v128`+`i32` operand TYPES are invisible to this encoder
  (a type-checker concern -- see `wasm-validator`), since operand
  count/values are driven entirely by the S-expression recursion that
  already ran. Verified via a dedicated test asserting the real
  single-byte LEB128 bytes for `i8x16`'s triple (`[0xFD, 0x6B]`
  through `[0xFD, 0x6D]`, all < 128) and the 2-byte LEB128 bytes for
  `i16x8`/`i32x4`/`i64x2`'s own triples (`[0xFD, 0x8B, 0x01]` through
  `[0xFD, 0xCD, 0x01]`, all >= 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.37 — 2026-08-18 — SIMD: i64x2 arith+cmp family text-form (task #156-158)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover i64x2's first REAL ARITHMETIC
  family: `i64x2.abs`/`neg`/`add`/`sub`/`mul`/`eq`/`ne`/`lt_s`/`gt_s`/
  `le_s`/`ge_s` -- same "no immediate beyond the opcode byte itself"
  shape every prior SIMD op in this family has, so no new parsing
  logic. Verified via a dedicated test asserting the real 2-byte
  LEB128-encoded sub-opcode bytes (`[0xFD, 0xC0, 0x01]` through
  `[0xFD, 0xDB, 0x01]` -- all >= 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.36 — 2026-08-18 — SIMD: boolean-reduction/bitmask family text-form (task #153-155)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `v128.any_true` +
  `ixNxM.all_true`/`bitmask` across all 4 lane widths -- same "no
  immediate beyond the opcode byte itself" shape every prior SIMD op
  has, so no new parsing logic. These produce an `i32` result instead
  of a `v128`, but that's invisible to this encoder (a type-checker
  concern -- see `wasm-validator`). Verified via a dedicated test
  asserting the real single-byte LEB128 bytes for `v128.any_true`
  (`[0xFD, 0x53]`) and `i8x16.all_true`/`bitmask` (`[0xFD, 0x63]`/
  `[0xFD, 0x64]`, both < 128), and the 2-byte LEB128 bytes for
  `i16x8`/`i32x4`/`i64x2`'s own `all_true`/`bitmask` pairs (`[0xFD,
  0x83, 0x01]` through `[0xFD, 0xC4, 0x01]`, all >= 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.35 — 2026-08-18 — SIMD: v128 bitwise family text-form (task #150-152)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover the lane-width-agnostic raw-byte
  bitwise family: `v128.not`/`and`/`andnot`/`or`/`xor`/`bitselect` --
  same "no immediate beyond the opcode byte itself" shape every prior
  SIMD op in this family has, so no new parsing logic, just a wider
  match-arm pattern list. `bitselect`'s new ternary (3-operand) shape
  needs zero special-casing here -- operand count is driven entirely
  by whatever S-expression recursion already ran before the encoder
  emits `[0xFD, LEB128(sub_opcode)]`. Verified via a dedicated test
  asserting the real single-byte LEB128-encoded sub-opcode bytes
  (`[0xFD, 0x4D]` through `[0xFD, 0x52]` -- all < 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.34 — 2026-08-18 — SIMD: i16x8-from-i8x16 widening text-form (task #147-149)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `i16x8`'s own widening family:
  `i16x8.extadd_pairwise_i8x16_s`/`_u`/`extmul_low`/
  `high_i8x16_s`/`_u` -- same "no immediate beyond the opcode byte
  itself" shape every prior SIMD op in this family has, so no new
  parsing logic, just a wider match-arm pattern list. Verified via a
  dedicated test asserting the real single-byte LEB128-encoded
  sub-opcode bytes for `extadd_pairwise_i8x16_s`/`_u` (`[0xFD, 0x7C]`,
  `[0xFD, 0x7D]` -- both < 128) and the 2-byte LEB128 bytes for
  `extmul_low`/`high_i8x16_s`/`_u` (`[0xFD, 0x9C, 0x01]` through
  `[0xFD, 0x9F, 0x01]` -- all >= 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.33 — 2026-08-18 — SIMD: i16x8 abs/min/max/avgr_u text-form (task #144-146)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `i16x8`'s own "arith2" family:
  `i16x8.abs`/`min_s`/`min_u`/`max_s`/`max_u`/`avgr_u` -- same "no
  immediate beyond the opcode byte itself" shape every prior SIMD op
  in this family has, so no new parsing logic, just a wider match-arm
  pattern list. Verified via a dedicated test asserting the real
  2-byte LEB128-encoded sub-opcode bytes (`[0xFD, 0x80, 0x01]`,
  `[0xFD, 0x96, 0x01]` through `[0xFD, 0x99, 0x01]`, `[0xFD, 0x9B,
  0x01]` -- all six are >= 128, unlike `i8x16`'s own arith2 family,
  all < 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.32 — 2026-08-18 — SIMD: i8x16 abs/popcnt/min/max/avgr_u text-form (task #141-143)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `i8x16`'s own "arith2" family:
  `i8x16.abs`/`popcnt`/`min_s`/`min_u`/`max_s`/`max_u`/`avgr_u` -- same
  "no immediate beyond the opcode byte itself" shape every prior SIMD
  op in this family has, so no new parsing logic, just a wider
  match-arm pattern list. Verified via a dedicated test asserting the
  real single-byte LEB128-encoded sub-opcode bytes (`[0xFD, 0x60]`,
  `[0xFD, 0x62]`, `[0xFD, 0x76]` through `[0xFD, 0x79]`, `[0xFD,
  0x7B]` -- all seven are < 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.31 — 2026-08-18 — SIMD: i8x16 comparison family text-form (task #137-140)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `i8x16`'s own comparison family:
  `i8x16.eq`/`ne`/`lt_s`/`lt_u`/`gt_s`/`gt_u`/`le_s`/`le_u`/`ge_s`/
  `ge_u` -- same "no immediate beyond the opcode byte itself" shape
  every prior SIMD op in this family has, so no new parsing logic, just
  a wider match-arm pattern list. Verified via a dedicated test
  asserting the real single-byte LEB128-encoded sub-opcode bytes
  (`[0xFD, 0x23]` through `[0xFD, 0x2C]` -- all ten are < 128, same
  single-byte shape as `i16x8`'s own comparison family).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.30 — 2026-08-18 — SIMD: i16x8 comparison family text-form (task #133-136)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `i16x8`'s own comparison family:
  `i16x8.eq`/`ne`/`lt_s`/`lt_u`/`gt_s`/`gt_u`/`le_s`/`le_u`/`ge_s`/
  `ge_u` -- same "no immediate beyond the opcode byte itself" shape
  every prior SIMD op in this family has, so no new parsing logic, just
  a wider match-arm pattern list. Verified via a dedicated test
  asserting the real single-byte LEB128-encoded sub-opcode bytes
  (`[0xFD, 0x2D]` through `[0xFD, 0x36]` -- all ten are < 128, unlike
  `i16x8.add`/`sub`/`mul`/`neg`'s own sub-opcodes, all >= 128).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.29 — 2026-08-18 — SIMD: i16x8 first primary-lane slice text-form (task #129-132)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover this crate's first opcodes where
  `i16x8` is a PRIMARY lane width: `i16x8.add`/`sub`/`mul`/`neg` -- same
  "no immediate beyond the opcode byte itself" shape every prior SIMD op
  in this family has, so no new parsing logic, just a wider match-arm
  pattern list. Verified via a dedicated test asserting the real
  2-byte LEB128-encoded sub-opcode bytes (`[0xFD, 0x8E, 0x01]`/
  `[0xFD, 0x91, 0x01]`/`[0xFD, 0x95, 0x01]`/`[0xFD, 0x81, 0x01]` for
  `add`/`sub`/`mul`/`neg` -- all four are >= 128, unlike `i8x16.add`/
  `sub`/`neg`'s own sub-opcodes, all single-byte).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.28 — 2026-08-18 — SIMD: i8x16 first slice text-form (task #125-128)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover this crate's first `i8x16`-lane-
  width ops: `i8x16.add`/`sub`/`neg` -- same "no immediate beyond the
  opcode byte itself" shape every prior SIMD op in this family has, so
  no new parsing logic, just a wider match-arm pattern list. Verified
  via a dedicated test asserting the real single-byte LEB128-encoded
  sub-opcode bytes (`[0xFD, 0x6E]`/`[0xFD, 0x71]`/`[0xFD, 0x61]` for
  `add`/`sub`/`neg` -- all three are < 128, unlike `i32x4.add`'s own
  `0xAE`, which needs a continuation byte).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.27 — 2026-08-18 — SIMD widening: i32x4-from-i16x8 family text-form (task #121-124)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened further to cover
  `i32x4.extadd_pairwise_i16x8_s`/`_u`, `i32x4.dot_i16x8_s`, and
  `i32x4.extmul_low`/`high_i16x8_s`/`_u` -- same "no immediate beyond the
  opcode byte itself" shape every prior SIMD op in this family has, so no
  new parsing logic, just a wider match-arm pattern list. Verified via a
  dedicated test asserting the real LEB128-encoded sub-opcode bytes:
  `[0xFD, 0x7E]`/`[0xFD, 0x7F]` for `extadd_pairwise_i16x8_s`/`_u` (both
  sub-opcodes are < 128, so SINGLE-byte LEB128 -- unlike every prior
  entry this test file covers, no continuation byte needed), and
  `[0xFD, 0xBA, 0x01]`/`0xBC`/`0xBD`/`0xBE`/`0xBF` for `dot_i16x8_s`/
  `extmul_low_i16x8_s`/`extmul_high_i16x8_s`/`extmul_low_i16x8_u`/
  `extmul_high_i16x8_u` (all >= 128, same 2-byte LEB128 shape as
  `min_s`/`max_u` above).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.26 — 2026-08-18 — SIMD widening: i32x4 abs/min/max family text-form (task #118-120)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened further to cover `i32x4.abs`/`min_s`/
  `min_u`/`max_s`/`max_u` -- same "no immediate beyond the opcode byte
  itself" shape every prior SIMD op in this family has, so no new
  parsing logic, just a wider match-arm pattern list. Verified via a
  dedicated test asserting the real LEB128-encoded sub-opcode bytes
  (`[0xFD, 0xA0, 0x01]` for `abs`, `[0xFD, 0xB6, 0x01]`/`0xB7`/`0xB8`/
  `0xB9` for `min_s`/`min_u`/`max_s`/`max_u`) -- all five sub-opcodes are
  ≥128 so each needs the LEB128 continuation byte, unlike every prior
  SIMD sub-opcode this crate has encoded so far.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.25 — 2026-08-18 — SIMD widening: i32x4 arithmetic + comparison family text-form (task #113-117)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms widened to cover `i32x4.sub`/`mul`/`neg` and the
  full comparison family (`ne`/`lt_s`/`lt_u`/`gt_s`/`gt_u`/`le_s`/`le_u`/
  `ge_s`/`ge_u`) -- same "no immediate beyond the opcode byte itself"
  shape `splat`/`add`/`eq` already had, so no new parsing logic, just a
  wider match-arm pattern list. `i32x4.neg` takes exactly one folded
  operand (unlike every binary op in this family), verified by a
  dedicated test.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.24 — 2026-08-17 — real multi-memory memarg leading token (task #92/#110)

### Added

- `i32.load`/`i64.load`/`f32.load`/`f64.load`/every narrow load and
  store variant/`i32.store`/`i64.store`/`f32.store`/`f64.store` now
  accept an optional LEADING memory-index token before their
  `offset=`/`align=` attributes -- `(i32.load $mem1 (i32.const 1))` --
  distinguished from the address operand (always a nested, parenthesized
  instruction in folded form, so always an `SExpr::List`) and from an
  `offset=`/`align=` attribute (a bare `Atom` with that literal prefix)
  by shape. Pulled into its own `resolve_leading_memidx_token` function
  rather than inlined into the match arm -- see the "Fixed" note below.
- `memory.fill` accepts the same optional leading memidx token --
  `(memory.fill $mem1 (dest) (value) (len))`.
- `memory.init` accepts an optional leading memidx token BEFORE its
  (always-required) data-segment index -- `(memory.init $mem1 $d (dest)
  (src) (len))`. Unlike `i32.load`'s token, this one isn't distinguishable
  from the data-segment index by shape (both are bare identifier/index
  atoms) -- disambiguated instead by COUNT of leading atoms before the
  first parenthesized operand: one atom means "just the required
  dataidx" (memidx defaults to 0, the pre-existing single-memory form
  from task #95); two means "memidx then dataidx".

### Fixed

- **Real regression, caught before merge**: adding the leading-memidx-
  token locals directly inline inside `encode_flat_instr`'s match arm
  measurably grew that function's own per-call stack frame (a debug
  build sizes a function's frame for the union of every match arm's
  locals, not just the arm actually taken) enough to overflow the REAL
  OS stack before `MAX_INSTR_NESTING_DEPTH`'s software depth counter
  ever tripped -- `deeply_nested_folded_arithmetic_errors_cleanly_not_
  stack_overflow` (which never even touches `i32.load`) started
  aborting with a genuine SIGABRT. `MAX_INSTR_NESTING_DEPTH` lowered
  from 100 to 60 and reverified with repeated (8x+) full-suite runs
  showing zero flakiness. `encode_flat_instr`'s own per-arm complexity
  (this arm and any future one) should keep this margin in mind --
  see the constant's own doc comment for the reasoning and the re-
  verification procedure this session used.

## 0.1.23 — 2026-08-17 — hex-literal table/memory limits (task #99)

### Fixed

- Table and memory limits (`(table 0xffff_ffff funcref)`, `(memory 0x1
  0x2)`) only ever accepted decimal literals -- three independent call
  sites (`parse_limits` itself, `build_table_limits_and_elements`'s
  declared-table path, `build_import_shell`'s imported-table path) each
  filtered atoms to ascii-digit-only. For `parse_limits`, a hex atom was
  silently dropped out of the limits list entirely (wrong `Limits`, not
  even a clean error). For the other two, a hex atom made the shape check
  fail entirely, sending parsing down the WRONG branch (mistaking real
  limits for a missing-limits form) and producing a confusing
  `UnexpectedEof` instead of a real number.
- New `numeric::parse_u32` (hex/decimal, `_`-separated, reusing the same
  `parse_int_magnitude` machinery `parse_i32`/`parse_i8`/etc. already
  share) plus a `looks_like_uint_literal` shape check used at all three
  call sites, so every WAT numeric-literal parser in this crate now
  agrees on what's a valid integer literal.
- Discovered while vendoring `table.wast` (task #99, previously logged
  as blocked on exactly this gap): the real testsuite's own `table.wast`
  uses `0xffff_ffff` table/memory limits directly.

## 0.1.22 — 2026-08-17 — call_indirect/return_call_indirect explicit table-index text syntax (task #107)

### Fixed

- Both `call_indirect`/`return_call_indirect` encoder arms (flat
  `encode_stream_instr` and folded `encode_flat_instr`) hardcoded
  `out.push(0x00)` for the table-index immediate, never reading a
  leading table-reference token from the WAT source at all -- the real
  reason every `call_indirect` ran against table 0 regardless of what a
  real module's `(call_indirect $t0 (type $t) ...)` text actually named
  (see `wasm-execution`'s own CHANGELOG entry for the runtime side of
  this bug). Both arms now detect an optional leading table token (a
  bare atom before the `(type ...)`/`(param...)`/`(result...)` fields,
  same detection shape as `table.init`/`table.copy`'s own precedent)
  and resolve it via the existing `resolve_idx(&icx.module.table_names,
  ...)` helper, defaulting to table 0 when absent -- fully backward
  compatible with every existing module that never names one. No new
  parsing infrastructure needed: the S-expr tokenizer already isn't
  opcode-specific, and this exact `resolve_idx`/`table_names` pattern
  is already how `table.grow`/`table.init`/`table.copy` resolve their
  own table references.
- Discovered while vendoring `table_init.wast`/`table_copy.wast` (task
  #97): most modules in both files use this syntax pervasively (6 uses
  in `table_init.wast`, 36 in `table_copy.wast`), and every module using
  it failed to build entirely under the old hardcoded encoder, cascading
  to `NotYetSupported` for that module's whole directive stream. Fixing
  this dropped the aggregate `assert_return`/`assert_trap` `NotYetSupported`
  counts from 983/1789 to 562/938 with zero regressions elsewhere (see
  `wasm-conformance`'s own CHANGELOG entry for this version).

## 0.1.21 — 2026-08-17 — table.init/table.copy/elem.drop + passive/exprs-list elem parsing (task #97)

### Fixed

- `/security-review` finding: `build_elem` used to accept an ACTIVE
  element segment using the exprs-list form (binary modes 4/6 -- e.g.
  `(elem (table $t) (i32.const 0) funcref (ref.func 0) (ref.null
  func))`), silently allowing a `None` (`ref.null`) entry into a
  segment marked `is_passive: false`. `wasm-module-encoder::
  encode_element`'s active-segment branch does `func_index.unwrap_or
  (0)`, which would turn that `ref.null` into a real reference to
  function 0 on re-encode -- wrong table contents with no error.
  `parse_element_section` (the binary decoder) already structurally
  can't produce this shape (only mode flags 0/1/2/5 are handled); the
  text parser now enforces the same restriction, rejecting the
  combination at parse time instead.

### Added

- `ModuleCtx.elem_names: HashMap<String, u32>` -- an element segment's
  own `$id` -> its index in `module.elements`, mirroring `data_names`
  exactly, registered by a new `collect_symbols` pre-pass branch.
- `build_elem` rewritten to detect passive segments (a leading `func`/
  `funcref`/`externref` keyword with no `(table ...)` clause) and parse
  BOTH funcidx-list (`(elem $e func $a $b)`) and exprs-list (`(elem $e
  funcref (ref.func $a) (ref.null func))`) forms, via a new
  `resolve_elem_expr_entry` helper.
- `table.init`/`table.copy`/`elem.drop` text-form parsing, both flat
  and folded instruction syntax -- entirely unimplemented before this
  pass. `table.init`'s TEXT operand order (`$table $elem`) is the
  OPPOSITE of its BINARY encoding order (`elemidx` first, `tableidx`
  second) -- confirmed against the real testsuite corpus, not guessed;
  `table.copy`'s text and binary orders match (dst-then-src both
  times). The flat/stream form defers all table/elem indices to 0,
  same documented scope boundary this crate's other bulk-op flat forms
  already use (no vendored corpus file exercises non-default indices
  through flat syntax).

### Fixed

- The three new folded-form encoders (`encode_table_init_flat`/
  `_table_copy_flat`/`_elem_drop_flat`) were proactively extracted into
  their own `#[inline(never)]` functions rather than inlined directly
  into `encode_flat_instr`, applying task #98's own fix up front
  instead of waiting to hit the identical debug-build stack-frame-size
  regression again (`deeply_nested_folded_arithmetic_errors_cleanly_
  not_stack_overflow` stayed green throughout).

## 0.1.20 — 2026-08-16 — table.grow/table.size/table.fill text-form parsing (task #98)

### Added

- `table.grow`/`table.size`/`table.fill` text-form parsing, both flat
  and folded instruction syntax -- entirely unimplemented before this
  pass. The folded form resolves a REAL, optional leading `$t` table
  index (defaulting to table 0), same disambiguation `table.get`/
  `table.set` (task #96) already established; the flat/stream form
  defers to table 0 always, same documented scope boundary `"memory.
  size" | "memory.grow"`'s own flat-form arm uses (no vendored corpus
  file exercises a non-default table index through flat syntax).

### Fixed

- The three new folded-form encoders were extracted into their own
  `#[inline(never)]` functions (`encode_table_grow_flat`/`_size_flat`/
  `_fill_flat`) rather than inlined directly into `encode_flat_instr`.
  `encode_flat_instr` is the recursive descent's own funnel
  (`encode_flat_instr` -> `encode_instr_list` -> `encode_one` -> ...),
  and a debug-build stack frame sizes to the union of ALL of a
  function's locals across every branch, not just the one actually
  taken -- inlining the 3 new arms there directly measurably pushed
  ordinary deeply-nested `i32.add` recursion (which never touches this
  code at all) past the real stack limit before `MAX_INSTR_NESTING_
  DEPTH`'s own guard could fire, breaking `deeply_nested_folded_
  arithmetic_errors_cleanly_not_stack_overflow`. Confirmed by direct
  before/after observation: the inlined version crashed that test with
  a real SIGABRT stack overflow; the extracted version passes it (and
  the full suite) consistently across repeated runs.

## 0.1.19 — 2026-08-16 — memory.init/data.drop + passive data segments (task #95)

### Added

- `memory.init`/`data.drop` text-form parsing (both flat and folded
  instruction syntax) -- entirely unimplemented before this pass. Unlike
  `memory.copy`/`memory.fill`'s discarded memory-index bytes, these take
  a REAL data-segment index immediate, resolved through a new
  `ModuleCtx::data_names: HashMap<String, u32>` (mirroring `memory_names`/
  `table_names`/etc.), populated by a new pre-pass in `collect_symbols`
  since data segments previously had no name/index-space tracking at all
  (nothing referenced them by index before this task).
- Passive data segment parsing: `(data $d "bytes")`, with no offset
  expression at all -- `build_data` previously required an offset
  expression unconditionally, so `(data)`/`(data $d "bytes")` with no
  memory/offset clause was always a parse error. Disambiguates active
  vs. passive by checking for a leading `(memory ...)` clause (always
  active) or whether the next field is a string literal / nothing at all
  (passive) vs. an offset expression (active).

### Changed (breaking)

- `(data)` alone -- no `$id`, no `(memory ...)`, no offset expression, no
  string literal -- used to be a parse error (`UnexpectedEof`); it is now
  a valid, empty PASSIVE segment, matching the real WAT grammar
  (`datasegment ::= '(' 'data' id? datastring* ')'` allows zero
  `datastring`s). `(elem)` is unaffected -- `elem` has no passive-segment
  support yet (task #97).

## 0.1.18 — 2026-08-16 — memory.copy/memory.fill text-form parsing (task #94)

### Added

- `memory.copy`/`memory.fill` (bulk-memory proposal) text-form parsing,
  in both flat and folded instruction syntax -- neither had ANY support
  before this (both are 0xFC-prefixed, so -- like trunc_sat/atomics/SIMD
  before them -- they need their own interception in `encode_stream_
  instr`/`encode_flat_instr` before the `get_opcode_by_name` lookup,
  which only this pass added). `memory.copy` already had a real
  `wasm-execution` interpreter handler (E4-dyn's runtime string concat
  needed it) but no way to reach it from `.wast` source text; `memory.
  fill`'s interpreter handler is new this same task (see `wasm-execution`'s
  own CHANGELOG). No vendored corpus file uses an explicit non-default
  memory index for either instruction (bulk-memory predates multi-
  memory), so -- same deferral as `memory.size`/`memory.grow`'s existing
  scope note -- both always emit index byte(s) `0x00`, not a real lookup.

## 0.1.17 — 2026-08-16 — read a table's real funcref/externref reftype; table.get/table.set default index (task #96)

### Fixed

- Both declared-table and imported-table parsing previously discarded a
  table's declared reftype keyword (`funcref`/`externref`) entirely --
  `parse_limits`'s digit-only scan correctly stopped before it, but
  nothing ever read it afterward, leaving every table's `element_type`
  silently hardcoded to `FUNCREF` regardless of the source. An imported
  table's real LIMITS were also discarded, in favor of an unconditional
  `{min: 0, max: None}` placeholder that (unlike function imports) was
  never fixed up in a documented "pass 2". Found while vendoring
  `table_get.wast`, a real corpus file mixing funcref and externref
  tables in one module.
- `table.get`/`table.set`'s table-index immediate is OPTIONAL, defaulting
  to table 0 when omitted -- `(table.get (local.get $i))` is just as
  legal as `(table.get $t2 (local.get $i))`. Same disambiguation shape as
  W16's `memory.size`/`memory.grow`: an explicit index is always a bare
  `Atom`, while every operand is always a nested, parenthesized
  instruction (`SExpr::List`) in folded form.

## 0.1.16 — 2026-08-15 — capture a module directive's own $id (task #93, linking.wast)

### Fixed (breaking)

- `Directive::Module` now carries the module's own `$id` --
  `(module $Mf ...)` -- as a new `id: Option<String>` field, instead of
  silently discarding it during parsing. Real WAT scripts address a
  SPECIFIC earlier module by this id (`(invoke $Mf "f" ...)`, `(register
  "name" $Mf)`), not just "the current module" -- discarding it meant
  every such reference was permanently unresolvable. `Directive::Module`'s
  shape changes from a tuple variant `Module(Result<WasmModule, String>)`
  to a struct variant `Module { id: Option<String>, result:
  Box<Result<WasmModule, String>> }` (boxed per clippy's `large_enum_
  variant` -- `WasmModule` is large relative to every other `Directive`
  variant, and this enum lives in `Vec<Directive>` for a whole script).

See `wasm-conformance`'s own changelog for how this is actually used
(the module registry now resolves these ids too).

## 0.1.15 — 2026-08-15 — multi-memory: named memory index on memory.size/memory.grow (W16, task #85)

### Fixed

- `memory.size`/`memory.grow`'s FOLDED-form encoding (`(memory.size
  $mem1)`, `(memory.grow $mem1 (i32.const 1))`) now resolves a leading
  memory-index token via the existing `ctx.memory_names`/`resolve_idx`
  machinery instead of unconditionally hardcoding the memory-index byte
  to `0x00` and misparsing the `$name` token as an unknown nested
  instruction. Matches the exact leading-immediate-then-operands shape
  `"call" | "return_call"` already use.
- The STREAM-form encoding (`encode_stream_instr`) is deliberately left
  unchanged: the token after `memory.grow` there is an unparenthesized
  atom indistinguishable from the next instruction's own keyword without
  a real lookahead check this crate doesn't have yet, and no vendored
  corpus file needs it through this form. See `code/specs/
  W16-wasm-multi-memory-first-slice.md` for the full design and this
  scope boundary's reasoning.

## 0.1.14 — 2026-08-15 — correctly-rounded hex-float literals (task #80)

### Fixed

- Hex-float literal parsing (`0x1.8p3`-style, used by `f32.const`/
  `f64.const`) no longer double-rounds. The previous implementation
  accumulated the mantissa digit-by-digit via plain `f64` arithmetic
  (`mantissa = mantissa * 16.0 + d as f64`), performing a SEPARATE
  rounding step at every hex digit instead of one correctly-rounded
  (round-to-nearest, ties-to-even) conversion from the true mathematical
  value. Invisible for short literals, this produced genuinely WRONG
  results for the real spec testsuite's own deliberately-crafted
  over-precision edge cases -- confirmed via `simd_const.wast` (vendored
  in 0.1.12) and `const.wast`, both of which have `assert_return` cases
  built specifically to catch this class of bug.
  New `round_hex_mantissa` extracts the leading-1-normalized mantissa bits
  directly from the hex digit sequence and applies the classic
  guard/round/sticky rule once, at the end, against the TRUE value --
  correctly handling exact ties (round to even), subnormals (values below
  the smallest normal exponent, which have no implicit leading bit), and
  overflow/underflow to signed infinity/zero.
- `f32.const` hex literals ALSO no longer double-round: the previous
  `parse_f32_bits` rounded to `f64` first and narrowed via `as f32`, which
  is itself a double rounding (round to 53-bit significand, then round
  that result to 24-bit) -- not always equivalent to rounding the true
  value to `f32` once directly. `round_hex_mantissa` is now
  width-parameterized (`(mantissa_bits, exp_bias)`, `(52, 1023)` for
  `f64`, `(23, 127)` for `f32`) so both formats round independently and
  correctly from the same exact digit sequence, instead of one narrowing
  the other's already-rounded result.
- Both fixes are DoS-guarded: an adversarial literal with an extreme
  `p<exponent>` (unbounded by the digit count, unlike the mantissa
  itself) is caught by an early flush-to-zero/overflow-to-infinity return
  before the guard/sticky bit-scan loop runs, rather than iterating a
  range sized by the raw exponent value.
- Security review (round 1) found the DoS guard above was itself
  unreachable on the exact inputs it was meant to protect against: the
  arithmetic computing `e` (`base_exp2 + bitlen - 1`) and `base_exp2`
  itself (`exponent - 4 * frac_part.len()`) used plain `+`/`-` on a fully
  attacker-controlled `i64` exponent -- a ~25-byte literal like
  `f64.const 0x1p9223372036854775807` overflow-panicked computing `e`,
  and `f64.const 0x1.5p-9223372036854775808` underflow-panicked computing
  `base_exp2`, in both cases BEFORE the guard checks ever ran. Both now
  use `saturating_add`/`saturating_sub`, which is semantically correct
  here (a saturated value is always far enough outside the guards'
  window that they still fire the right way) and closes the crash for
  both a hand-verified reproduction and a new regression test per case.
- Regenerated `wasm-conformance`'s baseline: `const.wast`'s
  `assert_return` tally goes from 260/300 to 300/300 (fully clean) and
  `simd_const.wast` improves from 209/240 to 235/240 -- zero regressions
  anywhere else in the 61-file vendored corpus (verified via a full
  before/after diff of every file's per-kind tally, not just the two
  files known to be affected).

## 0.1.13 — 2026-08-15 — lazy per-module build (W14, task #76)

### Changed (breaking)

- `Directive::Module` now wraps `Result<WasmModule, String>` instead of a
  bare `WasmModule`. `parse_script` no longer aborts the WHOLE file with
  an `Err` when one `(module ...)` directive's own instruction stream
  fails to build (e.g. names an opcode this repo doesn't implement yet) --
  that failure is captured as `Directive::Module(Err(_))`, a per-directive
  data value, so every other directive in the file (independently
  parseable, whether earlier or later) is still returned normally.
  Genuine tokenizer/S-expression *syntax* errors (`parse_source`) are
  unaffected -- a truly malformed script still fails `parse_script` as a
  whole, since directive boundaries can't be reliably identified at all
  in that case. See `code/specs/W14-wasm-conformance-lazy-module-build.md`
  for the full design and the concrete motivating case (`simd_const.wast`'s
  sole `i64x2.add` usage previously blocked grading ~445 other, unrelated
  directives in that same file).
- 2 new tests proving the split: one script with a broken module
  bracketed by two independently-buildable modules parses as a whole
  (the broken one captured as `Err`, the others as `Ok`); one genuinely
  malformed script (unbalanced parens) still fails `parse_script`
  entirely, unchanged.

## 0.1.12 — 2026-08-15 — ConstValue::V128 for assert_return/invoke (SIMD PR1b-3)

### Added

- `script::ConstValue::V128([u8; 16])` and `parse_const_value`'s new
  `"v128.const"` arm, so `assert_return`/`invoke`'s own `(v128.const
  ...)` literal syntax parses -- reuses `module::parse_v128_const`
  directly (now `pub(crate)`, not private) rather than duplicating the
  shape/lane-width table.

## 0.1.11 — 2026-08-15 — v128.const + first-slice SIMD opcodes (SIMD PR1b-2)

### Added

- `v128` as a recognized value type in `(param)`/`(result)`/`(local)`/
  `(global)` declarations -- `ValueType::V128` already existed in
  `wasm-types` (SIMD PR1a), but `parse_value_type`'s keyword match never
  had a `"v128"` arm, so any function signature or global naming it
  failed with `UnexpectedToken`.
- `v128.const <shape> <lane0> ... <laneN-1>` text syntax, all 6 shapes
  (`i8x16`/`i16x8`/`i32x4`/`i64x2`/`f32x4`/`f64x2`), in both folded and
  flat/stream instruction forms. Every shape packs down to the same 16
  raw bytes regardless of how it's spelled -- this slice's own executable
  opcodes (SIMD PR1a) only run `i32x4`-shaped ops, but real corpus files
  (e.g. `simd_splat.wast`) mix shapes even when testing a single op, so
  parsing only `i32x4` would leave those files unparseable as a whole
  (one bad literal fails the whole module's parse in this crate's
  design), not just missing coverage for the unimplemented shapes' own
  instructions -- see `parse_v128_const`'s doc comment.
- `i32x4.splat`/`i32x4.add`/`i32x4.eq`/`i32x4.extract_lane` text syntax,
  wired through `wasm_opcodes::get_simd_op_by_name` and
  `wasm_opcodes::SimdOpKind` exactly like the existing `0xFE`-atomics
  interception (same two-byte-prefix shape, but the sub-opcode is a
  LEB128 `u32`, not a raw byte -- `i32x4.add`'s real sub-opcode, 174,
  exercises the multi-byte continuation encoding). `extract_lane`'s lane
  index is a single raw (non-LEB128) byte immediate, leading in folded
  form (`(i32x4.extract_lane 2 (...))`) same as `local.get`'s index.
- `numeric::parse_i8`/`parse_i16` (mirroring the existing `parse_i32`/
  `parse_i64`), needed only by `v128.const`'s `i8x16`/`i16x8` shapes --
  WASM has no plain `i8.const`/`i16.const`, `i32` is the smallest scalar
  integer type.
- 10 new tests covering all 6 shapes' byte-packing, the multi-byte
  LEB128 sub-opcode path, folded/flat parity, and 2 clean-error cases
  (unknown shape keyword, too few lane literals).

See `code/specs/W13-wasm-simd-v128-first-slice.md`'s follow-up scope.

## 0.1.10 — 2026-08-15 — return_call/return_call_indirect (WASM16)

### Added

- Full grammar support (folded + flat forms) for `return_call`/
  `return_call_indirect`, reusing `call`/`call_indirect`'s exact
  immediate-decode paths -- no new immediate shape to parse, just the
  new opcode byte. 4 new tests. See `code/specs/
  W11-wasm-tail-calls.md`.

### Investigated, not fixed this release

- The real, pinned-commit `return_call.wast`/`return_call_indirect.wast`
  each fail to parse on one narrow `(func $f (result (ref null $t))
  (ref.null $t))` declaration -- a concrete typed nullable function
  reference (function-references proposal grammar this crate doesn't
  support). Considered a minimal erasure-to-`Funcref` fix; rejected it
  after finding `return_call.wast`'s own `type-ref-vs-funcref`
  `assert_invalid` case specifically tests that a concrete `(ref null
  $t)` and a bare `funcref` result type are NOT interchangeable under
  real subtyping -- an erasure fix would make that case falsely `Pass`
  for the wrong reason. Needs real typed-function-reference support
  first; not vendored this release, tracked as a follow-up.

## 0.1.9 — 2026-08-15 — atomic instructions + `shared` memory keyword (WASM18)

### Added

- `shared` keyword parsing on `(memory ...)` forms, including the named
  inline-import shorthand (`(memory $m shared 1 1)` and the
  import-shorthand variant), via a new `parse_memory_limits` wrapper
  around `parse_limits` that also scans for a `"shared"` atom.
- Full grammar support for every `0xFE`-prefixed atomic instruction
  (load/store/RMW/cmpxchg/fence/notify/wait, folded and flat forms),
  driven entirely off `wasm_opcodes::ATOMIC_OPS` -- no per-instruction
  special-casing needed once the shared table existed.
- ~10 new tests: `shared` keyword parsing, named shared-memory import
  shorthand, atomic instruction encoding (folded + flat), explicit vs.
  default alignment, unknown atomic instruction name error, notify/
  wait32/wait64 parsing.

### Fixed

- `parse_memarg` previously always defaulted a missing `align=`
  immediate to 1-byte alignment (`0`) -- correct for plain loads/stores,
  whose validator only enforces an *upper bound* on alignment, but wrong
  for atomics: the real corpus's typical style omits `align=` entirely,
  and `wasm-validator`'s new atomic type rule requires an *exact* match
  to natural alignment. Every atomic instruction without an explicit
  `align=` would have failed validation outright. Fixed by giving
  `parse_memarg` a `default_align_log2: u32` parameter, with atomic call
  sites passing the operation's own natural alignment instead of `0`.
  Locked in by a dedicated regression test
  (`atomic_op_with_no_explicit_align_defaults_to_natural_not_zero`).
- Named memory inline-import shorthand (`(memory $m (import "m" "n") 1
  1)`) mis-parsed `$m` itself as part of the limits -- the same bug class
  already fixed for named globals in 0.1.8. Fixed by skipping an
  optional `$name` atom before calling `parse_memory_limits`.

## 0.1.8 — 2026-08-15 — named global inline-import shorthand (WASM19)

`(global $g0 (import "m" "n") i32)` -- the NAMED form of the global
inline-import shorthand -- previously mis-parsed `$g0` itself as the
value type ("expected a value type, found \"$g0\""). Only the unnamed
form (`(global (import "m" "n") i32)`, no `$name`) worked.

Root cause: `desugar_one_inline_import` rewrites `(global $g0 (import
"m" "n") i32)` into `(import "m" "n" (global $g0 i32))`, so `desc` (the
inner `(global $g0 i32)` list `build_import_shell` reads) is `[global,
$g0, i32]` when a name is present, vs. `[global, i32]` when it isn't.
`build_import_shell`'s "global" arm unconditionally read `desc.get(1)`
as the value type -- correct only for the unnamed shape.

### Fixed

- `build_import_shell`'s "global" arm now skips an optional `$name` at
  `desc[1]` before reading the value-type field, matching every other
  place in this crate that has to handle an optional name in a
  fixed-position field list.
- 2 new unit tests: named non-mutable and named mutable inline-import
  globals, both round-tripping through a real `parse_module` call and a
  `global.get` reference to the resolved index. Discovered while
  regenerating WASM17's conformance baseline (`global.wast`'s real
  corpus content uses exactly this named form) -- confirmed via a full
  baseline diff that this fix moves `global.wast`'s parse failure
  further still, this time into the SAME already-out-of-scope extended
  `elem`-segment syntax gap `call_indirect.wast` also hits (see WASM17's
  spec, `code/specs/W08-wasm-funcref-externref.md`), with zero
  regressions on any already-parsing file.

## 0.1.7 — 2026-08-15 — funcref/externref, ref.null/ref.func/ref.is_null, table.get/table.set (WASM17)

This crate had ZERO reference-types-proposal support before this release
(confirmed by grep: no match on any `ref.*` instruction name anywhere in
`module.rs`) — everything below is added from scratch. Unblocks real
conformance-testsuite content in the already-vendored `global.wast`,
`select.wast`, `br_table.wast`, `unreached-valid.wast` — none of those four
files fully flips to passing yet (each has its own separate, already-scoped
gap: `select`'s `(result T)`-annotated opcode 0x1C, `call_indirect.wast`'s
extended `elem`-segment syntax, `br_table.wast`'s concrete `(ref null $t)`
heap type, and a newly-discovered `global.wast` gap logged as WASM19 — see
that file's own backlog note), but the parse error in three of them moved
substantially deeper into real content, confirmed via a full baseline diff
with zero regressions on any already-parsing file. See `code/specs/
W08-wasm-funcref-externref.md`.

### Added

- `funcref`/`externref` as `ValueType` keywords wherever `parse_value_type`
  already recognized `i32`/`i64`/`f32`/`f64` (params, results, locals,
  globals) -- plus the verbose `(ref null func)`/`(ref null extern)` form
  found in the real corpus (`br_table.wast`), which parses to the identical
  `ValueType`. Non-null `(ref func)` and concrete `(ref null $t)`/`(ref $t)`
  forms are deliberately NOT recognized (out of scope, see the spec).
- `ref.null func`/`ref.null extern` (0xD0 + heap-type byte 0x70/0x6F) as a
  genuinely new instruction in both folded and flat form -- intercepted
  before the `wasm_opcodes::get_opcode_by_name` lookup (like the existing
  `trunc_sat` interception), since `wasm-opcodes` deliberately has no entry
  for 0xD0. `ref.is_null` (0xD1) intercepted the same way.
- `ref.func $x` (0xD2, `funcidx`) and `table.get $t`/`table.set $t` (0x25/
  0x26, `tableidx`) as ordinary new instructions routing through the
  normal `get_opcode_by_name` path, resolving `$name`s against
  `func_names`/`table_names` the same way `call`/`global.get` already do.
- Script-level `(ref.extern N)` literal (`ConstValue::Ref(Some(n))`) --
  the official testsuite's own script-syntax convenience for an externref
  test value, not a real instruction (confirmed absent from every real
  `.wat` function body in the corpus). Exact `(ref.null func/extern)`
  literals (`ConstValue::Ref(None)`), and the bare, type-less wildcard
  forms `(ref.null)`/`(ref.func)` (new `Expected::RefNullAny`/`RefFuncAny`
  variants -- only ever meaningful as an `assert_return` expectation, per
  the real corpus's `select.wast`/`global.wast` usage).
- 9 new unit tests in `module.rs`, 4 in `script.rs`, plus a documented
  known-gap test (`select_with_explicit_result_type_annotation_is_a_known_gap`)
  proving `select (result T)` fails cleanly with `UnknownInstruction`
  rather than silently mis-parsing.

## 0.1.6 — 2026-08-13 — multi-value block/loop/if blocktypes (WASM04)

`block.wast`, `if.wast`, and `loop.wast` never parsed at all — every one
of their `(block (param ...) (result ...) ...)` / `(loop (param ...)
(result ...) ...)` / `(if (param ...) (result ...) ...)` headers (the
multi-value extension's blocktype-as-type-index form) hit "unknown
instruction" or a mismatched-signature error, since `encode_structured_instr`
and `encode_stream_structured_instr` only ever emitted the MVP's single-byte
`0x40`/valtype blocktype encoding.

- Added `encode_blocktype`: scans a leading `(type $t)`/`(param ...)`/
  `(result ...)` run (the same `is_type_or_param_or_result` predicate
  `call_indirect`'s own inline signature already used) and emits either the
  MVP single-byte shorthand (empty, or one result, no params) or a SLEB128
  type-section index — resolving an explicit `(type $t)` via `type_names`,
  or deduplicating an inline signature via `dedup_type` exactly like an
  anonymous `call_indirect`/`func` signature would.
- Wired into both the folded (`encode_structured_instr`) and flat
  (`encode_stream_structured_instr`) instruction paths.
- **Deeper pre-existing bug found and fixed**: `resolve_func_signature_ref`
  (used by `build_func` for a func's OWN signature, and by the explicit
  `(import ... (func ...))` fixup pass) scanned its entire input slice
  unbounded via `.iter().find(...)`. Safe for the import call site (always
  signature-only), but once a FLAT-form function body could contain a
  LATER block's own unnested `(param ...)`/`(result ...)` fields, those got
  mis-scanned as part of the FUNC's own signature, corrupting its inferred
  `param_count` and tripping the existing `TypeUseParamCountMismatch` guard
  on perfectly valid functions. Fixed by bounding the scan to the same
  leading `is_leading_field`-delimited region `build_func`'s own mismatch
  pre-scan already uses, with an explicit optional-`$name`-atom skip so
  both call sites' differing conventions (the import site's `desc[1..]`
  can start with an un-stripped `$name`; `build_func`'s never does) resolve
  correctly.
- 6 new tests covering: an unaffected empty/single-result blocktype
  baseline, a param-only block (proving body position, not just "doesn't
  crash"), block-to-block `dedup_type` reuse (two structurally-identical
  blocktypes share one type entry), the flat-syntax loop header (a
  separate code path from folded), an `if`'s multi-value blocktype, and an
  explicit `(type $t)` blocktype resolving to the already-declared type
  (no new entry).

## 0.1.5 — 2026-08-13 — sign-extension + saturating-truncation opcodes (WASM03)

`i32.wast`/`i64.wast` used `i32.extend8_s`/`i32.extend16_s`/
`i64.extend8_s`/`i64.extend16_s`/`i64.extend32_s` (the "sign-extension
operators" proposal); `conversions.wast` used `i32.trunc_sat_f32_s` and its
7 siblings (the "non-trapping float-to-int conversions" proposal). Neither
family parsed — all failed with "unknown instruction" (extend8_s/etc.) or
were entirely unreachable in a working module (conversions.wast never
parsed for an unrelated pre-existing reason, `nan:0x7f_ffff` in `const.wast`
is a separate remaining gap this doesn't touch).

- **Sign-extension** (5 opcodes, single-byte 0xC0-0xC4): added to
  `wasm-opcodes` 0.2.1's table, so this crate's existing generic
  no-immediate-instruction encoding path (`encode_stream_instr`/
  `encode_flat_instr`'s default `_` arm) handles them automatically —
  zero special-casing needed here beyond the opcode table entry existing.
- **`trunc_sat`** (8 opcodes, two-byte `0xFC <sub-opcode>`): intercepted
  directly in both `encode_stream_instr` and `encode_flat_instr`, before
  the `wasm_opcodes::get_opcode_by_name` lookup that would otherwise reject
  them as unknown — `wasm-opcodes` deliberately doesn't model 0xFC-prefixed
  opcodes (see that crate's own changelog). New `trunc_sat_sub_opcode`
  helper maps each of the 8 names to its spec-assigned sub-opcode byte.

Once `i32.wast`/`i64.wast`/`conversions.wast` could parse, running them
against `wasm-execution` surfaced 2 more, entirely pre-existing bugs in
that crate (a NaN/overflow-boundary bug in the trapping `trunc_*` handlers,
and 4 unrelated `reinterpret` NaN-payload cases now tracked as WASM13) —
see `wasm-execution`'s own `0.6.5` changelog entry.

New tests: opcode-table-driven encoding for both flat and folded syntax
(sign-extension, going through the ordinary no-special-case path; `trunc_sat`,
going through the new `0xFC` interception) and a direct name→sub-opcode
mapping test for all 8 `trunc_sat` names.

## 0.1.4 — 2026-08-13 — func/table/memory/global inline-import shorthand (WASM02)

`(func $f (import "m" "n") (type $t))` and its `table`/`memory`/`global`
equivalents — the WAT text format's **inline-import shorthand**, exactly
equivalent to `(import "m" "n" (func $f (type $t)))` per the spec — weren't
recognized: `script.rs`'s "module" directive parsing (and `module.rs`'s
`parse_module_expr`) called straight into the plain-definition builder,
which doesn't know about a `(import ...)` sub-form appearing where a
function body would start. The `import`/`quote`/`type` fields it found
there weren't recognized module-field or instruction shapes either, so the
official testsuite's `func_ptrs.wast` and `global.wast` — the two files
this shape blocks — failed to even parse: `func_ptrs.wast` with "unknown
instruction \"import\"" (the shorthand's fields fed straight into the
function-body instruction encoder), `global.wast` with "expected a value
type, found \"list\"" (fed into the global-type parser instead).

Fixed with a **pure syntactic desugaring pass** (`desugar_inline_imports`
in `module.rs`), run before `collect_symbols`/`build` ever see a module's
fields: any `func`/`table`/`memory`/`global` field whose first substantive
item (after an optional `$name`) is `(import "m" "n")` gets rewritten into
the equivalent explicit `(import "m" "n" (kind ...))` form. Every
downstream pass then only ever has to understand ONE import shape.

### A deeper, previously-unreachable bug this exposed

Actually exercising an import together with a same-kind real definition —
something no vendored file did until this desugaring made it possible —
crashed with `index out of bounds` (a `func` import followed by any real
`func`) or a clean-but-wrong `wasm-validator` rejection ("code section has
N entries but function section has N+1 entries"). Root cause:
`ctx.module.functions`/`tables`/`memories`/`globals` are supposed to mirror
the real WASM **binary** format's function/table/memory/global sections,
which never include imports (`wasm-module-parser`'s own section parsers
confirm this: an import's type info lives solely in the import section,
tracked here via `ctx.module.imports`). But `collect_symbols`'s import loop
was ALSO pushing a placeholder entry into these same arrays for every
import, and `build`'s per-kind arms indexed them with the COMBINED
import+real ("func-space") index — both silently correct only because,
until now, no module ever had a nonzero import count for a kind that also
had real definitions. Fixed by making imports stop touching these arrays
entirely (tracked via dedicated per-kind counters instead, which also
fixes a latent double-counting bug in the old `table`/`memory`/`global`
import-index formula that mixed a *named-imports-of-any-kind* count into a
*this-kind* position), and splitting every "func-space index" (still
needed for name resolution, exports, and element/data segment references)
from the separate "storage index" (real-definitions-only, used to actually
index into these arrays) everywhere `build`/`build_func`/
`build_table_limits_and_elements` touch them.

New tests: inline-import shorthand desugaring for `func` (with a
`(type $t)` reference, matching `func_ptrs.wast`'s `$print`) and `global`
(unnamed, matching `global.wast`'s two `spectest` imports); a `func` import
followed by a real `func` with a `call` between them, asserting both the
encoded call-site bytecode and the real func's export index; the same
combination for `global` and `table`. Baseline: `func_ptrs.wast` goes from
a full parse failure to 100% passing every directive kind it has
(`assert_return` 12219/12238 → 12235/12254, +16); verified via a full
per-file diff against the previous baseline that `func_ptrs.wast` is the
ONLY file whose tally changed anywhere in the corpus. `global.wast` still
doesn't parse — blocked by an unrelated, legitimate gap (`externref`, a
reference-types feature explicitly out of scope for this phase) further
down the file, past the two inline-import globals this fix does correctly
handle.

## 0.1.3 — 2026-08-13 — `(module quote/binary ...)` DIRECTIVES silently built an empty module (WASM12)

Started as the small fix the task description named: the tokenizer's `;;`
line comment only terminated at `\n`, not a bare `\r` or `\r\n` — the
official testsuite's `comments.wast` has three functions whose bodies
differ only in which line terminator follows a `;; comment`, so this alone
would only have fixed 2 of its 3 `assert_return` cases. Fixed by also
stopping the comment scan at `\r` (a following `\n` is then consumed as
ordinary whitespace on the next iteration, so CRLF "just works" too).

Investigating why `comments.wast`'s **third** case (the plain `\n`
terminator, unaffected by the above) *also* failed found the real, much
bigger bug: `parse_directive`'s `"module"` arm called `parse_module_expr`
directly on the raw `(module quote "..." ...)` or `(module binary
"...")` s-expression — a function that only understands the plain-text
form. For `quote`/`binary`, the `quote`/`binary` atom and the string
tokens aren't recognized module fields, so they were silently skipped,
producing a trivially-valid **empty** module. Any `assert_return` invoking
an export from it then failed with "no such export" — not a comment bug at
all for 2 of the 3 cases; the module was never actually being parsed as
WAT, ever, for this directive kind. This affected every already-vendored
file with a real (non-`assert_malformed`) `(module quote/binary ...)`
directive, not just `comments.wast` — `float_literals.wast` has a
`(module binary ...)` decoding a real f64 constant, `func.wast` and
`int_literals.wast` use `quote` inside several `assert_malformed` cases
that happened to accidentally "pass" for the wrong reason (see below).

Fixed with two changes:
- `script.rs`'s `"module"` directive now routes through the actual source
  kind: `quote` text re-parses via this crate's own `parse_module` (see
  next point), `binary` bytes decode via the new `wasm-module-parser`
  dependency (`WasmModuleParser::parse`), erroring for real
  (`EmbeddedBinaryModuleError`) if either fails, rather than silently
  discarding.
- `module::parse_module` now accepts the WAT text format's **abbreviated
  module** form — a source with no enclosing `(module ...)` at all, its
  fields written directly at the top level — not just the explicit
  `(module ...)` form it required before. Both are real, independently
  valid WAT; the official testsuite's `(module quote ...)` directives use
  BOTH conventions depending on the file (`comments.wast`/`block.wast`
  quote bare fields; `align.wast`/`global.wast` quote the explicit
  `(module ...)` form) — the old code silently mishandled the bare-field
  convention as "one big unrecognized field," not a parse error.

**A real, understood side effect on `assert_malformed`'s baseline**: many
vendored `(module quote ...)` cases inside `assert_malformed` were
previously graded `Pass` because the quote text failed to even parse (the
missing-wrapper bug) — coincidentally the right VERDICT, for the wrong
REASON (the harness never actually got to check whether the case's real,
intended malformation was caught). Now that quote text parses correctly,
many of these build into a perfectly valid module — this repo has no
instruction-level type-checker (`W02` Phase 2, unimplemented) to catch the
specific defect the case was designed to probe, so they correctly
reclassify from an accidental `Pass` to an honest `NotYetSupported`
(`assert_malformed` 145/147 → 33/35 graded, 46 → 158 `NotYetSupported`;
zero new `Fail`s — confirmed by diffing every changed file's tally
against the previous baseline). This matches this crate's own documented
grading philosophy (see `wasm-conformance`'s module doc comment): a lucky
`Pass` from the wrong layer is worse than an honest "we don't know."

Net baseline effect: `assert_return` 12215/12238 → 12219/12238 (+4: 3 from
`comments.wast`, 1 from `float_literals.wast`'s binary-module case);
`assert_malformed` reclassified as above, no regressions. New tests:
2 tokenizer tests (bare-CR and CRLF line-comment termination), 2
`module.rs` tests (abbreviated-form parsing, single and multi-field), 2
`script.rs` tests (`module quote`/`module binary` directives building a
real, invokable module).

## 0.1.2 — 2026-08-13 — a local-index bug found investigating real assert_return failures (WASM14)

`build_func` assigned local indices by re-walking a function's own literal
`(param ...)` forms, incrementing a counter as it went. That undercounts
the moment a function references its signature purely via `(type $sig)`
(no `(param ...)` forms of its own at all — the official testsuite's
`func.wast` has several such cases: `"type-use-1"` through `"type-use-5"`)
and *also* declares a `(local ...)`: the counter never advances past 0
for the (invisible-to-this-function) params from the referenced type, so
the first declared local silently gets assigned parameter index 0 again
instead of the index right after the real params. `local.get` on that
local then read the PARAM's value instead of the local's own
zero-initialized default — a real, wrong computed VALUE, not a trap
(`func.wast`'s `"f"`/`"g"` cases expected 0, got 42, the argument passed
in).

Fixed by seeding the local-index counter from `ctx.module.types[type_idx]
.params.len()` — the function's REAL resolved param count — rather than
from a count built by re-walking this function's own literal `(param
...)` forms, which can legitimately be empty. Uses `.get()`, not direct
indexing: an already-regression-tested case
(`func_with_out_of_range_numeric_type_reference_does_not_panic`) exercises
a numeric `(type N)` reference with no matching `(type ...)` section entry
at all, which this text-level parser deliberately does not reject (that's
`wasm-validator`'s job) — falls back to a param count of 0 rather than
panicking on the out-of-range index.

1 new regression test
(`local_declared_after_a_type_only_referenced_param_gets_the_next_free_index`)
reproducing `func.wast`'s exact shape in isolation. Baseline: `assert_return`
12169/12238 (99.4%) → 12171/12238 (99.5%).

**A security review of this fix found a residual edge case**: it split
one shared counter into two independent ones (literal `(param ...)`
forms counted as written, vs. the referenced type's real param count),
which only agree when a function's literal params match its `(type
$sig)` reference exactly — not something `resolve_func_signature_ref`
itself enforces. A syntactically-valid but semantically-inconsistent
module (literal params disagreeing in count with a same-function `(type
$sig)` reference — deliberately adversarial input, not something a real
`.wat` file produces) could make a declared local alias whichever count
was smaller. Fixed by seeding the local-index counter from
`max(literal param count, the type's real param count)` the first time a
`(local ...)` form is actually reached, so a declared local can never
collide with a position either count considers a parameter. 1 more
regression test
(`local_index_never_collides_with_a_param_even_if_literal_params_and_the_type_disagree`).
No conformance-baseline change (the real testsuite never disagrees with
its own type references).

**A second round of security review found that round 1's fix wasn't
actually closed by round 2's `max()` patch — it moved the failure mode.**
Since the compiled `FunctionBody` and the function's real type only ever
account for the type's real param count, an "extra" literal param (in
the same mismatched-arity scenario round 2 was defending against) still
encoded a `local.get`/`.set`/`.tee` index past the function's real local
array. Confirmed empirically: `wasm-execution`'s raw, unchecked
`ctx.typed_locals[index]` panics once such a module actually runs — not
memory-unsafe (checked Rust indexing), but a real crash/DoS surface
reachable through this repo's own pipeline
(`wasm-conformance`/`wasm-runtime`/`wasm-execution`), since the only
validation currently wired up (`WasmRuntime::validate`) is structural
only and doesn't check instruction operand bounds. The real fix is
upstream of both prior patches: a new `WastParseError::TypeUseParamCountMismatch`
now REJECTS at parse time when a func's literal `(param ...)` forms
disagree in arity with an explicit `(type $sig)` reference, instead of
silently accepting the inconsistency and hoping every later index
computation stays safe. This is also the spec-correct behavior — a real
`.wat` file's literal params, when given alongside a type reference,
always already match it exactly. The `max()`-based local-index seeding
from round 2 stays as defense in depth (harmless: once this new check
passes, the two counts are always equal whenever literal params were
given), but this rejection is what actually makes the invariant hold. 2
more regression tests: the mismatched case now asserts a clean `Err`
instead of successfully (and unsoundly) parsing, plus a new positive
case confirming the legitimate "type reference + matching literal
params" pattern (`func.wast`'s own `"type-use-6"` shape) still parses
and indexes correctly.

**A third round of security review found round 3's own rejection check
could itself be bypassed.** Its pre-scan stopped at the first field that
wasn't `param`/`result`/`type` — but a `(local ...)` form placed BEFORE
some of a func's trailing `(param ...)` forms (this parser doesn't
enforce that params all precede locals; that's `wasm-validator`'s job
too) made the pre-scan stop before ever counting those later params,
silently skipping the mismatch check while the main assignment loop
still processed them — reproducing round 2's exact out-of-bounds local
index, just via reordering instead of an outright count mismatch. Fixed
by giving the pre-scan the identical leading-region membership test
(`is_leading_field`: `param`/`result`/`type`/`local` are ALL "still in
the prefix," only a real instruction ends it) the main loop already
uses, so the two passes can no longer silently disagree on where the
leading region ends. 1 more regression test reproducing the reordered
bypass directly.

**A fourth (final) round of security review, after re-verifying the
round-3 fix genuinely closed the OOB class, found a functional
regression the mismatch check itself had introduced**: it compared
against `param_count`'s `0` fallback for an out-of-range numeric `(type
N)` reference, silently violating this file's own documented contract
(`func_with_out_of_range_numeric_type_reference_does_not_panic`) that an
unresolvable type reference must NOT be rejected here — that's
`wasm-validator`'s job. `(func (type 0) (param i32))` — ordinary,
spec-legal literal params alongside an unresolvable type index — got
hard-rejected instead of passed through. Fixed by gating the check on
the type reference actually resolving to a real type first. Also
extracted `count_literal_param` (the named-vs-unnamed param-counting
arithmetic) as a single function shared by the pre-scan and the main
loop, the same way `is_leading_field` already is — the review flagged
two independently-maintained copies of that arithmetic as exactly the
drift pattern that produced rounds 2 and 3's findings, even though the
two copies were still identical today. 2 more regression tests: the
false-positive case now confirmed fixed, plus the legitimate
"out-of-range type, no literal params" case re-confirmed unaffected.

## 0.1.1 — 2026-08-13 — 4 grammar bugs found running the real testsuite (W05 PR-4)

`wasm-conformance` (W05 PR-4) is this crate's first real workout: running
every vendored file from the official `WebAssembly/testsuite` corpus, not
just this crate's own hand-written unit tests. That surfaced four genuine
parsing bugs, each fixed with its own regression test:

- **Folded `br_table`'s label/operand split was backwards.** WAT's grammar
  lists all label targets FIRST, then an OPTIONAL folded index operand
  LAST — `(br_table $a $b (i32.const 0))` — the opposite of every other
  instruction's own "immediates trail operands" convention. The original
  code searched from the END of the argument list for the first non-atom
  element (assuming trailing atoms were the labels), found the folded
  operand's own position instead, and silently produced a zero-label
  `br_table` while dropping the real label references. Affected any file
  using a folded `br_table` with more than one label — a majority of the
  corpus's control-flow files.
- **`(table reftype (elem e*))` — a table with its size implied by an
  inline element list instead of explicit numeric limits — was completely
  unhandled.** `funcref` isn't a digit atom, so `parse_limits` always hit
  its "expected 1 or 2 limit numbers" error path. Now recognized as its
  own form: `min`/`max` are set to the element count, and the elem
  segment referenced by those functions is synthesized directly (`i32.const
  0` offset), matching the shorthand's defined meaning.
- **A bare hex integer (no `.` fraction, no `p`/`P` exponent) wasn't a
  valid float literal.** `f32.const 0xf32` means the plain number
  `3890.0`, not a bit reinterpretation and not a hex *float* (which
  requires an exponent) — but the parser required a `p`/`P` exponent
  unconditionally for anything hex-prefixed.
- **A hex float's `p`/`P` exponent is optional even WITH a fractional
  part**, not just on a bare integer — `0xa0_ff.f141_a59a` (no exponent
  at all) defaults to exponent 0. The mantissa parsing was previously
  reachable only via the exponent-bearing branch.

A security review of this same PR found one more, related bug in the
`(table reftype (elem e*))` fix above: `(table funcref ())` — a
syntactically valid but EMPTY inline list, with no `"elem"` head atom at
all — indexed `elem_form[1..]` without first confirming the list was
both non-empty and actually headed by `"elem"`, panicking with a
slice-range-out-of-bounds. Fixed by validating the head atom before
slicing, with its own regression tests (`(table funcref ())` and
`(table funcref (notelem)))`, both now clean `Err`s). None of the 48
currently-vendored files trigger this — every real table-elem shorthand
names at least one function — but it's exactly the shape of input a
future `assert_malformed`/`assert_invalid` fixture (or wider corpus
vendoring) could hit.

Net effect on the vendored corpus: file-level parse failures dropped from
33/48 to 16/48. The remaining 16 are legitimate, out-of-scope gaps
(multi-value block signatures, reference-types `externref` and the
generalized `elem` syntax, post-MVP saturating-truncation/sign-extension
opcodes, and the `func`/`global` inline-import shorthand — the last is
linking-adjacent and shares this phase's already-documented `spectest`
deferral) — see `code/specs/W05-wasm-conformance-harness.md` section 6 and
`wasm-conformance`'s own report output for the exact breakdown.

## 0.1.0 — 2026-08-12 — initial release (W05 PR-2)

New crate. Parses the WebAssembly text format — both plain `.wat` modules
and the official spec testsuite's `.wast` script dialect — into
`wasm-types::WasmModule` and a sequence of test directives. Phase A of the
`wasm-execution` conformance-harness arc; see
`code/specs/W05-wasm-conformance-harness.md`.

- **`tokenizer`**: S-expression tokenizer — atoms, parens, quoted strings
  (with standard, `\u{XXXX}`, and raw `\XX` hex-byte escapes for embedding
  intentionally-invalid bytes), line comments, and **nestable** block
  comments (`(; a (; b ;) c ;)` is one comment, not two).
- **`sexpr`**: generic S-expression tree the tokenizer's flat stream is
  grouped into — folded instruction syntax (`(i32.add (i32.const 1) ...)`)
  is structurally identical to any other nested list, so there is no
  separate "folded vs. flat" parsing code path at this layer.
- **`numeric`**: WAT numeric literal parsing beyond what `str::parse`
  offers — hex integers, hex floats (`0x1.8p3`, computed bit-exact via
  digit-by-digit mantissa accumulation scaled by an exact power of two,
  not an approximate float parse), `inf`/`nan`, and `nan:0x<payload>` (an
  *exact* NaN bit pattern). `i32`/`i64` literals accept the WAT-permitted
  range union of both the signed and unsigned spelling of the same bit
  pattern (`-1` and `0xffffffff` both denote the identical i32 bits).
- **`module`**: the core — two-pass `(module ...)` parsing (pass 1 collects
  every symbolic name in every index space, imports always occupying the
  lowest indices regardless of textual interleaving with non-import
  definitions per the WAT spec; pass 2 encodes function bodies, globals'
  init expressions, and element/data segments straight to raw WASM
  bytecode). Supports both **folded** and **flat** instruction syntax for
  every MVP opcode with immediates (control flow, local/global access,
  calls, memory load/store, the four `*.const`s) via two structurally
  distinct encoders (`encode_flat_instr` for folded-list operand/immediate
  splitting, `encode_stream_instr`/`encode_stream_structured_instr` for a
  bare-atom instruction consuming however many *following* stream elements
  its own immediates need) — the two forms have different immediate
  ordering rules (folded: instruction's own index/label leads, operand
  sub-expressions trail; flat: operands were already pushed by whatever
  came before in the stream, so only trailing immediate atoms belong to
  this instruction) that a single shared code path could not represent
  correctly; the crate's own tests were written specifically to catch this
  after development first got it backwards for `br`/`call`/`local.set`/
  `local.tee`/`global.set` (immediate-first, not immediate-last).
- **`script`**: `.wast` script-directive parsing — `module`, `register`,
  `invoke`/`get`, `assert_return` (including `nan:canonical`/
  `nan:arithmetic` NaN-class result forms), `assert_trap`,
  `assert_exhaustion`, `assert_invalid`, `assert_malformed`,
  `assert_unlinkable`. A plain `module` directive is built eagerly
  (propagating a real syntax error immediately, since `assert_return`/
  `assert_trap` need an already-valid module to invoke against);
  `assert_invalid`/`assert_malformed`'s module is captured as a **raw,
  unparsed S-expression** instead, since failing to build it is exactly
  what those two directives test for — eagerly building it here would turn
  every legitimate fixture into a hard error aborting the whole script.
  Also supports the `(module binary "...")`/`(module quote "...")` module
  variants, concatenating their string-literal bytes for the caller.
- **Hardening pass** (pre-merge security review): this crate will eventually
  process the official testsuite's `assert_malformed`/`assert_invalid`
  fixtures, which are deliberately adversarial — so every reachable panic
  on malformed-but-syntactically-parseable input was replaced with a clean
  `Result::Err`. Fixed: `parse_i32` overflow on an extreme-magnitude
  negative literal (unary negation panicking in debug builds — switched to
  `wrapping_neg`); `parse_limits` panicking via `.unwrap()` on a
  non-numeric or out-of-`u32`-range limit; folded `br_table` underflowing
  `labels.len() - 1` on an empty label list; multiple `script.rs` directive
  parsers and `module.rs`'s `build()`/`build_elem`/`build_data` indexing
  past the end of a too-short field list (`(register)`, `(export "e")`,
  an empty `elem`/`data` segment, etc.) — all now go through a shared
  `sexpr::expect_get` helper instead of `items[N]`; and unbounded `(...)`
  nesting recursion, now capped by `sexpr::MAX_NESTING_DEPTH` (512) with a
  new `WastParseError::TooDeeplyNested` variant instead of a stack
  overflow. Each fix has a dedicated regression test proving the old code
  path would have panicked.
- **Hardening pass, round 2** (same pre-merge security review, second
  pass over `module.rs` specifically): five more `items[N]`-style panics
  of the exact same class, all in spots round 1's sweep missed —
  `build_import_shell`'s error-path indexing an empty import description
  (`(import "m" "n" ())`); `parse_global_type` indexing a `(mut)` form
  with no trailing value type; the `"start"` directive with no function
  reference (`(module (start))`); `handle_inline_export`'s `(export)`
  shorthand with no name string; and a bare `(type)` reference with no
  index/name, reachable from three call sites (`func` import
  descriptions, and both the flat and folded forms of `call_indirect`).
  All converted to `sexpr::expect_get`, each with its own regression
  test.
- **Hardening pass, round 3**: `build_func` indexed `ctx.module.types` by
  an unvalidated numeric `(type N)` reference (`(module (func (type 0)))`
  with no `(type ...)` declared anywhere panics indexing an empty `Vec`)
  while fetching a value that was, on inspection, entirely dead code
  (immediately discarded, never used) — fixed by deleting the dead fetch
  rather than adding unused bounds-checking; bounds-checking a type index
  is `wasm-validator`'s job (structural "index bounds" validation), not
  this text-parser's, so this module now correctly parses to a
  (structurally invalid) `WasmModule` instead of panicking or duplicating
  validation this crate doesn't own.
- **Hardening pass, round 4**: `sexpr::MAX_NESTING_DEPTH` only bounds
  `(...)` parenthesis nesting — but WAT's **flat** instruction syntax lets
  `block`/`loop`/`if` nest with NO parentheses at all (`block block
  block ... end end end`, all sibling atoms in one unnested list), driving
  unbounded `encode_one` <-> `encode_stream_structured_instr` recursion
  the S-expression-level guard never sees. Empirically confirmed as a real
  stack-overflow abort (not a catchable panic) before this fix. Added a
  second, independent `InstrCtx::depth` counter (`enter_block`/
  `exit_block`, covering both the flat and folded structured-instruction
  encoders uniformly) capped by a NEW, deliberately lower
  `MAX_INSTR_NESTING_DEPTH` (100, not 512) — this recursion carries more
  per-frame state than the lightweight S-expression tree-builder, so 512
  levels of it measurably overflows a real thread's stack around depth
  ~487, well before the counter would ever stop it.
- **Hardening pass, round 5**: round 4's guard only covered `block`/
  `loop`/`if` recursion — a deeply nested FOLDED operand of an ordinary
  instruction (`(i32.add (i32.add (i32.add ...) ...) ...)`, no control
  flow involved at all) recurses through `encode_flat_instr` ->
  `encode_instr_list` -> `encode_one` with no depth guard whatsoever, and
  empirically aborted with a real stack overflow around depth ~165 — well
  under `sexpr::MAX_NESTING_DEPTH` (512), so that guard never tripped
  first either. Fixed by consolidating the depth guard into `encode_one`
  itself — the single point every form of instruction nesting (folded
  operands, folded `block`/`loop`/`if`, and flat `block`/`loop`/`if`)
  funnels through — instead of gating only the `block`/`loop`/`if`-
  specific encoders. The now-redundant per-block guards were removed to
  keep exactly one depth-accounting mechanism. A regression test confirms
  a long FLAT (sibling, not nested) instruction sequence well past
  `MAX_INSTR_NESTING_DEPTH` in length still parses fine — the guard tracks
  nesting depth, not instruction count.
- 82 unit tests across all five modules, ~95%+ line coverage.
