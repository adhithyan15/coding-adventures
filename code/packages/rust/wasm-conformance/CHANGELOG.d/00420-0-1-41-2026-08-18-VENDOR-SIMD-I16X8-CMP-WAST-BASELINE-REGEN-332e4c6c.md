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

