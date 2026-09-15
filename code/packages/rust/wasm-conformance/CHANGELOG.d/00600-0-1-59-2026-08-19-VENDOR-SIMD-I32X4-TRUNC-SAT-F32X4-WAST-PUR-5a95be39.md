## 0.1.59 — 2026-08-19 — vendor simd_i32x4_trunc_sat_f32x4.wast: pure vendoring, zero new opcodes (task #188-189)

### Added

- Vendored `simd_i32x4_trunc_sat_f32x4.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. Like PR23, this PR adds
  ZERO new opcodes to `wasm-opcodes` -- the file was confirmed, before
  vendoring, to use only `i32x4.trunc_sat_f32x4_s`/
  `i32x4.trunc_sat_f32x4_u`, both already fully implemented since SIMD
  widen PR20 (task #177-179). This is the dedicated upstream file for
  that opcode pair's own full boundary-value corpus (zero/negative-zero/
  fractional/exact-integer/in-range/out-of-range/huge-finite/subnormal/
  inf/nan/signed-and-quiet-nan-payload/octal-literal cases, `_s` and `_u`
  each tested independently) plus dedicated `assert_invalid` type-check
  coverage (wrong-operand-type and empty-argument, for both ops) -- a
  much larger, more direct slice than the single directive reachable
  through `simd_load.wast`'s own `v128.load`-wrapped coverage. 100% pass
  on EVERY directive (1/1 module, 102/102 assert_return, 4/4
  assert_invalid). Aggregate `assert_return` rose from 24334/24351 to
  24436/24453 (+102 pass, +102 gradeable, exactly this file's own
  `assert_return` count); `assert_invalid` rose from 1720/1720 to
  1724/1724 (+4, still 100.0% of gradeable directives); `module` pass
  count rose from 1168 to 1169 (+1). No source changes to
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`/`wasm-opcodes` --
  only this crate's fixtures/baseline/version/changelog are touched, same
  as PR23. See `tests/fixtures/testsuite/NOTICE` for the full breakdown.

