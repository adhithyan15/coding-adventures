# Changelog

All notable changes to this package will be documented in this file.

## [0.2.37] - 2026-08-24 - SIMD widen PR34: f32x4.max/pmin/pmax (task #217-219)

### Added

- 3 new `SIMD_OPS` entries: `f32x4.max` (`0xE9`), `f32x4.pmin` (`0xEA`),
  `f32x4.pmax` (`0xEB`) -- the last 3 opcodes of f32x4's arithmetic
  family, sitting immediately past the already-implemented `f32x4.min`'s
  `0xE8` (PR19) with no gap. 188 SIMD opcodes total, up from 185. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `f32x4.min` entry: `0xE9`/`0xEA`/`0xEB` run contiguously past it,
  confirmed free of collision with every existing `SIMD_OPS` entry.
- 3 new `SimdOpKind` variants: `MaxF32x4`, `PminF32x4`, `PmaxF32x4`.
  `MaxF32x4` mirrors `MinF32x4`'s WASM-spec `fmax` NaN-propagating,
  signed-zero-aware semantics exactly (the exact per-lane transplant of
  this crate's own scalar `f32.max` opcode, `0x97`). `PminF32x4`/
  `PmaxF32x4` are a DIFFERENT, deliberately SIMPLER "pseudo-min"/
  "pseudo-max" shape -- a plain IEEE-754 `<`-based conditional select
  (`pmin(a, b) = b < a ? b : a`, `pmax(a, b) = a < b ? b : a`) with NO
  NaN canonicalization: since IEEE-754 `<` is always `false` when either
  operand is NaN, `pmin`/`pmax` return the FIRST operand `a` UNCHANGED
  whenever either operand is NaN, NOT a canonicalized NaN result the way
  `MinF32x4`/`MaxF32x4` would produce -- the classic point of confusion
  porting real WASM SIMD implementations, deliberately NOT implemented
  by reusing `min`/`max`'s NaN logic.
- Table-size test updated from 185 to 188 entries. New test
  `simd_f32x4_max_pmin_pmax_have_the_real_verified_sub_opcode_values`.

### Added

- 8 new `SIMD_OPS` entries: `i8x16.add_sat_s` (`0x6F`), `i8x16.add_sat_u`
  (`0x70`), `i8x16.sub_sat_s` (`0x72`), `i8x16.sub_sat_u` (`0x73`),
  `i16x8.add_sat_s` (`0x8F`), `i16x8.add_sat_u` (`0x90`),
  `i16x8.sub_sat_s` (`0x92`), `i16x8.sub_sat_u` (`0x93`) -- the
  saturating integer add/sub family, BINARY, same pop-order/lane-count
  shape as the already-implemented `i8x16.add`/`.sub`/`i16x8.add`/
  `.sub`, except the result is CLAMPED to the lane type's range instead
  of wrapped on overflow/underflow. 185 SIMD opcodes total, up from 177.
  Each sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against every existing `SIMD_OPS`
  entry: `0x6F`/`0x70` sit immediately past `i8x16.add`'s `0x6E` with no
  gap, `0x72`/`0x73` sit immediately past `i8x16.sub`'s `0x71` with no
  gap, `0x8F`/`0x90` sit immediately past `i16x8.add`'s `0x8E` with no
  gap, `0x92`/`0x93` sit immediately past `i16x8.sub`'s `0x91` with no
  gap -- all eight confirmed free of collision with every existing
  `SIMD_OPS` entry.
- 8 new `SimdOpKind` variants: `AddSatI8x16S`, `AddSatI8x16U`,
  `SubSatI8x16S`, `SubSatI8x16U`, `AddSatI16x8S`, `AddSatI16x8U`,
  `SubSatI16x8S`, `SubSatI16x8U`.
- New test: `simd_sat_add_sub_family_has_the_real_verified_sub_opcode_values`
  (all eight sub-opcode values, plus confirming `i8x16.add`/`.sub` and
  `i16x8.add`/`.sub` still resolve to their own unchanged sub-opcodes).
  Table-size test updated 177 -> 185.

## [0.2.35] - 2026-08-24 - SIMD widen PR32: f64x2 eq/ne/lt/gt/le/ge (task #211-213)

### Added

- 6 new `SIMD_OPS` entries: `f64x2.eq` (`0x47`), `f64x2.ne` (`0x48`),
  `f64x2.lt` (`0x49`), `f64x2.gt` (`0x4A`), `f64x2.le` (`0x4B`),
  `f64x2.ge` (`0x4C`) -- the `f64x2` comparison family, a direct
  structural mirror of PR30's `f32x4` comparison family (`0x41`-`0x46`),
  at `f64x2`'s 2-lane width. 177 SIMD opcodes total, up from 171. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against every existing `SIMD_OPS`
  entry: `0x41`-`0x46` (the already-implemented `f32x4` comparison
  family) precede this run with no overlap, `v128.not` (`0x4D`, just
  above) confirms `0x47`-`0x4C` are genuinely free (also confirmed
  distinct from the unrelated `ATOMIC_OPS` table's own `0x47`-`0x4C`
  cmpxchg/xchg entries, behind a completely different `0xFE` prefix, not
  `0xFD`).
- 6 new `SimdOpKind` variants: `EqF64x2`, `NeF64x2`, `LtF64x2`,
  `GtF64x2`, `LeF64x2`, `GeF64x2`.
- New test: `simd_f64x2_cmp_family_has_the_real_verified_sub_opcode_values`
  (all six sub-opcode values, plus confirming `0x41`-`0x46` still resolve
  to `f32x4` ops and `0x4D` is still `v128.not`). Table-size test updated
  171 -> 177.

## [0.2.34] - 2026-08-24 - SIMD widen PR31: f64x2 neg/sqrt/add/sub/mul/div (task #208-210)

### Added

- 6 new `SIMD_OPS` entries: `f64x2.neg` (`0xED`), `f64x2.sqrt` (`0xEF`),
  `f64x2.add` (`0xF0`), `f64x2.sub` (`0xF1`), `f64x2.mul` (`0xF2`),
  `f64x2.div` (`0xF3`) -- a direct structural mirror of PR29's `f32x4`
  arithmetic family, at `f64x2`'s 2-lane width, plus `mul` (`f32x4.mul`
  already existed pre-PR29; `f64x2.mul` did not exist until now). 171
  SIMD opcodes total, up from 165. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked against
  every existing `SIMD_OPS` entry: `f64x2.abs` (`0xEC`, still
  unimplemented) precedes this run, `0xEE` is genuinely unassigned (same
  "real gap" shape as `f32x4`'s own `0xE2`), and `f64x2.min`/`max`
  (`0xF4`/`0xF5`, still unimplemented) sit immediately past this run
  with no overlap.
- 6 new `SimdOpKind` variants: `NegF64x2`, `SqrtF64x2`, `AddF64x2`,
  `SubF64x2`, `MulF64x2`, `DivF64x2`.
- New tests: `simd_f64x2_arith_family_has_the_real_verified_sub_opcode_values`
  (all six sub-opcode values, plus the `0xEE` gap). Table-size test
  updated 165 -> 171.

## [0.2.33] - 2026-08-24 - SIMD widen PR30: f32x4 eq/ne/lt/gt/le/ge (task #205-207)

### Added

- 6 new `SIMD_OPS` entries: `f32x4.eq` (`0x41`), `f32x4.ne` (`0x42`),
  `f32x4.lt` (`0x43`), `f32x4.gt` (`0x44`), `f32x4.le` (`0x45`),
  `f32x4.ge` (`0x46`) -- the `f32x4` comparison family, joining the
  arithmetic family PR29 just closed. 165 SIMD opcodes total, up from
  159. Each sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against every existing `SIMD_OPS`
  entry: the closest neighbors are `i32x4.eq`..`i32x4.ge_u`
  (`0x37`-`0x40`, just below) and `v128.not` (`0x4D`, just above),
  confirming `0x41`-`0x46` are genuinely free (a naive grep for that
  byte range also hits unrelated `ATOMIC_OPS`/scalar
  `numeric_i32`/`numeric_i64`/`numeric_f32`/`numeric_f64` entries behind
  DIFFERENT opcode prefixes -- verified against `SIMD_OPS` specifically).
  Unlike every `f32x4` opcode added since PR19 (all `>= 0x80`, needing a
  real 2-byte LEB128 encoding), these six values are all `< 0x80`
  (single-byte LEB128). 6 new `SimdOpKind` variants: `EqF32x4`,
  `NeF32x4`, `LtF32x4`, `GtF32x4`, `LeF32x4`, `GeF32x4`.
- New tests:
  `simd_ops_table_has_the_expected_165_entries_and_no_duplicates`
  (renamed from `_159_`, was 159) and
  `simd_f32x4_cmp_family_has_the_real_verified_sub_opcode_values`.

## [0.2.32] - 2026-08-24 - SIMD widen PR29: f32x4 add/sub/div/neg/sqrt (task #202-204)

### Added

- 5 new `SIMD_OPS` entries: `f32x4.neg` (`0xE1`), `f32x4.sqrt` (`0xE3`),
  `f32x4.add` (`0xE4`), `f32x4.sub` (`0xE5`), `f32x4.div` (`0xE7`) --
  closes the last remaining gap in `f32x4`'s core arithmetic family
  (`abs`/`mul`/`min` landed in PR19; `max`/`pmin`/`pmax` remain future
  work). 159 SIMD opcodes total, up from 154. Each sub-opcode byte
  fetched live from the SIMD proposal's own `BinarySIMD.md` and
  cross-checked against this table's own already-implemented
  neighbors: `f32x4.abs` (`0xE0`)/`f32x4.mul` (`0xE6`)/`f32x4.min`
  (`0xE8`) (all matched exactly; `0xE2` is genuinely unassigned in the
  spec, not a skipped op). 5 new `SimdOpKind` variants: `NegF32x4`,
  `SqrtF32x4`, `AddF32x4`, `SubF32x4`, `DivF32x4`.
- New tests: `simd_ops_table_has_the_expected_159_entries_and_no_duplicates`
  (was 154) and `simd_f32x4_arith_family_has_the_real_verified_sub_opcode_values`.

## [0.2.31] - 2026-08-19 - SIMD widen PR28: promote/demote/convert_low family (task #199-201)

### Added

- 4 new `SIMD_OPS` entries: `f32x4.demote_f64x2_zero` (`0x5E`),
  `f64x2.promote_low_f32x4` (`0x5F`), `f64x2.convert_low_i32x4_s`
  (`0xFE`), `f64x2.convert_low_i32x4_u` (`0xFF`) -- the
  "promote/demote/convert_low" family, the THIRD and FINAL PR of a
  3-PR sequence (PR26 "extend", 8 opcodes; PR27 "narrow", 4 opcodes;
  this PR, 4 opcodes) needed to land all 16 opcodes the upstream
  `simd_conversions.wast` corpus file's two modules bundle together.
  154 SIMD opcodes total, up from 150. Each sub-opcode byte fetched
  live from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against this table's own already-implemented neighbors: `0x5E`/
  `0x5F` sit immediately past `v128.xor`'s (`0x51`)/`v128.bitselect`'s
  (`0x52`) run with no collision; `0xFE`/`0xFF` sit immediately past
  `i32x4.trunc_sat_f64x2_u_zero`'s `0xFD` (PR25) with no gap.
- `SimdOpKind::DemoteF64x2Zero`/`PromoteLowF32x4`/`ConvertLowI32x4S`/
  `ConvertLowI32x4U`.

### Notes

- Semantics (implemented in `wasm-execution`, not this crate): all
  UNARY (pop one `v128`, push one). `f32x4.demote_f64x2_zero` reads 2
  `f64` lanes, demotes each to `f32` (plain IEEE-754 narrowing -- CAN
  overflow to +/-infinity for out-of-range magnitudes, that's expected
  behavior, not saturation), writes 4 `f32` lanes: 0-1 demoted, 2-3
  ALWAYS zero (mirrors PR25's `trunc_sat_f64x2_*_zero` zero-fill
  shape). `f64x2.promote_low_f32x4` reads 4 `f32` lanes but only uses
  the LOW 2 (indices 0-1 are promoted, 2-3 are DROPPED, never read),
  writes 2 `f64` lanes (exact, lossless widening). `f64x2.convert_low_
  i32x4_s`/`_u` read 4 `i32` lanes, use only the LOW 2 (same
  lane-DROPPING as `promote_low_f32x4`), convert each to `f64` (signed
  or unsigned-bit-pattern interpretation), write 2 `f64` lanes -- the
  reverse direction of PR25's `trunc_sat_f64x2_s/u_zero` (that went
  `f64x2` -> `i32x4` with zero-padding; this goes `i32x4` -> `f64x2`
  with lane-dropping).
