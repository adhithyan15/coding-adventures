## 0.1.39 — 2026-08-18 — vendor simd_i8x16_arith.wast; baseline regen (task #125-128)

### Changed

- Baseline regen: vendored `simd_i8x16_arith.wast` -- this arc's first
  pivot to a brand-new lane width rather than a further widening of
  `i32x4` (`i32x4` had run out of small increments; everything left
  needs a float lane width). `i8x16.add`/`sub`/`neg` are the first
  `i8x16` opcodes this repo implements, see `wasm-opcodes`'s own
  CHANGELOG entry. 100% pass on every directive kind (2/2 modules,
  121/121 assert_return, 8/8 assert_invalid). Aggregate `assert_return`
  rose from 21456/21473 to 21577/21594; `assert_invalid` from 1452 to
  1460.

