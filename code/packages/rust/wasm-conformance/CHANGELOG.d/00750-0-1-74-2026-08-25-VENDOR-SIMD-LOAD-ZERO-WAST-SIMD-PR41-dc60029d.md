## 0.1.74 — 2026-08-25 — vendor simd_load_zero.wast (SIMD PR41)

### Added

- Vendored `simd_load_zero.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering the new
  `v128.load32_zero`/`load64_zero` opcodes added in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated with full
  provenance and per-file directive-kind counts. Second bite into the
  wider `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` memory-access family PR39 deferred and PR40
  opened with `simd_load_splat.wast`.
- The file passes 100% of its directives: 2/2 `module`, 23/23
  `assert_return`, 4/4 `assert_trap`, 4/4 `assert_invalid`, 6/6
  `assert_malformed`.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 44297/44314 to 44320/44337 (+23
  pass, `fail` unchanged at 17); `assert_trap` rose from 1462/1462 to
  1466/1466 (+4, still 100%); `assert_invalid` rose from 1997/1997 to
  2001/2001 (+4, still 100%); `assert_malformed` rose from 501/501 to
  507/507 (+6, still 100%); `module` pass count rose from 1210/1211 to
  1212/1213 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

