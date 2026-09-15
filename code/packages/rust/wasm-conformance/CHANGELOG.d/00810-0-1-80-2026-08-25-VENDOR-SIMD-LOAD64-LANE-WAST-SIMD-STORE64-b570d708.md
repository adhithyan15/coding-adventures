## 0.1.80 — 2026-08-25 — vendor simd_load64_lane.wast/simd_store64_lane.wast (SIMD PR47, 2 new opcodes)

### Added

- Vendored `simd_load64_lane.wast`/`simd_store64_lane.wast` (pinned
  commit `28864811cf03bdbf880733786148feaba339582d`), the dedicated
  upstream files for `v128.load64_lane`/`v128.store64_lane` -- the
  FOURTH and FINAL bite into the `load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` family, one width up from PR46's 32-bit pair.
  Added to `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated
  with full provenance and per-file directive-kind counts. This closes
  the entire lane-load/store family (all 8 opcodes across 8 vendored
  files, PR44-47) and, with it, the larger load-extend/splat/zero/lane
  epic started in PR40.
- Both files pass 100% on `module` (2/2) and `assert_return` (24/24
  combined) -- every byte-pattern/lane-preservation case passes.
  `simd_load64_lane.wast`'s `assert_invalid` grades 2/2 pass (type
  mismatch, out-of-range lane index `2`) with 1 `NotYetSupported` (an
  invalid `align=16` case -- same pre-existing alignment-validation gap
  `simd_align.wast`/`simd_load8_lane.wast`/`simd_load16_lane.wast`/
  `simd_load32_lane.wast` already surfaced, not newly introduced here).
  `simd_store64_lane.wast`'s `assert_invalid` grades 3/3 pass (its own
  `align=16` case is independently caught by the pre-existing
  "declared-result-type mismatch" check, same reason `simd_store8_
  lane.wast`/`simd_store16_lane.wast`/`simd_store32_lane.wast`'s
  equivalent cases pass).
- Regenerated `testsuite-status.json` baseline: aggregate `module` rose
  from 1266/1267 to 1268/1269 (+2 pass); `assert_return` rose from
  44600/44617 to 44624/44641 (+24 pass); `assert_invalid` rose from
  2028/2028 to 2033/2033 pass (+5) with `NotYetSupported` rising from 91
  to 92 (+1); `assert_malformed` unchanged (525/525 pass, 538
  `NotYetSupported` -- neither file has any `assert_malformed`
  directives). No other already-vendored file's stats changed.

