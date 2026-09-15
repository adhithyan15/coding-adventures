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

