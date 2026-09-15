## 0.1.49 — 2026-08-18 — vendor simd_bit_shift.wast; baseline regen (task #159-161)

### Changed

- Baseline regen: vendored `simd_bit_shift.wast` -- `ixNxM.shl`/
  `shr_s`/`shr_u` across all 4 lane widths, the first mixed-type
  binary SIMD op family (`v128` + scalar `i32`, not two `v128`s), see
  `wasm-opcodes`'s own CHANGELOG entry. 100% pass on every gradeable
  directive kind (1/1 modules, 187/187 assert_return, 24/24
  assert_invalid, 15/15 assert_malformed; 24 assert_return directives
  grade `NotYetSupported`). Aggregate `assert_return` rose from
  23750/23767 to 23937/23954 (+187, exactly matching the new file's
  own count, no ripple into unrelated files this time); `assert_invalid`
  rose by 24 and `assert_malformed` rose by 15 (both still 100.0% of
  gradeable directives).

