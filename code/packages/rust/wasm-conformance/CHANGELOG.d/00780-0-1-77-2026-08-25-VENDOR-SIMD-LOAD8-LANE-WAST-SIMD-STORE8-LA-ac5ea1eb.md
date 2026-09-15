## 0.1.77 — 2026-08-25 — vendor simd_load8_lane.wast/simd_store8_lane.wast (SIMD PR44, 2 new opcodes)

### Added

- Vendored `simd_load8_lane.wast`/`simd_store8_lane.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`), the dedicated upstream
  files for `v128.load8_lane`/`v128.store8_lane` -- the FIRST bite into
  the `load{8,16,32,64}_lane`/`store{8,16,32,64}_lane` family every PR
  since PR39 has deferred (a genuinely new instruction shape: an
  existing `v128` operand PLUS a lane-index immediate PLUS a memarg, all
  in one instruction). Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`; NOTICE updated with full provenance and per-file
  directive-kind counts.
- Both files pass 100% on `module` (2/2) and `assert_return` (96/96
  combined) -- every byte-pattern/lane-preservation case passes.
  `simd_load8_lane.wast`'s `assert_invalid` grades 2/2 pass (type
  mismatch, out-of-range lane index) with 1 `NotYetSupported` (an
  invalid `align=2` case -- this repo's SIMD memarg decode path still
  never checks an over-large `align=` against natural alignment, the
  SAME pre-existing gap PR43's `simd_align.wast` vendoring surfaced and
  documented, not newly introduced here).
  `simd_store8_lane.wast`'s `assert_invalid` grades 3/3 pass (its own
  `align=2` case is independently caught by the pre-existing
  "declared-result-type mismatch" check, since that test's function
  wrongly declares `(result v128)` on a `store8_lane`-only body).
- Regenerated `testsuite-status.json` baseline: aggregate `module` rose
  from 1260/1261 to 1262/1263 (+2 pass); `assert_return` rose from
  44400/44417 to 44496/44513 (+96 pass); `assert_invalid` rose from
  2013/2013 to 2018/2018 pass (+5) with `NotYetSupported` rising from 88
  to 89 (+1); `assert_malformed` unchanged (525/525 pass, 538
  `NotYetSupported` -- neither file has any `assert_malformed`
  directives). No other already-vendored file's stats changed.