- **Campaign complete, corpus now vendored.** With this PR, all 16
  opcodes across PR26/PR27/PR28 exist, so `wasm-conformance` now
  vendors `simd_conversions.wast` for the first time -- 100% pass on
  every directive (2/2 modules, 232/232 `assert_return`, 18/18
  `assert_invalid`, 30/30 `assert_malformed`), zero `NotYetSupported`.
  See `wasm-conformance`'s own CHANGELOG for the full grading detail,
  including a real parser gap this vendoring surfaced and fixed
  (`nan:canonical`/`nan:arithmetic` NaN-class lanes inside a
  `v128.const f32x4`/`f64x2` expected value, previously unsupported).

## [0.2.30] - 2026-08-19 - SIMD widen PR27: narrow saturating family (task #196-198)

### Added

- 4 new `SIMD_OPS` entries: `i8x16.narrow_i16x8_s` (`0x65`),
  `i8x16.narrow_i16x8_u` (`0x66`), `i16x8.narrow_i32x4_s` (`0x85`),
  `i16x8.narrow_i32x4_u` (`0x86`) -- the "narrow" family, the
  saturating-demote OPPOSITE of PR26's "extend" family. 150 SIMD
  opcodes total, up from 146. Each sub-opcode byte fetched live from
  the SIMD proposal's own `BinarySIMD.md` and cross-checked against
  the already-implemented `i8x16.bitmask` (`0x64`)/`i16x8.all_true`
  (`0x83`)/`i16x8.bitmask` (`0x84`) entries: `0x65`/`0x66` sit
  immediately past `i8x16.bitmask`'s `0x64` with no gap, `0x85`/`0x86`
  sit immediately past `i16x8.bitmask`'s `0x84` with no gap -- both
  confirmed free of collision with every existing `SIMD_OPS` entry.
- `SimdOpKind::NarrowI16x8S`/`NarrowI16x8U`/`NarrowI32x4S`/
  `NarrowI32x4U`.

### Notes

- Semantics (implemented in `wasm-execution`, not this crate): BINARY
  (pop TWO `v128`s, push one `v128`) -- unlike PR26's UNARY "extend"
  family. `i8x16.narrow_i16x8_s/u` reinterpret both operands as 8 `i16`
  lanes each, saturate every lane to the `i8` range (signed:
  `i8::MIN..=i8::MAX`; unsigned: `0..=u8::MAX`, a negative lane
  saturates to 0, it does NOT wrap), and concatenate: the FIRST
  operand's 8 saturated lanes become the LOW half (0-7) of the
  `i8x16` result, the SECOND operand's 8 saturated lanes become the
  HIGH half (8-15). Same pattern one lane width up for
  `i16x8.narrow_i32x4_s/u` (4 `i32` lanes per operand, saturated to
  the `i16` range, LOW 0-3 / HIGH 4-7).
