## 0.1.71 — 2026-08-24 — regenerate baseline: SIMD widen PR38 unlocks 268 stuck simd_lane.wast directives (task #229-231)

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
  after `i8x16.shuffle` (SIMD widen PR38) landed in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser` -- the single
  opcode PR37's own changelog flagged as this file's remaining gap. No
  new corpus file: `simd_lane.wast` was already vendored (PR37) at the
  same pinned commit, so this is a pure baseline flip, not a re-vendor.
- `simd_lane.wast` before -> after this PR: `module` 8/12 -> **12/12**
  (100%, up from 4 `not_yet_supported`); `assert_return` 6/274 ->
  **274/274** (100%, ALL 268 previously-`not_yet_supported` directives
  now pass -- exactly the count PR37's own changelog predicted);
  `assert_invalid` 83/83 (unchanged, already 100%); `assert_malformed`
  106/106 (unchanged, already 100%). Every directive in this file is now
  gradeable and passing -- zero `not_yet_supported` left in
  `simd_lane.wast`.
- Aggregate corpus-wide `assert_return` moved from 43597/43882 to
  43865/43882 (545 `not_yet_supported` remaining, down from 813) as a
  direct consequence of the same 268-directive flip; every other file's
  own tally is unchanged by this PR.

