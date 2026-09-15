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

