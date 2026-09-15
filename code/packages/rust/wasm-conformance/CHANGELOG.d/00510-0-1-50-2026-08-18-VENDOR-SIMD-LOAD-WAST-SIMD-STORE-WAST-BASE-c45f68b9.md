## 0.1.50 — 2026-08-18 — vendor simd_load.wast/simd_store.wast; baseline regen (task #162-164)

### Changed

- Baseline regen: vendored `simd_load.wast`/`simd_store.wast` --
  `v128.load`/`v128.store`, the first SIMD ops touching real linear
  memory, see `wasm-opcodes`'s own CHANGELOG entry. 100% pass on every
  gradeable directive kind across both files (9/9 modules, 27/27
  assert_return, 11/11 assert_invalid, 6/6 assert_malformed); 7
  modules and 7 assert_return directives in `simd_load.wast` grade
  `NotYetSupported` (float-lane ops and `i8x16.swizzle`, none
  implemented yet). Landing `v128.load` ALSO retroactively resolved 76
  previously-`NotYetSupported` `assert_return` directives (and 5
  previously-`NotYetSupported` modules) spread across five UNRELATED,
  already-vendored files whose own tails depended on a real
  `v128.load`: `simd_bitwise.wast` (+13), `simd_bit_shift.wast` (+24),
  `simd_i16x8_cmp.wast` (+13), `simd_i32x4_cmp.wast` (+13), and
  `simd_i8x16_cmp.wast` (+13) -- see `tests/fixtures/testsuite/NOTICE`
  for the full breakdown. Aggregate `assert_return` rose from
  23937/23954 to 24040/24057 (+103 gradeable: +27 from the two new
  files' own passing directives, +76 from the five-file ripple);
  `assert_invalid` rose by 11 and `assert_malformed` rose by 6 (both
  still 100.0% of gradeable directives).

