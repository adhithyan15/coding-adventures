## 0.1.125 — 2026-09-02 — regenerated baseline: `array.fill`/`array.copy` -- W38 slices 0-2 (1 file, 25 directives)

No code changes in this crate — regenerated `tests/fixtures/testsuite-
status.json` (`--write-baseline`) after `wasm-wast-parser`/`wasm-
execution`/`wasm-validator` implemented `array.fill`/`array.copy`
(`code/specs/W38-wasm-gc-array-bulk-ops.md` slices 0-2). Diffed
programmatically against the pre-slice baseline across all 257 files —
exactly ONE file's tallies changed:

- `array_fill.wast`: 3/30 → 28/30 pass, 27 → 2 not-yet-supported. The
  remaining 2 (`assert_invalid`) are a pre-existing, unrelated
  "no instruction-level type-checker" scope gap (`wasm-validator`'s own
  CHANGELOG entry), not a regression.

`array_copy.wast` was re-verified directly and, honestly, its AGGREGATE
tallies do NOT change (4/35 pass either way) despite `array.copy` now
being for-real implemented and independently unit-tested: its 4
`assert_invalid` cases already passed trivially before this release (a
whole-module parse failure vacuously satisfies `assert_invalid` — the
same phenomenon `array_fill.wast`'s own pre-slice 3/30 baseline shows),
and now pass for the real reason instead; its remaining 31 directives
depend on `array.new_data` (confirmed by direct read of the file's own
"array_copy_overlap_test-1"/"-2" functions), which this spec's own slice
2 deliberately does not implement — a real corpus coupling this spec's
own summary table ("31 array.copy unimplemented, 100%") did not surface,
found by this release's own re-verification pass per this campaign's
standing "verify every claim against the corpus directly" discipline.
Expected to resolve once a later W38 slice implements `array.new_data`.

Zero regressions: every other file's tallies are byte-for-byte identical
to the pre-slice baseline, `fail`/`trap` counts are `0` everywhere both
before and after, and `parse_failures` is empty both before and after.

