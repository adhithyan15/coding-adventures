## 0.1.83 — 2026-08-25 — Relaxed SIMD epic PR3: vendor relaxed_min_max.wast, N-ary `either` grading test

### Added

- Vendored `relaxed_min_max.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`, byte-identical re-fetch,
  8577 bytes), the THIRD file from the relaxed-simd proposal. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py` (lives at the testsuite
  repo root, no `PROPOSAL_FILES` entry needed); NOTICE updated with full
  provenance. Grades 1/1 `module`, 24/24 `assert_return` (all real
  passes). This is the FIRST relaxed-simd file whose `either` groups
  carry FOUR alternatives, not two -- `value_matches_expected`'s
  existing recursive `||` grading needed NO code changes to support the
  deeper nested `Either` chain `wasm-wast-parser`'s generalized `either`
  parsing arm now produces (see that crate's own changelog). See
  `code/specs/W19-wasm-relaxed-simd-first-slice.md`.
- New test: `either_nested_four_way_matches_any_of_the_four_
  alternatives`, confirming grading accepts a match on the 3rd/4th
  alternative too, not just the first two the original binary-only
  `either` arm would have exposed.
- Regenerated `tests/fixtures/testsuite-status.json` baseline
  (`--write-baseline`): aggregate `module` rose from 1270/1271 to
  1271/1272 (+1 pass, +1 gradeable); `assert_return` rose from
  44631/44648 to 44655/44672 (+24 pass, +24 gradeable). No other
  already-vendored file's stats changed -- zero regressions.

