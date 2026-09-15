## 0.1.79 — 2026-08-25 — vendor simd_load32_lane.wast/simd_store32_lane.wast (SIMD PR46, 2 new opcodes)

### Added

- Vendored `simd_load32_lane.wast`/`simd_store32_lane.wast` (pinned
  commit `28864811cf03bdbf880733786148feaba339582d`), the dedicated
  upstream files for `v128.load32_lane`/`v128.store32_lane` -- the
  THIRD bite into the `load{8,16,32,64}_lane`/`store{8,16,32,64}_lane`
  family, one width up from PR45's 16-bit pair. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated with full
  provenance and per-file directive-kind counts.
- Both files pass 100% on `module` (2/2) and `assert_return` (40/40
  combined) -- every byte-pattern/lane-preservation case passes.
  `simd_load32_lane.wast`'s `assert_invalid` grades 2/2 pass (type
  mismatch, out-of-range lane index `4`) with 1 `NotYetSupported` (an
  invalid `align=8` case -- same pre-existing alignment-validation gap
  `simd_align.wast`/`simd_load8_lane.wast`/`simd_load16_lane.wast`
  already surfaced, not newly introduced here). `simd_store32_lane.
  wast`'s `assert_invalid` grades 3/3 pass (its own `align=8` case is
  independently caught by the pre-existing "declared-result-type
  mismatch" check, same reason `simd_store8_lane.wast`/`simd_store16_
  lane.wast`'s equivalent cases pass).
- Regenerated `testsuite-status.json` baseline: aggregate `module` rose
  from 1264/1265 to 1266/1267 (+2 pass); `assert_return` rose from
  44560/44577 to 44600/44617 (+40 pass); `assert_invalid` rose from
  2023/2023 to 2028/2028 pass (+5) with `NotYetSupported` rising from 90
  to 91 (+1); `assert_malformed` unchanged (525/525 pass, 538
  `NotYetSupported` -- neither file has any `assert_malformed`
  directives). No other already-vendored file's stats changed.

