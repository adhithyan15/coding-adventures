# Changelog — wasm-conformance

## 0.1.77 — 2026-08-25 — vendor simd_load8_lane.wast/simd_store8_lane.wast (SIMD PR44, 2 new opcodes)

### Added

- Vendored `simd_load8_lane.wast`/`simd_store8_lane.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`), the dedicated upstream
  files for `v128.load8_lane`/`v128.store8_lane` -- the FIRST bite into
  the `load{8,16,32,64}_lane`/`store{8,16,32,64}_lane` family every PR
  since PR39 has deferred (a genuinely new instruction shape: an
  existing `v128` operand PLUS a lane-index immediate PLUS a memarg, all
  in one instruction). Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`; NOTICE updated with full provenance and per-file
  directive-kind counts.
- Both files pass 100% on `module` (2/2) and `assert_return` (96/96
  combined) -- every byte-pattern/lane-preservation case passes.
  `simd_load8_lane.wast`'s `assert_invalid` grades 2/2 pass (type
  mismatch, out-of-range lane index) with 1 `NotYetSupported` (an
  invalid `align=2` case -- this repo's SIMD memarg decode path still
  never checks an over-large `align=` against natural alignment, the
  SAME pre-existing gap PR43's `simd_align.wast` vendoring surfaced and
  documented, not newly introduced here).
  `simd_store8_lane.wast`'s `assert_invalid` grades 3/3 pass (its own
  `align=2` case is independently caught by the pre-existing
  "declared-result-type mismatch" check, since that test's function
  wrongly declares `(result v128)` on a `store8_lane`-only body).
- Regenerated `testsuite-status.json` baseline: aggregate `module` rose
  from 1260/1261 to 1262/1263 (+2 pass); `assert_return` rose from
  44400/44417 to 44496/44513 (+96 pass); `assert_invalid` rose from
  2013/2013 to 2018/2018 pass (+5) with `NotYetSupported` rising from 88
  to 89 (+1); `assert_malformed` unchanged (525/525 pass, 538
  `NotYetSupported` -- neither file has any `assert_malformed`
  directives). No other already-vendored file's stats changed.

## 0.1.76 — 2026-08-25 — vendor simd_align.wast (SIMD PR43, zero new opcodes)

### Added

- Vendored `simd_align.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering `align=` hint
  validation for `v128.load`/`v128.store` plus the `load_splat`/
  `load_extend` families landed in PR15/PR39-42. Adds ZERO new opcodes
  — every instruction this file exercises was already implemented.
  Added to `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated
  with full provenance and per-file directive-kind counts.
- The file passes 100% on `module` (46/46) and `assert_return` (8/8),
  the directives that only exercise already-implemented opcode
  execution. `assert_invalid` grades 0/12 (all `NotYetSupported`) and
  `assert_malformed` grades 12/34 pass with 22 `NotYetSupported`: this
  repo's SIMD memarg decode path in `wasm-validator` never checks an
  over-large `align=` against each v128 opcode's natural alignment (the
  scalar `iNN.load`/`iNN.store` arm does, via `memory_op_shape`/
  `max_align_for`, but the shared v128 memarg arm doesn't), and
  `wasm-wast-parser`'s `parse_memarg` never validates that `align=N` is
  a power of two. Both gaps are pre-existing and shared with the class
  already tracked as `NotYetSupported` in the plain (non-SIMD)
  `align.wast` vendored earlier — this file surfaces the same known gap
  for the v128 opcode family, tracked for a dedicated follow-up, not a
  new regression.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `module` rose from 1214/1215 to 1260/1261 (+46 pass, 67
  `NotYetSupported` unchanged); `assert_return` rose from 44392/44409 to
  44400/44417 (+8 pass, 545 `NotYetSupported` unchanged); `assert_invalid`
  stayed at 2013/2013 pass with `NotYetSupported` rising from 76 to 88
  (+12, exactly this file's own gap); `assert_malformed` rose from
  513/513 to 525/525 pass (+12) with `NotYetSupported` rising from 516
  to 538 (+22, exactly this file's own gap). No other already-vendored
  file's stats changed — zero regressions.

## 0.1.75 — 2026-08-25 — vendor simd_load_extend.wast (SIMD PR42)

### Added

- Vendored `simd_load_extend.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering the new
  `v128.load8x8_s`/`_u`, `v128.load16x4_s`/`_u`, `v128.load32x2_s`/`_u`
  opcodes added in `wasm-opcodes`/`wasm-execution`/`wasm-validator`/
  `wasm-wast-parser`. Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`; NOTICE updated with full provenance and per-file
  directive-kind counts (also backfilled a missing PR41 header entry
  found while updating this same list). Third and FINAL bite into the
  wider `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` memory-access family PR39 deferred and
  PR40/PR41 opened.
- The file passes 100% of its directives: 2/2 `module`, 72/72
  `assert_return`, 12/12 `assert_trap`, 12/12 `assert_invalid`, 6/6
  `assert_malformed`.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 44320/44337 to 44392/44409 (+72
  pass, `fail` unchanged at 17); `assert_trap` rose from 1466/1466 to
  1478/1478 (+12, still 100%); `assert_invalid` rose from 2001/2001 to
  2013/2013 (+12, still 100%); `assert_malformed` rose from 507/507 to
  513/513 (+6, still 100%); `module` pass count rose from 1212/1213 to
  1214/1215 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

## 0.1.74 — 2026-08-25 — vendor simd_load_zero.wast (SIMD PR41)

### Added

- Vendored `simd_load_zero.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering the new
  `v128.load32_zero`/`load64_zero` opcodes added in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated with full
  provenance and per-file directive-kind counts. Second bite into the
  wider `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` memory-access family PR39 deferred and PR40
  opened with `simd_load_splat.wast`.
- The file passes 100% of its directives: 2/2 `module`, 23/23
  `assert_return`, 4/4 `assert_trap`, 4/4 `assert_invalid`, 6/6
  `assert_malformed`.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 44297/44314 to 44320/44337 (+23
  pass, `fail` unchanged at 17); `assert_trap` rose from 1462/1462 to
  1466/1466 (+4, still 100%); `assert_invalid` rose from 1997/1997 to
  2001/2001 (+4, still 100%); `assert_malformed` rose from 501/501 to
  507/507 (+6, still 100%); `module` pass count rose from 1210/1211 to
  1212/1213 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

## 0.1.73 — 2026-08-25 — vendor simd_load_splat.wast (SIMD PR40)

### Added

- Vendored `simd_load_splat.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering the new
  `v128.load8_splat`/`load16_splat`/`load32_splat`/`load64_splat`
  opcodes added in `wasm-opcodes`/`wasm-execution`/`wasm-validator`/
  `wasm-wast-parser`. Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`; NOTICE updated with full provenance and
  per-file directive-kind counts. First bite into the wider
  `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` memory-access family PR39 deferred --
  deliberately scoped to just this one file, the smallest/simplest of
  that family.
- The file passes 100% of its directives: 2/2 `module`, 80/80
  `assert_return`, 32/32 `assert_trap`, 8/8 `assert_invalid`, 4/4
  `assert_malformed`.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 44217/44234 to 44297/44314 (+80
  pass, `fail` unchanged at 17); `assert_trap` rose from 1430/1430 to
  1462/1462 (+32, still 100%); `assert_invalid` rose from 1989/1989 to
  1997/1997 (+8, still 100%); `assert_malformed` rose from 497/497 to
  501/501 (+4, still 100%); `module` pass count rose from 1208/1209 to
  1210/1211 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

## 0.1.72 — 2026-08-25 — vendor simd_f32x4_rounding.wast/simd_f64x2_rounding.wast (SIMD widen PR39)

### Added

- Vendored `simd_f32x4_rounding.wast` and `simd_f64x2_rounding.wast`
  (pinned commit `28864811cf03bdbf880733786148feaba339582d`) covering
  the new `f32x4.ceil`/`floor`/`trunc`/`nearest` and `f64x2.ceil`/
  `floor`/`trunc`/`nearest` opcodes added in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated with full
  provenance and per-file directive-kind counts.
