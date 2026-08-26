# Changelog — wasm-wast-parser

## 0.1.81 — 2026-08-26 — W11 addendum: `(ref null $t)` / `ref.null $t`

### Added

- `(ref null $t)` as a value type (function/param/result/local/global),
  resolving `$t` via the same `type_names` map every `(type $t)`
  reference already uses, into `wasm_types::ValueType::ConcreteFuncRef`
  (`wasm-types` 0.1.12). `parse_value_type` and `parse_func_signature`
  both gained a `type_names: &HashMap<String, u32>` parameter, threaded
  through every call site (type declarations, locals, globals, imports,
  `call_indirect`/`return_call_indirect`'s inline signatures, blocktypes).
- `ref.null $t` / `ref.null <numeric type index>` as an instruction, in
  both folded and flat form: `parse_ref_null_heap_type` now returns the
  full heap-type-immediate byte sequence (`Vec<u8>`, not a single `u8`) —
  `0x63` followed by `LEB128(idx)` for a concrete type, matching
  `ConcreteFuncRef`'s own binary encoding, alongside the pre-existing
  single-byte abstract heap types (`func`/`extern`/`i31`).

### Fixed

- `encode_blocktype`'s single-result fast path called
  `ValueType::byte_tag().unwrap()` unconditionally — a latent panic for
  any single-result blocktype with no single-byte shorthand (`StructRef`
  or the new `ConcreteFuncRef`), newly reachable now that concrete
  function-type refs exist as a real value type. Now falls through to the
  existing type-index-encoding path (the same one multi-param/
  multi-result blocktypes already use) instead of panicking.

### Changed

- `ref_null_unknown_heap_type_is_a_clean_error_not_a_panic` (a `$t` heap
  type used to be unconditionally out of scope) is now
  `ref_null_undeclared_concrete_heap_type_is_a_clean_error_not_a_panic`:
  an undeclared `$t` still errors cleanly, but now as
  `WastParseError::UnknownIdentifier` (a real, resolvable name space), not
  `UnexpectedToken` (an unrecognized grammar shape).

## 0.1.80 — 2026-08-26 — table64 proposal, first slice (W26)

### Added

- `(table i64 <min> [<max>] funcref)` text syntax (table64 proposal): an
  `i64` keyword atom in the same leading position `(memory i64 ...)`
  already established sets `TableType::is64` (`wasm-types` 0.1.11), both
  for a declared table (`build_table_limits_and_elements`) and an imported
  one (explicit `(import "m" "n" (table i64 ...))` and the inline-import
  shorthand `(table $t (import "m" "n") i64 ...)`, where `i64` sits AFTER
  the `(import ...)` clause). Reuses `numeric::parse_u64`/`parse_limits64`
  verbatim (already built for memory64, table-agnostic).

See `code/specs/W26-wasm-table64-first-slice.md`.

## 0.1.79 — 2026-08-26 — memory64 proposal, first slice (W25)

### Added

- `(memory i64 <min> <max>?)` text syntax (memory64 proposal): an `i64`
  keyword atom immediately after any optional `$name`, before the limit
  numbers, sets `MemoryType::is64` (`wasm-types` 0.1.10). New
  `numeric::parse_u64` (mirrors `parse_u32`, using the same
  `parse_int_magnitude` u128 core `parse_i32`/`parse_i64` already share)
  for a 64-bit memory's limit literals, which can reach `2^48`.
- `(memory (data <string>*))` — the "memory sized by inline data" text
  abbreviation, for BOTH 32- and 64-bit memories. This is a genuine,
  pre-existing gap this crate never supported at all (found while
  vendoring `memory64.wast`, whose first three modules use exactly this
  form) — fixing the shared parser also unlocked three already-vendored
  32-bit files that use the identical form and were silently grading
  `NotYetSupported`: `memory.wast`, `float_exprs.wast`, and
  `float_memory.wast` (previously ENTIRELY not-yet-supported). New
  `build_memory_limits_and_data`, analogous to the existing
  `build_table_limits_and_elements` (`(table funcref (elem ...))`).

See `code/specs/W25-wasm-memory64-first-slice.md`.

## 0.1.78 — 2026-08-26 — Exceptions proposal, fourth slice W24: `throw_ref` text form

### Added

- `throw_ref` parses and encodes in both folded and flat (bare-atom) text
  forms — no special-casing needed: `wasm-opcodes` now registers it with
  `immediates: &[]`, so this crate's existing generic no-immediate default
  arm (the same one `unreachable`/`nop`/`return` already use) handles it
  automatically. See `code/specs/W24-wasm-exceptions-exnref-catch-ref.md`.

## 0.1.77 — 2026-08-25 — Exceptions proposal, second slice W22: `exnref` recognized (inert)

### Added

- `"exnref"` recognized in `parse_value_type` (`ValueType::Exnref`, new
  in `wasm-types` 0.1.8) — purely so a module mentioning it still parses
  as a whole; no new runtime semantics. See `code/specs/
  W22-wasm-exceptions-catch-clause-matching.md`.

## 0.1.76 — 2026-08-25 — Exceptions proposal, first slice W21: tag/throw/try_table text syntax

### Added

- `tag` as a 5th inline-import-sugar kind (alongside func/table/memory/
  global): `(tag $name? (param ...)*(result ...)* | (type $t))`,
  `(export "x" (tag $idx))`, inline `(tag $name (export "x") ...)`
  export sugar, both inline (`(tag $t (import "m" "n") ...)`) and
  explicit (`(import "m" "n" (tag $t ...))`) import forms — reuses
  `resolve_func_signature_ref` verbatim (a tag's grammar is identical to
  a function's own signature-or-type-ref grammar). New `tag_names` index
  space (combined imports + module-defined, same convention as
  `func_names`/`table_names`/etc.).
- `throw $tag` instruction encoding (folded + flat): a single `tagidx`
  immediate resolved via `tag_names`.
- `try_table` as a 4th structured-instruction form (alongside `block`/
  `loop`/`if`), folded + flat: parses `(catch $tag $label)`/
  `(catch_ref $tag $label)`/`(catch_all $label)`/`(catch_all_ref $label)`
  clauses into the real binary catch-clause encoding (`0x00`-`0x03`
  clause-kind byte + tag/label index(es)) via a new shared
  `parse_try_table_catches` helper. Label depths resolve against
  `icx.labels` BEFORE `try_table`'s own label is pushed, matching the
  real spec's "relative to the ENCLOSING scope" rule.
- `script.rs`: new `Directive::AssertException { action: Action }` --
  `(assert_exception (invoke ...))`, no message string (unlike
  `assert_trap`/`assert_exhaustion`), matching the real corpus's own
  shape.

See `code/specs/W21-wasm-exceptions-tag-throw-slice.md`.

## 0.1.75 — 2026-08-25 — GC epic, first slice W20: i31ref text syntax + real i31.wast conformance

### Added

- First-ever wast TEXT syntax for this repo's existing struct/i31 GC
  slice (previously only exercised via hand-built binary bytecode in
  `wasm-execution`'s own unit tests):
  - `parse_value_type`: `(ref i31)`, `(ref null i31)`, and bare `i31ref`
    all parse to `ValueType::I31ref`. Null vs. non-null is deliberately
    NOT distinguished (same simplification this crate already makes for
    `funcref`/`externref`).
  - `parse_ref_null_heap_type`: recognizes `i31` (heap-type byte `0x6C`).
  - Instruction names `ref.i31`, `i31.get_s`, `i31.get_u` in both the
    flat/stream encoder (`encode_stream_instr`) and the folded encoder
    (`encode_flat_instr`) -- the same "intercept by name before
    `wasm_opcodes::get_opcode_by_name`" shape `ref.null`/`ref.is_null`
    already use.
- `Expected::RefI31Any` (new `assert_return` expected-value variant):
  bare `(ref.i31)` with no argument -- matches ANY `i31ref`, the same
  "can't predict the exact value, just the shape" wildcard `Expected::
  RefFuncAny` already provides for funcref. Needed by the real
  `i31.wast`'s own `(assert_return (invoke "new" (i32.const 1))
  (ref.i31))`. Grading lives in `wasm-conformance`'s
  `value_matches_expected` (this repo's i31ref values are plain `i32`s
  on the stack, never a `WasmValue::Ref` -- see `wasm-execution`'s own
  `0xFB` handler doc comment -- so the grading check is just "is this an
  I32").
- Unit tests for all of the above, in both flat and folded instruction
  form, plus a params/results/locals/globals round trip and a
  `ref.null i31` encoding check.

### See also

- `code/specs/W20-wasm-gc-i31-conformance.md` for the full GC epic
  prioritization writeup and why this slice (not `br_on_null`/array
  types/etc.) was picked first.

## 0.1.74 — 2026-08-25 — Relaxed SIMD epic PR6: i16x8/i32x4.relaxed_dot_i8x16_i7x16_s/_add_s

### Added

- `SimdOpKind::RelaxedDotI8x16I7x16S`/`RelaxedDotI8x16I7x16AddS` joined
  the existing no-immediate SIMD match arm in both `module.rs` encoding
  functions (the same arm `DotI16x8S`/`RelaxedMaddF32x4` and every
  earlier relaxed-simd opcode are already in) -- these 2 opcodes (one
  BINARY, one TERNARY) take no immediate beyond the opcode byte itself;
  operand count is driven by the S-expression recursion, not the
  encoder, so no special-casing was needed.
- New test: `simd_relaxed_dot_product_family_encodes_the_real_sub_opcodes`,
  confirming `i16x8.relaxed_dot_i8x16_i7x16_s`/
  `i32x4.relaxed_dot_i8x16_i7x16_add_s` encode as `[0xFD, 0x92, 0x02]`/
  `[0xFD, 0x93, 0x02]` respectively.

## 0.1.73 — 2026-08-25 — Relaxed SIMD epic PR5: f32x4/f64x2.relaxed_madd/relaxed_nmadd

### Added

- `SimdOpKind::RelaxedMaddF32x4`/`RelaxedNmaddF32x4`/`RelaxedMaddF64x2`/
  `RelaxedNmaddF64x2` joined the existing no-immediate SIMD match arm
  in both `module.rs` encoding functions (the same arm `Bitselect` and
  every earlier relaxed-simd opcode are already in) -- these 4 opcodes
  are TERNARY, same as `Bitselect`/`RelaxedLaneselectI8x16`, but still
  take no immediate beyond the `0xFD` prefix + 2-byte LEB128
  sub-opcode, so no new parsing logic was needed beyond the table
  lookup this crate already does for every SIMD opcode.
- No changes needed to `script.rs`'s `Expected::Either`/`parse_expected`
  -- the N-ary `either` generalization PR3 added already handles this
  file's `either` alternatives unchanged.
- New test: `simd_relaxed_madd_nmadd_family_encodes_the_real_sub_opcodes`,
  confirming all 4 new opcodes encode to their real 2-byte LEB128
  sub-opcode values.

## 0.1.72 — 2026-08-25 — Relaxed SIMD epic PR4: i8x16/i16x8/i32x4/i64x2.relaxed_laneselect

### Added

- `SimdOpKind::RelaxedLaneselectI8x16`/`RelaxedLaneselectI16x8`/
  `RelaxedLaneselectI32x4`/`RelaxedLaneselectI64x2` joined the existing
  no-immediate SIMD match arm in both `module.rs` encoding functions
  (the same arm `Bitselect` and the earlier relaxed-simd opcodes are
  already in) -- these 4 opcodes are TERNARY, same as `Bitselect`, but
  still take no immediate beyond the `0xFD` prefix + 2-byte LEB128
  sub-opcode, so no new parsing logic was needed beyond the table
  lookup this crate already does for every SIMD opcode.
- No changes needed to `script.rs`'s `Expected::Either`/`parse_expected`
  -- the N-ary `either` generalization PR3 added already handles the
  real `relaxed_laneselect.wast` corpus's THREE-alternative "pblendvb"
  special-case group unchanged.

### Added

- `SimdOpKind::RelaxedMinF32x4`/`RelaxedMaxF32x4`/`RelaxedMinF64x2`/
  `RelaxedMaxF64x2` joined the existing no-immediate SIMD match arm
  (same list `RelaxedSwizzle`/`RelaxedQ15mulrI16x8S` are already in) in
  both `encode_stream_instr` and `encode_flat_instr` -- all 4 opcodes
  take no immediate beyond the opcode itself, identical encoding shape
  to `f32x4.pmin`/`pmax`/`f64x2.pmin`/`pmax`. Third/fourth/fifth/sixth
  opcodes of the relaxed-simd epic -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`.

### Changed

- **Generalized `parse_expected`'s `(either A B)` arm from exactly 2
  children to N (>= 2)**: the real `relaxed_min_max.wast` corpus is the
  FIRST relaxed-simd file whose `either` groups carry FOUR alternatives,
  not the two `i8x16_relaxed_swizzle.wast`/`i16x8_relaxed_q15mulr_s.
  wast` each used. The original `items[1]`/`items[2]`-only version would
  have silently DROPPED alternatives 3 and 4 rather than erroring -- a
  real correctness bug (a test whose actual result matches only the
  3rd/4th alternative would have wrongly failed to grade as passing).
  `Expected::Either` itself stays binary (its own doc comment already
  anticipated "a nested `either`") -- N children now fold into a
  right-leaning chain of nested `Either`s, so `wasm-conformance`'s
  existing recursive `||`-based grading needed NO changes at all to
  support the deeper nesting.
- New tests: `either_with_four_alternatives_folds_into_nested_binary_
  either` (v128 shape, mirrors the real corpus) and `either_with_four_
  alternatives_on_scalar_i32_expected_values` (confirms the fold isn't
  v128-specific).

## 0.1.70 — 2026-08-25 — Relaxed SIMD epic PR2: i16x8.relaxed_q15mulr_s

### Added

- `SimdOpKind::RelaxedQ15mulrI16x8S` joined the existing no-immediate
  SIMD match arm (same list `RelaxedSwizzle`/`Q15mulrSatI16x8S` are
  already in) in both `encode_stream_instr` and `encode_flat_instr` --
  `i16x8.relaxed_q15mulr_s` takes no immediate beyond the opcode itself,
  identical encoding shape to `i16x8.q15mulr_sat_s`. Second opcode of
  the relaxed-simd epic -- see `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`.
- Reuses the `Expected::Either` combinator PR1 added, unchanged -- no
  new parsing infrastructure needed for this opcode.

## 0.1.69 — 2026-08-25 — Relaxed SIMD epic PR1: i8x16.relaxed_swizzle + the `either` assert_return combinator

### Added

- `SimdOpKind::RelaxedSwizzle` joined the existing no-immediate SIMD
  match arm (same list `Swizzle` is already in) in both
  `encode_stream_instr` and `encode_flat_instr` -- `i8x16.relaxed_
  swizzle` takes no immediate beyond the opcode itself, identical
  encoding shape to `i8x16.swizzle`. First opcode of the relaxed-simd
  epic that follows the now-complete base SIMD epic (PR1-PR47) -- see
  `code/specs/W19-wasm-relaxed-simd-first-slice.md`.
- New top-level `assert_return` expected-value combinator:
  `Expected::Either(Box<Expected>, Box<Expected>)`, parsed from
  `(either A B)`. The relaxed-simd spec deliberately leaves certain ops
  implementation-defined for certain inputs; the upstream corpus grades
  those cases with `either` instead of one exact expected value.
  `parse_expected` gained a new `("either", _)` arm that recurses into
  itself for both children, so `either` can wrap any other `Expected`
  shape (a NaN class, a nested `either`, etc.), not just a plain
  `v128.const`. Every relaxed-simd `.wast` file in the pinned upstream
  corpus uses `either` at least once -- a genuine prerequisite for
  vendoring any relaxed-simd fixture, discovered by reading the real
  corpus content, not assumed from the opcode list.
- `Expected` is no longer `Copy` (only `Clone`) -- a `Box<Expected>`
  can't be `Copy`. Every existing call site already took `&Expected` or
  moved a freshly constructed value, so this cost nothing; confirmed by
  this crate's full test suite passing unchanged.
- New tests: `either_of_two_v128_const_values_parses_as_expected_either`,
  `either_recurses_through_parse_expected_for_non_v128_children`.

## 0.1.68 — 2026-08-25 — SIMD PR47: v128.load64_lane/store64_lane text-form encoding

### Added

- `SimdOpKind::Load64Lane`/`Store64Lane` joined the existing
  `Load8Lane`/`Store8Lane`/`Load16Lane`/`Store16Lane`/`Load32Lane`/
  `Store32Lane` match arm (unchanged otherwise) in both
  `encode_stream_instr` and `encode_flat_instr` -- the encoding shape
  PR44 built (memarg first, then the raw lane-index byte) is IDENTICAL
  across the 8-bit, 16-bit, 32-bit, and 64-bit pairs; only
  `simd_op.sub_opcode` differs, already captured generically. Confirmed
  against the pinned `simd_load64_lane.wast`/`simd_store64_lane.wast`
  corpus's own source order (`(v128.load64_lane offset=N lane addr x)`,
  same order as the 8-bit, 16-bit, and 32-bit pairs).
- 1 new unit test:
  `v128_load64_lane_and_store64_lane_encode_the_real_sub_opcode_with_a_combined_memarg_and_lane_index`,
  covering the folded form (with explicit `offset=`) for both opcodes
  and the flat/stream form for one, verifying the exact byte sequence
  (sub-opcode `0x57`/`0x5B`, align, offset, THEN the raw lane-index
  byte). Closes the entire lane-load/store family's text-form encoding
  coverage (PR44-47).

## 0.1.67 — 2026-08-25 — SIMD PR46: v128.load32_lane/store32_lane text-form encoding

### Added

- `SimdOpKind::Load32Lane`/`Store32Lane` joined the existing
  `Load8Lane`/`Store8Lane`/`Load16Lane`/`Store16Lane` match arm
  (unchanged otherwise) in both `encode_stream_instr` and
  `encode_flat_instr` -- the encoding shape PR44 built (memarg first,
  then the raw lane-index byte) is IDENTICAL across the 8-bit, 16-bit,
  and 32-bit pairs; only `simd_op.sub_opcode` differs, already captured
  generically. Confirmed against the pinned `simd_load32_lane.wast`/
  `simd_store32_lane.wast` corpus's own source order
  (`(v128.load32_lane offset=N lane addr x)`, same order as the 8-bit
  and 16-bit pairs).
- 1 new unit test:
  `v128_load32_lane_and_store32_lane_encode_the_real_sub_opcode_with_a_combined_memarg_and_lane_index`,
  covering the folded form (with explicit `offset=`) for both opcodes
  and the flat/stream form for one, verifying the exact byte sequence
  (sub-opcode `0x56`/`0x5A`, align, offset, THEN the raw lane-index
  byte).

## 0.1.66 — 2026-08-25 — SIMD PR45: v128.load16_lane/store16_lane text-form encoding

### Added

- `SimdOpKind::Load16Lane`/`Store16Lane` joined the existing
  `Load8Lane`/`Store8Lane` match arm (unchanged otherwise) in both
  `encode_stream_instr` and `encode_flat_instr` -- the encoding shape
  PR44 built (memarg first, then the raw lane-index byte) is IDENTICAL
  between the 8-bit and 16-bit pairs; only `simd_op.sub_opcode` differs,
  already captured generically. Confirmed against the pinned
  `simd_load16_lane.wast`/`simd_store16_lane.wast` corpus's own source
  order (`(v128.load16_lane offset=N lane addr x)`, same order as the
  8-bit pair).
- 1 new unit test:
  `v128_load16_lane_and_store16_lane_encode_the_real_sub_opcode_with_a_combined_memarg_and_lane_index`,
  covering the folded form (with explicit `offset=`) for both opcodes
  and the flat/stream form for one, verifying the exact byte sequence
  (sub-opcode `0x55`/`0x59`, align, offset, THEN the raw lane-index
  byte).

## 0.1.65 — 2026-08-25 — SIMD PR44: v128.load8_lane/store8_lane text-form encoding

### Added

- New `SimdOpKind::Load8Lane | Store8Lane` match arm in both
  `encode_stream_instr` and `encode_flat_instr` -- a GENUINELY NEW
  encoder shape, distinct from the existing memarg-only arm
  (`Load`/`Store`/etc.) and the existing lane-index-only arm
  (`ExtractLane`/`ReplaceLane`): this carries BOTH a memarg AND a
  lane-index immediate, in that order (memarg first), confirmed against
  the pinned `simd_load8_lane.wast`/`simd_store8_lane.wast` corpus's own
  source order (`(v128.load8_lane offset=N lane addr x)`). In folded
  form: memarg tokens lead, the lane-index literal comes next, the two
  operand expressions (address, existing v128) trail. In flat/stream
  form: the two operands are already emitted by preceding instructions,
  so only the memarg tokens and the trailing lane-index literal need
  reading.
- 1 new unit test:
  `v128_load8_lane_and_store8_lane_encode_the_real_sub_opcode_with_a_combined_memarg_and_lane_index`,
  covering the folded form (with explicit `offset=`) for both opcodes
  and the flat/stream form for one, verifying the exact byte sequence
  (sub-opcode, align, offset, THEN the raw lane-index byte).

## 0.1.64 — 2026-08-25 — SIMD PR42: v128.load_extend family text-form encoding

### Added

- 6 new `SimdOpKind` variants (`Load8x8S`/`Load8x8U`/`Load16x4S`/
  `Load16x4U`/`Load32x2S`/`Load32x2U`) joined the existing
  memarg-encoding match arm in both `encode_stream_instr` and
  `encode_flat_instr` alongside `Load`/`Store`/the `load_splat`/
  `load_zero` families -- identical `parse_memarg` (align, offset)
  handling, no explicit leading memidx token support (memory 0 only,
  same scope as `v128.load`/`v128.store`). Only the sub-opcode value
  differs between all fourteen kinds sharing this arm now.
- 1 new unit test:
  `v128_load_extend_family_encodes_the_real_sub_opcodes_with_a_memarg`,
  covering the folded form with an explicit `offset=` attribute for all
  6 opcodes plus the flat/stream form for one, mirroring
  `v128_load_zero_family_encodes_the_real_sub_opcodes_with_a_memarg`.

## 0.1.63 — 2026-08-25 — SIMD PR41: v128.loadN_zero family text-form encoding

### Added

- 2 new `SimdOpKind` variants (`Load32Zero`/`Load64Zero`) joined the
  existing memarg-encoding match arm in both `encode_stream_instr` and
  `encode_flat_instr` alongside `Load`/`Store`/the `load_splat` family
  -- identical `parse_memarg` (align, offset) handling, no explicit
  leading memidx token support (memory 0 only, same scope as
  `v128.load`/`v128.store`). Only the sub-opcode value differs between
  all eight kinds sharing this arm now.
- 1 new unit test:
  `v128_load_zero_family_encodes_the_real_sub_opcodes_with_a_memarg`,
  covering the folded form with an explicit `offset=` attribute for both
  opcodes plus the flat/stream form for one, mirroring
  `v128_load_splat_family_encodes_the_real_sub_opcodes_with_a_memarg`.

## 0.1.62 — 2026-08-25 — SIMD PR40: v128.loadN_splat family text-form encoding

### Added

- 4 new `SimdOpKind` variants (`Load8Splat`/`Load16Splat`/`Load32Splat`/
  `Load64Splat`) joined the existing memarg-encoding match arm in both
  `encode_stream_instr` and `encode_flat_instr` alongside `Load`/`Store`
  -- identical `parse_memarg` (align, offset) handling, no explicit
  leading memidx token support (memory 0 only, same scope as
  `v128.load`/`v128.store`). Only the sub-opcode value differs between
  all six kinds sharing this arm now.
- 1 new unit test:
  `v128_load_splat_family_encodes_the_real_sub_opcodes_with_a_memarg`,
  covering the folded form with an explicit `offset=` attribute for all
  4 opcodes plus the flat/stream form for one, mirroring
  `v128_load_and_store_encode_the_real_sub_opcodes_with_a_memarg`.

## 0.1.61 — 2026-08-25 — SIMD widen PR39: f32x4/f64x2 rounding family text-form encoding

### Added

- 8 new `SimdOpKind` variants (`CeilF32x4`/`FloorF32x4`/`TruncF32x4`/
  `NearestF32x4`/`CeilF64x2`/`FloorF64x2`/`TruncF64x2`/`NearestF64x2`)
  joined the existing no-immediate encoding arm in both
  `encode_stream_instr` and `encode_flat_instr` -- all 8 are UNARY with
  no immediate beyond the opcode bytes themselves, same encoding shape
  as `AbsF32x4`/`AbsF64x2`/etc. `f64x2.nearest`'s `0x94` sub-opcode is
  the one entry in this family that encodes as a real 2-byte LEB128
  sequence (`[0xFD, 0x94, 0x01]`); the other 7 are single-byte-safe.
- 2 new unit tests: `f32x4_rounding_family_encodes_the_real_single_
  byte_leb128_sub_opcodes` and `f64x2_rounding_family_encodes_ceil_
  floor_trunc_single_byte_and_nearest_two_byte`, each covering both the
  folded and flat text forms.

## 0.1.60 — 2026-08-24 — SIMD widen PR38: i8x16.shuffle text-form encoding (task #229-231)

### Added

- New `SimdOpKind::Shuffle` match arms in both `encode_stream_instr`
  (flat/stream form) and `encode_flat_instr` (folded form) -- the real
  WAT grammar, confirmed against the vendored `simd_lane.wast` corpus
  itself (`(i8x16.shuffle 0 1 2 ... 15 (local.get 0) (local.get 1))`),
  puts all 16 lane-index literals BEFORE the two v128 operand
  expressions, same "immediates lead, operands trail" convention as
  `ExtractLane*`/`ReplaceLane*`, just 16 immediates instead of 1. In
  folded form the 16 leading atoms are `args[0..16]`, the two operand
  expressions recurse through `encode_instr_list` as `args[16..]`; in
  flat/stream form the 16 trailing atoms are read directly (the two
  operands are already emitted onto `out` by whatever preceding
  instructions produced them), consuming exactly 16 tokens. Each byte's
  own `0..=31` range check is deferred to `wasm-validator` (validation-
  time, not this parser's job), reusing the existing `parse_lane_index`
  digit grammar (decimal/hex/underscore) for each of the 16 literals.
- 2 new tests: one confirming both forms encode the 16-byte immediate
  in exact source order for the identity case, one confirming the same
  for a genuinely mixed (non-monotonic) immediate, matching the shape of
  immediates the real `simd_lane.wast` corpus's `v8x16_shuffle-3`/`-4`
  functions actually use.

## 0.1.59 — 2026-08-24 — SIMD widen PR37: extract_lane/replace_lane family, remaining shapes + real lane-index literal grammar (task #226-228)

### Added

- The 10 new opcodes join the existing lane-immediate encoding arms in
  both `encode_stream_instr` (flat form: lane index trails) and
  `encode_folded_instr` (folded form: lane index leads, operands
  trail): `ExtractLaneI16x8S`/`ExtractLaneI16x8U`/`ExtractLaneI64x2`/
  `ExtractLaneF32x4`/`ExtractLaneF64x2` join the `ExtractLane`/
  `ExtractLaneI8x16S`/`ExtractLaneI8x16U` arm; `ReplaceLaneI16x8`/
  `ReplaceLaneI32x4`/`ReplaceLaneI64x2`/`ReplaceLaneF32x4`/
  `ReplaceLaneF64x2` join the `ReplaceLaneI8x16` arm. Both arms' bodies
  were already shape-agnostic (the encoding mechanics don't depend on
  lane width or scalar type), so no NEW encoding logic was needed --
  only widening the `|`-pattern match.

### Changed

- **`parse_lane_index` now supports the real WAT lane-index literal
  grammar** (decimal or `0x`-hex, `_`-separated), not just a plain
  `str::parse::<u8>()`. New `numeric::parse_lane_index` -- deliberately
  NOT built on `parse_int_magnitude`'s sign-stripping (correct for
  `parse_i32`/`parse_u32`'s dual signed/unsigned spelling, WRONG here):
  a lane index's own WAT grammar (`laneidx`, a plain unsigned token)
  does not allow a leading `+`/`-` sign AT ALL, confirmed against the
  real upstream `simd_lane.wast` corpus, which vendors BOTH directions
  of this exact distinction in the same file -- plain hex/underscore/
  leading-zero-decimal literals (`0x0f`, `0x0_7`, `03`) must be
  ACCEPTED, while the same literals with an explicit `+` prefix
  (`+0x0f`, `+03`) must be REJECTED as malformed. This fix also
  retroactively unlocks a previously-unbuildable module in the
  already-vendored `simd_splat.wast` (a lane-index literal in a form
  the old decimal-only parser couldn't parse) -- see `wasm-conformance`'s
  own CHANGELOG/NOTICE for the real before/after pass-rate numbers.
- New tests: folded/flat encoding for all 10 new opcodes at their valid
  lane-index extremes; hex/underscore/leading-zero-decimal lane-index
  literal parsing; leading `+`/`-` sign rejection.

## 0.1.58 — 2026-08-24 — SIMD widen PR36: i64x2.extend_low/high_i32x4_s/u text-form (task #223-225)

### Added

- `SimdOpKind::ExtendLowI32x4S`/`ExtendHighI32x4S`/`ExtendLowI32x4U`/
  `ExtendHighI32x4U` join the shared "no immediate beyond the opcode
  byte itself" SIMD dispatch arm (already used for `ExtendLowI16x8S`/
  etc., PR26) in both the folded (`encode_stream_instr`) and flat
  (`encode_flat_instr`) instruction encoders -- verified byte-identical
  at both call sites before editing. All four sub-opcodes are looked up
  by name from `wasm_opcodes::SIMD_OPS` (data-driven, via
  `get_simd_op_by_name`), so no new encoding logic was needed. All four
  sub-opcode values (`0xC7`/`0xC8`/`0xC9`/`0xCA`) are `>= 0x80`, so each
  encodes as a real 2-byte LEB128 sequence (`[0xFD, byte, 0x01]`), same
  as `ExtendLowI16x8S`/etc. (PR26) before them.
- New test:
  `simd_i64x2_extend_low_high_family_encodes_the_real_two_byte_leb128_sub_opcodes`

## 0.1.57 — 2026-08-24 — SIMD widen PR35: f64x2.abs/min/max/pmin/pmax text-form (task #220-222)

### Added

- `SimdOpKind::AbsF64x2`/`MinF64x2`/`MaxF64x2`/`PminF64x2`/`PmaxF64x2`
  join the shared "no immediate beyond the opcode byte itself" SIMD
  dispatch arm (already used for `AbsF32x4`/`MinF32x4`/`MaxF32x4`/etc.)
  in both the folded (`encode_stream_instr`) and flat
  (`encode_flat_instr`) instruction encoders -- verified byte-identical
  at both call sites before editing. All five sub-opcodes are looked up
  by name from `wasm_opcodes::SIMD_OPS` (data-driven, via
  `get_simd_op_by_name`). All five sub-opcode values (`0xEC`/`0xF4`/
  `0xF5`/`0xF6`/`0xF7`) are `>= 0x80`, so each encodes as a real 2-byte
  LEB128 sequence (`[0xFD, byte, 0x01]`), same as `f64x2.neg`/`div`
  (PR31) before them.
- New test:
  `f64x2_abs_min_max_pmin_pmax_family_encodes_the_real_two_byte_leb128_sub_opcodes`

## 0.1.56 — 2026-08-24 — SIMD widen PR34: f32x4.max/pmin/pmax text-form (task #217-219)

### Added

- `SimdOpKind::MaxF32x4`/`PminF32x4`/`PmaxF32x4` join the shared "no
  immediate beyond the opcode byte itself" SIMD dispatch arm (already
  used for `MinF32x4`/`MulF32x4`/etc.) in both the folded
  (`encode_stream_instr`) and flat (`encode_flat_instr`) instruction
  encoders -- verified byte-identical at both call sites before editing.
  All three sub-opcodes are looked up by name from
  `wasm_opcodes::SIMD_OPS` (data-driven, via `get_simd_op_by_name`).
  All three sub-opcode values (`0xE9`/`0xEA`/`0xEB`) are `>= 0x80`, so
  each encodes as a real 2-byte LEB128 sequence (`[0xFD, byte, 0x01]`),
  same as `f32x4.min` (PR19) before them.
- New test:
  `f32x4_max_pmin_pmax_family_encodes_the_real_two_byte_leb128_sub_opcodes`

## 0.1.55 — 2026-08-24 — SIMD widen PR33: i8x16/i16x8 add_sat/sub_sat text-form (task #214-216)

### Added

- `SimdOpKind::AddSatI8x16S`/`AddSatI8x16U`/`SubSatI8x16S`/
  `SubSatI8x16U`/`AddSatI16x8S`/`AddSatI16x8U`/`SubSatI16x8S`/
  `SubSatI16x8U` join the shared "no immediate beyond the opcode byte
  itself" SIMD dispatch arm (already used for `NarrowI16x8S`/`_U`/etc.)
  in both the folded (`encode_stream_instr`) and flat
  (`encode_flat_instr`) instruction encoders -- verified byte-identical
  at both call sites before editing. All eight sub-opcodes are looked up
  by name from `wasm_opcodes::SIMD_OPS` (data-driven, via
  `get_simd_op_by_name`). The `i8x16` quartet (`0x6F`/`0x70`/`0x72`/
  `0x73`) is all `< 0x80`, so each encodes as a SINGLE byte (`[0xFD,
  byte]`); the `i16x8` quartet (`0x8F`/`0x90`/`0x92`/`0x93`) is all
  `>= 0x80`, so each encodes as a real 2-byte LEB128 sequence (`[0xFD,
  byte, 0x01]`).
- New test: `simd_sat_add_sub_family_encodes_the_real_sub_opcodes`
  (folded and flat forms, all eight sub-opcodes).

## 0.1.54 — 2026-08-24 — SIMD widen PR32: f64x2 eq/ne/lt/gt/le/ge text-form (task #211-213)

### Added

- `SimdOpKind::EqF64x2`/`NeF64x2`/`LtF64x2`/`GtF64x2`/`LeF64x2`/`GeF64x2`
  join the shared "no immediate beyond the opcode byte itself" SIMD
  dispatch arm (already used for `EqF32x4`/`AddF64x2`/etc.) in both the
  folded (`encode_stream_instr`) and flat (`encode_flat_instr`)
  instruction encoders -- verified byte-identical at both call sites
  before editing. All six sub-opcodes are looked up by name from
  `wasm_opcodes::SIMD_OPS` (data-driven, via `get_simd_op_by_name`).
  Same as PR30's six entries, all six sub-opcode values are `< 0x80`,
  so each encodes as a SINGLE byte (`[0xFD, byte]`, no continuation
  byte).
- New test:
  `f64x2_cmp_family_encodes_the_real_single_byte_leb128_sub_opcodes`
  (folded and flat forms, all six sub-opcodes).

## 0.1.53 — 2026-08-24 — SIMD widen PR31: f64x2 neg/sqrt/add/sub/mul/div text-form (task #208-210)

### Added

- `SimdOpKind::NegF64x2`/`SqrtF64x2`/`AddF64x2`/`SubF64x2`/`MulF64x2`/
  `DivF64x2` join the shared "no immediate beyond the opcode byte
  itself" SIMD dispatch arm (already used for
  `NegF32x4`/`AddF32x4`/etc.) in both the folded
  (`encode_stream_instr`) and flat (`encode_flat_instr`) instruction
  encoders -- verified byte-identical at both call sites before
  editing. All six sub-opcodes are looked up by name from
  `wasm_opcodes::SIMD_OPS` (data-driven, via `get_simd_op_by_name`).
  Same as PR29's five entries, all six sub-opcode values are `>= 0x80`,
  so each encodes as a real 2-byte LEB128 sequence.
