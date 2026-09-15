## 0.1.75 — 2026-08-25 — vendor simd_load_extend.wast (SIMD PR42)

### Added

- Vendored `simd_load_extend.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering the new
  `v128.load8x8_s`/`_u`, `v128.load16x4_s`/`_u`, `v128.load32x2_s`/`_u`
  opcodes added in `wasm-opcodes`/`wasm-execution`/`wasm-validator`/
  `wasm-wast-parser`. Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`; NOTICE updated with full provenance and per-file
  directive-kind counts (also backfilled a missing PR41 header entry
  found while updating this same list). Third and FINAL bite into the
  wider `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` memory-access family PR39 deferred and
  PR40/PR41 opened.
- The file passes 100% of its directives: 2/2 `module`, 72/72
  `assert_return`, 12/12 `assert_trap`, 12/12 `assert_invalid`, 6/6
  `assert_malformed`.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 44320/44337 to 44392/44409 (+72
  pass, `fail` unchanged at 17); `assert_trap` rose from 1466/1466 to
  1478/1478 (+12, still 100%); `assert_invalid` rose from 2001/2001 to
  2013/2013 (+12, still 100%); `assert_malformed` rose from 507/507 to
  513/513 (+6, still 100%); `module` pass count rose from 1212/1213 to
  1214/1215 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

