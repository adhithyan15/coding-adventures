## 0.1.48 — 2026-08-18 — vendor simd_i64x2_arith.wast/simd_i64x2_arith2.wast/simd_i64x2_cmp.wast; baseline regen (task #156-158)

### Changed

- Baseline regen: vendored `simd_i64x2_arith.wast`/`simd_i64x2_arith2.
  wast`/`simd_i64x2_cmp.wast` -- i64x2's first REAL ARITHMETIC family
  (`abs`/`neg`/`add`/`sub`/`mul`/`eq`/`ne`/`lt_s`/`gt_s`/`le_s`/
  `ge_s`), see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on
  every directive kind across all three files (5/5 modules, 310/310
  assert_return, 23/23 assert_invalid). Implementing real i64x2
  arithmetic also unblocked 22 previously-`NotYetSupported`
  `assert_return` directives in the UNRELATED, already-vendored
  `simd_const.wast` (its i64x2 `v128.const` round-trip cases needed a
  real i64x2 op to observe the result through) -- a legitimate ripple
  effect, not a regression. Aggregate `assert_return` rose from
  23418/23435 to 23750/23767 (332 = 310 new + 22 newly-unblocked);
  `assert_invalid` rose by 23 (still 100.0% of gradeable directives).