- New test:
  `f64x2_arith_family_encodes_the_real_two_byte_leb128_sub_opcodes`
  (folded and flat forms, all six sub-opcodes).

## 0.1.52 — 2026-08-24 — SIMD widen PR30: f32x4 eq/ne/lt/gt/le/ge text-form (task #205-207)

### Added

- `SimdOpKind::EqF32x4`/`NeF32x4`/`LtF32x4`/`GtF32x4`/`LeF32x4`/`GeF32x4`
  join the shared "no immediate beyond the opcode byte itself" SIMD
  dispatch arm (already used for `AddF32x4`/`MulF32x4`/`EqI64x2`/etc.)
  in both the folded (`encode_stream_instr`) and flat
  (`encode_flat_instr`) instruction encoders -- verified byte-identical
  at both call sites before editing. All six sub-opcodes are looked up
  by name from `wasm_opcodes::SIMD_OPS` (data-driven, via
  `get_simd_op_by_name`), so no separate name-to-encoding table was
  needed. Unlike PR29's five entries (all 2-byte LEB128), these six
  sub-opcode values are all `< 0x80`, so each encodes as a single byte.
- New test: `f32x4_cmp_family_encodes_the_real_single_byte_leb128_sub_opcodes`
  (folded and flat forms, all six sub-opcodes).