- Both files pass 100% of their directives: 1/1 `module`, 176/176
  `assert_return`, 8/8 `assert_invalid`, 16/16 `assert_malformed`, in
  EACH file (352/352, 16/16, 32/32, 2/2 combined).

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 43865/43882 to 44217/44234 (+352
  pass, `fail` unchanged at 17); `assert_invalid` rose from 1973/1973 to
  1989/1989 (+16, still 100%); `assert_malformed` rose from 465/465 to
  497/497 (+32, still 100%); `module` pass count rose from 1206/1207 to
  1208/1209 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

## 0.1.71 — 2026-08-24 — regenerate baseline: SIMD widen PR38 unlocks 268 stuck simd_lane.wast directives (task #229-231)

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
  after `i8x16.shuffle` (SIMD widen PR38) landed in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser` -- the single
  opcode PR37's own changelog flagged as this file's remaining gap. No
  new corpus file: `simd_lane.wast` was already vendored (PR37) at the
  same pinned commit, so this is a pure baseline flip, not a re-vendor.
- `simd_lane.wast` before -> after this PR: `module` 8/12 -> **12/12**
  (100%, up from 4 `not_yet_supported`); `assert_return` 6/274 ->
  **274/274** (100%, ALL 268 previously-`not_yet_supported` directives
  now pass -- exactly the count PR37's own changelog predicted);
  `assert_invalid` 83/83 (unchanged, already 100%); `assert_malformed`
  106/106 (unchanged, already 100%). Every directive in this file is now
  gradeable and passing -- zero `not_yet_supported` left in
  `simd_lane.wast`.
- Aggregate corpus-wide `assert_return` moved from 43597/43882 to
  43865/43882 (545 `not_yet_supported` remaining, down from 813) as a
  direct consequence of the same 268-directive flip; every other file's
  own tally is unchanged by this PR.

## 0.1.70 — 2026-08-24 — vendor simd_lane.wast: SIMD widen PR37 (task #226-228)

### Added

- Vendored `simd_lane.wast` (a new file) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  remaining 10 extract_lane/replace_lane opcodes, CLOSING that family
  across all six SIMD vector shapes, implemented in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser` as part of the
  same PR. This PR also retrofits validation-time lane-index bounds
  checking onto the 4 pre-existing lane-immediate opcodes (previously
  runtime-only), and fixes `wasm-wast-parser`'s lane-index literal
  grammar to accept hex/underscore/leading-zero-decimal forms (was
  plain-decimal-only).
- 475 real directives: 12 module, 274 `assert_return`, 83
  `assert_invalid`, 106 `assert_malformed`. This upstream file also
  bundles `i8x16.shuffle` (a genuinely different, unimplemented opcode,
  out of this PR's scope -- 16 lane-index immediates across two source
  operands, its own future PR) into 4 of its 5 multi-function modules;
  because a WASM module builds as one atomic unit, those 4 modules (and
  the 268 `assert_return` directives invoking their exports, including
  ones that exercise an already-correct extract_lane/replace_lane
  export sitting right next to the unsupported shuffle export) grade
  `not_yet_supported`, a real capability gap, not a `Fail`. Every
  GRADEABLE directive in this file passes at 100%: 8/8 module, 6/6
  `assert_return` (the one shuffle-free multi-function module, memory
  load/store), 83/83 `assert_invalid` (every lane-index-out-of-range
  case, for all 14 lane-immediate opcodes old and new), 106/106
  `assert_malformed` (sign-prefixed/float/empty-argument lane-index
  literal rejection). See the vendored `NOTICE` file for the full
  breakdown and the exact aggregate before/after numbers, including an
  unplanned second effect: the literal-grammar fix also retroactively
  unlocks a previously-unbuildable module in the already-vendored
  `simd_splat.wast` (+1 module, +43 `assert_return`, both from
  `not_yet_supported` to passing).

## 0.1.69 — 2026-08-24 — vendor simd_int_to_int_extend.wast: SIMD widen PR36 (task #223-225)

### Added

- Vendored `simd_int_to_int_extend.wast` (a new file) at the existing
  pinned commit `28864811cf03bdbf880733786148feaba339582d`. This PR
  implements the 4 opcodes needed to complete the third and FINAL rung
  of the integer "extend" family: `i64x2.extend_low_i32x4_s` (`0xC7`),
  `i64x2.extend_high_i32x4_s` (`0xC8`), `i64x2.extend_low_i32x4_u`
  (`0xC9`), `i64x2.extend_high_i32x4_u` (`0xCA`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR. Unlike `simd_conversions.wast`'s PR26/27/28 split
  (three DIFFERENT opcode families bundled together), this single file
  bundles all THREE RUNGS of the SAME "extend" family in one module --
  `i16x8.extend_low/high_i8x16_s/_u` and `i32x4.extend_low/
  high_i16x8_s/_u` (both landed opcode-only in PR26, with no dedicated
  corpus file until now) alongside this PR's new `i64x2` rung -- so
  landing it integration-tests all 12 opcodes across all three rungs at
  once, not just this PR's new 4.
- 253 real directives: 1 module, 228 `assert_return`, 24
  `assert_invalid`, 0 `assert_malformed`. ALL 100% passing on the first
  baseline regen after implementation (1/1 module, 228/228
  `assert_return`, 24/24 `assert_invalid`) -- including every one of the
  file's i16x8/i32x4 rung directives (PR26's own opcodes, integration-
  tested against a real upstream corpus file for the first time)
  alongside this PR's new i64x2 rung. Aggregate `assert_return` rose
  from 43320/43337 to 43548/43565 (+228 pass, `fail` unchanged at 17);
  `assert_invalid` rose from 1866/1866 to 1890/1890 (+24, still 100.0%
  of gradeable directives); `module` pass count rose from 1192/1193 to
  1193/1194 (+1, passing); `assert_malformed` unchanged at 359/359 (this
  file has none). All deltas are EXACTLY this file's own directive
  counts -- the pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions. See
  `tests/fixtures/testsuite/NOTICE` for the full accounting.

## 0.1.68 — 2026-08-24 — vendor simd_f64x2.wast/simd_f64x2_pmin_pmax.wast: SIMD widen PR35 (task #220-222)

### Added

