## 0.1.37 — 2026-08-18 — vendor simd_i32x4_arith2.wast; baseline regen (task #118-120)

### Changed

- Baseline regen: vendored `simd_i32x4_arith2.wast`, the upstream
  corpus's own "second half" of `i32x4` arithmetic coverage -- `i32x4.abs`
  (the first UNARY opcode besides `neg`) plus the `min_s`/`min_u`/
  `max_s`/`max_u` family (see `wasm-opcodes`'s own CHANGELOG entry for
  the 5 newly-added opcodes). 100% pass on every directive kind with
  zero `NotYetSupported` at all (2/2 modules, 121/121 assert_return,
  14/14 assert_invalid, 12/12 assert_malformed) -- the first SIMD file
  this repo vendors with no partial-credit directives whatsoever.
  Aggregate `assert_return` rose from 21187/21204 to 21308/21325;
  `assert_invalid` from 1419 to 1433; `assert_malformed` from 229 to 241.