## 0.1.51 — 2026-08-24 — SIMD widen PR29: f32x4 add/sub/div/neg/sqrt text-form (task #202-204)

### Added

- `SimdOpKind::NegF32x4`/`SqrtF32x4`/`AddF32x4`/`SubF32x4`/`DivF32x4`
  join the shared "no immediate beyond the opcode byte itself" SIMD
  dispatch arm (already used for `AbsF32x4`/`MulF32x4`/`MinF32x4`/etc.)
  in both the folded (`encode_stream_instr`) and flat
  (`encode_flat_instr`) instruction encoders -- verified byte-identical
  at both call sites before editing. All five sub-opcodes are looked up
  by name from `wasm_opcodes::SIMD_OPS` (data-driven, via
  `get_simd_op_by_name`), so no separate name-to-encoding table was
  needed.
- New test: `f32x4_arith_family_encodes_the_real_two_byte_leb128_sub_opcodes`
  (folded and flat forms, all five sub-opcodes).

## 0.1.50 — 2026-08-19 — SIMD widen PR28: promote/demote/convert_low family text-form, v128 NaN-class expected lanes (task #199-201)

### Added

- `SimdOpKind::DemoteF64x2Zero`/`PromoteLowF32x4`/`ConvertLowI32x4S`/
  `ConvertLowI32x4U` join the shared "no immediate beyond the opcode
  byte itself" SIMD dispatch arm (already used for `AddI8x16`/
  `ExtendLowI8x16S`/`NarrowI16x8S`/etc.) in both the folded
  (`encode_stream_instr`) and flat (`encode_flat_instr`) instruction
  encoders -- verified byte-identical at both call sites before
  editing, per this campaign's own documented past gotcha. All four
  sub-opcodes are looked up by name from `wasm_opcodes::SIMD_OPS`
  (data-driven, via `get_simd_op_by_name`), so no separate
  name-parsing change was needed beyond the two match-arm additions.
  All four are UNARY -- same no-immediate encoding shape as `ExtendLow/
  HighI8x16S/_U` (PR26), unlike PR27's BINARY "narrow" family.
