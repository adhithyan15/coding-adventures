## 0.1.81 — 2026-08-25 — Relaxed SIMD epic PR1: vendor i8x16_relaxed_swizzle.wast + the `either` grading combinator

### Added

- Vendored `i8x16_relaxed_swizzle.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`), the FIRST file from the
  relaxed-simd proposal -- a SEPARATE proposal from base SIMD (PR1-PR47)
  with its own encoding table. Confirmed live via the GitHub API tree
  listing that relaxed-simd `.wast` files genuinely exist at this
  repo's existing pinned SHA. Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py` (lives at the testsuite repo root, no
  `PROPOSAL_FILES` entry needed); NOTICE updated with full provenance.
  Grades 1/1 `module`, 5/5 `assert_return` (all real passes). See
  `code/specs/W19-wasm-relaxed-simd-first-slice.md`.
- `value_matches_expected` gained an `Expected::Either(a, b)` arm --
  tries `a` then `b`, generic over any nested `Expected` shape. This
  grades the upstream corpus's new `(either A B)` assert_return
  combinator (relaxed-simd ops are spec-sanctioned as implementation-
  defined for certain inputs), reusable unchanged by every future
  relaxed-simd PR in this epic.
- Baseline regenerated: `testsuite-status.json` now includes
  `i8x16_relaxed_swizzle.wast`. Aggregate `module` 1268/1269 ->
  1269/1270 (+1); `assert_return` 44624/44641 -> 44629/44646 (+5). No
  other file's stats changed.
- New tests: `either_accepts_a_value_matching_the_first_alternative`,
  `either_accepts_a_value_matching_the_second_alternative`,
  `either_rejects_a_value_matching_neither_alternative`,
  `either_v128_matches_the_real_relaxed_swizzle_out_of_range_shape`.

