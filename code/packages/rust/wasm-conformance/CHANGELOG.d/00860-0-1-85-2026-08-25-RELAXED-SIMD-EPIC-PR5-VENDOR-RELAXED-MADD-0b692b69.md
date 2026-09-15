## 0.1.85 — 2026-08-25 — Relaxed SIMD epic PR5: vendor relaxed_madd_nmadd.wast

### Added

- Vendored `relaxed_madd_nmadd.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`, byte-identical re-fetch,
  12550 bytes), the FIFTH file from the relaxed-simd proposal. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py` (lives at the testsuite
  repo root, no `PROPOSAL_FILES` entry needed) and to `NOTICE`.
- Regenerated the committed conformance baseline
  (`testsuite-status.json`): `module` +2/+2, `assert_return` +17/+17,
  ALL REAL passes, ZERO `NotYetSupported` -- no other already-vendored
  file's stats changed, zero regressions.
- No grading-logic changes needed: this file's `either` alternatives
  (fused-vs-unfused floating-point multiply-add roundings) already
  parse and grade correctly through the existing
  `Expected::Either`/`value_matches_expected` machinery PR1-PR3
  introduced and generalized, unchanged.
- Verified the corpus's own `*_cmp`/"test-consistent-nondeterminism"
  assertions (checking repeated-invocation self-consistency) pass
  identically under both native macOS/ARM64 and Linux/x86_64 (via
  Docker, `rust:latest --platform linux/amd64`) -- the chosen FUSED
  `mul_add`-based implementation is single-rounding by language
  guarantee regardless of platform FMA hardware, so both platforms
  agree bit-for-bit against the committed baseline.