- 1 new test confirming all 4 opcodes encode their real sub-opcodes --
  `0x5E`/`0x5F` as single-byte LEB128 (`< 128`), `0xFE`/`0xFF` as real
  2-byte LEB128 (`>= 128`) -- in both folded and flat/stream syntax.
- **New parser feature, found while vendoring `simd_conversions.wast`
  (not opcode-related):** `Expected::V128F32x4`/`V128F64x2` and their
  `F32LaneExpected`/`F64LaneExpected` per-lane representations. A
  `v128.const f32x4`/`f64x2` used as an `assert_return` EXPECTED value
  can now mix exact literal lanes with `nan:canonical`/
  `nan:arithmetic` NaN-CLASS lanes -- something the pre-existing
  byte-exact `ConstValue::V128` representation genuinely cannot
  express (it has no way to say "this lane must be SOME NaN, exact
  payload unconstrained", the same problem the scalar
  `Expected::NanCanonicalF32`/etc. variants already solve for a whole
  `f32`/`f64` result, just never extended to individual v128 lanes
  before). `parse_expected`'s `v128.const` match arm now checks
  whether ANY lane token is a NaN-class token before choosing this new
  path -- a `v128.const` with NO NaN-class lanes is untouched, still
  routing through the original byte-exact path (zero regression risk,
  confirmed by a dedicated test). 3 new tests: mixed exact/NaN-class
  `f32x4` lanes, all-NaN-class `f64x2` lanes, and the no-regression
  proof for exact-only lanes.