- Vendored `simd_f64x2.wast` and `simd_f64x2_pmin_pmax.wast` (both new
  files, not re-fetches of already-vendored ones) at the existing
  pinned commit `28864811cf03bdbf880733786148feaba339582d`. This PR
  implements the 5 opcodes needed to close f64x2's arithmetic family:
  `f64x2.abs` (`0xEC`), `f64x2.min` (`0xF4`), `f64x2.max` (`0xF5`),
  `f64x2.pmin` (`0xF6`), `f64x2.pmax` (`0xF7`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR, a direct structural mirror of PR34's f32x4
  closure. `simd_f64x2.wast` is the upstream corpus's general f64x2
  smoke-test file; the DIFFERENT, SIMPLER "pseudo-min"/"pseudo-max"
  semantics of `pmin`/`pmax` (a plain IEEE-754 `<`-based conditional
  select, no NaN canonicalization -- see wasm-opcodes'
  `SimdOpKind::PminF64x2`/`PmaxF64x2` doc comments) get their own much
  larger dedicated corpus file, `simd_f64x2_pmin_pmax.wast` -- together,
  4687 real directives across 2 files for 5 opcodes.
- `simd_f64x2.wast`: 793/793 `assert_return`, 8/8 `assert_invalid`, 2/2
  modules, ALL 100% passing on the first baseline regen after
  implementation. `simd_f64x2_pmin_pmax.wast`: 3872/3872 `assert_return`,
  6/6 `assert_invalid`, 8/8 `assert_malformed`, 1/1 module, ALL 100%
  passing -- including every one of the corpus's own NaN-operand-order
  vectors, the highest-risk correctness area for this PR (a `pmin`/
  `pmax` implementation that wrongly reused `min`/`max`'s
  NaN-canonicalization logic would have failed a meaningful chunk of
  that file's 3872 `assert_return` directives; it did not). Aggregate
  `assert_return` rose from 38655/38672 to 43320/43337 (+4665 pass,
  `fail` unchanged at 17); `assert_invalid` rose from 1852/1852 to
  1866/1866 (+14, still 100.0% of gradeable directives);
  `assert_malformed` rose from 351/351 to 359/359 (+8, still 100.0% of
  gradeable directives); `module` pass count rose from 1189/1190 to
  1192/1193 (+3, all passing). All deltas are EXACTLY these two files'
  own directive counts -- the pre-existing, unrelated baseline failures
  (17 `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`)
  are byte-for-byte unchanged, confirming zero regressions. See
  `tests/fixtures/testsuite/NOTICE` for the full accounting.

## 0.1.67 — 2026-08-24 — vendor simd_f32x4.wast/simd_f32x4_pmin_pmax.wast: SIMD widen PR34 (task #217-219)

### Added

- Vendored `simd_f32x4.wast` and `simd_f32x4_pmin_pmax.wast` (both new
  files, not re-fetches of already-vendored ones) at the existing
  pinned commit `28864811cf03bdbf880733786148feaba339582d`. This PR
  implements the 3 opcodes needed to close f32x4's arithmetic family:
  `f32x4.max` (`0xE9`), `f32x4.pmin` (`0xEA`), `f32x4.pmax` (`0xEB`),
  implemented in `wasm-opcodes`/`wasm-execution`/`wasm-validator`/
  `wasm-wast-parser` as part of the same PR. `simd_f32x4.wast` is the
  upstream corpus's general f32x4 smoke-test file; the DIFFERENT,
  SIMPLER "pseudo-min"/"pseudo-max" semantics of `pmin`/`pmax` (a plain
  IEEE-754 `<`-based conditional select, no NaN canonicalization -- see
  wasm-opcodes' `SimdOpKind::PminF32x4`/`PmaxF32x4` doc comments) get
  their own much larger dedicated corpus file, `simd_f32x4_pmin_pmax
  .wast` -- together, 4674 real directives across 2 files for 3
  opcodes, the best directive-per-opcode ratio in this campaign so far.
- `simd_f32x4.wast`: 772/772 `assert_return`, 8/8 `assert_invalid`,
  8/8 `assert_malformed`, 2/2 modules, ALL 100% passing on the first
  baseline regen after implementation. `simd_f32x4_pmin_pmax.wast`:
  3872/3872 `assert_return`, 6/6 `assert_invalid`, 8/8
  `assert_malformed`, 1/1 module, ALL 100% passing -- including every
  one of the corpus's own NaN-operand-order vectors, the highest-risk
  correctness area for this PR (a `pmin`/`pmax` implementation that
  wrongly reused `min`/`max`'s NaN-canonicalization logic would have
  failed a meaningful chunk of that file's 3872 `assert_return`
  directives; it did not). Aggregate `assert_return` rose from
  34011/34028 to 38655/38672 (+4644 pass, `fail` unchanged at 17);
  `assert_invalid` rose from 1838/1838 to 1852/1852 (+14, still 100.0%
  of gradeable directives); `assert_malformed` rose from 335/335 to
  351/351 (+16, still 100.0% of gradeable directives); `module` pass
  count rose from 1186/1187 to 1189/1190 (+3, all passing). All deltas
  are EXACTLY these two files' own directive counts -- the pre-existing,
  unrelated baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register`) are byte-for-byte unchanged,
  confirming zero regressions. See `tests/fixtures/testsuite/NOTICE`
  for the full accounting.

## 0.1.66 — 2026-08-24 — vendor simd_i8x16_sat_arith.wast/simd_i16x8_sat_arith.wast: SIMD widen PR33 (task #214-216)

### Added

- Vendored `simd_i8x16_sat_arith.wast` and `simd_i16x8_sat_arith.wast`
  (both new files, not re-fetches of already-vendored ones) at the
  existing pinned commit `28864811cf03bdbf880733786148feaba339582d`.
  This PR implements the 8 opcodes needed for the saturating integer
  add/sub family: `i8x16.add_sat_s` (`0x6F`), `i8x16.add_sat_u` (`0x70`),
  `i8x16.sub_sat_s` (`0x72`), `i8x16.sub_sat_u` (`0x73`),
  `i16x8.add_sat_s` (`0x8F`), `i16x8.add_sat_u` (`0x90`),
  `i16x8.sub_sat_s` (`0x92`), `i16x8.sub_sat_u` (`0x93`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- `simd_i8x16_sat_arith.wast`: 188/188 `assert_return`, 12/12
  `assert_invalid`, 12/12 `assert_malformed`, 2/2 modules, ALL 100%
  passing on the first baseline regen after implementation.
  `simd_i16x8_sat_arith.wast`: 204/204 `assert_return`, 12/12
  `assert_invalid`, 4/4 `assert_malformed`, 2/2 modules, ALL 100%
  passing -- including every one of the corpus's own boundary-value and
  underflow/overflow-saturation vectors (the unsigned-underflow-
  saturates-to-zero direction, this family's classic bug spot, passes
  cleanly at both lane widths). Aggregate `assert_return` rose from
  33619/33636 to 34011/34028 (+392 pass, +392 gradeable, exactly these
  two files' combined `assert_return` count, `fail` unchanged at 17);
  `assert_invalid` rose from 1814/1814 to 1838/1838 (+24, exactly these
  two files' combined count, still 100.0% of gradeable directives);
  `assert_malformed` rose from 319/319 to 335/335 (+16, exactly these
  two files' combined count, still 100.0% of gradeable directives);
  `module` pass count rose from 1182/1183 to 1186/1187 (+4, one per
  module across both files, all passing). The pre-existing, unrelated
  baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register` -- present before this PR and
  tracked separately) are byte-for-byte unchanged, confirming zero
  regressions.
- `fetch_testsuite.py`'s `TESTSUITE_FILES` gains both new filenames at
  the end of the SIMD block, following the established chronological/
  PR-commented convention.
- `NOTICE` updated with the fetch-header extension line and a new
  paragraph account for this PR.
- Baseline regenerated via `--write-baseline`; `corpus_matches_the_
  committed_baseline` passes cleanly against the new baseline.

## 0.1.65 — 2026-08-24 — vendor simd_f64x2_cmp.wast: SIMD widen PR32 (task #211-213)

### Added

- Vendored `simd_f64x2_cmp.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  6 opcodes needed for `f64x2`'s comparison family: `f64x2.eq` (`0x47`),
  `f64x2.ne` (`0x48`), `f64x2.lt` (`0x49`), `f64x2.gt` (`0x4A`),
  `f64x2.le` (`0x4B`), `f64x2.ge` (`0x4C`) -- a direct structural mirror
  of PR30's `simd_f32x4_cmp.wast`, implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR. This is now the single BIGGEST directive-count
  win in this campaign so far, surpassing PR30's `simd_f32x4_cmp.wast`
  (2605 total).
- `simd_f64x2_cmp.wast`: 2659 `assert_return` + 18 `assert_invalid` + 6
  `assert_malformed` = 2683 total gradeable directives across 2 modules,
  ALL 100% passing on the first baseline regen after implementation (0
  `NotYetSupported`, 0 failures). Aggregate `assert_return` rose from
  30960/30977 to 33619/33636 (+2659 pass, +2659 gradeable, exactly this
  file's own `assert_return` count); `assert_invalid` rose from
  1796/1796 to 1814/1814 (+18, exactly this file's own count, still
  100.0% of gradeable directives); `assert_malformed` rose from
  313/313 to 319/319 (+6, exactly this file's own count, still 100.0%
  of gradeable directives); `module` pass count rose from 1180/1181 to
  1182/1183 (+2, one per module in this file, both passing). The
  pre-existing, unrelated baseline failures (17 `assert_return`, 1
  `module`, 1 `assert_unlinkable`, 2 `register` -- present before this
  PR and tracked separately, none touching `f64x2` or this campaign)
  are byte-for-byte unchanged by this PR, confirming zero regressions.
  No other already-vendored file's stats changed.
- Added `simd_f64x2_cmp.wast` to `TESTSUITE_FILES` in
  `fetch_testsuite.py`, at the end of the SIMD block, and updated the
  accompanying `NOTICE` file with the same real vendored/pass counts
  above.

## 0.1.64 — 2026-08-24 — vendor simd_f64x2_arith.wast: SIMD widen PR31 (task #208-210)

### Added

- Vendored `simd_f64x2_arith.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  6 opcodes needed for `f64x2`'s core arithmetic family: `f64x2.neg`
  (`0xED`), `f64x2.sqrt` (`0xEF`), `f64x2.add` (`0xF0`), `f64x2.sub`
  (`0xF1`), `f64x2.mul` (`0xF2`), `f64x2.div` (`0xF3`) -- a direct
  structural mirror of PR29's `simd_f32x4_arith.wast`, implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- `simd_f64x2_arith.wast`: 1806 `assert_return` + 16 `assert_invalid` =
  1822 total gradeable directives across 3 modules, ALL 100% passing on
  the first baseline regen after implementation (0 `NotYetSupported`, 0
  failures). Aggregate `assert_return` rose from 29154/29171 to
  30960/30977 (+1806 pass, +1806 gradeable, exactly this file's own
  `assert_return` count); `assert_invalid` rose from 1780/1780 to
  1796/1796 (+16, exactly this file's own count, still 100.0% of
  gradeable directives); `module` pass count rose from 1177/1178 to
  1180/1181 (+3, one per module in this file, all passing). The
  pre-existing, unrelated baseline failures (17 `assert_return`, 1
  `module`, 1 `assert_unlinkable`, 2 `register` -- present before this
  PR and tracked separately, none touching `f64x2` or this campaign)
  are byte-for-byte unchanged by this PR, confirming zero regressions.
  No other already-vendored file's stats changed.
- Added `simd_f64x2_arith.wast` to `TESTSUITE_FILES` in
  `fetch_testsuite.py`, and regenerated
  `tests/fixtures/testsuite-status.json` via `--write-baseline`.

## 0.1.63 — 2026-08-24 — vendor simd_f32x4_cmp.wast: SIMD widen PR30, now the biggest directive-count win in the campaign (task #205-207)

### Added

- Vendored `simd_f32x4_cmp.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  6 opcodes needed to close `f32x4`'s comparison family: `f32x4.eq`
  (`0x41`), `f32x4.ne` (`0x42`), `f32x4.lt` (`0x43`), `f32x4.gt`
  (`0x44`), `f32x4.le` (`0x45`), `f32x4.ge` (`0x46`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- **Now the single biggest directive-count win in this campaign,
  surpassing PR29's `simd_f32x4_arith.wast`:** 2581 `assert_return` +
  18 `assert_invalid` + 6 `assert_malformed` = 2605 total gradeable
  directives across 2 modules, ALL 100% passing on the first baseline
  regen after implementation (0 `NotYetSupported`, 0 failures).
  Aggregate `assert_return` rose from 26573/26590 to 29154/29171
  (+2581); `assert_invalid` rose from 1762/1762 to 1780/1780 (+18);
  `assert_malformed` rose from 307/307 to 313/313 (+6); `module` pass
  count rose from 1175 to 1177 (+2). The pre-existing, unrelated
  baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register`) are byte-for-byte unchanged by
  this PR. No other already-vendored file's stats changed. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.62 — 2026-08-24 — vendor simd_f32x4_arith.wast: SIMD widen PR29, biggest directive-count win in the campaign (task #202-204)

### Added

- Vendored `simd_f32x4_arith.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  last 5 opcodes needed to close `f32x4`'s core arithmetic family:
  `f32x4.neg` (`0xE1`), `f32x4.sqrt` (`0xE3`), `f32x4.add` (`0xE4`),
  `f32x4.sub` (`0xE5`), `f32x4.div` (`0xE7`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- **The single biggest directive-count win in this campaign so far:**
  1803 `assert_return` + 16 `assert_invalid` = 1819 total gradeable
  directives across 3 modules, ALL 100% passing on the first baseline
  regen after implementation (0 `NotYetSupported`, 0 failures).
  Aggregate `assert_return` rose from 24770/24787 to 26573/26590
  (+1803); `assert_invalid` rose from 1746/1746 to 1762/1762 (+16);
  `module` pass count rose from 1172 to 1175 (+3). The pre-existing,
  unrelated baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register`) are byte-for-byte unchanged by
  this PR. No other already-vendored file's stats changed. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.61 — 2026-08-19 — vendor simd_conversions.wast: SIMD widen PR28, campaign complete (task #199-201)

