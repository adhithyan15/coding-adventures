## 0.1.21 — 2026-08-15 — baseline regen: correctly-rounded hex floats (task #80)

### Changed

- Baseline regen following `wasm-wast-parser` 0.1.14 (task #80 -- the
  hex-float literal parser now rounds correctly instead of double-
  rounding). `const.wast`'s `assert_return` tally moves from 260/300 to
  300/300 (fully clean) and `simd_const.wast` improves from 209/240 to
  235/240. Verified via a full before/after diff of every vendored
  file's per-directive-kind tally: zero regressions anywhere in the
  61-file corpus, confirming the rounding fix is a strict improvement.

