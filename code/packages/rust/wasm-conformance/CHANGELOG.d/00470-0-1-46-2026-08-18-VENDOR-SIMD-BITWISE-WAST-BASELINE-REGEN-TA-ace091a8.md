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