### Added

- Vendored `simd_conversions.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This is the THIRD and
  FINAL PR of a 3-PR sequence (PR26 "extend", 8 opcodes; PR27
  "narrow", 4 opcodes; this PR's "promote/demote/convert_low", 4
  opcodes) needed to land all 16 opcodes this single upstream file's
  two modules bundle together -- unlike every earlier SIMD file in
  this campaign, `simd_conversions.wast` could NOT be partially
  vendored, since both modules export functions exercising opcodes
  from all three PRs at once. This PR adds the last 4:
  `f32x4.demote_f64x2_zero` (`0x5E`), `f64x2.promote_low_f32x4`
  (`0x5F`), `f64x2.convert_low_i32x4_s` (`0xFE`),
  `f64x2.convert_low_i32x4_u` (`0xFF`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser`
  as part of the same PR.
- **100% pass on EVERY directive** (2/2 modules, 232/232
  `assert_return`, 18/18 `assert_invalid`, 30/30 `assert_malformed` --
  280 directives total), zero `NotYetSupported` anywhere -- the first
  real integration test exercising opcodes from all three PRs
  (extend/narrow/promote-demote-convert_low) together in one corpus
  file, not just three separate opcode-inventory claims. Aggregate
  `assert_return` rose from 24538/24555 to 24770/24787 (+232);
  `assert_invalid` rose from 1728/1728 to 1746/1746 (+18);
  `assert_malformed` rose from 277/277 to 307/307 (+30); `module` pass
  count rose from 1170 to 1172 (+2). No other already-vendored file's
  stats changed. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.
