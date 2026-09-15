## 0.1.65 — 2026-08-24 — vendor simd_f64x2_cmp.wast: SIMD widen PR32 (task #211-213)

### Added

- Vendored `simd_f64x2_cmp.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  6 opcodes needed for `f64x2`'s comparison family: `f64x2.eq` (`0x47`),
  `f64x2.ne` (`0x48`), `f64x2.lt` (`0x49`), `f64x2.gt` (`0x4A`),
  `f64x2.le` (`0x4B`), `f64x2.ge` (`0x4C`) -- a direct structural mirror
  of PR30's `simd_f32x4_cmp.wast`, implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR. This is now the single BIGGEST directive-count
  win in this campaign so far, surpassing PR30's `simd_f32x4_cmp.wast`
  (2605 total).
- `simd_f64x2_cmp.wast`: 2659 `assert_return` + 18 `assert_invalid` + 6
  `assert_malformed` = 2683 total gradeable directives across 2 modules,
  ALL 100% passing on the first baseline regen after implementation (0
  `NotYetSupported`, 0 failures). Aggregate `assert_return` rose from
  30960/30977 to 33619/33636 (+2659 pass, +2659 gradeable, exactly this
  file's own `assert_return` count); `assert_invalid` rose from
  1796/1796 to 1814/1814 (+18, exactly this file's own count, still
  100.0% of gradeable directives); `assert_malformed` rose from
  313/313 to 319/319 (+6, exactly this file's own count, still 100.0%
  of gradeable directives); `module` pass count rose from 1180/1181 to
  1182/1183 (+2, one per module in this file, both passing). The
  pre-existing, unrelated baseline failures (17 `assert_return`, 1
  `module`, 1 `assert_unlinkable`, 2 `register` -- present before this
  PR and tracked separately, none touching `f64x2` or this campaign)
  are byte-for-byte unchanged by this PR, confirming zero regressions.
  No other already-vendored file's stats changed.
- Added `simd_f64x2_cmp.wast` to `TESTSUITE_FILES` in
  `fetch_testsuite.py`, at the end of the SIMD block, and updated the
  accompanying `NOTICE` file with the same real vendored/pass counts
  above.

