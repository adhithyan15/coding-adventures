## 0.1.124 — 2026-09-02 — regenerated baseline: `wasm-wast-parser`'s malformed-numeric-literal fix (225 directives)

No code changes in this crate — regenerated
`tests/fixtures/testsuite-status.json` (`--write-baseline`) after
`wasm-wast-parser` 0.1.101 fixed the numeric-literal grammar gap that made
`simd_const.wast` the single largest "not yet supported" file in the
report (see that crate's own CHANGELOG entry for the root cause and fix).

`assert_malformed` moved from 1489/1940 (451 NYS) to 1714/1940 (226 NYS)
aggregate. Exactly 4 files' tallies changed, each to a clean 100%/0-NYS
across every directive category — confirmed programmatically, comparing
this file's `files` dict key-by-key against the pre-fix baseline, not just
eyeballing the printed report:

- `simd_const.wast` (this task's target): `assert_malformed` 72/181 (was
  72/181 with 109 NYS) — 109 → 0 not-yet-supported.
- `const.wast`: `assert_malformed` 56/76 → 76/76 — 20 → 0.
- `float_literals.wast`: `assert_malformed` 2/78 → 78/78 — 76 → 0.
- `int_literals.wast`: `assert_malformed` 0/20 → 20/20 — 20 → 0.

Every other file's tally (all 253 others) is byte-for-byte identical to
the pre-fix baseline — the fix lives in numeric-literal parsing shared by
plain `i32.const`/`f32.const`/`f64.const`/etc. and `v128.const` alike, so
this was checked deliberately rather than assumed: a shared-code change
like this could plausibly have shifted an already-100% SIMD file's tally
too, and it didn't.