- **Real parser gap found and fixed while vendoring, not just opcode
  work:** before this PR, `simd_conversions.wast` failed to PARSE AT
  ALL (a hard script-level parse error, not a per-directive grading
  gap) because two of its `assert_return` directives use `(v128.const
  f64x2 nan:canonical nan:canonical)` as an EXPECTED value --
  `nan:canonical`/`nan:arithmetic` NaN-class tokens inside a v128
  literal's individual lanes, something `wasm-wast-parser` had no
  representation for at all (only whole-scalar `f32.const`/`f64.const`
  NaN classes were supported). Fixed in `wasm-wast-parser` (see that
  crate's own CHANGELOG): a new `Expected::V128F32x4`/`V128F64x2`
  per-lane representation, gated so a `v128.const` with no NaN-class
  lanes still uses the original byte-exact path unchanged (zero
  regression risk across every already-vendored file using
  `v128.const`, confirmed by the full baseline diff above showing no
  other file's stats moved).

## 0.1.60 — 2026-08-19 — vendor simd_i32x4_trunc_sat_f64x2.wast: SIMD widen PR25, 2 new opcodes (task #190-192)

### Added

- Vendored `simd_i32x4_trunc_sat_f64x2.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. Unlike PR23/PR24, this PR
  DOES add 2 new opcodes: `i32x4.trunc_sat_f64x2_s_zero` (`0xFC`) and
  `i32x4.trunc_sat_f64x2_u_zero` (`0xFD`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR -- the file exercises only those two ops. This is
  the dedicated upstream file for that opcode pair's own full
  boundary-value corpus (zero/negative-zero/fractional/exact-integer/
  in-range/out-of-range/huge-finite/subnormal/inf/nan/signed-and-quiet-
  nan-payload/octal-literal cases, `_s_zero` and `_u_zero` each tested
  independently) plus dedicated `assert_invalid` type-check coverage
  (wrong-operand-type and empty-argument, for both ops). 100% pass on
  EVERY directive (1/1 module, 102/102 assert_return, 4/4
  assert_invalid). Aggregate `assert_return` rose from 24436/24453 to
  24538/24555 (+102 pass, +102 gradeable, exactly this file's own
  `assert_return` count); `assert_invalid` rose from 1724/1724 to
  1728/1728 (+4, still 100.0% of gradeable directives); `module` pass
  count rose from 1169 to 1170 (+1). No other already-vendored file's
  stats changed. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.

## 0.1.59 — 2026-08-19 — vendor simd_i32x4_trunc_sat_f32x4.wast: pure vendoring, zero new opcodes (task #188-189)

### Added

- Vendored `simd_i32x4_trunc_sat_f32x4.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. Like PR23, this PR adds
  ZERO new opcodes to `wasm-opcodes` -- the file was confirmed, before
  vendoring, to use only `i32x4.trunc_sat_f32x4_s`/
  `i32x4.trunc_sat_f32x4_u`, both already fully implemented since SIMD
  widen PR20 (task #177-179). This is the dedicated upstream file for
  that opcode pair's own full boundary-value corpus (zero/negative-zero/
  fractional/exact-integer/in-range/out-of-range/huge-finite/subnormal/
  inf/nan/signed-and-quiet-nan-payload/octal-literal cases, `_s` and `_u`
  each tested independently) plus dedicated `assert_invalid` type-check
  coverage (wrong-operand-type and empty-argument, for both ops) -- a
  much larger, more direct slice than the single directive reachable
  through `simd_load.wast`'s own `v128.load`-wrapped coverage. 100% pass
  on EVERY directive (1/1 module, 102/102 assert_return, 4/4
  assert_invalid). Aggregate `assert_return` rose from 24334/24351 to
  24436/24453 (+102 pass, +102 gradeable, exactly this file's own
  `assert_return` count); `assert_invalid` rose from 1720/1720 to
  1724/1724 (+4, still 100.0% of gradeable directives); `module` pass
  count rose from 1168 to 1169 (+1). No source changes to
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`/`wasm-opcodes` --
  only this crate's fixtures/baseline/version/changelog are touched, same
  as PR23. See `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.58 — 2026-08-19 — vendor simd_select.wast + simd_address.wast: pure vendoring, zero new opcodes (task #186-187)

### Added

- Vendored `simd_select.wast` and `simd_address.wast` (both new files,
  not re-fetches of already-vendored ones) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. Unlike every prior PR in
  this SIMD widening campaign, this PR adds ZERO new opcodes to
  `wasm-opcodes` -- both files were confirmed, before vendoring, to use
  only opcodes already fully implemented: `simd_select.wast` exercises
  untyped `select` (`0x1B`) with `v128` operands (the parametric `select`
  handler and the validator's `select` type-check rule are both already
  fully generic over `ValueType`, `V128` included, with no SIMD-specific
  gating anywhere); `simd_address.wast` exercises `v128.load`/
  `v128.store` memarg offset/align edge cases, both implemented since
  PR15 (task #162-164). 100% pass on EVERY directive in both files:
  `simd_select.wast` (1/1 module, 6/6 assert_return); `simd_address.wast`
  (3/3 modules, 36/36 assert_return, 6/6 assert_trap, 2/2 assert_invalid,
  2/2 assert_malformed) -- 46 directives total, zero `NotYetSupported`
  anywhere. Aggregate `assert_return` rose from 24292/24309 to
  24334/24351 (+42 pass, +42 gradeable, exactly the two files' combined
  `assert_return` count); `assert_trap` +6, `assert_invalid` +2,
  `assert_malformed` +2 (all still 100.0% of gradeable directives);
  `module` pass count rose from 1164 to 1168 (+4). No source changes to
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`/`wasm-opcodes` --
  only this crate's fixtures/baseline/version/changelog are touched. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.57 — 2026-08-19 — vendor simd_i16x8_q15mulr_sat_s.wast: i16x8.q15mulr_sat_s Q15 rounding saturating multiply (task #183-185)

### Added

- Vendored `simd_i16x8_q15mulr_sat_s.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d` -- `i16x8.q15mulr_sat_s`
  (see `wasm-opcodes`'s own CHANGELOG entry) is the first genuinely new
  SIMD op family/semantic since the "extmul" widening-multiply arc
  completed in PR21. 100% pass on EVERY directive kind in the new file
  (1/1 module, 26/26 assert_return, 3/3 assert_invalid) on the first
  baseline regen after implementation, including the upstream corpus's
  own saturating-edge-case vectors -- confirming the Q15
  rounding-then-saturating formula (not a wrapping cast) is correct.
  Aggregate `assert_return` rose from 24266/24283 to 24292/24309 (+26
  pass, +26 gradeable, exactly this file's own `assert_return` count);
  `assert_invalid` rose from 1715/1715 to 1718/1718 (+3, still 100.0%
  of gradeable directives); `module` pass count rose from 1163 to 1164
  (+1). See `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.56 — 2026-08-19 — vendor simd_i64x2_extmul_i32x4.wast: i64x2.extmul_i32x4 widening-multiply family (task #180-182)

### Added

- Vendored `simd_i64x2_extmul_i32x4.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d` -- `i64x2.extmul_low`/
  `high_i32x4_s`/`_u` (see `wasm-opcodes`'s own CHANGELOG entry) are
  the third and final "extmul" rung this crate now implements. 100%
  pass on EVERY directive kind in the new file (1/1 module, 104/104
  assert_return, 12/12 assert_invalid) on the first baseline regen
  after implementation. Aggregate `assert_return` rose from
  24162/24179 to 24266/24283 (+104 pass, +104 gradeable, exactly this
  file's own `assert_return` count); `assert_invalid` rose from
  1703/1703 to 1715/1715 (+12, still 100.0% of gradeable directives);
  `module` pass count rose from 1162 to 1163 (+1). See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.55 — 2026-08-19 — baseline regen: i32x4/f32x4 trunc_sat/convert fully resolve simd_load.wast (task #177-179)

### Changed

- Baseline regen, no new file vendored: `i32x4.trunc_sat_f32x4_s`/`_u`/
  `f32x4.convert_i32x4_s`/`_u` (see `wasm-opcodes`'s own CHANGELOG
  entry) unblock the LAST 2 remaining `assert_return` directives in the
  already-vendored `simd_load.wast` (task #162-164) --
  `as-i32x4.trunc_sat_f32x4_s-operand` and
  `as-f32x4.convert_i32x4_u-operand`, each the sole directive of its
  own single-func module, and each depending on nothing else
  unimplemented (just `v128.load` plus the one new op). `simd_load.wast`
  now has ZERO stuck directives -- 100% `assert_return`/`module` pass
  rate for this file. Aggregate `assert_return` rose from 24160/24177
  to 24162/24179 (+2 pass, +2 gradeable, exactly matching the predicted
  unblock count); `module` pass count rose from 1160 to 1162 (+2) and
  its `NotYetSupported` count fell from 70 to 68. Also re-checked
  `simd_splat.wast`'s own still-`NotYetSupported` module (task
  #165-167): its still-missing-opcode count dropped from 16 to 14 (2 of
  this PR's 4 new ops are used inside it), but its instantiation still
  fails as a whole on the other 14 opcodes, so it stays
  `NotYetSupported`. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.

## 0.1.54 — 2026-08-19 — baseline regen: f32x4.abs/mul/min unblock simd_load.wast (task #174-176)

### Changed

- Baseline regen, no new file vendored: `f32x4.abs`/`f32x4.mul`/
  `f32x4.min` (see `wasm-opcodes`'s own CHANGELOG entry) unblock the 3
  remaining `assert_return` directives in the already-vendored
  `simd_load.wast` (task #162-164) that needed exactly these ops:
  `as-f32x4.abs-operand`, `as-f32x4.mul-operand`,
  `as-f32x4.min-operand`, each the sole directive of its own
  single-func module, and each depending on nothing else unimplemented
  (just `v128.load` plus the one new op). The other 2 previously-stuck
  directives (`as-i32x4.trunc_sat_f32x4_s-operand`,
  `as-f32x4.convert_i32x4_u-operand`) need float<->int conversion
  opcodes this PR doesn't touch, and stay `NotYetSupported` -- this
  file's `NotYetSupported` tally is now fully accounted for. Aggregate
  `assert_return` rose from 24157/24174 to 24160/24177 (+3 pass, +3
  gradeable, exactly matching the predicted unblock count); `module`
  pass count rose from 1157 to 1160 (+3) and its `NotYetSupported`
  count fell from 73 to 70. Also checked (cheaply) whether
  `simd_splat.wast`'s own still-`NotYetSupported` module picked up any
  credit: it did not, since that module's instantiation fails as a
  whole on at least 11 OTHER still-unimplemented opcodes used
  elsewhere in the same module. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.53 — 2026-08-19 — baseline regen: i8x16.swizzle/extract_lane_s unblock simd_load.wast (task #171-173)

### Changed

- Baseline regen, no new file vendored: `i8x16.swizzle`/
  `i8x16.extract_lane_s` (see `wasm-opcodes`'s own CHANGELOG entry;
  `i8x16.extract_lane_u`/`replace_lane` also landed in the same PR but
  exercise no directive in this file) unblock 2 of the 7
  `assert_return` directives in the already-vendored `simd_load.wast`
  (task #162-164) that were stuck `NotYetSupported`:
  `as-i8x16_extract_lane_s-value/0` and `as-i8x16.swizzle-operand`,
  each the sole directive of its own single-func module, and each
  depending on nothing else unimplemented (just `v128.load` plus the
  one new op) -- exactly as confirmed by re-reading the file before
  regenerating. The other 5 stuck directives need float-lane
  arithmetic/conversion ops (`f32x4.mul`/`f32x4.abs`/`f32x4.min`/
  `i32x4.trunc_sat_f32x4_s`/`f32x4.convert_i32x4_u`) this PR doesn't
  touch, and stay `NotYetSupported`. Aggregate `assert_return` rose
  from 24155/24172 to 24157/24174 (+2 pass, +2 gradeable, exactly
  matching the predicted unblock count); `module` pass count rose
  from 1155 to 1157 (+2) and its `NotYetSupported` count fell from 75
  to 73. See `tests/fixtures/testsuite/NOTICE` for the full breakdown.

## 0.1.52 — 2026-08-19 — baseline regen: f32x4/f64x2.splat unblocks simd_splat.wast (task #168-170)

### Changed

- Baseline regen, no new file vendored: `f32x4.splat`/`f64x2.splat`
  (see `wasm-opcodes`'s own CHANGELOG entry) unblock 115 of the 158
  `assert_return` directives in the already-vendored `simd_splat.wast`
  (task #165-167) that were stuck `NotYetSupported` -- exactly as
  predicted when that file was vendored. 3 of its 4 modules (pure
  splat, splat-into-store/load, splat-into-control-construct) needed
  ONLY the two new float splats to build; the remaining module (43
  directives) stays `NotYetSupported`, since it additionally needs
  `extract_lane`/`replace_lane` for multiple lane widths and
  `i8x16.swizzle`, none implemented yet. Aggregate `assert_return`
  rose from 24040/24057 to 24155/24172 (+115 pass, +115 gradeable,
  matching the predicted unblock count exactly); `module` pass count
  rose from 1152 to 1155 (+3) and its `NotYetSupported` count fell
  from 78 to 75. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.

## 0.1.51 — 2026-08-19 — vendor simd_splat.wast; baseline regen (task #165-167)

### Changed

- Baseline regen: vendored `simd_splat.wast` -- `i8x16.splat`/
  `i16x8.splat`/`i64x2.splat`, widening lane-width coverage of the
  already-implemented `i32x4.splat`, see `wasm-opcodes`'s own
  CHANGELOG entry. Vendoring this file first surfaced a real,
  unrelated `wasm-wast-parser` bug (a `_` digit separator inside a
  `nan:0x<payload>` literal made the whole script fail to parse) --
  fixed as part of this same PR, see that crate's own CHANGELOG.
- Once the file parses, its own upstream structure bundles all 6 lane
  widths' splat exports (including `f32x4.splat`/`f64x2.splat`,
  neither implemented -- this crate has zero float-lane SIMD support
  at all) into the SAME modules the new integer-lane directives
  depend on, so every one of this file's 158 `assert_return`
  directives grades `NotYetSupported` until float-lane SIMD support
  lands in a future PR. `assert_invalid` (22/22) and `assert_malformed`
  (1/1) DO pass today, since those directives don't require the
  shared modules to build. Vendored now anyway since it's the real
  upstream file, the 3 new opcodes are independently verified via
  dedicated unit tests in the meantime, and this file will
  automatically flip to real credit with zero further changes once
  float-lane support exists -- see `tests/fixtures/testsuite/NOTICE`
  for the full breakdown. Aggregate `assert_return` stays 24040/24057
  (pass/gradeable, unchanged) but its `NotYetSupported` count rose
  from 552 to 710 (+158, exactly this file's own `assert_return`
  count); `assert_invalid` rose from 1681/1681 to 1703/1703 (+22) and
  `assert_malformed` from 274/274 to 275/275 (+1).

## 0.1.50 — 2026-08-18 — vendor simd_load.wast/simd_store.wast; baseline regen (task #162-164)

### Changed

- Baseline regen: vendored `simd_load.wast`/`simd_store.wast` --
  `v128.load`/`v128.store`, the first SIMD ops touching real linear
  memory, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on every
  gradeable directive kind across both files (9/9 modules, 27/27
  assert_return, 11/11 assert_invalid, 6/6 assert_malformed); 7
  modules and 7 assert_return directives in `simd_load.wast` grade
  `NotYetSupported` (float-lane ops and `i8x16.swizzle`, none
  implemented yet). Landing `v128.load` ALSO retroactively resolved 76
  previously-`NotYetSupported` `assert_return` directives (and 5
  previously-`NotYetSupported` modules) spread across five UNRELATED,
  already-vendored files whose own tails depended on a real
  `v128.load`: `simd_bitwise.wast` (+13), `simd_bit_shift.wast` (+24),
  `simd_i16x8_cmp.wast` (+13), `simd_i32x4_cmp.wast` (+13), and
  `simd_i8x16_cmp.wast` (+13) -- see `tests/fixtures/testsuite/NOTICE`
  for the full breakdown. Aggregate `assert_return` rose from
  23937/23954 to 24040/24057 (+103 gradeable: +27 from the two new
  files' own passing directives, +76 from the five-file ripple);
  `assert_invalid` rose by 11 and `assert_malformed` rose by 6 (both
  still 100.0% of gradeable directives).

## 0.1.49 — 2026-08-18 — vendor simd_bit_shift.wast; baseline regen (task #159-161)

### Changed

- Baseline regen: vendored `simd_bit_shift.wast` -- `ixNxM.shl`/
  `shr_s`/`shr_u` across all 4 lane widths, the first mixed-type
  binary SIMD op family (`v128` + scalar `i32`, not two `v128`s), see
  `wasm-opcodes`'s own CHANGELOG entry. 100% pass on every gradeable
  directive kind (1/1 modules, 187/187 assert_return, 24/24
  assert_invalid, 15/15 assert_malformed; 24 assert_return directives
  grade `NotYetSupported`). Aggregate `assert_return` rose from
  23750/23767 to 23937/23954 (+187, exactly matching the new file's
  own count, no ripple into unrelated files this time); `assert_invalid`
  rose by 24 and `assert_malformed` rose by 15 (both still 100.0% of
  gradeable directives).

## 0.1.48 — 2026-08-18 — vendor simd_i64x2_arith.wast/simd_i64x2_arith2.wast/simd_i64x2_cmp.wast; baseline regen (task #156-158)

### Changed

- Baseline regen: vendored `simd_i64x2_arith.wast`/`simd_i64x2_arith2.
  wast`/`simd_i64x2_cmp.wast` -- i64x2's first REAL ARITHMETIC family
  (`abs`/`neg`/`add`/`sub`/`mul`/`eq`/`ne`/`lt_s`/`gt_s`/`le_s`/
  `ge_s`), see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on
  every directive kind across all three files (5/5 modules, 310/310
  assert_return, 23/23 assert_invalid). Implementing real i64x2
  arithmetic also unblocked 22 previously-`NotYetSupported`
  `assert_return` directives in the UNRELATED, already-vendored
  `simd_const.wast` (its i64x2 `v128.const` round-trip cases needed a
  real i64x2 op to observe the result through) -- a legitimate ripple
  effect, not a regression. Aggregate `assert_return` rose from
  23418/23435 to 23750/23767 (332 = 310 new + 22 newly-unblocked);
  `assert_invalid` rose by 23 (still 100.0% of gradeable directives).

## 0.1.47 — 2026-08-18 — vendor simd_boolean.wast; baseline regen (task #153-155)

### Changed

- Baseline regen: vendored `simd_boolean.wast` -- `v128.any_true` +
  `ixNxM.all_true`/`bitmask` across all 4 lane widths, the first
  `v128`-in/`i32`-out reduction shape besides `extract_lane` and the
  first opcodes in this interpreter to read the operand as 8-byte
  (`i64`) lanes, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass
  on every directive kind (2/2 modules, 259/259 assert_return, 12/12
  assert_invalid, 4/4 assert_malformed). Aggregate `assert_return`
  rose from 23159/23176 to 23418/23435; `assert_invalid` rose by 12
  (still 100.0% of gradeable directives); `assert_malformed` rose by 4
  (still 100.0% of gradeable directives).

## 0.1.46 — 2026-08-18 — vendor simd_bitwise.wast; baseline regen (task #150-152)

### Changed

- Baseline regen: vendored `simd_bitwise.wast` -- `v128.not`/`and`/
  `andnot`/`or`/`xor`/`bitselect`, the lane-width-agnostic raw-byte
  bitwise family, a strategic pivot from "widen the next narrow
  per-lane-width family" to "close the highest-real-world-impact
  remaining gap" now that `i8x16`/`i16x8`/`i32x4` all have complete
  arith+cmp+arith2+widening coverage, see `wasm-opcodes`'s own
  CHANGELOG entry. 100% pass on every gradeable directive kind (1/1
  modules, 126/126 assert_return, 28/28 assert_invalid; 13
  assert_return directives grade `NotYetSupported` because they
  depend on `v128.load`, not a real failure). Aggregate
  `assert_return` rose from 23033/23050 to 23159/23176;
  `assert_invalid` rose by 28 (still 100.0% of gradeable directives).

## 0.1.45 — 2026-08-18 — vendor simd_i16x8_extadd_pairwise_i8x16.wast/simd_i16x8_extmul_i8x16.wast; baseline regen (task #147-149)

### Changed

- Baseline regen: vendored `simd_i16x8_extadd_pairwise_i8x16.wast` and
  `simd_i16x8_extmul_i8x16.wast` -- `i16x8`'s own widening family
  (extadd_pairwise_i8x16_s/u, extmul_low/high_i8x16_s/u), mirroring
  the already-implemented `i32x4`-from-`i16x8` widening family one
  lane width down, closing the last remaining gap between `i16x8` and
  `i8x16`'s coverage, see `wasm-opcodes`'s own CHANGELOG entry. 100%
  pass on EVERY directive kind across both files (2/2 modules,
  120/120 assert_return, 16/16 assert_invalid). Aggregate
  `assert_return` rose from 22913/22930 to 23033/23050;
  `assert_invalid` rose by 16 (still 100.0% of gradeable directives).

## 0.1.44 — 2026-08-18 — vendor simd_i16x8_arith2.wast; baseline regen (task #144-146)

### Changed

- Baseline regen: vendored `simd_i16x8_arith2.wast` -- `i16x8`'s own
  abs/min_s/min_u/max_s/max_u/avgr_u family, closing the same
  "arith2" gap PR8 just closed for `i8x16` (no `i16x8.popcnt` -- WASM
  SIMD only defines `popcnt` for `i8x16`), see `wasm-opcodes`'s own
  CHANGELOG entry. 100% pass on EVERY directive kind (2/2 modules,
  151/151 assert_return, 17/17 assert_invalid, 2/2 assert_malformed).
  Aggregate `assert_return` rose from 22762/22779 to 22913/22930;
  `assert_invalid` rose by 17 (still 100.0% of gradeable directives);
  `assert_malformed` rose by 2 (also 100.0% of gradeable directives).

## 0.1.43 — 2026-08-18 — vendor simd_i8x16_arith2.wast; baseline regen (task #141-143)

### Changed

- Baseline regen: vendored `simd_i8x16_arith2.wast` -- `i8x16`'s own
  abs/popcnt/min_s/min_u/max_s/max_u/avgr_u family, mirroring `i32x4`'s
  own abs/min/max widening plus two op shapes (popcnt, avgr_u) with no
  `i32x4`/`i16x8` precedent, see `wasm-opcodes`'s own CHANGELOG entry.
  100% pass on EVERY directive kind (2/2 modules, 184/184
  assert_return, 19/19 assert_invalid, 6/6 assert_malformed) -- no
  `NotYetSupported` tail this time, unlike `simd_i8x16_cmp.wast`'s own
  `v128.load`-dependent one. Aggregate `assert_return` rose from
  22578/22595 to 22762/22779; `assert_invalid` rose by 19 (all still
  100.0% of gradeable directives); `assert_malformed` rose by 6 (also
  100.0% of gradeable directives).

## 0.1.42 — 2026-08-18 — vendor simd_i8x16_cmp.wast; baseline regen (task #137-140)

### Changed

- Baseline regen: vendored `simd_i8x16_cmp.wast` -- `i8x16`'s own
  comparison family (eq/ne/lt_s/lt_u/gt_s/gt_u/le_s/le_u/ge_s/ge_u),
  closing the same gap PR6 closed for `i16x8`: `i8x16.add`/`sub`/`neg`
  landed without one, see `wasm-opcodes`'s own CHANGELOG entry. 100%
  pass on every GRADEABLE directive (400/400 assert_return, 30/30
  assert_invalid); the file's own small "combination" tail references
  `v128.load` (not yet implemented), so 1 module and 13 assert_return
  directives grade `NotYetSupported`, same lazy-grading discipline
  already established for `simd_i16x8_cmp.wast`'s own tail. Aggregate
  `assert_return` rose from 22178/22195 to 22578/22595; `assert_invalid`
  from 1501 to 1531.

## 0.1.41 — 2026-08-18 — vendor simd_i16x8_cmp.wast; baseline regen (task #133-136)

### Changed

- Baseline regen: vendored `simd_i16x8_cmp.wast` -- `i16x8`'s own
  comparison family (eq/ne/lt_s/lt_u/gt_s/gt_u/le_s/le_u/ge_s/ge_u),
  closing the gap left when `i16x8.add`/`sub`/`mul`/`neg` landed
  without one, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on
  every GRADEABLE directive (420/420 assert_return, 30/30
  assert_invalid); the file's own small "combination" tail references
  `v128.load` (not yet implemented), so 1 module and 13 assert_return
  directives grade `NotYetSupported`, same lazy-grading discipline
  already established for `simd_i32x4_cmp.wast`'s own `trunc_sat`-
  dependent tail. Aggregate `assert_return` rose from 21758/21775 to
  22178/22195; `assert_invalid` from 1471 to 1501.

## 0.1.40 — 2026-08-18 — vendor simd_i16x8_arith.wast; baseline regen (task #129-132)

### Changed

- Baseline regen: vendored `simd_i16x8_arith.wast` -- the first file
  where `i16x8` is a PRIMARY lane width (produces `i16x8` results)
  rather than merely an input to an `i32x4`-producing widening op.
  `i16x8.add`/`sub`/`mul`/`neg` are the first such opcodes this repo
  implements, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on
  every directive kind (2/2 modules, 181/181 assert_return, 11/11
  assert_invalid). Aggregate `assert_return` rose from 21577/21594 to
  21758/21775; `assert_invalid` from 1460 to 1471.

## 0.1.39 — 2026-08-18 — vendor simd_i8x16_arith.wast; baseline regen (task #125-128)

### Changed

- Baseline regen: vendored `simd_i8x16_arith.wast` -- this arc's first
  pivot to a brand-new lane width rather than a further widening of
  `i32x4` (`i32x4` had run out of small increments; everything left
  needs a float lane width). `i8x16.add`/`sub`/`neg` are the first
  `i8x16` opcodes this repo implements, see `wasm-opcodes`'s own
  CHANGELOG entry. 100% pass on every directive kind (2/2 modules,
  121/121 assert_return, 8/8 assert_invalid). Aggregate `assert_return`
  rose from 21456/21473 to 21577/21594; `assert_invalid` from 1452 to
  1460.

## 0.1.38 — 2026-08-18 — vendor i32x4-from-i16x8 SIMD files; baseline regen (task #121-124)

### Changed

- Baseline regen: vendored `simd_i32x4_extadd_pairwise_i16x8.wast`,
  `simd_i32x4_dot_i16x8.wast`, and `simd_i32x4_extmul_i16x8.wast` -- the
  first SIMD opcodes this repo implements whose input lane width
  (`i16x8`) differs from their output lane width (`i32x4`), see
  `wasm-opcodes`'s own CHANGELOG entry for the 7 newly-added opcodes.
  100% pass on every directive kind across all three files: 3/3 modules,
  148/148 assert_return, 19/19 assert_invalid. Aggregate `assert_return`
  rose from 21308/21325 to 21456/21473; `assert_invalid` from 1433 to
  1452.

## 0.1.37 — 2026-08-18 — vendor simd_i32x4_arith2.wast; baseline regen (task #118-120)

### Changed

- Baseline regen: vendored `simd_i32x4_arith2.wast`, the upstream
  corpus's own "second half" of `i32x4` arithmetic coverage -- `i32x4.abs`
  (the first UNARY opcode besides `neg`) plus the `min_s`/`min_u`/
  `max_s`/`max_u` family (see `wasm-opcodes`'s own CHANGELOG entry for
  the 5 newly-added opcodes). 100% pass on every directive kind with
  zero `NotYetSupported` at all (2/2 modules, 121/121 assert_return,
  14/14 assert_invalid, 12/12 assert_malformed) -- the first SIMD file
  this repo vendors with no partial-credit directives whatsoever.
  Aggregate `assert_return` rose from 21187/21204 to 21308/21325;
  `assert_invalid` from 1419 to 1433; `assert_malformed` from 229 to 241.

## 0.1.36 — 2026-08-18 — vendor simd_i32x4_arith.wast + simd_i32x4_cmp.wast; baseline regen (task #113-117)

### Changed

- Baseline regen: vendored `simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast`,
  previously deferred as needing more `i32x4` SIMD opcode coverage than
  the original 5-opcode first slice provided (see `wasm-opcodes`'s own
  CHANGELOG entry for the 12 newly-added opcodes). `simd_i32x4_arith.wast`:
  100% pass on every gradeable directive (2/2 modules, 181/181
  assert_return, 11/11 assert_invalid). `simd_i32x4_cmp.wast`: 100% pass
  on every gradeable directive (420/420 assert_return, 30/30
  assert_invalid); its one `NotYetSupported` module and 13
  `NotYetSupported` assert_return directives exercise
  `i32x4.trunc_sat_f32x4_s`/`f32x4.const` float-to-int boundary values,
  genuinely out of scope for this i32x4-only widening pass. Aggregate
  `assert_return` rose from 20586/20603 to 21187/21204.

## 0.1.35 — 2026-08-17 — vendor memory-multi.wast; baseline regen (task #92/#112)

### Changed

- Baseline regen: vendored `memory-multi.wast`, previously blocked on
  real multi-memory memarg support (flags-bit `0x40` + memidx across all
  23 memarg opcodes, plus `memory.init`/`memory.fill`'s own memidx --
  see `wasm-execution`/`wasm-wast-parser`/`wasm-validator`'s own
  CHANGELOG entries and `code/specs/W18-wasm-multi-memory-memarg.md`).
  100% pass on every directive kind: 2/2 modules, 4/4 assert_return.

## 0.1.34 — 2026-08-17 — vendor table.wast; baseline regen (task #99)

### Changed

- Baseline regen: vendored `table.wast`, previously deferred as blocked
  on hex-literal table limits and a `spectest` import -- both rescoped
  as tractable this session (see `wasm-wast-parser`'s own CHANGELOG
  entry for the hex-literal fix; the `spectest` directive grades
  `NotYetSupported` via the harness's existing unresolved-import
  handling, same pattern `linking.wast` already established).
- One genuine, EXPECTED `module` failure, not a bug: `(module
  definition (table 0xffff_ffff funcref))` declares a table with a
  4-billion-element minimum. The real spec permits arbitrarily large
  declared minimums; this interpreter's own `MAX_TABLE_ELEMENTS`
  resource-limit guard (task #96/#98, a deliberate DoS-safety
  tradeoff) rejects it. This is the same class of documented,
  accepted divergence as the pre-existing 17 `assert_return`/2
  `register`/1 `assert_unlinkable` failures already in the baseline --
  not something to "fix" by loosening the cap.

## 0.1.33 — 2026-08-17 — baseline regen after call_indirect real table-index fix (task #107)

### Changed

- Baseline regen: fixing `call_indirect`/`return_call_indirect`'s
  explicit-table-index handling (see `wasm-execution`/`wasm-wast-parser`'s
  own CHANGELOG entries for this version) let most modules in
  `table_init.wast`/`table_copy.wast` build and run for the first time.
  Aggregate `assert_return` `NotYetSupported` dropped 983 -> 562,
  `assert_trap` 1789 -> 938, `module` 88 -> 65, with the pre-existing 17
  `assert_return`/2 `register`/1 `assert_unlinkable` failures byte-for-
  byte unchanged -- zero regressions, only new passes.

## 0.1.32 — 2026-08-17 — vendor table_init.wast + table_copy.wast; baseline regen (task #97)

### Changed

- Baseline regen: vendored `table_init.wast` and `table_copy.wast` --
  every directive that actually runs passes (module 17/17 and 23/23,
  assert_invalid 67/67, assert_trap 8/8 and 14/14, assert_return 9/9
  combined), zero regressions anywhere else in the corpus (the
  pre-existing 17 `assert_return`/2 `register`/1 `assert_unlinkable`
  failures are byte-for-byte unchanged from the prior baseline).
- Both files' `module`/`assert_return`/`assert_trap` directive
  coverage is real but partial: most modules in each file use
  `(call_indirect $t0 (type N) ...)` -- the reference-types proposal's
  explicit-table-index text syntax -- which `wasm-wast-parser` doesn't
  parse yet (it hardcodes table 0 for every `call_indirect`, see task
  #107, logged as new backlog work discovered by this vendoring pass).
  A module using that syntax fails to build entirely and every
  directive against it grades `NotYetSupported`, not `Fail` -- this is
  the SAME "vendor now with a real, non-silent capability-gap count;
  widen coverage later" pattern this crate already uses for
  `simd_const.wast`'s partial opcode coverage.

## 0.1.31 — 2026-08-16 — vendor table_size.wast + table_fill.wast; baseline regen (task #98)

### Changed

- Baseline regen: vendored `table_size.wast` and `table_fill.wast` --
  both 100% pass, every directive kind, on the first regen after
  implementing `table.grow`/`table.size`/`table.fill` (entirely
  unimplemented before this task). Zero regressions anywhere else in
  the now-69-file vendored corpus (per-file/per-kind aggregate deltas
  confirmed to match exactly what the 2 new files contribute, nothing
  else). Deliberately excludes their sibling `table_grow.wast`: its own
  corpus uses `(elem declare func $f)` -- declarative element segments,
  a third element-segment mode (alongside active/passive) this repo has
  no concept of yet -- and cross-module `register`/import table-growth
  propagation, both fine follow-on work once `elem declare` lands,
  most naturally alongside task #97's own Element/is_passive rework.

## 0.1.30 — 2026-08-16 — vendor memory_init.wast; baseline regen (task #95)

### Changed

- Baseline regen: vendored `memory_init.wast` -- 100% pass, every
  directive kind, on the first regen after implementation. Needed real
  new interpreter state (`memory.init`/`data.drop` were entirely
  unimplemented) and real passive data segment support (`wasm-module-
  parser`'s binary decoder only ever handled segment-mode flag `0x00`
  before this task). Zero regressions anywhere else in the now-67-file
  vendored corpus (full before/after per-file/per-kind diff). Only uses
  single-memory numeric segment indices, so it's vendorable standalone
  -- unlike its siblings `bulk.wast` (task #97 still pending) and
  `memory-multi.wast` (task #92, now unblocked and vendorable as a
  near-zero-cost follow-up).

See `code/packages/rust/wasm-module-parser/CHANGELOG.md`, `code/
packages/rust/wasm-wast-parser/CHANGELOG.md`, `code/packages/rust/
wasm-execution/CHANGELOG.md`, and `code/packages/rust/wasm-validator/
CHANGELOG.md`/`code/packages/rust/wasm-runtime/CHANGELOG.md` for the
parsing, interpreter, and validation-layer changes this vendoring pass
needed.

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
