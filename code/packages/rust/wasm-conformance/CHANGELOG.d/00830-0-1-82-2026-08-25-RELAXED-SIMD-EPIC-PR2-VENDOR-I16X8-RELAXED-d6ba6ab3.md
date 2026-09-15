## 0.1.82 — 2026-08-25 — Relaxed SIMD epic PR2: vendor i16x8_relaxed_q15mulr_s.wast

### Added

- Vendored `i16x8_relaxed_q15mulr_s.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`, byte-identical re-fetch,
  1264 bytes), the SECOND file from the relaxed-simd proposal. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py` (lives at the testsuite
  repo root, no `PROPOSAL_FILES` entry needed); NOTICE updated with full
  provenance. Grades 1/1 `module`, 2/2 `assert_return` (all real
  passes), reusing PR1's `Expected::Either` grading unchanged -- no new
  harness work needed. See `code/specs/
  W19-wasm-relaxed-simd-first-slice.md`.
- Baseline regenerated: `testsuite-status.json` now includes
  `i16x8_relaxed_q15mulr_s.wast`. Aggregate `module` 1269/1270 ->
  1270/1271 (+1); `assert_return` 44629/44646 -> 44631/44648 (+2). No
  other already-vendored file's stats changed -- zero regressions.

