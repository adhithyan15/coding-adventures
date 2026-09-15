## 0.1.47 — 2026-08-18 — vendor simd_boolean.wast; baseline regen (task #153-155)

### Changed

- Baseline regen: vendored `simd_boolean.wast` -- `v128.any_true` +
  `ixNxM.all_true`/`bitmask` across all 4 lane widths, the first
  `v128`-in/`i32`-out reduction shape besides `extract_lane` and the
  first opcodes in this interpreter to read the operand as 8-byte
  (`i64`) lanes, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass
  on every directive kind (2/2 modules, 259/259 assert_return, 12/12
  assert_invalid, 4/4 assert_malformed). Aggregate `assert_return`
  rose from 23159/23176 to 23418/23435; `assert_invalid` rose by 12
  (still 100.0% of gradeable directives); `assert_malformed` rose by 4
  (still 100.0% of gradeable directives).

