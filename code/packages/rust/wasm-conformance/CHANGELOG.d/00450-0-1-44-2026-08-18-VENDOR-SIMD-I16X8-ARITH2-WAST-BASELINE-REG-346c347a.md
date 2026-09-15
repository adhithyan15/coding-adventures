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

