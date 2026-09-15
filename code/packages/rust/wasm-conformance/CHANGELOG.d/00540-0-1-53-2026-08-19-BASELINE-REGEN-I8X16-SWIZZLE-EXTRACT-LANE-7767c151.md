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

