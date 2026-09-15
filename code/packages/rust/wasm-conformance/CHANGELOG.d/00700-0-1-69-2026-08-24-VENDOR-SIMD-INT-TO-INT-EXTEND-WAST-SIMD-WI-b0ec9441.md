## 0.1.69 — 2026-08-24 — vendor simd_int_to_int_extend.wast: SIMD widen PR36 (task #223-225)

### Added

- Vendored `simd_int_to_int_extend.wast` (a new file) at the existing
  pinned commit `28864811cf03bdbf880733786148feaba339582d`. This PR
  implements the 4 opcodes needed to complete the third and FINAL rung
  of the integer "extend" family: `i64x2.extend_low_i32x4_s` (`0xC7`),
  `i64x2.extend_high_i32x4_s` (`0xC8`), `i64x2.extend_low_i32x4_u`
  (`0xC9`), `i64x2.extend_high_i32x4_u` (`0xCA`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR. Unlike `simd_conversions.wast`'s PR26/27/28 split
  (three DIFFERENT opcode families bundled together), this single file
  bundles all THREE RUNGS of the SAME "extend" family in one module --
  `i16x8.extend_low/high_i8x16_s/_u` and `i32x4.extend_low/
  high_i16x8_s/_u` (both landed opcode-only in PR26, with no dedicated
  corpus file until now) alongside this PR's new `i64x2` rung -- so
  landing it integration-tests all 12 opcodes across all three rungs at
  once, not just this PR's new 4.
- 253 real directives: 1 module, 228 `assert_return`, 24
  `assert_invalid`, 0 `assert_malformed`. ALL 100% passing on the first
  baseline regen after implementation (1/1 module, 228/228
  `assert_return`, 24/24 `assert_invalid`) -- including every one of the
  file's i16x8/i32x4 rung directives (PR26's own opcodes, integration-
  tested against a real upstream corpus file for the first time)
  alongside this PR's new i64x2 rung. Aggregate `assert_return` rose
  from 43320/43337 to 43548/43565 (+228 pass, `fail` unchanged at 17);
  `assert_invalid` rose from 1866/1866 to 1890/1890 (+24, still 100.0%
  of gradeable directives); `module` pass count rose from 1192/1193 to
  1193/1194 (+1, passing); `assert_malformed` unchanged at 359/359 (this
  file has none). All deltas are EXACTLY this file's own directive
  counts -- the pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions. See
  `tests/fixtures/testsuite/NOTICE` for the full accounting.

