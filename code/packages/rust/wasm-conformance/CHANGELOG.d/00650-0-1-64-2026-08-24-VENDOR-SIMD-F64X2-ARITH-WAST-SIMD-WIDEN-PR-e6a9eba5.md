## 0.1.64 — 2026-08-24 — vendor simd_f64x2_arith.wast: SIMD widen PR31 (task #208-210)

### Added

- Vendored `simd_f64x2_arith.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  6 opcodes needed for `f64x2`'s core arithmetic family: `f64x2.neg`
  (`0xED`), `f64x2.sqrt` (`0xEF`), `f64x2.add` (`0xF0`), `f64x2.sub`
  (`0xF1`), `f64x2.mul` (`0xF2`), `f64x2.div` (`0xF3`) -- a direct
  structural mirror of PR29's `simd_f32x4_arith.wast`, implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- `simd_f64x2_arith.wast`: 1806 `assert_return` + 16 `assert_invalid` =
  1822 total gradeable directives across 3 modules, ALL 100% passing on
  the first baseline regen after implementation (0 `NotYetSupported`, 0
  failures). Aggregate `assert_return` rose from 29154/29171 to
  30960/30977 (+1806 pass, +1806 gradeable, exactly this file's own
  `assert_return` count); `assert_invalid` rose from 1780/1780 to
  1796/1796 (+16, exactly this file's own count, still 100.0% of
  gradeable directives); `module` pass count rose from 1177/1178 to
  1180/1181 (+3, one per module in this file, all passing). The
  pre-existing, unrelated baseline failures (17 `assert_return`, 1
  `module`, 1 `assert_unlinkable`, 2 `register` -- present before this
  PR and tracked separately, none touching `f64x2` or this campaign)
  are byte-for-byte unchanged by this PR, confirming zero regressions.
  No other already-vendored file's stats changed.
- Added `simd_f64x2_arith.wast` to `TESTSUITE_FILES` in
  `fetch_testsuite.py`, and regenerated
  `tests/fixtures/testsuite-status.json` via `--write-baseline`.

