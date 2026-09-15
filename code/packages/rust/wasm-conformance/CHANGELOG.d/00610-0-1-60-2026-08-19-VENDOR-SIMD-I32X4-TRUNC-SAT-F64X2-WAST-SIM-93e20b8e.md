## 0.1.60 — 2026-08-19 — vendor simd_i32x4_trunc_sat_f64x2.wast: SIMD widen PR25, 2 new opcodes (task #190-192)

### Added

- Vendored `simd_i32x4_trunc_sat_f64x2.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. Unlike PR23/PR24, this PR
  DOES add 2 new opcodes: `i32x4.trunc_sat_f64x2_s_zero` (`0xFC`) and
  `i32x4.trunc_sat_f64x2_u_zero` (`0xFD`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR -- the file exercises only those two ops. This is
  the dedicated upstream file for that opcode pair's own full
  boundary-value corpus (zero/negative-zero/fractional/exact-integer/
  in-range/out-of-range/huge-finite/subnormal/inf/nan/signed-and-quiet-
  nan-payload/octal-literal cases, `_s_zero` and `_u_zero` each tested
  independently) plus dedicated `assert_invalid` type-check coverage
  (wrong-operand-type and empty-argument, for both ops). 100% pass on
  EVERY directive (1/1 module, 102/102 assert_return, 4/4
  assert_invalid). Aggregate `assert_return` rose from 24436/24453 to
  24538/24555 (+102 pass, +102 gradeable, exactly this file's own
  `assert_return` count); `assert_invalid` rose from 1724/1724 to
  1728/1728 (+4, still 100.0% of gradeable directives); `module` pass
  count rose from 1169 to 1170 (+1). No other already-vendored file's
  stats changed. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.

