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