- **Staged campaign, no corpus vendoring yet.** These 4 opcodes are the
  second of a 3-PR sequence (`extend_low`/`high` done in PR26,
  `narrow` here, `promote`/`demote`/`convert_low` in a future PR)
  needed to unlock the upstream `simd_conversions.wast` corpus file --
  its modules bundle all 16 together, so it can't be vendored until
  every PR in the set has landed (12 of 16 opcodes done after this
  PR). This PR is opcode-only, verified by unit tests.
- Table-size test bumped from 146 to 150 entries; new dedicated
  sub-opcode-value test added alongside the existing per-family tests.

## [0.2.29] - 2026-08-19 - SIMD widen PR26: extend_low/high family (task #193-195)

### Added

- 8 new `SIMD_OPS` entries: `i16x8.extend_low_i8x16_s` (`0x87`),
  `i16x8.extend_high_i8x16_s` (`0x88`), `i16x8.extend_low_i8x16_u`
  (`0x89`), `i16x8.extend_high_i8x16_u` (`0x8A`),
  `i32x4.extend_low_i16x8_s` (`0xA7`), `i32x4.extend_high_i16x8_s`
  (`0xA8`), `i32x4.extend_low_i16x8_u` (`0xA9`),
  `i32x4.extend_high_i16x8_u` (`0xAA`) -- the "extend" family: EXACTLY
  the lane-selection + sign/zero-extend half of the already-implemented
  `ExtmulLowI8x16S`/`ExtmulHighI8x16S`/etc. handlers, minus the
  multiply. 146 SIMD opcodes total, up from 138. Each sub-opcode byte
  fetched live from the SIMD proposal's own `BinarySIMD.md` and
  cross-checked against the already-implemented
  `i16x8.extmul_low_i8x16_s` (`0x9C`)/`i16x8.shl` (`0x8B`)/
  `i16x8.q15mulr_sat_s` (`0x82`)/`i32x4.extmul_low_i16x8_s` (`0xBC`)/
  `i32x4.all_true` (`0xA3`)/`i32x4.shl` (`0xAB`) entries (all six
  matched exactly, confirming `0x87`-`0x8A` and `0xA7`-`0xAA` are free
  gaps in their respective runs).
- `SimdOpKind::ExtendLowI8x16S`/`ExtendHighI8x16S`/`ExtendLowI8x16U`/
  `ExtendHighI8x16U`/`ExtendLowI16x8S`/`ExtendHighI16x8S`/
  `ExtendLowI16x8U`/`ExtendHighI16x8U`.

### Notes

- Semantics (implemented in `wasm-execution`, not this crate): UNARY
  (pop one `v128`, push one `v128`). `i16x8.extend_low/high_i8x16_s/u`
  reinterpret the operand as 16 `i8` lanes, take only the LOW (0-7) or
  HIGH (8-15) 8 lanes, sign- or zero-extend each to `i16`. Same pattern
  one lane width up for `i32x4.extend_low/high_i16x8_s/u` (8 `i16`
  lanes in, LOW 0-3 / HIGH 4-7 selected, extended to `i32`).
- **Staged campaign, no corpus vendoring yet.** These 8 opcodes are
  part of a 16-opcode set (`extend_low`/`high` here, `narrow` in a
  future PR, `promote`/`demote`/`convert_low` in a future PR) needed to
  unlock the upstream `simd_conversions.wast` corpus file -- its
  modules bundle all 16 together, so it can't be vendored until every
  PR in the set has landed. This PR is opcode-only, verified by unit
  tests.
- Table-size test bumped from 138 to 146 entries; new dedicated
  sub-opcode-value test added alongside the existing per-family tests.

## [0.2.28] - 2026-08-19 - SIMD widen PR25: i32x4.trunc_sat_f64x2_s/u_zero (task #190-192)

### Added

- 2 new `SIMD_OPS` entries: `i32x4.trunc_sat_f64x2_s_zero` (`0xFC`) and
  `i32x4.trunc_sat_f64x2_u_zero` (`0xFD`) -- the f64x2-source rung of the
  "_zero" `trunc_sat` family, joining the already-implemented
  `i32x4.trunc_sat_f32x4_s`/`_u` (PR20) f32x4-source pair. 138 SIMD
  opcodes total, up from 136. Each sub-opcode byte fetched live from the
  SIMD proposal's own `BinarySIMD.md` and cross-checked against the
  already-implemented `i32x4.trunc_sat_f32x4_s`/`_u` (`0xF8`/`0xF9`) and
  `f32x4.convert_i32x4_s`/`_u` (`0xFA`/`0xFB`) entries (all four matched
  exactly, confirming `0xFC`/`0xFD` sit immediately past that conversion
  family with no gap).
- `SimdOpKind::TruncSatF64x2SZero`/`TruncSatF64x2UZero`.

### Notes

- Semantics (implemented in `wasm-execution`, not this crate): UNARY,
  read the operand `v128` as 2 `f64` lanes (not 4 `f32` lanes), convert
  each to a SATURATING `i32` (signed for `_s_zero`, unsigned bit pattern
  for `_u_zero`) the same way `trunc_sat_f32x4_s`/`_u` does, and produce
  a `v128` with 4 `i32` lanes: lanes 0-1 hold the two truncated results,
  lanes 2-3 are ALWAYS zero -- hence "_zero" in the name.
- Table-size test bumped from 136 to 138 entries; new dedicated
  sub-opcode-value test added alongside the existing per-family tests.

## [0.2.27] - 2026-08-19 - SIMD widen PR22: i16x8.q15mulr_sat_s (task #183-185)

### Added

