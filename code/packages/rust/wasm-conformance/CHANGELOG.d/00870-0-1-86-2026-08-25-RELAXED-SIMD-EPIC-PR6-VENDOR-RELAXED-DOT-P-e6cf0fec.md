## 0.1.86 — 2026-08-25 — Relaxed SIMD epic PR6: vendor relaxed_dot_product.wast

### Added

- Vendored `relaxed_dot_product.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`, byte-identical re-fetch,
  5935 bytes), the SIXTH and LAST substantive file from the relaxed-simd
  proposal. Added to `TESTSUITE_FILES` in `fetch_testsuite.py` (lives at
  the testsuite repo root, no `PROPOSAL_FILES` entry needed) and to
  `NOTICE`.
- Regenerated the committed conformance baseline
  (`testsuite-status.json`): `module` +1/+1, `assert_return` +10/+10,
  ALL REAL passes, ZERO `NotYetSupported` -- no other already-vendored
  file's stats changed, zero regressions.
- No grading-logic changes needed: this file's `either` alternatives
  (the different signed/unsigned readings a host's native dot-product
  instruction might apply) already parse and grade correctly through
  the existing `Expected::Either`/`value_matches_expected` machinery
  PR1-PR3 introduced and generalized, unchanged. This closes the entire
  19-opcode relaxed-simd range's substantive vendoring scope: only
  `i32x4_relaxed_trunc.wast` (flagged since PR1 as having ZERO real
  `assert_return` directives at this pinned SHA) remains unvendored.

