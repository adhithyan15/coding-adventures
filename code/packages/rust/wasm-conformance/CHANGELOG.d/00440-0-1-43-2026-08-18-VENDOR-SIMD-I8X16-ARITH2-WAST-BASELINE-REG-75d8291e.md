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

