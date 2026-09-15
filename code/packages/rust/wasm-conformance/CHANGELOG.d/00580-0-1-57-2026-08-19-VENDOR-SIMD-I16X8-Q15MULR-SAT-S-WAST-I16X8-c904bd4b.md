## 0.1.57 — 2026-08-19 — vendor simd_i16x8_q15mulr_sat_s.wast: i16x8.q15mulr_sat_s Q15 rounding saturating multiply (task #183-185)

### Added

- Vendored `simd_i16x8_q15mulr_sat_s.wast` (new file, not a re-fetch of
  an already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d` -- `i16x8.q15mulr_sat_s`
  (see `wasm-opcodes`'s own CHANGELOG entry) is the first genuinely new
  SIMD op family/semantic since the "extmul" widening-multiply arc
  completed in PR21. 100% pass on EVERY directive kind in the new file
  (1/1 module, 26/26 assert_return, 3/3 assert_invalid) on the first
  baseline regen after implementation, including the upstream corpus's
  own saturating-edge-case vectors -- confirming the Q15
  rounding-then-saturating formula (not a wrapping cast) is correct.
  Aggregate `assert_return` rose from 24266/24283 to 24292/24309 (+26
  pass, +26 gradeable, exactly this file's own `assert_return` count);
  `assert_invalid` rose from 1715/1715 to 1718/1718 (+3, still 100.0%
  of gradeable directives); `module` pass count rose from 1163 to 1164
  (+1). See `tests/fixtures/testsuite/NOTICE` for the full breakdown.