- 1 new `SIMD_OPS` entry: `i16x8.q15mulr_sat_s` (`0x82`) -- a Q15
  fixed-point ROUNDING SATURATING multiply, the first genuinely new
  SIMD op family/semantic in this table since the "extmul"
  widening-multiply arc completed in PR21. 136 SIMD opcodes total, up
  from 135. Sub-opcode `0x82` fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i16x8.neg` (`0x81`)/`i16x8.all_true` (`0x83`) entries that straddle
  it on either side -- `0x82` was the one gap in that run, confirmed
  unused by any other `SIMD_OPS` entry.
- `SimdOpKind::Q15mulrSatI16x8S`.

### Notes

- Semantics (implemented in `wasm-execution`, not this crate): per
  lane, sign-extend both `i16`s to `i32`, multiply, add the Q15
  rounding constant `0x4000`, arithmetic-shift right by 15, then
  saturate to `i16::MIN..=i16::MAX`. The saturating clamp fires in
  exactly one case: both lanes at `i16::MIN`, which the unsaturated
  formula would push one past `i16::MAX`.

## [0.2.26] - 2026-08-19 - SIMD widen PR21: i64x2.extmul_i32x4 widening-multiply family (task #180-182)

### Added

- 4 new `SIMD_OPS` entries: `i64x2.extmul_low_i32x4_s` (`0xDC`),
  `i64x2.extmul_high_i32x4_s` (`0xDD`), `i64x2.extmul_low_i32x4_u`
  (`0xDE`), `i64x2.extmul_high_i32x4_u` (`0xDF`) -- the third and
  final rung of this table's "extmul" widening-multiply family, now
  complete: `i8x16.extmul_low/high` (widens to `i16x8`), `i32x4.
  extmul_low/high_i16x8` (widens `i16x8` to `i32x4`), and now this
  PR's `i32x4` -> `i64x2` rung. 135 SIMD opcodes total, up from 131.
  Same narrow-input/wide-output BINARY shape as
  `SimdOpKind::ExtmulLowI16x8S`/etc., just `i32x4` operands producing
  an `i64x2` result -- `_low` reads lane indices 0-1, `_high` reads
  lane indices 2-3, `_s` sign-extends each `i32` lane to `i64` before
  multiplying, `_u` zero-extends. No `i64x2.dot_i32x4_s` -- WASM SIMD
  does not define a dot-product for this pair, same as the
  `i16x8`-from-`i8x16` rung. Each sub-opcode byte fetched live from
  the SIMD proposal's own `BinarySIMD.md` and cross-checked against
  the already-implemented `i32x4.extmul_low_i16x8_s` (`0xBC`)/
  `i64x2.abs` (`0xC0`)/`i64x2.ge_s` (`0xDB`) entries (all three
  matched exactly, confirming `0xDC`-`0xDF` sits immediately past
  `i64x2`'s own comparison family with no gap).
- `SimdOpKind::ExtmulLowI64x2S`/`ExtmulHighI64x2S`/`ExtmulLowI64x2U`/
  `ExtmulHighI64x2U`.

## [0.2.25] - 2026-08-19 - SIMD widen PR20: i32x4<->f32x4 trunc_sat/convert conversion family (task #177-179)

### Added

- 4 new `SIMD_OPS` entries: `i32x4.trunc_sat_f32x4_s` (`0xF8`),
  `i32x4.trunc_sat_f32x4_u` (`0xF9`), `f32x4.convert_i32x4_s` (`0xFA`),
  `f32x4.convert_i32x4_u` (`0xFB`) -- this table's FIRST `i32x4`<->
  `f32x4` CONVERSION ops (a lane TYPE change, not just a value change
  within one lane type, unlike every prior `f32x4` addition: PR17's
  splats, PR19's abs/mul/min). 131 SIMD opcodes total, up from 127.
  All four reuse the plain UNARY `v128->v128` shape (same as
  `f32x4.abs`). `trunc_sat_f32x4_s`/`_u` NEVER trap -- unlike this
  table's TRAPPING scalar `i32.trunc_f32_s`/`_u` MVP opcodes -- NaN
  saturates to 0, out-of-range saturates to the target bound, matching
  the semantics of this crate's own `0xFC`-prefixed scalar `trunc_sat`
  conversions. `convert_i32x4_u` needs its `i32` lane's bit pattern
  reinterpreted as `u32` BEFORE the cast to `f32`, not converted
  directly from the signed interpretation -- see
  `SimdOpKind::ConvertI32x4U`'s own doc comment for the exact bug this
  avoids. Each sub-opcode byte fetched live from the SIMD proposal's
  own `BinarySIMD.md`.
- `SimdOpKind::TruncSatF32x4S`/`TruncSatF32x4U`/`ConvertI32x4S`/
  `ConvertI32x4U`.

## [0.2.24] - 2026-08-19 - SIMD widen PR19: f32x4.abs/f32x4.mul/f32x4.min (task #174-176)

### Added

- 3 new `SIMD_OPS` entries: `f32x4.abs` (`0xE0`), `f32x4.mul` (`0xE6`),
  `f32x4.min` (`0xE8`) -- the FIRST genuine floating-point ARITHMETIC
  ops in this table (PR17's `f32x4.splat`/`f64x2.splat` were pure
  bit-pattern broadcasts, no arithmetic). 127 SIMD opcodes total, up
  from 124. `abs` reuses the plain UNARY `v128->v128` shape (same as
  `i8x16.abs`); `mul` reuses the plain BINARY `v128,v128->v128` shape
  (same as `i16x8.mul`). `min` is the same BINARY shape too, but its
  runtime semantics are NOT a plain `f32::min()`/IEEE `minNum` --
  WASM's `fmin` propagates NaN unconditionally (if either operand is
  NaN the result is NaN) and treats `-0.0` as winning a `-0.0`/`+0.0`
  tie, the exact per-lane transplant of this crate's own scalar
  `f32.min` (sub-opcode `0x96`) semantics -- see that opcode's own
  handler in `wasm-execution` for the original scalar bug this
  mirrors. Each sub-opcode byte fetched live from the SIMD proposal's
  own `BinarySIMD.md`.
- `SimdOpKind::AbsF32x4`/`MulF32x4`/`MinF32x4`.

## [0.2.23] - 2026-08-19 - SIMD widen PR18: i8x16 swizzle/extract_lane_s/extract_lane_u/replace_lane (task #171-173)

### Added

- 4 new `SIMD_OPS` entries filling the `0x0E`/`0x15`-`0x17` gap inside
  the already-implemented `0x0C`-`0x22` const/splat/extract_lane
  encoding run: `i8x16.swizzle` (`0x0E`), `i8x16.extract_lane_s`
  (`0x15`), `i8x16.extract_lane_u` (`0x16`), `i8x16.replace_lane`
  (`0x17`). 124 SIMD opcodes total, up from 120. `swizzle` reuses the
  plain BINARY `v128,v128->v128` shape (same as `i8x16.add`);
  `extract_lane_s`/`_u` reuse `i32x4.extract_lane`'s "v128 + lane
  immediate -> i32" shape, just at `i8x16`'s 0-15 lane range with a
  genuine signed/unsigned split (the first `extract_lane` family
  member to need one). `replace_lane` is a GENUINELY NEW shape: the
  first kind to combine a lane-index immediate with a mixed-type
  (`v128`, `i32`) binary pop that produces a `v128` -- deliberately
  not force-fit into `ExtractLane`'s shape, since neither its pop
  count nor its result type match. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against the already-implemented `i32x4.extract_lane` (`0x1B`)/
  `i8x16.eq` (`0x23`) entries, which sit exactly one past this run's
  own end (both matched exactly, confirming the whole `0x0C`-`0x23`
  run is contiguous and self-consistent).
- `SimdOpKind::Swizzle`/`ExtractLaneI8x16S`/`ExtractLaneI8x16U`/
  `ReplaceLaneI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.22] - 2026-08-19 - SIMD: float splat family, first float-lane ops (task #168-170)

### Added

- 2 new `SIMD_OPS` entries: `f32x4.splat` (`0x13`), `f64x2.splat`
  (`0x14`) -- the FIRST floating-point-typed SIMD opcodes in this
  table, and the immediate continuation of the `0x0F`-`0x12`
  integer-splat run PR16 landed. 120 SIMD opcodes total, up from 118.
  Splat itself is a pure bit-pattern broadcast -- no rounding, NaN
  canonicalization, or comparison semantics -- so it reuses the exact
  "pop one scalar, push one v128" shape every prior splat already
  established, just popping `F32`/`F64` instead of `I32`/`I64`. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i64x2.splat` (`0x12`) entry (matched exactly, confirming the whole
  `0x0F`-`0x14` splat run is contiguous and self-consistent).
- `SimdOpKind::SplatF32x4`/`SplatF64x2`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.21] - 2026-08-19 - SIMD: splat family widening (task #165-167)

### Added

- 3 new `SIMD_OPS` entries widening lane-width coverage of the
  already-implemented `i32x4.splat` (`0x11`): `i8x16.splat` (`0x0F`),
  `i16x8.splat` (`0x10`), `i64x2.splat` (`0x12`) -- 118 SIMD opcodes
  total, up from 115. Same "pop one scalar, push one v128" shape as
  `i32x4.splat`; `i64x2.splat` is the first splat whose popped operand
  type differs from `i32` (it pops a real `i64`). Each sub-opcode byte
  fetched live from the SIMD proposal's own `BinarySIMD.md` and
  cross-checked against the already-implemented `i32x4.splat` entry
  (matched exactly, and confirms this crate's whole `0x0C`(const)/
  `0x0E`(swizzle, not yet implemented)/`0x0F`-`0x14`(splat family)
  encoding-space run is self-consistent).
- `SimdOpKind::SplatI8x16`/`SplatI16x8`/`SplatI64x2`.

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.20] - 2026-08-18 - SIMD: v128.load/v128.store (task #162-164)

### Added

- 2 new `SIMD_OPS` entries -- the FIRST SIMD ops in this table that
  need a `memarg` immediate (align + offset, plus an optional memidx
  under the multi-memory proposal): `v128.load` (`0x00`) and
  `v128.store` (`0x0B`), both single-byte sub-opcodes -- 115 SIMD
  opcodes total, up from 113. These are the first SIMD ops touching
  real linear memory; every one of the prior 113 only reads/writes the
  per-instance `v128` heap. Chosen via a fresh prioritization survey:
  `simd_bitwise.wast` (already vendored, task #150-152) has 13
  `assert_return` directives stuck at `NotYetSupported` specifically
  pending a real `v128.load` -- landing this unblocks those with zero
  changes to already-merged code. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against the already-implemented `v128.const`(`0x0C`) entry (its
  neighbor in the encoding space).
- `SimdOpKind::Load`/`Store`. Doc comments record the memarg shape and
  that execution is scoped to memory index 0 only for this first PR
  (multi-memory `v128.load`/`v128.store` is deferred, same as WASM92
  later widened the scalar load/store family).

See `code/specs/W13-wasm-simd-v128-first-slice.md`.

## [0.2.19] - 2026-08-18 - SIMD: shift family (task #159-161)

### Added

- 12 new `SIMD_OPS` entries -- the FIRST mixed-type binary SIMD op
  family: `ixNxM.shl`/`shr_s`/`shr_u` across all 4 lane widths --
  `i8x16.shl` (`0x6B`), `shr_s` (`0x6C`), `shr_u` (`0x6D`), `i16x8.shl`
  (`0x8B`), `shr_s` (`0x8C`), `shr_u` (`0x8D`), `i32x4.shl` (`0xAB`),
  `shr_s` (`0xAC`), `shr_u` (`0xAD`), `i64x2.shl` (`0xCB`), `shr_s`
  (`0xCC`), `shr_u` (`0xCD`) -- 113 SIMD opcodes total, up from 101.
  Chosen over closing any remaining narrower per-lane-width gap:
  shifts are load-bearing in nearly all real SIMD code (packing,
  masking, bit-manipulation) and every prior 101 opcodes are pure
  `v128`-to-`v128`/`v128`-to-`i32` -- none pop a MIX of both types.
  Every width's `shl`/`shr_s`/`shr_u` triple sits immediately BEFORE
  that width's already-implemented `add` sub-opcode (e.g.
  `i8x16.add`=`0x6E`, `i16x8.add`=`0x8E`, `i32x4.add`=`0xAE`,
  `i64x2.add`=`0xCE`), the same regular numbering scheme already
  confirmed for every other family in this table. Each sub-opcode byte
  fetched live from the SIMD proposal's own `BinarySIMD.md` and
  cross-checked against those already-implemented `add` entries.
- `SimdOpKind::ShlI8x16`/`ShrSI8x16`/`ShrUI8x16`/`ShlI16x8`/
  `ShrSI16x8`/`ShrUI16x8`/`ShlI32x4`/`ShrSI32x4`/`ShrUI32x4`/
  `ShlI64x2`/`ShrSI64x2`/`ShrUI64x2`. Per the SIMD spec, the shift
  amount is taken MODULO the lane's bit width (8/16/32/64
  respectively) before shifting -- both spec-mandated and required
  for Rust safety, since shifting a primitive by >= its bit width
  panics.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_bit_shift.wast` and the resulting baseline delta.

## [0.2.18] - 2026-08-18 - SIMD: i64x2 arith+cmp family (task #156-158)

### Added

- 11 new `SIMD_OPS` entries -- i64x2's first REAL ARITHMETIC family
  (task #153-155 only added the `all_true`/`bitmask` reduction ops, no
  computation): `i64x2.abs` (`0xC0`), `neg` (`0xC1`), `add` (`0xCE`),
  `sub` (`0xD1`), `mul` (`0xD5`), `eq` (`0xD6`), `ne` (`0xD7`), `lt_s`
  (`0xD8`), `gt_s` (`0xD9`), `le_s` (`0xDA`), `ge_s` (`0xDB`) -- 101
  SIMD opcodes total, up from 90. No `lt_u`/`gt_u`/`le_u`/`ge_u` -- the
  SIMD proposal never defines unsigned `i64x2` comparisons, unlike
  every narrower lane width. `abs`/`neg` fill the gap left by the
  already-implemented `all_true` (`0xC3`)/`bitmask` (`0xC4`), matching
  the identical `abs`/`neg`/`[gap]`/`all_true`/`bitmask` cluster layout
  already confirmed for `i8x16` (`0x60`/`0x61`/../`0x63`/`0x64`),
  `i16x8` (`0x80`/`0x81`/../`0x83`/`0x84`), and `i32x4`
  (`0xA0`/`0xA1`/../`0xA3`/`0xA4`). `eq`..`ge_s` form one contiguous
  `0xD5-0xDB` run, matching the contiguous cmp blocks of every other
  lane width. Each sub-opcode byte fetched live from the SIMD
  proposal's own `BinarySIMD.md` (twice, identical both times) and
  cross-checked against the already-implemented `i64x2.all_true`/
  `bitmask` entries.
- `SimdOpKind::AbsI64x2`/`NegI64x2`/`AddI64x2`/`SubI64x2`/`MulI64x2`/
  `EqI64x2`/`NeI64x2`/`LtSI64x2`/`GtSI64x2`/`LeSI64x2`/`GeSI64x2`.
  Reuses the existing `v128,v128->v128`/`v128->v128` shapes already
  used everywhere else -- this closes a lane-width coverage gap, not a
  new operand shape.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i64x2_arith.wast`/`simd_i64x2_arith2.wast`/`simd_i64x2_cmp.wast`
and the resulting baseline delta.

## [0.2.17] - 2026-08-18 - SIMD: boolean-reduction/bitmask family (task #153-155)

### Added

- 9 new `SIMD_OPS` entries -- `v128.any_true` (`0x53`) plus
  `ixNxM.all_true`/`bitmask` across all 4 lane widths: `i8x16.all_true`
  (`0x63`), `i8x16.bitmask` (`0x64`), `i16x8.all_true` (`0x83`),
  `i16x8.bitmask` (`0x84`), `i32x4.all_true` (`0xA3`), `i32x4.bitmask`
  (`0xA4`), `i64x2.all_true` (`0xC3`), `i64x2.bitmask` (`0xC4`) -- 90
  SIMD opcodes total, up from 81. `i64x2.all_true`/`bitmask` are the
  first `i64x2` opcodes in this table. Chosen over shift-op and
  i64x2-arithmetic candidates in a fresh prioritization survey: highest
  opcode count (9) behind a single new operand shape and one 72KB
  corpus file, and unlocks real use of the comparison families from
  earlier PRs (a `v128` mask result is otherwise inert without a
  reduction op to consume it). Each sub-opcode byte fetched live from
  the SIMD proposal's own `BinarySIMD.md` and cross-checked against the
  already-implemented `v128.bitselect` (`0x52`)/`i8x16.popcnt`
  (`0x62`)/`i16x8.abs` (`0x80`)/`neg` (`0x81`)/`i32x4.abs` (`0xA0`)/
  `neg` (`0xA1`) entries (every value landed exactly where the
  `abs`/`neg`/`[popcnt]`/`all_true`/`bitmask` per-lane-width pattern
  predicts).
- `SimdOpKind::AnyTrue`/`AllTrueI8x16`/`AllTrueI16x8`/`AllTrueI32x4`/
  `AllTrueI64x2`/`BitmaskI8x16`/`BitmaskI16x8`/`BitmaskI32x4`/
  `BitmaskI64x2`. The first `v128`-in/`i32`-out reduction shape besides
  `ExtractLane`, but with NO lane-index immediate (these reduce over
  ALL lanes, not select one).

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_boolean.wast` and the resulting baseline delta.

## [0.2.16] - 2026-08-18 - SIMD: v128 bitwise family (task #150-152)

### Added

- 6 new `SIMD_OPS` entries -- the lane-width-agnostic raw-byte bitwise
  family, a strategic pivot from "widen the next narrow per-lane-width
  family" to "close the highest-real-world-impact remaining gap" now
  that `i8x16`/`i16x8`/`i32x4` all have complete
  arith+cmp+arith2+widening coverage relative to each other:
  `v128.not` (`0x4D`), `v128.and` (`0x4E`), `v128.andnot` (`0x4F`),
  `v128.or` (`0x50`), `v128.xor` (`0x51`), `v128.bitselect` (`0x52`)
  -- 81 SIMD opcodes total, up from 75. Unlike every prior family,
  these have only ONE spelling each (no `i8x16`/`i16x8`/`i32x4`
  suffix) since they operate on raw bytes, not typed lanes. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i8x16.add` (`0x6E`)/`i32x4.add` (`0xAE`) entries.
- `SimdOpKind::Not`/`And`/`AndNot`/`Or`/`Xor`/`Bitselect`.
  `Bitselect` is the first TERNARY `SimdOpKind` in this crate (pops
  three `v128`s, pushes one): `(a AND c) OR (b AND (NOT c))`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_bitwise.wast` and the resulting baseline delta.

## [0.2.15] - 2026-08-18 - SIMD: i16x8-from-i8x16 widening family (task #147-149)

### Added

- 6 new `SIMD_OPS` entries -- `i16x8`'s own widening family, mirroring
  the already-implemented `i32x4`-from-`i16x8` widening family one
  lane width down, closing the last remaining gap between `i16x8` and
  `i8x16`'s coverage: `i16x8.extadd_pairwise_i8x16_s` (`0x7C`),
  `extadd_pairwise_i8x16_u` (`0x7D`), `extmul_low_i8x16_s` (`0x9C`),
  `extmul_high_i8x16_s` (`0x9D`), `extmul_low_i8x16_u` (`0x9E`),
  `extmul_high_i8x16_u` (`0x9F`) -- 75 SIMD opcodes total, up from 69.
  Unlike the `i32x4`-from-`i16x8` family, there is no
  `i16x8.dot_i8x16_s` -- WASM SIMD does not define a dot-product for
  this lane-width pair, so this family is 6 ops, not 7. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i8x16.add` (`0x6E`)/`i16x8.mul` (`0x95`)/`i16x8.avgr_u` (`0x9B`)/
  `i32x4.dot_i16x8_s` (`0xBA`)/`i8x16.popcnt` (`0x62`)/
  `i32x4.extadd_pairwise_i16x8_s` (`0x7E`) entries (all six matched
  exactly).
- `SimdOpKind::ExtaddPairwiseI8x16S`/`ExtaddPairwiseI8x16U`/
  `ExtmulLowI8x16S`/`ExtmulHighI8x16S`/`ExtmulLowI8x16U`/
  `ExtmulHighI8x16U`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_extadd_pairwise_i8x16.wast`/`simd_i16x8_extmul_i8x16.wast`
corpus files this widening unblocks.

## [0.2.14] - 2026-08-18 - SIMD: i16x8 abs/min/max/avgr_u family (task #144-146)

### Added

- 6 new `SIMD_OPS` entries -- `i16x8`'s own "arith2" family, closing
  the same gap PR8 (task #141-143) just closed for `i8x16` (no
  `i16x8.popcnt` -- WASM SIMD only defines `popcnt` for `i8x16`):
  `i16x8.abs` (`0x80`), `min_s` (`0x96`), `min_u` (`0x97`), `max_s`
  (`0x98`), `max_u` (`0x99`), `avgr_u` (`0x9B`) -- 69 SIMD opcodes
  total, up from 63. All six sub-opcodes are >= 128 (2-byte LEB128),
  same shape as `i16x8`'s own `add`/`sub`/`mul`/`neg`, unlike
  `i8x16`'s own arith2 family (all < 128). Each sub-opcode byte
  fetched live from the SIMD proposal's own `BinarySIMD.md` and
  cross-checked against the already-implemented `i16x8.neg`
  (`0x81`)/`add` (`0x8E`)/`sub` (`0x91`)/`mul` (`0x95`) entries (all
  four matched exactly).
- `SimdOpKind::AbsI16x8`/`MinSI16x8`/`MinUI16x8`/`MaxSI16x8`/
  `MaxUI16x8`/`AvgrUI16x8`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_arith2.wast` corpus file this widening unblocks.

## [0.2.13] - 2026-08-18 - SIMD: i8x16 abs/popcnt/min/max/avgr_u family (task #141-143)

### Added

- 7 new `SIMD_OPS` entries -- `i8x16`'s own "arith2" family, mirroring
  `i32x4`'s own `abs`/`min_s`/`min_u`/`max_s`/`max_u` widening, plus
  two op SHAPES with no `i32x4`/`i16x8` precedent in this table:
  `i8x16.abs` (`0x60`), `popcnt` (`0x62`, lane-wise Hamming weight --
  WASM SIMD only defines `popcnt` for `i8x16`), `min_s` (`0x76`),
  `min_u` (`0x77`), `max_s` (`0x78`), `max_u` (`0x79`), `avgr_u`
  (`0x7B`, lane-wise unsigned rounding average `(a + b + 1) >> 1` --
  WASM SIMD defines `avgr_u` for `i8x16`/`i16x8` but not `i32x4`) -- 63
  SIMD opcodes total, up from 56. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against the already-implemented `i8x16.add` (`0x6E`)/`i8x16.neg`
  (`0x61`)/`i8x16.sub` (`0x71`) entries (all three matched exactly).
- `SimdOpKind::AbsI8x16`/`PopcntI8x16`/`MinSI8x16`/`MinUI8x16`/
  `MaxSI8x16`/`MaxUI8x16`/`AvgrUI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i8x16_arith2.wast` corpus file this widening unblocks.

## [0.2.12] - 2026-08-18 - SIMD: i8x16 comparison family (task #137-140)

### Added

- 10 new `SIMD_OPS` entries -- `i8x16`'s own comparison family, closing
  the same gap PR6 closed for `i16x8`: `i8x16.add`/`sub`/`neg` (PR4)
  landed without a comparison family. `i8x16.eq` (`0x23`), `ne`
  (`0x24`), `lt_s` (`0x25`), `lt_u` (`0x26`), `gt_s` (`0x27`), `gt_u`
  (`0x28`), `le_s` (`0x29`), `le_u` (`0x2A`), `ge_s` (`0x2B`), `ge_u`
  (`0x2C`) -- same boolean-mask convention and signed/unsigned split as
  `i16x8`'s and `i32x4`'s own comparison families, just at the
  narrowest lane width -- 56 SIMD opcodes total, up from 46. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i8x16.add` (`0x6E`)/`i16x8.eq` (`0x2D`) entries (both matched
  exactly), same verification discipline as every widening above.
- `SimdOpKind::EqI8x16`/`NeI8x16`/`LtSI8x16`/`LtUI8x16`/`GtSI8x16`/
  `GtUI8x16`/`LeSI8x16`/`LeUI8x16`/`GeSI8x16`/`GeUI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i8x16_cmp.wast` corpus file this widening unblocks.

## [0.2.11] - 2026-08-18 - SIMD: i16x8 comparison family (task #133-136)

### Added

- 10 new `SIMD_OPS` entries -- `i16x8`'s own comparison family, closing
  the gap left when `i16x8.add`/`sub`/`mul`/`neg` (PR5) landed without
  one (unlike `i32x4`, which got arith+cmp together): `i16x8.eq`
  (`0x2D`), `ne` (`0x2E`), `lt_s` (`0x2F`), `lt_u` (`0x30`), `gt_s`
  (`0x31`), `gt_u` (`0x32`), `le_s` (`0x33`), `le_u` (`0x34`), `ge_s`
  (`0x35`), `ge_u` (`0x36`) -- same boolean-mask convention and signed/
  unsigned split as `i32x4`'s own comparison family, just at `i16x8`'s
  narrower lane width -- 46 SIMD opcodes total, up from 36. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i16x8.add` (`0x8E`)/`i32x4.eq` (`0x37`) entries (both matched
  exactly), same verification discipline as every widening above.
- `SimdOpKind::EqI16x8`/`NeI16x8`/`LtSI16x8`/`LtUI16x8`/`GtSI16x8`/
  `GtUI16x8`/`LeSI16x8`/`LeUI16x8`/`GeSI16x8`/`GeUI16x8`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_cmp.wast` corpus file this widening unblocks.

## [0.2.10] - 2026-08-18 - SIMD: i16x8 first primary-lane slice (task #129-132)

### Added

- 4 new `SIMD_OPS` entries -- the first opcodes in this table where
  `i16x8` is a PRIMARY lane width (produces `i16x8` results), not just
  an INPUT to an `i32x4`-producing widening op (`extadd_pairwise`/
  `dot`/`extmul`, already implemented): `i16x8.neg` (`0x81`),
  `i16x8.add` (`0x8E`), `i16x8.sub` (`0x91`), `i16x8.mul` (`0x95`).
  Unlike `i8x16` (whose spec defines no `mul`), WASM SIMD DOES define
  `i16x8.mul` -- included here since the real upstream corpus file
  bundles all four ops together -- 36 SIMD opcodes total, up from 32.
  Each sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i32x4.add` (`0xAE`)/`i8x16.add` (`0x6E`) entries (both matched
  exactly), same verification discipline as every prior addition.
- `SimdOpKind::AddI16x8`/`SubI16x8`/`MulI16x8`/`NegI16x8`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i16x8_arith.wast` corpus file this slice unblocks.

## [0.2.9] - 2026-08-18 - SIMD: i8x16 first slice (task #125-128)

### Added

- 3 new `SIMD_OPS` entries -- this table's first `i8x16` lane-width
  opcodes, a brand-new "first slice" (following the same pattern
  `i32x4` itself started with, not a widening of an existing lane
  width): `i8x16.add` (`0x6E`), `i8x16.sub` (`0x71`), `i8x16.neg`
  (`0x61`). No `i8x16.mul` -- WASM SIMD defines none (8-bit lanes are
  too narrow for a useful lane-wise multiply). No `i8x16.splat`/
  `extract_lane` either: unlike `i32x4`'s original 5-opcode slice,
  they aren't needed, since `v128.const i8x16 ...` (already supported
  for all 6 shapes since PR1b) covers both operand construction and
  result comparison for this slice's own test corpus on its own -- 32
  SIMD opcodes total, up from 29. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked
  against the already-implemented `i32x4.add` (`0xAE`)/`i32x4.abs`
  (`0xA0`) entries (both matched exactly), same verification
  discipline as every widening above.
