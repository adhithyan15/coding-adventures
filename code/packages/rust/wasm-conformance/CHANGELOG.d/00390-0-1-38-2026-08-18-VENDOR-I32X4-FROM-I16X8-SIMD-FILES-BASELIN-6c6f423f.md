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

