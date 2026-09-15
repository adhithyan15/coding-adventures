## 0.1.78 — 2026-08-25 — vendor simd_load16_lane.wast/simd_store16_lane.wast (SIMD PR45, 2 new opcodes)

### Added

- Vendored `simd_load16_lane.wast`/`simd_store16_lane.wast` (pinned
  commit `28864811cf03bdbf880733786148feaba339582d`), the dedicated
  upstream files for `v128.load16_lane`/`v128.store16_lane` -- the
  SECOND bite into the `load{8,16,32,64}_lane`/`store{8,16,32,64}_lane`
  family, one width up from PR44's 8-bit pair. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated with full
  provenance and per-file directive-kind counts.
- Both files pass 100% on `module` (2/2) and `assert_return` (64/64
  combined) -- every byte-pattern/lane-preservation case passes.
  `simd_load16_lane.wast`'s `assert_invalid` grades 2/2 pass (type
  mismatch, out-of-range lane index `8`) with 1 `NotYetSupported` (an
  invalid `align=4` case -- same pre-existing alignment-validation gap
  `simd_align.wast`/`simd_load8_lane.wast` already surfaced, not newly
  introduced here). `simd_store16_lane.wast`'s `assert_invalid` grades
  3/3 pass (its own `align=4` case is independently caught by the
  pre-existing "declared-result-type mismatch" check, same reason
  `simd_store8_lane.wast`'s equivalent case passes).
- Regenerated `testsuite-status.json` baseline: aggregate `module` rose
  from 1262/1263 to 1264/1265 (+2 pass); `assert_return` rose from
  44496/44513 to 44560/44577 (+64 pass); `assert_invalid` rose from
  2018/2018 to 2023/2023 pass (+5) with `NotYetSupported` rising from 89
  to 90 (+1); `assert_malformed` unchanged (525/525 pass, 538
  `NotYetSupported` -- neither file has any `assert_malformed`
  directives). No other already-vendored file's stats changed.