### Notes

- `simd_conversions.wast`'s own two failing assert_return directives
  before this fix were actually a hard SCRIPT PARSE FAILURE, not a
  per-directive grading gap: `(v128.const f64x2 nan:canonical
  nan:canonical)` used as an expected value hit
  `numeric::parse_f64_bits`, which correctly rejects `nan:canonical`
  as not a valid CONCRETE bit pattern (it isn't one -- that's the
  whole point of a NaN class) and returns `InvalidNumericLiteral`,
  aborting the ENTIRE script's parse (this crate's parser design: one
  bad top-level form fails the whole file, see this crate's own module
  doc comments). Real modules never legitimately use `nan:canonical`/
  `nan:arithmetic` as an ACTUAL instruction operand (only the WAST
  *script* syntax allows it, and only in `assert_return`'s own expected
  position) -- so this fix lives entirely in `script.rs`'s
  `parse_expected`, not in `module.rs`'s `parse_v128_const` (which
  stays exact-bytes-only, correctly, since real code needs concrete
  values).
- **Campaign complete, corpus now vendored.** With this PR, all 16
  opcodes across PR26 (`extend`)/PR27 (`narrow`)/PR28
  (`promote`/`demote`/`convert_low`) exist, and the NaN-class-lane
  parser gap above is fixed, so `wasm-conformance` now vendors
  `simd_conversions.wast` for the first time -- 100% pass, 280/280
  directives. See `wasm-conformance`'s own CHANGELOG.

