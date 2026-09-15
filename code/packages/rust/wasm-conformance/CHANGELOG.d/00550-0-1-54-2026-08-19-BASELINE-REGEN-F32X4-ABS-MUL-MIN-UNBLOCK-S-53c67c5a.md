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

