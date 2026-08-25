# Changelog

All notable changes to this package will be documented in this file.

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