## 0.1.49 — 2026-08-19 — SIMD widen PR27: narrow saturating family text-form (task #196-198)

### Added

- `SimdOpKind::NarrowI16x8S`/`NarrowI16x8U`/`NarrowI32x4S`/
  `NarrowI32x4U` join the shared "no immediate beyond the opcode byte
  itself" SIMD dispatch arm (already used for `AddI8x16`/`Swizzle`/
  `ExtendLowI8x16S`/etc.) in both the folded (`encode_stream_instr`)
  and flat (`encode_flat_instr`) instruction encoders -- verified
  byte-identical at both call sites before editing, per this
  campaign's own documented past gotcha. All four sub-opcodes are
  looked up by name from `wasm_opcodes::SIMD_OPS` (data-driven, via
  `get_simd_op_by_name`), so no separate name-parsing change was
  needed beyond the two match-arm additions. Unlike `ExtendLow/
  HighI8x16S/_U` (PR26, UNARY), the "narrow" family is BINARY -- two
  v128 operands -- but has the identical no-immediate encoding shape.
- 1 new test confirming all 4 `narrow` opcodes encode their real
  sub-opcodes -- `0x65`/`0x66` as single-byte LEB128 (`< 128`),
  `0x85`/`0x86` as real 2-byte LEB128 (`>= 128`) -- in both folded and
  flat/stream syntax.