- `SimdOpKind::AddI8x16`/`SubI8x16`/`NegI8x16`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i8x16_arith.wast` corpus file this slice unblocks.

## [0.2.8] - 2026-08-18 - SIMD widening: i32x4-from-i16x8 family (task #121-124)

### Added

- 7 new `SIMD_OPS` entries -- the first opcodes in this table whose INPUT
  lane width (16-bit `i16x8`) differs from their OUTPUT lane width
  (32-bit `i32x4`): `i32x4.extadd_pairwise_i16x8_s` (`0x7E`)/`_u`
  (`0x7F`) (pairwise-add adjacent `i16x8` lanes into `i32x4`),
  `i32x4.dot_i16x8_s` (`0xBA`) (pairwise signed multiply-accumulate
  across all 8 `i16x8` lanes into 4 `i32x4` results), and
  `i32x4.extmul_low_i16x8_s` (`0xBC`)/`extmul_high_i16x8_s` (`0xBD`)/
  `extmul_low_i16x8_u` (`0xBE`)/`extmul_high_i16x8_u` (`0xBF`) (widening
  multiply over only the low or high 4 `i16x8` lanes of each operand) --
  29 SIMD opcodes total, up from 22. Each sub-opcode byte fetched live
  from the SIMD proposal's own `BinarySIMD.md` and cross-checked against
  the already-implemented `i32x4.eq` (`0x37`)/`i32x4.add` (`0xAE`)
  entries (both matched exactly), same verification discipline as every
  widening pass above.
- `SimdOpKind::ExtaddPairwiseI16x8S`/`ExtaddPairwiseI16x8U`/`DotI16x8S`/
  `ExtmulLowI16x8S`/`ExtmulHighI16x8S`/`ExtmulLowI16x8U`/
  `ExtmulHighI16x8U`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i32x4_extadd_pairwise_i16x8.wast`/`simd_i32x4_dot_i16x8.wast`/
