## 0.1.84 — 2026-08-25 — Relaxed SIMD epic PR4: vendor relaxed_laneselect.wast

### Added

- Vendored `relaxed_laneselect.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`, byte-identical re-fetch,
  6517 bytes), the FOURTH file from the relaxed-simd proposal. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py` (lives at the testsuite
  repo root, no `PROPOSAL_FILES` entry needed) and to `NOTICE`.
- Regenerated the committed conformance baseline
  (`testsuite-status.json`): `module` +1/+1, `assert_return` +11/+11,
  ALL REAL passes, ZERO `NotYetSupported` -- no other already-vendored
  file's stats changed, zero regressions.
- No grading-logic changes needed: the existing recursive `||`-based
  `value_matches_expected` (including the N-ary `Expected::Either`
  chain PR3 generalized) already handles this file's THREE-alternative
  "pblendvb special case" `either` group unchanged.