### Notes

- **Staged campaign, no corpus vendoring yet.** These 4 opcodes are the
  second of a 3-PR sequence (`extend_low`/`high` done in PR26, `narrow`
  here, `promote`/`demote`/`convert_low` in a future PR) needed to
  unlock the upstream `simd_conversions.wast` corpus file. This PR is
  opcode-only.

## 0.1.48 — 2026-08-19 — SIMD widen PR26: extend_low/high family text-form (task #193-195)

### Added

- `SimdOpKind::ExtendLowI8x16S`/`ExtendHighI8x16S`/`ExtendLowI8x16U`/
  `ExtendHighI8x16U`/`ExtendLowI16x8S`/`ExtendHighI16x8S`/
  `ExtendLowI16x8U`/`ExtendHighI16x8U` join the shared "no immediate
  beyond the opcode byte itself" SIMD dispatch arm (already used for
  `ExtaddPairwiseI8x16S`/`_U`/`ExtaddPairwiseI16x8S`/`_U`) in both the
  folded (`encode_stream_instr`) and flat (`encode_flat_instr`)
  instruction encoders -- verified byte-identical at both call sites
  before editing, per this campaign's own documented past gotcha. All
  eight sub-opcodes are looked up by name from
  `wasm_opcodes::SIMD_OPS` (data-driven, via `get_simd_op_by_name`), so
  no separate name-parsing change was needed beyond the two match-arm
  additions.
- 1 new test confirming all 8 `extend_low`/`high` opcodes encode their
  real 2-byte LEB128 sub-opcodes (all `>= 128`) in both folded and
  flat/stream syntax.

### Notes

- **Staged campaign, no corpus vendoring yet.** Part of the 16-opcode
  set (`extend_low`/`high` here, `narrow` and `promote`/`demote`/
  `convert_low` in future PRs) needed to unlock the upstream
  `simd_conversions.wast` corpus file. This PR is opcode-only.

## 0.1.47 — 2026-08-19 — SIMD widen PR25: i32x4.trunc_sat_f64x2_s/u_zero text-form (task #190-192)

### Added

- `SimdOpKind::TruncSatF64x2SZero`/`TruncSatF64x2UZero` join the shared
  "no immediate beyond the opcode byte itself" SIMD dispatch arm
  (already used for `TruncSatF32x4S`/`_U`/`ConvertI32x4S`/`_U`) in both
  the folded (`encode_stream_instr`) and flat (`encode_flat_instr`)
  instruction encoders -- verified byte-identical at both call sites
  before editing, per this campaign's own documented past gotcha. Both
  sub-opcodes are looked up by name from `wasm_opcodes::SIMD_OPS`
  (data-driven, via `get_simd_op_by_name`), so no separate name-parsing
  change was needed beyond the two match-arm additions.
- 1 new test confirming `i32x4.trunc_sat_f64x2_s_zero`/`_u_zero` each
  encode their real 2-byte LEB128 sub-opcode (`0xFC, 0x01`/`0xFD, 0x01`,
  both `>= 128`) in both folded and flat syntax.

## 0.1.46 — 2026-08-19 — SIMD widen PR22: i16x8.q15mulr_sat_s text-form (task #183-185)

### Added

- `SimdOpKind::Q15mulrSatI16x8S` joins the shared "no immediate beyond
  the opcode byte itself" SIMD dispatch arm (already used for
  `AddI16x8`/`SubI16x8`/`MulI16x8`/`NegI16x8`) in both the folded and
  flat instruction encoders -- verified byte-identical at both call
  sites before using `replace_all`, per this campaign's own documented
  past gotcha.
- 1 new test confirming `i16x8.q15mulr_sat_s` encodes its real 2-byte
  LEB128 sub-opcode (`0x82, 0x01`, `>= 128`) in both folded and flat
  syntax.

## 0.1.45 — 2026-08-19 — SIMD widen PR21: i64x2.extmul_i32x4 widening-multiply text-form (task #180-182)