`simd_i32x4_extmul_i16x8.wast` corpus files this widening unblocks.

## [0.2.7] - 2026-08-18 - SIMD widening: i32x4 abs/min/max family (task #118-120)

### Added

- 5 new `SIMD_OPS` entries widening the `i32x4` lane coverage further:
  `i32x4.abs` (the second UNARY kind, alongside `neg`) plus the
  `min_s`/`min_u`/`max_s`/`max_u` family (same `v128,v128->v128` binary
  shape as `add`/`sub`/`mul`) -- 22 SIMD opcodes total, up from 17. Each
  sub-opcode byte fetched live from the SIMD proposal's own
  `BinarySIMD.md` and cross-checked against the already-implemented
  `i32x4.eq` (`0x37`)/`i32x4.add` (`0xAE`) entries (both matched
  exactly), same verification discipline as every widening pass above.
- `SimdOpKind::Abs`/`MinS`/`MinU`/`MaxS`/`MaxU`.

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i32x4_arith2.wast` corpus file this widening unblocks.

## [0.2.6] - 2026-08-18 - SIMD widening: i32x4 arithmetic + comparison family (task #113-117)

### Added

- 12 new `SIMD_OPS` entries widening the first slice's `i32x4` lane
  coverage: `i32x4.mul`/`neg`/`sub` (joining the already-implemented
  `add`) and the full comparison family `ne`/`lt_s`/`lt_u`/`gt_s`/
  `gt_u`/`le_s`/`le_u`/`ge_s`/`ge_u` (joining `eq`) -- 17 SIMD opcodes
  total, up from 5. Each sub-opcode byte fetched live from the SIMD
  proposal's own `BinarySIMD.md` and cross-checked against the already-
  implemented `i32x4.eq` (`0x37`)/`i32x4.add` (`0xAE`) entries (both
  matched exactly), same verification discipline as the original 5.
- `SimdOpKind::Neg` -- the first UNARY kind (pops one `v128`, pushes
  one), unlike every other kind so far (all binary, pop two push one).

See `code/specs/W13-wasm-simd-v128-first-slice.md` and the `wasm-
conformance` crate's own CHANGELOG entry for the newly-vendored
`simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast` corpus files this widening
unblocks.

## [0.2.5] - 2026-08-15 - SIMD first slice: SIMD_OPS table (0xFD prefix)

### Added

- `SIMD_OPS`/`get_simd_op`/`get_simd_op_by_name`/`SimdOpInfo`/`SimdOpKind`:
  5 SIMD opcodes for this first slice -- `v128.const` (`0x0C`),
  `i32x4.extract_lane` (`0x1B`), `i32x4.splat` (`0x11`), `i32x4.eq`
  (`0x37`), `i32x4.add` (`0xAE`) -- verified against the SIMD proposal's
  own `BinarySIMD.md`, cross-checked against the W3C core spec for 4 of
  the 5. Structurally different from `ATOMIC_OPS`: SIMD's sub-opcode is a
  **LEB128-encoded `u32`**, not a raw byte (`i32x4.add`'s real value, 174,
  doesn't fit in one byte) -- `SimdOpInfo::sub_opcode` is `u32`, not `u8`.
  `i32x4.extract_lane` is the one opcode beyond the original 4-opcode
  spec scope, added because it's the only way to observe a `v128`
  result's contents as a scalar (see `code/specs/
  W13-wasm-simd-v128-first-slice.md`). 3 new tests, including one pinning
  `i32x4.add`'s value specifically as the multi-byte-LEB128 case (the
  other 4 opcodes are all single-byte-safe).

## [0.2.4] - 2026-08-15 - tail-call opcodes: return_call/return_call_indirect (WASM16)

### Added

- `return_call` (`0x12`) and `return_call_indirect` (`0x13`) -- confirmed
  free/unclaimed sub-opcodes, same immediate shapes as `call`/
  `call_indirect` respectively (a `funcidx`, or a `typeidx`+`tableidx`
  pair). 1 new unit test. See `code/specs/W11-wasm-tail-calls.md`.

## [0.2.3] - 2026-08-15 - atomic memory opcode table, 0xFE prefix (WASM18)

### Added

- New `AtomicOpKind` enum: `Load`, `Store`, `Rmw`, `Cmpxchg`, `Fence`,
  `Notify`, `Wait` -- each documenting its own pop/push shape.
- New `AtomicOpInfo` struct (`name`, `sub_opcode`, `kind`, `value_type`,
  `natural_align`) and `pub static ATOMIC_OPS: &[AtomicOpInfo]`, a single
  shared 67-entry table covering the entire `0xFE`-prefixed atomics
  family in one place -- unlike the `0xFB`/`0xFC` prefix families, whose
  sub-opcode dispatch is duplicated ad hoc per consumer, `wasm-execution`
  and `wasm-validator` both look atomic ops up here rather than
  hand-rolling their own tables, since this family is large (67 entries)
  but fully regular.
- `get_atomic_op(sub_opcode: u8)` / `get_atomic_op_by_name(name: &str)`
  lookup helpers.
- 4 new unit tests, including a count/no-duplicates check
  (`3 + 1 + 7 + 7 + 7*7 == 67`: notify/wait32/wait64 + fence + 7 loads +
  7 stores + 7 RMW-op-kinds × 7 width variants) and a byte-range
  contiguity check (`0x00-0x03` and `0x10-0x4E` present, `0x04-0x0F`
  absent).

### Corrected (implementation-time, vs. the merged W09 spec)

- The merged spec described `memory.atomic.notify`/`wait32`/`wait64` as
  "deliberately absent -- meaningless without real threads." Reading the
  real, pinned-commit `atomic.wast` testsuite file directly showed these
  three instructions are declared inline in the SAME top-level module as
  every other atomic op under test, so omitting them would have made
  `wasm-wast-parser` fail to *parse* the entire file on the first
  occurrence -- losing all conformance value from vendoring it, not just
  leaving 3 opcodes ungraded as intended. They also turn out to have
  fully deterministic, single-agent-computable semantics: `notify`
  always returns 0 woken (no second agent can ever be waiting);
  `wait32`/`wait64` return 1 ("not-equal") or 2 ("timed-out") based on a
  plain memory comparison, since nothing can ever notify a
  single-threaded VM. Both are implemented for real here, not stubbed.

## [0.2.2] - 2026-08-15 - reference-types table/ref.func opcodes (WASM17)

### Added

- `table.get` (0x25) and `table.set` (0x26) -- the previously-reserved MVP
  gap the reference-types proposal fills, `tableidx` immediate. `ref.func`
  (0xD2) -- `funcidx` immediate. All three use the crate's normal metadata
  table with no special-casing (`tableidx`/`funcidx` were already generic
  immediate kinds this crate's decoders understood).
- 1 new unit test (`test_reference_types_opcodes`) round-tripping all three
  by byte and name, and asserting `ref.null`/`ref.is_null` (0xD0/0xD1) are
  deliberately absent from this table -- both already have working
  `wasm-execution`/`wasm-validator` handlers from before WASM17 and were
  never entries here, matching the existing `0xFB`/`0xFC` GC/misc-prefix
  precedent. See `code/specs/W08-wasm-funcref-externref.md`.

## [0.2.1] - 2026-08-13 - sign-extension opcodes (WASM03)

### Added

- The 5 single-byte opcodes from the "sign-extension operators" proposal
  (widely implemented, MVP-adjacent, still single-byte unlike later
  0xFC-prefixed proposals): `i32.extend8_s` (0xC0), `i32.extend16_s`
  (0xC1), `i64.extend8_s` (0xC2), `i64.extend16_s` (0xC3),
  `i64.extend32_s` (0xC4). Each pops one int, sign-extends its low N bits
  to the full i32/i64 width, pushes one int -- category `"conversion"`,
  matching every other unary numeric conversion already in the table.
- 1 new unit test (`test_sign_extension_opcodes`) round-tripping all 5 by
  both byte and name; the existing `test_conversions_stack_effects`
  automatically covers their pop=1/push=1/no-immediates invariant since
  they share the `"conversion"` category.

### Not added (deliberately)

- The same proposal's 8 `trunc_sat` opcodes are NOT in this table -- they
  use a two-byte `0xFC <sub-opcode>` encoding this crate's single-byte
  `OpcodeInfo`/`get_opcode(byte: u8)` model doesn't fit (consistent with
  this crate's existing, pre-2.0 scoping decision to leave `0xFC`-prefixed
  opcodes to their callers -- see the module doc comment). `wasm-wast-parser`
  and `wasm-execution` special-case the `0xFC` prefix directly, the same
  way they already did for bulk-memory's `memory.copy`/`memory.fill`.

### Fixed (also this release, no version-worthy behavior change here)

- `Cargo.toml`'s `version` had drifted to `0.1.0` while this changelog's
  last real entry was already `0.2.0` -- corrected to continue from the
  changelog's actual history, not a code change.

## [0.2.0] - 2026-03-23

### Added

- Complete WASM 1.0 MVP opcode table: 172 instructions from 0x00 to 0xBF
- `OpcodeInfo` struct with `name`, `opcode`, `category`, `immediates`,
  `stack_pop`, `stack_push` fields (all `&'static` for zero heap allocation)
- `OPCODES` static slice — entire table lives in read-only memory at compile time
- `get_opcode(byte: u8) -> Option<&'static OpcodeInfo>` — lookup by byte value
- `get_opcode_by_name(name: &str) -> Option<&'static OpcodeInfo>` — lookup by name
- 17 unit tests covering: count, byte/name lookups, stack effects, immediates,
  uniqueness, category spot checks, category-wide invariants, and doc-tests
- Literate inline comments explaining: stack machine model, immediates encoding,
  memarg format, structured control flow, signedness conventions, IEEE 754 notes,
  and type conversion semantics

### Notes

- The WASM 1.0 MVP has exactly 172 instructions; the "~183" figure sometimes cited
  includes post-MVP proposals (SIMD, bulk-memory, etc.) that use a 0xFC prefix
  encoding not covered here
- Lookup is a linear scan over 183 entries — negligible cost, avoids runtime map

## [0.1.0] - 2026-03-23

### Added

- Initial package scaffolding generated by scaffold-generator