### Added

- `SimdOpKind::ExtmulLowI64x2S`/`ExtmulHighI64x2S`/`ExtmulLowI64x2U`/
  `ExtmulHighI64x2U` join the shared "no immediate beyond the opcode
  byte itself" SIMD dispatch arm (already used for
  `ExtmulLowI16x8S`/etc.) in both the folded and flat instruction
  encoders -- verified byte-identical at both call sites before using
  `replace_all`, per this campaign's own documented past gotcha. The
  third and final "extmul" rung's text-form support.
- 1 new test covering all 4 new ops, confirming each encodes its real
  2-byte LEB128 sub-opcode (`0xDC`/`0xDD`/`0xDE`/`0xDF`, all `>= 128`).

## 0.1.44 — 2026-08-19 — SIMD widen PR20: i32x4<->f32x4 trunc_sat/convert text-form (task #177-179)

### Added

- `SimdOpKind::TruncSatF32x4S`/`TruncSatF32x4U`/`ConvertI32x4S`/
  `ConvertI32x4U` join the shared "no immediate beyond the opcode byte
  itself" SIMD dispatch arm (already used for `f32x4.abs`/`mul`/`min`)
  in both the folded and flat instruction encoders -- these change the
  lane TYPE at the runtime/type-checker level, but that distinction is
  invisible at this encoding shape, same as PR19's own additions.
- 1 new test covering all 4 new ops (folded + flat), confirming each
  encodes its real 2-byte LEB128 sub-opcode (`0xF8`/`0xF9`/`0xFA`/
  `0xFB`, all `>= 128`).

## 0.1.43 — 2026-08-19 — SIMD widen PR19: f32x4.abs/f32x4.mul/f32x4.min text-form (task #174-176)

### Added

- `SimdOpKind::AbsF32x4`/`MulF32x4`/`MinF32x4` join the shared "no
  immediate beyond the opcode byte itself" SIMD dispatch arm (already
  used for `i8x16.add`/`Swizzle`/etc.) in both the folded and flat
  instruction encoders -- the unary/binary distinction only matters at
  the type-checker/runtime level, not at this encoding shape.
- 1 new test covering all 3 new ops (folded + flat), confirming each
  encodes its real 2-byte LEB128 sub-opcode (`0xE0`/`0xE6`/`0xE8`, all
  `>= 128`) -- the first SIMD widen PR whose new opcodes all need the
  genuine multi-byte LEB128 path rather than the single-byte
  happy-path most of this table's opcodes take.

## 0.1.42 — 2026-08-19 — SIMD widen PR18: i8x16 swizzle/extract_lane_s/extract_lane_u/replace_lane text-form (task #171-173)

### Added

- `SimdOpKind::Swizzle` joins the shared "no immediate beyond the
  opcode byte itself" SIMD dispatch arm (already used for
  `i8x16.add`/etc.) in both the folded and flat instruction encoders.
- `SimdOpKind::ExtractLaneI8x16S`/`ExtractLaneI8x16U` join the
  existing `ExtractLane` arm in both encoders -- same "lane index
  leads in folded form (`(i8x16.extract_lane_s <lane> <v128-expr>)`),
  trails in flat/stream form" shape as `i32x4.extract_lane`, just at
  `i8x16`'s 0-15 lane range instead of `i32x4`'s 0-3.
- New `SimdOpKind::ReplaceLaneI8x16` arm in both encoders: the
  genuinely new shape at the runtime/type level (see `wasm-opcodes`'s
  own CHANGELOG entry), but mechanically the SAME encoding call
  `ExtractLane*` makes -- lane index leads/trails, `encode_instr_list`
  handles the (now two, not one) trailing operand expressions in
  source order regardless of count.
- 3 new encoding tests: `i8x16.swizzle` (folded + flat, no immediate);
  `i8x16.extract_lane_s`/`_u` (folded + flat, lane index leading/
  trailing, covering both ends of the 0-15 range); `i8x16.replace_lane`
  (folded + flat, confirming both operands encode in source order
  around the trailing lane-index byte).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.41 — 2026-08-19 — SIMD: float splat family text-form (task #168-170)

### Added

- `SimdOpKind::SplatF32x4`/`SplatF64x2` join the shared "no immediate
  beyond the opcode byte itself" SIMD dispatch arm in both the folded
  and flat instruction encoders -- the mixed `f32`/`f64` operand
  TYPES are invisible to this encoder (a type-checker concern -- see
  `wasm-validator`), same as every prior SIMD op.
- Dedicated encoding test proving the real single-byte LEB128 bytes
  for both (`[0xFD, 0x13]`/`[0xFD, 0x14]`), in both folded and flat
  form.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.40 — 2026-08-19 — SIMD: splat family text-form; NaN payload underscore fix (task #165-167)

### Added

- `SimdOpKind::SplatI8x16`/`SplatI16x8`/`SplatI64x2` join the shared
  "no immediate beyond the opcode byte itself" SIMD dispatch arm
  (already used for `i32x4.splat` and every other no-operand SIMD op)
  in both the folded and flat instruction encoders. Name-to-op
  resolution is fully generic (`get_simd_op_by_name`), so no separate
  name-matching logic was needed.
- Dedicated encoding test proving the real single-byte LEB128 bytes
  for all three (`[0xFD, 0x0F]`/`[0xFD, 0x10]`/`[0xFD, 0x12]`), in
  both folded and flat form.

### Fixed

- Real corpus bug found while vendoring `simd_splat.wast` (see
  `wasm-conformance`'s own CHANGELOG): `nan:0x7f_ffff` (a `_` digit
  separator inside a NaN payload literal) made the WHOLE script fail
  to parse, since `parse_f32_bits`/`parse_f64_bits`'s `nan:0x<payload>`
  arm called `from_str_radix` directly on the raw payload text instead
  of routing it through `strip_underscores` first, unlike every other
  numeric literal path in `numeric.rs`. 2 new regression tests confirm
  underscored payloads now parse correctly for both `f32` and `f64`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## 0.1.39 — 2026-08-18 — SIMD: v128.load/v128.store text-form (task #162-164)

### Added

- Both the folded (`encode_flat_instr`) and flat (`encode_stream_instr`)
  SIMD dispatch arms gain a new `SimdOpKind::Load | SimdOpKind::Store`
  case -- the FIRST SIMD ops this crate encodes that carry a `memarg`
  immediate (`offset=`/`align=` attributes). Reuses the existing
  `parse_memarg` helper the scalar `i32.load`/etc. family already
  uses, then emits `0xFD`, the sub-opcode LEB128, and the memarg's
  align/offset LEB128s -- same encoding shape as the scalar ops, just
  behind the `0xFD` SIMD prefix. Verified via a dedicated test covering
  the folded form (with an explicit `offset=` attribute, for both
  `v128.load` and `v128.store`) and the flat/stream form.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

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
